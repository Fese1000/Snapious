//! Qualitätswerte (Phred) und ihre ASCII-Kodierungen.
//!
//! * Sanger / Illumina 1.8+: Phred+33
//! * Illumina 1.3–1.7: Phred+64
//! * Solexa (Illumina 1.0): Solexa-Skala+64 (Odds-basiert, kann negativ sein)

use std::collections::BTreeMap;

use crate::error::{Error, Result};

/// Kodierung von Qualitätszeichen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingType {
    Sanger,
    Solexa,
    Illumina13,
}

impl EncodingType {
    pub fn offset(&self) -> i32 {
        match self {
            EncodingType::Sanger => 33,
            EncodingType::Solexa | EncodingType::Illumina13 => 64,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EncodingType::Sanger => "Sanger / Illumina 1.8+ (Phred+33)",
            EncodingType::Solexa => "Solexa / Illumina 1.0 (Solexa+64)",
            EncodingType::Illumina13 => "Illumina 1.3–1.7 (Phred+64)",
        }
    }
}

/// Größter darstellbarer Phred-Wert.
pub const MAX_PHRED: i32 = 93;

/// Klassifikation über das kleinste vorkommende Zeichen (übliche Heuristik): unter `;` (59) nur Phred+33,
/// unter `@` (64) nur Solexa, ab `@` Phred+64. Ohne niedrige Qualitäten ist Phred+64 nicht sicher von
/// Phred+33 zu unterscheiden.
fn classify(min_char: u8) -> EncodingType {
    if min_char < 59 {
        EncodingType::Sanger
    } else if min_char < 64 {
        EncodingType::Solexa
    } else {
        EncodingType::Illumina13
    }
}

/// Erkennt die Kodierung eines Qualitätsstrings. Leerer String → Fehler.
pub fn detect_encoding(quality: &str) -> Result<EncodingType> {
    quality
        .bytes()
        .min()
        .map(classify)
        .ok_or_else(|| Error::InvalidInput("Quality string must not be empty".to_string()))
}

/// Erkennt die Kodierung über mehrere Strings gemeinsam (leer → Sanger).
pub fn detect_encoding_many(qualities: &[String]) -> EncodingType {
    qualities
        .iter()
        .filter_map(|q| q.bytes().min())
        .min()
        .map(classify)
        .unwrap_or(EncodingType::Sanger)
}

fn solexa_to_phred(s: i32) -> i32 {
    (10.0 * (10f64.powf(s as f64 / 10.0) + 1.0).log10()).round() as i32
}

fn phred_to_solexa(p: i32) -> i32 {
    if p <= 0 {
        -5
    } else {
        (10.0 * (10f64.powf(p as f64 / 10.0) - 1.0).log10()).round() as i32
    }
}

/// Phred-Werte (mindestens 0, höchstens 93) für einen Qualitätsstring.
pub fn to_phred_scores(quality: &str, enc: EncodingType) -> Vec<i32> {
    quality
        .bytes()
        .map(|b| {
            let raw = b as i32 - enc.offset();
            let p = if enc == EncodingType::Solexa {
                solexa_to_phred(raw)
            } else {
                raw
            };
            p.clamp(0, MAX_PHRED)
        })
        .collect()
}

/// Qualitätsstring aus Phred-Werten (druckbare Zeichen `!` bis `~`).
pub fn from_phred_scores(scores: &[i32], enc: EncodingType) -> String {
    scores
        .iter()
        .map(|p| {
            let raw = if enc == EncodingType::Solexa {
                phred_to_solexa(*p)
            } else {
                *p
            };
            (raw + enc.offset()).clamp(b'!' as i32, b'~' as i32) as u8 as char
        })
        .collect()
}

/// Wandelt einen Qualitätsstring von einer Kodierung in eine andere.
pub fn convert_encoding(quality: &str, from: EncodingType, to: EncodingType) -> String {
    from_phred_scores(&to_phred_scores(quality, from), to)
}

/// Fehlerwahrscheinlichkeit zu einem Phred-Wert: 10^(−Q/10).
pub fn error_probability(phred: i32) -> f64 {
    10f64.powf(-(phred as f64) / 10.0)
}

