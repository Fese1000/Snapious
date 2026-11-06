//! Kurze Referenz-Features (z. B. Promotoren, Primerstellen) in einer Sequenz mit Fehlpaarungen wiederfinden.

use crate::cloning::enzymes::iupac_reverse_complement;

/// Ein Treffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureMatch {
    /// Beginn (0-basiert) in `seq`.
    pub start: usize,
    /// `true`, wenn das Reverskomplement der Referenz passt.
    pub reverse: bool,
    pub mismatches: usize,
}

/// Sucht `reference` ohne Lücken an allen Positionen von `seq`, vorwärts und als Reverskomplement (`iupac_reverse_complement`),
/// und liefert alle Treffer mit höchstens `max_mismatch` Unterschieden (Groß-/Kleinschreibung egal, es zählt Byte-Gleichheit).
/// Sortiert nach `start`, bei gleichem Beginn vorwärts vor rückwärts. Leere Referenz oder Referenz länger als `seq` → leer.
/// Beispiel: `seq = AACCTTGGTTAACC`, `reference = AACC`, 0 Unterschiede → (0, vorwärts), (6, rückwärts), (10, vorwärts).
pub fn find_feature_matches(seq: &str, reference: &str, max_mismatch: usize) -> Vec<FeatureMatch> {
    let seq_up = seq.to_ascii_uppercase();
    let fwd = reference.to_ascii_uppercase();
    if fwd.is_empty() || fwd.len() > seq_up.len() {
        return Vec::new();
    }
    let rev = iupac_reverse_complement(&fwd);
    let s = seq_up.as_bytes();
    let n = fwd.len();
    let diff = |a: &[u8], b: &[u8]| a.iter().zip(b).filter(|(x, y)| x != y).count();
    let mut out = Vec::new();
    for start in 0..=(s.len() - n) {
        let window = &s[start..start + n];
        let f = diff(window, fwd.as_bytes());
        if f <= max_mismatch {
            out.push(FeatureMatch {
                start,
                reverse: false,
                mismatches: f,
            });
        }
        let r = diff(window, rev.as_bytes());
        if r <= max_mismatch {
            out.push(FeatureMatch {
                start,
                reverse: true,
                mismatches: r,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(start: usize, reverse: bool, mismatches: usize) -> FeatureMatch {
        FeatureMatch {
            start,
            reverse,
            mismatches,
        }
    }

    #[test]
    fn genau() {
        assert_eq!(
            find_feature_matches("AACCTTGGTTAACC", "AACC", 0),
            vec![m(0, false, 0), m(6, true, 0), m(10, false, 0)]
        );
        assert_eq!(find_feature_matches("aaccttggttaacc", "AACC", 0).len(), 3);
    }

    #[test]
    fn mit_fehlern_und_randfaelle() {
        assert_eq!(
            find_feature_matches("AACGTT", "AACC", 1),
            vec![m(0, false, 1), m(2, true, 1)]
        );
        assert!(find_feature_matches("AACGTT", "AACC", 0).is_empty());
        assert!(find_feature_matches("AAC", "AACC", 4).is_empty());
        assert!(find_feature_matches("AAC", "", 1).is_empty());
        assert!(find_feature_matches("", "A", 1).is_empty());
    }
}
