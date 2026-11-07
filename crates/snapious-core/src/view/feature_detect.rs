//! Automatic feature detection: known sequence motifs and open reading frames (ORFs).

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::Result;
use crate::model::feature::{Feature, Interval, Location, Strand};

/// A known sequence that is searched for in documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureTemplate {
    pub name: String,
    pub feature_type: String,
    /// Bases in uppercase (`A`, `C`, `G`, `T`).
    pub bases: String,
}

/// Built-in templates from `standard-features.json` (name, type, sequence), in file order:
/// promoters (T7, T3, SP6, lac), primer sites (M13), tags (6xHis, FLAG, HA), lac operator, T7 terminator, loxP, attB1/attB2
/// and, from the pUC19 entry L09137, AmpR (861 bp), ori (589 bp) and lacZα (324 bp).
pub fn common_templates() -> Vec<FeatureTemplate> {
    #[derive(serde::Deserialize)]
    struct Entry {
        name: String,
        #[serde(rename = "type")]
        kind: String,
        sequence: String,
    }
    let entries: Vec<Entry> =
        serde_json::from_str(include_str!("standard-features.json")).unwrap_or_default();
    entries
        .into_iter()
        .map(|e| FeatureTemplate {
            name: e.name,
            feature_type: e.kind,
            bases: e.sequence,
        })
        .collect()
}

/// Uppercase, `U` becomes `T`.
fn normalize(bases: &str) -> String {
    bases.to_ascii_uppercase().replace('U', "T")
}

fn make_feature(
    kind: &str,
    name: &str,
    start: usize,
    end: usize,
    strand: Strand,
    qualifiers: Vec<(String, Vec<String>)>,
) -> Result<Feature> {
    let location = Location::new(vec![Interval::new(start, end)?], strand, false, false)?;
    Ok(Feature::new(kind, name, location, qualifiers))
}

/// Searches each template exactly (case-insensitive, `U` counts as `T`) on both strands.
/// - Hits on the forward strand: `Strand::Forward`; hits of the template's reverse complement:
///   `Strand::Reverse`, each with the interval on the forward strand.
/// - If the template is its own reverse complement (palindrome), each location is reported only once
///   (as `Forward`).
/// - Overlapping hits are all reported. Templates with empty `bases` are skipped.
/// - Feature: `feature_type` and `name` of the template, qualifiers empty.
/// - Result sorted by start, then end, then name, then Forward before Reverse.
///
/// There is no circular search across the origin.
pub fn detect_features(bases: &str, templates: &[FeatureTemplate]) -> Result<Vec<Feature>> {
    let text = normalize(bases);
    let mut hits: Vec<(usize, usize, String, bool, Feature)> = Vec::new();
    for template in templates {
        let needle = normalize(&template.bases);
        if needle.is_empty() || needle.len() > text.len() {
            continue;
        }
        let rc = iupac_reverse_complement(&needle);
        let palindrome = rc == needle;
        for start in 0..=(text.len() - needle.len()) {
            let end = start + needle.len();
            let Some(window) = text.get(start..end) else {
                continue;
            };
            if window == needle {
                let f = make_feature(
                    &template.feature_type,
                    &template.name,
                    start,
                    end,
                    Strand::Forward,
                    Vec::new(),
                )?;
                hits.push((start, end, template.name.clone(), false, f));
            }
            if !palindrome && window == rc {
                let f = make_feature(
                    &template.feature_type,
                    &template.name,
                    start,
                    end,
                    Strand::Reverse,
                    Vec::new(),
                )?;
                hits.push((start, end, template.name.clone(), true, f));
            }
        }
    }
    hits.sort_by(|a, b| (a.0, a.1, &a.2, a.3).cmp(&(b.0, b.1, &b.2, b.3)));
    Ok(hits.into_iter().map(|h| h.4).collect())
}

/// ORFs on one strand: `(start, end)` in coordinates of that strand.
fn orfs_on(text: &str, min_codons: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for frame in 0..3 {
        let mut first_start: Option<usize> = None;
        let mut i = frame;
        while i + 3 <= bytes.len() {
            let codon = bytes.get(i..i + 3).unwrap_or(&[]);
            if codon == b"ATG" && first_start.is_none() {
                first_start = Some(i);
            } else if codon == b"TAA" || codon == b"TAG" || codon == b"TGA" {
                if let Some(s) = first_start {
                    if (i - s) / 3 >= min_codons {
                        found.push((s, i + 3));
                    }
                }
                first_start = None;
            }
            i += 3;
        }
    }
    found
}