/// Kennzahlen zu einem Qualitätsstring.
#[derive(Clone, Debug, PartialEq)]
pub struct QualityStats {
    pub mean: f64,
    /// Unterer Median (Element mit Index `(n−1)/2` der sortierten Werte).
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub length: usize,
}

impl QualityStats {
    pub fn label(&self) -> &'static str {
        if self.mean >= 30.0 {
            "High (Q30+)"
        } else if self.mean >= 20.0 {
            "Medium (Q20+)"
        } else if self.mean >= 10.0 {
            "Low (Q10+)"
        } else {
            "Insufficient (<Q10)"
        }
    }
}

/// Mittel, Median, Minimum und Maximum; alles 0 bei leerem String.
pub fn calculate_stats(quality: &str, enc: EncodingType) -> QualityStats {
    let mut scores = to_phred_scores(quality, enc);
    if scores.is_empty() {
        return QualityStats {
            mean: 0.0,
            median: 0.0,
            min: 0.0,
            max: 0.0,
            length: 0,
        };
    }
    scores.sort_unstable();
    QualityStats {
        mean: scores.iter().sum::<i32>() as f64 / scores.len() as f64,
        median: scores[(scores.len() - 1) / 2] as f64,
        min: scores[0] as f64,
        max: scores[scores.len() - 1] as f64,
        length: scores.len(),
    }
}

/// Häufigkeit jedes Phred-Wertes.
pub fn histogram(quality: &str, enc: EncodingType) -> BTreeMap<i32, usize> {
    let mut h = BTreeMap::new();
    for s in to_phred_scores(quality, enc) {
        *h.entry(s).or_insert(0) += 1;
    }
    h
}

/// Ersetzt Zeichen mit Qualität unter `min_quality` durch `!` (Q0).
pub fn filter_by_quality(quality: &str, enc: EncodingType, min_quality: i32) -> String {
    quality
        .bytes()
        .zip(to_phred_scores(quality, enc))
        .map(|(b, q)| if q < min_quality { '!' } else { b as char })
        .collect()
}

/// Zahl der Basen mit mindestens `min_quality`.
pub fn count_bases_above(quality: &str, enc: EncodingType, min_quality: i32) -> usize {
    to_phred_scores(quality, enc)
        .into_iter()
        .filter(|q| *q >= min_quality)
        .count()
}

/// Ersetzt Basen mit zu geringer Qualität durch `N`. Fehlt für eine Base die Qualität, gilt sie als 0.
pub fn mask_low_quality(
    sequence: &str,
    quality: &str,
    enc: EncodingType,
    min_quality: i32,
) -> String {
    let scores = to_phred_scores(quality, enc);
    sequence
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if scores.get(i).copied().unwrap_or(0) >= min_quality {
                c
            } else {
                'N'
            }
        })
        .collect()
}

