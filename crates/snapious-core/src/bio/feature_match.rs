//! Find short reference features (e.g. promoters, primer sites) in a sequence, allowing mismatches.

use crate::cloning::enzymes::iupac_reverse_complement;

/// A hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureMatch {
    /// Start (0-based) in `seq`.
    pub start: usize,
    /// `true` if the reverse complement of the reference matches.
    pub reverse: bool,
    pub mismatches: usize,
}

/// Searches the gap-free `reference` at all positions of `seq`, forward and as reverse complement (`iupac_reverse_complement`),
/// and returns all hits with at most `max_mismatch` differences (case-insensitive, byte equality counts).
/// Sorted by `start`, with equal start forward before reverse. Empty reference or reference longer than `seq` → empty.
/// Example: `seq = AACCTTGGTTAACC`, `reference = AACC`, 0 differences → (0, forward), (6, reverse), (10, forward).
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