/// Finds open reading frames on both strands as `CDS` features.
/// - An ORF starts with `ATG` and ends with the first in-frame stop codon (`TAA`, `TAG`, `TGA`); the stop codon
///   is part of it. Without a stop codon up to the end of the sequence there is no ORF.
/// - Per stop codon only the longest ORF counts (the first `ATG` before it in the same frame with no stop in between).
/// - Minimum length: `min_codons` codons WITHOUT the stop codon (`min_codons` 0 is treated as 1).
/// - Reverse strand: search on the reverse complement and convert the interval back; `Strand::Reverse`,
///   interval in coordinates of the forward sequence.
/// - Name: `ORF {start+1}..{end} (+)` or `(-)` with the 1-based bounds of the interval on the forward strand.
/// - Qualifier: `("note", ["{n} amino acids"])` with n = codons without the stop codon.
/// - Result sorted by start, then end, then Forward before Reverse. `U` counts as `T`, case is ignored.
pub fn find_orf_features(bases: &str, min_codons: usize) -> Result<Vec<Feature>> {
    let min_codons = min_codons.max(1);
    let text = normalize(bases);
    let len = text.len();
    let rc = iupac_reverse_complement(&text);
    let mut all: Vec<(usize, usize, bool, Feature)> = Vec::new();
    for (reverse, strand_text) in [(false, &text), (true, &rc)] {
        for (s, e) in orfs_on(strand_text, min_codons) {
            let (start, end) = if reverse { (len - e, len - s) } else { (s, e) };
            let amino = (e - s) / 3 - 1;
            let sign = if reverse { "-" } else { "+" };
            let name = format!("ORF {}..{} ({})", start + 1, end, sign);
            let strand = if reverse {
                Strand::Reverse
            } else {
                Strand::Forward
            };
            let note = vec![("note".to_string(), vec![format!("{} amino acids", amino)])];
            all.push((
                start,
                end,
                reverse,
                make_feature("CDS", &name, start, end, strand, note)?,
            ));
        }
    }
    all.sort_by_key(|a| (a.0, a.1, a.2));
    Ok(all.into_iter().map(|a| a.3).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(name: &str, bases: &str) -> FeatureTemplate {
        FeatureTemplate {
            name: name.to_string(),
            feature_type: "misc_feature".to_string(),
            bases: bases.to_string(),
        }
    }

    fn span(f: &Feature) -> (usize, usize, Strand) {
        (
            f.location().start(),
            f.location().end(),
            f.location().strand(),
        )
    }

    #[test]
    fn eingebaute_vorlagen() {
        let v = common_templates();
        assert_eq!(v.len(), 17);
        assert_eq!(v[0].name, "T7 promoter");
        assert_eq!(v[0].feature_type, "promoter");
        assert_eq!(v[0].bases, "TAATACGACTCACTATAG");
        assert_eq!(v[7].name, "lac operator");
        assert_eq!(v[7].feature_type, "protein_bind");
        assert!(v
            .iter()
            .all(|x| x.bases.chars().all(|c| "ACGT".contains(c))));
    }

    #[test]
    fn vorwaerts_treffer() {
        let seq = format!("GGG{}CCC", "TAATACGACTCACTATAG");
        let f = detect_features(&seq, &common_templates()).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name(), "T7 promoter");
        assert_eq!(f[0].feature_type(), "promoter");
        assert_eq!(span(&f[0]), (3, 21, Strand::Forward));
        assert!(f[0].qualifiers().is_empty());
    }

    #[test]
    fn reverse_treffer_und_schreibweise() {
        let rc = iupac_reverse_complement("TAATACGACTCACTATAG");
        let seq = format!("aaa{}ccc", rc.to_ascii_lowercase());
        let f = detect_features(&seq, &[t("x", "TAATACGACTCACTATAG")]).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(span(&f[0]), (3, 21, Strand::Reverse));
    }

    #[test]
    fn palindrom_nur_einmal() {
        let f = detect_features("AAGAATTCAA", &[t("EcoRI-Stelle", "GAATTC")]).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(span(&f[0]), (2, 8, Strand::Forward));
    }

    #[test]
    fn ueberlappend_und_sortiert() {
        let f = detect_features("AAAAA", &[t("b", "AA")]).unwrap();
        let starts: Vec<usize> = f.iter().map(|x| x.location().start()).collect();
        assert_eq!(starts, vec![0, 1, 2, 3]);
        let g = detect_features("ACGTACGT", &[t("zz", "ACG"), t("aa", "ACG")]).unwrap();
        let names: Vec<&str> = g
            .iter()
            .filter(|x| x.location().strand() == Strand::Forward)
            .map(|x| x.name())
            .collect();
        assert_eq!(names, vec!["aa", "zz", "aa", "zz"]);
    }

    #[test]
    fn leere_faelle() {
        assert!(detect_features("", &common_templates()).unwrap().is_empty());
        assert!(detect_features("ACGT", &[]).unwrap().is_empty());
        assert!(detect_features("ACGT", &[t("leer", "")])
            .unwrap()
            .is_empty());
        assert!(detect_features("ACGT", &common_templates())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn orf_vorwaerts() {
        let f = find_orf_features("ATGAAATAG", 2).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].feature_type(), "CDS");
        assert_eq!(f[0].name(), "ORF 1..9 (+)");
        assert_eq!(span(&f[0]), (0, 9, Strand::Forward));
        assert_eq!(f[0].qualifier("note"), Some("2 amino acids"));
        assert!(find_orf_features("ATGAAATAG", 3).unwrap().is_empty());
    }

    #[test]
    fn orf_rueckwaerts() {
        // Reverse complement of ATGAAATAG
        let f = find_orf_features("CTATTTCAT", 2).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name(), "ORF 1..9 (-)");
        assert_eq!(span(&f[0]), (0, 9, Strand::Reverse));
    }

    #[test]
    fn orf_mit_versatz_im_reverse_strang() {
        let f = find_orf_features("GGCTATTTCATCC", 2).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(span(&f[0]), (2, 11, Strand::Reverse));
        assert_eq!(f[0].name(), "ORF 3..11 (-)");
    }

    #[test]
    fn orf_nur_laengster_pro_stopcodon() {
        let f = find_orf_features("ATGATGAAATAG", 1).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(span(&f[0]), (0, 12, Strand::Forward));
        assert_eq!(f[0].qualifier("note"), Some("3 amino acids"));
    }

    #[test]
    fn orf_ohne_stopcodon_oder_ohne_start() {
        assert!(find_orf_features("ATGAAAAAA", 1).unwrap().is_empty());
        assert!(find_orf_features("AAATAG", 1).unwrap().is_empty());
        assert!(find_orf_features("", 1).unwrap().is_empty());
    }

    #[test]
    fn orf_min_null_wie_eins_und_u_wie_t() {
        let f = find_orf_features("augtaa", 0).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(span(&f[0]), (0, 6, Strand::Forward));
        assert_eq!(f[0].qualifier("note"), Some("1 amino acids"));
    }

    #[test]
    fn orf_sortierung() {
        // forward ATGAAATAG at 0..9, reverse (reverse complement of ATGCCCTAG = CTAGGGCAT) at 9..18
        let f = find_orf_features("ATGAAATAGCTAGGGCAT", 2).unwrap();
        let s: Vec<_> = f.iter().map(span).collect();
        assert_eq!(s, vec![(0, 9, Strand::Forward), (9, 18, Strand::Reverse)]);
    }
}

#[cfg(test)]
mod perf_tests {
    use super::*;

    #[test]
    fn eine_megabase_in_vernuenftiger_zeit() {
        let mut rng = crate::util::Rng::new(7);
        let seq: String = (0..1_000_000)
            .map(|_| ['A', 'C', 'G', 'T'][rng.below(4)])
            .collect();
        let t = std::time::Instant::now();
        let _ = detect_features(&seq, &common_templates()).unwrap();
        assert!(t.elapsed().as_secs() < 20, "{:?}", t.elapsed());
    }
}