/// Kurzer Textbericht über mehrere Qualitätsstrings.
pub fn report(qualities: &[String]) -> String {
    let enc = detect_encoding_many(qualities);
    let mut out = format!(
        "=== Quality report ===\n\nReads: {}\nEncoding: {}\n",
        qualities.len(),
        enc.description()
    );
    let stats: Vec<QualityStats> = qualities
        .iter()
        .filter(|q| !q.is_empty())
        .map(|q| calculate_stats(q, enc))
        .collect();
    if !stats.is_empty() {
        let n = stats.len() as f64;
        out.push_str(&format!(
            "Mean quality: {:.1}\nRating: {}\n",
            stats.iter().map(|s| s.mean).sum::<f64>() / n,
            QualityStats {
                mean: stats.iter().map(|s| s.mean).sum::<f64>() / n,
                median: 0.0,
                min: 0.0,
                max: 0.0,
                length: 0
            }
            .label()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennung() {
        assert_eq!(
            detect_encoding("!#$$$%&'()*+").unwrap(),
            EncodingType::Sanger
        );
        assert_eq!(
            detect_encoding("@ABCDEFGHI").unwrap(),
            EncodingType::Illumina13
        );
        assert_eq!(detect_encoding(";<=>?@A").unwrap(), EncodingType::Solexa);
        assert!(detect_encoding("").is_err());
        assert_eq!(
            detect_encoding_many(&["hhhh".to_string(), "BBBB".to_string()]),
            EncodingType::Illumina13
        );
        assert_eq!(
            detect_encoding_many(&["hhhh".to_string(), "!hhh".to_string()]),
            EncodingType::Sanger
        );
        assert_eq!(detect_encoding_many(&[]), EncodingType::Sanger);
    }

    #[test]
    fn phred_werte_und_rueckweg() {
        assert_eq!(
            to_phred_scores("!5I", EncodingType::Sanger),
            vec![0, 20, 40]
        );
        assert_eq!(from_phred_scores(&[0, 20, 40], EncodingType::Sanger), "!5I");
        assert_eq!(
            to_phred_scores("@Th", EncodingType::Illumina13),
            vec![0, 20, 40]
        );
    }

    #[test]
    fn umrechnung_zwischen_kodierungen() {
        assert_eq!(
            convert_encoding("!5I", EncodingType::Sanger, EncodingType::Illumina13),
            "@Th"
        );
        assert_eq!(
            convert_encoding("@Th", EncodingType::Illumina13, EncodingType::Sanger),
            "!5I"
        );
        assert_eq!(
            convert_encoding("", EncodingType::Sanger, EncodingType::Solexa),
            ""
        );
    }

    #[test]
    fn solexa_umrechnung() {
        // Phred 10 ↔ Solexa 10 (Odds 9:1 ≈ 10 dB), Phred 0 → Solexa −5
        assert_eq!(phred_to_solexa(10), 10);
        assert_eq!(phred_to_solexa(0), -5);
        assert_eq!(solexa_to_phred(10), 10);
        assert_eq!(solexa_to_phred(-5), 1);
        let s = convert_encoding("5I", EncodingType::Sanger, EncodingType::Solexa);
        assert_eq!(to_phred_scores(&s, EncodingType::Solexa), vec![20, 40]);
    }

    #[test]
    fn grenzen_werden_geklemmt() {
        // Zeichen unter dem Offset ergeben Phred 0
        assert_eq!(to_phred_scores("!", EncodingType::Illumina13), vec![0]);
        // Sehr hohe Werte werden auf druckbares Zeichen begrenzt
        assert_eq!(from_phred_scores(&[200], EncodingType::Sanger), "~");
        assert_eq!(from_phred_scores(&[-5], EncodingType::Sanger), "!");
    }

    #[test]
    fn statistik() {
        let s = calculate_stats("!5I+", EncodingType::Sanger); // 0, 20, 40, 10
        assert_eq!(s.length, 4);
        assert_eq!(s.mean, 17.5);
        assert_eq!(s.median, 10.0); // sortiert 0,10,20,40 → Index 1
        assert_eq!((s.min, s.max), (0.0, 40.0));
        assert_eq!(s.label(), "Low (Q10+)");
        let leer = calculate_stats("", EncodingType::Sanger);
        assert_eq!(leer.length, 0);
        assert_eq!(leer.label(), "Insufficient (<Q10)");
        assert_eq!(
            calculate_stats("IIII", EncodingType::Sanger).label(),
            "High (Q30+)"
        );
    }

    #[test]
    fn histogramm() {
        let h = histogram("!!!###", EncodingType::Sanger);
        assert_eq!(h[&0], 3);
        assert_eq!(h[&2], 3);
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn filtern_zaehlen_maskieren() {
        assert_eq!(filter_by_quality("!5I", EncodingType::Sanger, 20), "!5I");
        assert_eq!(filter_by_quality("+5I", EncodingType::Sanger, 20), "!5I");
        assert_eq!(count_bases_above("!5I", EncodingType::Sanger, 20), 2);
        assert_eq!(
            mask_low_quality("ACG", "!5I", EncodingType::Sanger, 20),
            "NCG"
        );
        assert_eq!(
            mask_low_quality("ACGT", "5I", EncodingType::Sanger, 20),
            "ACNN"
        );
        assert_eq!(mask_low_quality("", "5I", EncodingType::Sanger, 20), "");
    }

    #[test]
    fn fehlerwahrscheinlichkeit() {
        assert!((error_probability(20) - 0.01).abs() < 1e-12);
        assert!((error_probability(0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn bericht() {
        let r = report(&["!!!###".to_string(), "%%%&&&".to_string()]);
        assert!(r.contains("Quality report"));
        assert!(r.contains("Reads: 2"));
        assert!(r.contains("Sanger"));
    }
}
