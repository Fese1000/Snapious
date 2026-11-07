//! Translation of sequence ranges and features into amino acids (for display below the sequence).

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::model::feature::{Feature, Strand};
use crate::model::sequence::amino_acid_for;

/// An amino acid with the position of its first base letter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AminoAcidCell {
    /// Position of the first base of the codon in the document (0-based; for features on the opposite strand, the
    /// position of the base that comes first in the feature's reading direction, i.e. the highest coordinate).
    pub base_position: usize,
    /// One-letter code; stop = `*`, unknown = `X`.
    pub amino_acid: char,
    /// The codon is `ATG`.
    pub is_start: bool,
    /// The codon is a stop codon (`TAA`, `TAG`, `TGA`).
    pub is_stop: bool,
}

/// Translation in reading frame `frame` (0, 1, 2; larger values count as `frame % 3`) on the forward strand.
/// Only codons lying completely within `[from, to)` are translated (`to` is clamped to the length of `bases`)
/// and whose first position `p` satisfies `p % 3 == frame % 3`. `U` counts as `T`,
/// case-insensitive. Result ascending by `base_position`.
pub fn translate_frame(bases: &str, frame: usize, from: usize, to: usize) -> Vec<AminoAcidCell> {
    let frame = frame % 3;
    let len = bases.len();

    // Find the first base that is in frame and >= from.
    let mut pos = from.min(len);
    // Find the first base whose position modulo 3 matches the frame and >= from.
    while pos < len && pos % 3 != frame % 3 {
        pos += 1;
    }

    let mut cells = Vec::new();
    while pos + 2 < to.min(len) {
        // Convert to uppercase, count U as T.
        let codon: String = (pos..pos + 3)
            .map(|i| match bases.as_bytes()[i].to_ascii_uppercase() {
                b'U' => 'T',
                c => c as char,
            })
            .collect();

        let aa = amino_acid_for(codon.as_bytes());
        cells.push(AminoAcidCell {
            base_position: pos,
            amino_acid: aa as char,
            is_start: codon == "ATG",
            is_stop: codon == "TAA" || codon == "TAG" || codon == "TGA",
        });

        pos += 3;
    }

    cells
}

