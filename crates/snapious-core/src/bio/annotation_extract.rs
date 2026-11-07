//! Cut sequence segments for annotations (with flanks, reverse-complemented on request).

use crate::cloning::enzymes::iupac_reverse_complement;

/// For each range `(start, end, reverse)`, cuts out the bases: `bases[start - flank .. end + flank]`, clamped at the sequence
/// boundaries (`start.saturating_sub(flank)` and `min(end + flank, len)`). With `reverse`, the reverse complement
/// (`iupac_reverse_complement`) is returned. If `start >= end` or `start >= len`, the entry is an empty string. The result has
/// as many entries as `spans`. Access via `str::get` so that special characters do not panic (the result is then an empty string).
/// Example `AACCGGTT`: `(2, 4, false)` without flank → `CC`, with flank 1 → `ACCG`, `(2, 4, true)` → `GG`.
pub fn extract_sequences(bases: &str, spans: &[(usize, usize, bool)], flank: usize) -> Vec<String> {
    let len = bases.len();

    spans
        .iter()
        .map(|&(start, end, reverse)| {
            // If `start >= end` or `start >= len`, the entry is an empty string.
            if start >= end || start >= len {
                return String::new();
            }

            // Clamping at the sequence boundaries
            let actual_start = start.saturating_sub(flank);
            let actual_end = (end + flank).min(len);

            // Access via `str::get` so that special characters do not panic (the result is then an empty string).
            match bases.get(actual_start..actual_end) {
                Some(seq) => {
                    if reverse {
                        iupac_reverse_complement(seq)
                    } else {
                        seq.to_string()
                    }
                }
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ausschneiden() {
        let b = "AACCGGTT";
        assert_eq!(extract_sequences(b, &[(2, 4, false)], 0), vec!["CC"]);
        assert_eq!(extract_sequences(b, &[(2, 4, false)], 1), vec!["ACCG"]);
        assert_eq!(extract_sequences(b, &[(2, 4, true)], 0), vec!["GG"]);
        assert_eq!(extract_sequences(b, &[(6, 8, false)], 3), vec!["CGGTT"]);
        assert_eq!(extract_sequences(b, &[(0, 2, true)], 2), vec!["GGTT"]);
    }

    #[test]
    fn randfaelle() {
        let b = "AACCGGTT";
        assert_eq!(
            extract_sequences(b, &[(5, 3, false), (3, 3, false), (20, 30, false)], 0),
            vec!["", "", ""]
        );
        assert_eq!(extract_sequences(b, &[(6, 99, false)], 0), vec!["TT"]);
        assert!(extract_sequences(b, &[], 1).is_empty());
        assert_eq!(extract_sequences("ÄCGT", &[(0, 2, false)], 0).len(), 1);
    }
}
