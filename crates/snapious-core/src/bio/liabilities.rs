//! Known liabilities in protein sequences (e.g. antibodies): motifs that cause problems chemically or through glycosylation.

/// A liability found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Liability {
    /// Start (0-based) in the protein sequence.
    pub position: usize,
    /// Length of the motif (1–3).
    pub length: usize,
    /// Kind, exactly one of the strings: `N-Glykosylierung`, `Deamidierung`, `Isomerisierung`, `Oxidation`, `Freies Cystein`.
    pub kind: String,
    /// The amino acids of the motif in uppercase.
    pub motif: String,
}

/// Finds all liabilities (uppercase expected, other characters are only counted as placeholders; case-insensitive).
/// Rules, overlapping hits are allowed:
/// * `N-Glykosylierung`: three amino acids `N`, then any except `P`, then `S` or `T` (length 3).
/// * `Deamidierung`: `NG` or `NS` (length 2).
/// * `Isomerisierung`: `DG` or `DS` (length 2).
/// * `Oxidation`: every `M` and every `W` (length 1).
/// * `Freies Cystein`: every `C` (length 1).
///
/// Sorting: by `position`, for equal position in the order of the list above (glycosylation, deamidation,
/// isomerization, oxidation, cysteine). Example `ANSTGDSMWC`: glycosylation `NST` (1), deamidation `NS` (1), isomerization `DS` (5),
/// oxidation `M` (7) and `W` (8), cysteine `C` (9).
pub fn find_liabilities(protein: &str) -> Vec<Liability> {
    let seq: Vec<u8> = protein.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let mut results: Vec<Liability> = Vec::new();

    for i in 0..seq.len() {
        let c = seq[i];

        // Rule 1: N-glycosylation (length 3)
        if c == b'N' && i + 2 < seq.len() {
            let s = seq[i + 1];
            if s != b'P' {
                let t = seq[i + 2];
                if t == b'S' || t == b'T' {
                    results.push(Liability {
                        position: i,
                        length: 3,
                        kind: "N-Glykosylierung".to_string(),
                        motif: String::from_utf8_lossy(&seq[i..i + 3]).to_string(),
                    });
                }
            }
        }

        // Rule 2: deamidation (length 2)
        if c == b'N' && i + 1 < seq.len() {
            let next = seq[i + 1];
            if next == b'G' || next == b'S' {
                results.push(Liability {
                    position: i,
                    length: 2,
                    kind: "Deamidation".to_string(),
                    motif: String::from_utf8_lossy(&seq[i..i + 2]).to_string(),
                });
            }
        }

        // Rule 3: isomerization (length 2)
        if c == b'D' && i + 1 < seq.len() {
            let next = seq[i + 1];
            if next == b'G' || next == b'S' {
                results.push(Liability {
                    position: i,
                    length: 2,
                    kind: "Isomerization".to_string(),
                    motif: String::from_utf8_lossy(&seq[i..i + 2]).to_string(),
                });
            }
        }

        // Rule 4: oxidation (length 1)
        if c == b'M' || c == b'W' {
            results.push(Liability {
                position: i,
                length: 1,
                kind: "Oxidation".to_string(),
                motif: String::from_utf8_lossy(&seq[i..i + 1]).to_string(),
            });
        }

        // Rule 5: free cysteine (length 1)
        if c == b'C' {
            results.push(Liability {
                position: i,
                length: 1,
                kind: "Free cysteine".to_string(),
                motif: String::from_utf8_lossy(&seq[i..i + 1]).to_string(),
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(position: usize, length: usize, kind: &str, motif: &str) -> Liability {
        Liability {
            position,
            length,
            kind: kind.to_string(),
            motif: motif.to_string(),
        }
    }

    #[test]
    fn beispiel() {
        assert_eq!(
            find_liabilities("ANSTGDSMWC"),
            vec![
                l(1, 3, "N-Glykosylierung", "NST"),
                l(1, 2, "Deamidation", "NS"),
                l(5, 2, "Isomerization", "DS"),
                l(7, 1, "Oxidation", "M"),
                l(8, 1, "Oxidation", "W"),
                l(9, 1, "Free cysteine", "C"),
            ]
        );
    }

    #[test]
    fn prolin_klein_und_leer() {
        assert!(find_liabilities("NPT").is_empty());
        assert!(find_liabilities("").is_empty());
        assert!(find_liabilities("AKLV").is_empty());
        assert_eq!(
            find_liabilities("ngt"),
            vec![
                l(0, 3, "N-Glykosylierung", "NGT"),
                l(0, 2, "Deamidation", "NG")
            ]
        );
        // two glycosylation sites overlap: NNSS
        let r = find_liabilities("NNST");
        assert_eq!(r[0], l(0, 3, "N-Glykosylierung", "NNS"));
        assert_eq!(r[1], l(1, 3, "N-Glykosylierung", "NST"));
    }
}