/// Translation of a feature (e.g. CDS), following the course of its intervals and its strand:
/// 1. Concatenate the bases of the intervals in ascending order and remember which document position
///    each base has.
/// 2. For `Strand::Reverse`, reverse the bases and the position list and reverse-complement the bases
///    (`iupac_reverse_complement`).
/// 3. Qualifier `codon_start` (1, 2 or 3; default 1) says how many bases (`codon_start - 1`) at the start
///    are skipped.
/// 4. From there translate in groups of three; an incomplete final codon is dropped.
///
/// Errors: feature extends beyond `bases` → `Error::OutOfRange`; invalid `codon_start` → `Error::InvalidInput`.
pub fn translate_feature(bases: &str, feature: &Feature) -> Result<Vec<AminoAcidCell>> {
    let intervals = feature.location().intervals();

    // Check that all intervals lie within the sequence.
    let seq_len = bases.len();
    for interval in intervals {
        if interval.end() > seq_len {
            return Err(Error::OutOfRange(format!(
                "Feature interval [{}..{}) lies beyond the end of the sequence (length {})",
                interval.start(),
                interval.end(),
                seq_len
            )));
        }
    }

    // Get codon_start from the qualifiers (default 1). Check for invalid values.
    let codon_start: usize = match feature.qualifier("codon_start") {
        None => 1,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|&v| (1..=3).contains(&v))
            .ok_or_else(|| {
                Error::InvalidInput(format!(
                    "Invalid codon_start value '{}'. Must be 1, 2 or 3.",
                    v
                ))
            })?,
    };

    // Build: concatenate all bases of the intervals, remember the document position of each base.
    let mut merged_bases: Vec<u8> = Vec::new();
    let mut doc_positions: Vec<usize> = Vec::new();

    for interval in intervals {
        for i in interval.start()..interval.end() {
            merged_bases.push(bases.as_bytes()[i]);
            doc_positions.push(i);
        }
    }

    // For the reverse strand: reverse the position list (highest coordinate first),
    // reverse-complement the bases (`iupac_reverse_complement` already does reverse + complement).
    let (working_bases, working_positions) = match feature.location().strand() {
        Strand::Reverse => {
            let rc = iupac_reverse_complement(&String::from_utf8_lossy(&merged_bases));
            let rc_bytes: Vec<u8> = rc.bytes().collect();
            doc_positions.reverse();
            (rc_bytes, doc_positions)
        }
        _ => (merged_bases, doc_positions),
    };

    // Skip codon_start - 1 bases.
    let skip = codon_start - 1;

    let mut cells = Vec::new();
    let mut i = skip;
    while i + 2 < working_bases.len() {
        let codon: String = (i..i + 3)
            .map(|j| match working_bases[j].to_ascii_uppercase() {
                b'U' => 'T',
                c => c as char,
            })
            .collect();

        let aa = amino_acid_for(codon.as_bytes());
        // Document position of the first base of this codon.
        let doc_pos = working_positions[i];

        cells.push(AminoAcidCell {
            base_position: doc_pos,
            amino_acid: aa as char,
            is_start: codon == "ATG",
            is_stop: codon == "TAA" || codon == "TAG" || codon == "TGA",
        });

        i += 3;
    }

    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Interval, Location};

    fn cds(intervals: &[(usize, usize)], strand: Strand, codon_start: Option<&str>) -> Feature {
        let loc = Location::new(
            intervals
                .iter()
                .map(|(a, b)| Interval::new(*a, *b).unwrap())
                .collect(),
            strand,
            false,
            false,
        )
        .unwrap();
        let q = codon_start
            .map(|c| vec![("codon_start".to_string(), vec![c.to_string()])])
            .unwrap_or_default();
        Feature::new("CDS", "c", loc, q)
    }

    fn summary(cells: &[AminoAcidCell]) -> (String, Vec<usize>) {
        (
            cells.iter().map(|c| c.amino_acid).collect(),
            cells.iter().map(|c| c.base_position).collect(),
        )
    }

    #[test]
    fn rahmen_null() {
        let c = translate_frame("ATGAAATAG", 0, 0, 9);
        assert_eq!(summary(&c), ("MK*".to_string(), vec![0, 3, 6]));
        assert!(c[0].is_start && !c[0].is_stop);
        assert!(!c[1].is_start && !c[1].is_stop);
        assert!(c[2].is_stop);
    }

    #[test]
    fn rahmen_eins_und_zwei() {
        let c = translate_frame("AATGAAATAG", 1, 0, 10);
        assert_eq!(summary(&c), ("MK*".to_string(), vec![1, 4, 7]));
        let d = translate_frame("AAATGAAATAG", 2, 0, 11);
        assert_eq!(summary(&d), ("MK*".to_string(), vec![2, 5, 8]));
    }

    #[test]
    fn bereich_und_grenzen() {
        // from = 2: the first codon in frame 0 starts at 3
        let c = translate_frame("ATGAAATAG", 0, 2, 9);
        assert_eq!(summary(&c), ("K*".to_string(), vec![3, 6]));
        // to cuts off a codon: only complete codons count
        let d = translate_frame("ATGAAATAG", 0, 0, 7);
        assert_eq!(summary(&d), ("MK".to_string(), vec![0, 3]));
        // to beyond the end is clamped
        assert_eq!(translate_frame("ATGAAA", 0, 0, 100).len(), 2);
        assert!(translate_frame("ATG", 0, 5, 9).is_empty());
        assert!(translate_frame("", 0, 0, 0).is_empty());
        assert!(translate_frame("ATGAAA", 0, 4, 2).is_empty());
    }

    #[test]
    fn frame_modulo_drei_und_schreibweise() {
        let c = translate_frame("atgaaauag", 3, 0, 9);
        assert_eq!(summary(&c), ("MK*".to_string(), vec![0, 3, 6]));
        let r = translate_frame("AUGAAAUAG", 0, 0, 9);
        assert_eq!(summary(&r), ("MK*".to_string(), vec![0, 3, 6]));
    }

    #[test]
    fn unbekanntes_codon() {
        let c = translate_frame("ANGAAA", 0, 0, 6);
        assert_eq!(c[0].amino_acid, 'X');
        assert!(!c[0].is_start && !c[0].is_stop);
    }

    #[test]
    fn feature_vorwaerts() {
        let c = translate_feature("ATGAAATAG", &cds(&[(0, 9)], Strand::Forward, None)).unwrap();
        assert_eq!(summary(&c), ("MK*".to_string(), vec![0, 3, 6]));
        assert!(c[0].is_start && c[2].is_stop);
    }

    #[test]
    fn feature_rueckwaerts() {
        // Reverse complement of CTATTTCAT is ATGAAATAG
        let c = translate_feature("CTATTTCAT", &cds(&[(0, 9)], Strand::Reverse, None)).unwrap();
        assert_eq!(summary(&c), ("MK*".to_string(), vec![8, 5, 2]));
    }

    #[test]
    fn feature_mit_join() {
        // Bases 0..3 and 6..9 give ATG + TAG
        let c =
            translate_feature("ATGCCCTAG", &cds(&[(0, 3), (6, 9)], Strand::Forward, None)).unwrap();
        assert_eq!(summary(&c), ("M*".to_string(), vec![0, 6]));
    }

    #[test]
    fn feature_teilbereich_mitten_in_der_sequenz() {
        let c =
            translate_feature("GGGATGAAATAGCCC", &cds(&[(3, 12)], Strand::Forward, None)).unwrap();
        assert_eq!(summary(&c), ("MK*".to_string(), vec![3, 6, 9]));
    }

    #[test]
    fn codon_start_ueberspringt_basen() {
        let c =
            translate_feature("ATGAAATAG", &cds(&[(0, 9)], Strand::Forward, Some("2"))).unwrap();
        // TGA AAT AG → '*' (position 1), 'N' (position 4); "AG" is incomplete
        assert_eq!(summary(&c), ("*N".to_string(), vec![1, 4]));
        assert!(c[0].is_stop);
    }

    #[test]
    fn fehlerfaelle() {
        let ausserhalb = translate_feature("ATGAAA", &cds(&[(0, 20)], Strand::Forward, None));
        assert!(matches!(ausserhalb, Err(Error::OutOfRange(_))));
        for ungueltig in ["0", "4", "x", ""] {
            let r = translate_feature(
                "ATGAAATAG",
                &cds(&[(0, 9)], Strand::Forward, Some(ungueltig)),
            );
            assert!(matches!(r, Err(Error::InvalidInput(_))), "{}", ungueltig);
        }
    }

    #[test]
    fn zu_kurzes_feature_ergibt_keine_zellen() {
        let c = translate_feature("ATGAAATAG", &cds(&[(0, 2)], Strand::Forward, None)).unwrap();
        assert!(c.is_empty());
    }

    #[test]
    fn strand_none_wird_wie_vorwaerts_behandelt() {
        let c = translate_feature("ATGAAATAG", &cds(&[(0, 9)], Strand::None, None)).unwrap();
        assert_eq!(c.len(), 3);
    }
}
