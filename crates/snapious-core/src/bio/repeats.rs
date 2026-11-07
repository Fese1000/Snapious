//! Repeats in a sequence: tandem repeats and inverted repeats (hairpins).

/// A tandem repeat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TandemRepeat {
    /// Start (0-based, inclusive).
    pub start: usize,
    /// End (exclusive).
    pub end: usize,
    /// Repeated unit in uppercase.
    pub unit: String,
    /// Number of copies (`(end - start) / unit.len()`).
    pub copies: usize,
}

/// Searches tandem repeats in `seq` (uppercase; exact repeats only).
/// Procedure: `i = 0`; while `i < length`: for unit lengths `k = min_unit..=max_unit` (ascending; stop if `i + k > length`):
/// unit `u = seq[i..i+k]`; skip `k` if `u` itself consists of a shorter unit
/// (e.g. "ACAC" consists of "AC"; only the shortest unit counts); count the consecutive copies `c` of `u` from `i`;
/// if `c >= min_copies`, report `(i, i + k*c, u, c)`, set `i = i + k*c` and move on to the next `i` (the remaining `k` are dropped).
/// If no `k` was reported, `i += 1`. `min_unit` must be at least 1 (treat smaller values like 1); if `min_unit == 0`,
/// `max_unit < min_unit` or `min_copies < 2`, the result is empty.
pub fn find_tandem_repeats(
    seq: &str,
    min_unit: usize,
    max_unit: usize,
    min_copies: usize,
) -> Vec<TandemRepeat> {
    if min_unit == 0 || max_unit < min_unit || min_copies < 2 {
        return Vec::new();
    }

    let bytes = seq.as_bytes();
    let len = bytes.len();
    let mut result = Vec::new();
    // Units longer than the sequence can never occur twice
    let max_unit = max_unit.min(len);

    let mut i = 0;
    while i < len {
        let mut found = false;
        for k in min_unit..=max_unit {
            if k > len - i {
                break;
            }

            let u_bytes = &bytes[i..i + k];

            // Skip if the unit itself consists of a shorter unit
            let mut is_shorter = false;
            for p in 1..k {
                if k % p == 0 {
                    let sub = &u_bytes[..p];
                    if (0..k).all(|j| u_bytes[j] == sub[j % p]) {
                        is_shorter = true;
                        break;
                    }
                }
            }
            if is_shorter {
                continue;
            }

            // Count consecutive copies
            let mut c: usize = 0;
            while i + c * k + k <= len {
                if &bytes[i + c * k..i + (c + 1) * k] == u_bytes {
                    c += 1;
                } else {
                    break;
                }
            }

            if c >= min_copies {
                result.push(TandemRepeat {
                    start: i,
                    end: i + k * c,
                    unit: seq[i..i + k].to_string(),
                    copies: c,
                });
                i += k * c;
                found = true; // Remaining k for this i are dropped
                break;
            }
        }
        if !found {
            i += 1;
        }
    }

    result
}

/// An inverted repeat: left arm, spacer, right arm (reverse complement of the left arm).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvertedRepeat {
    /// Start of the left arm.
    pub start: usize,
    /// Length of one arm.
    pub arm: usize,
    /// Length of the spacer between the arms.
    pub spacer: usize,
}

impl InvertedRepeat {
    /// End (exclusive) of the right arm: `start + 2 * arm + spacer`.
    pub fn end(&self) -> usize {
        self.start + 2 * self.arm + self.spacer
    }
}

/// Searches inverted repeats in `seq` (uppercase, only A, C, G, T; other characters never pair).
/// For each position `p` in `0..=length` (end of the left arm, exclusive) and each spacer length `s` in `0..=max_spacer`:
/// count `k` while `p - k - 1 >= 0`, `p + s + k < length` and `seq[p-k-1]` is the complement of `seq[p+s+k]` (A–T, C–G).
/// If `k >= min_arm`, report `InvertedRepeat { start: p - k, arm: k, spacer: s }`.
/// Sorting by `start`, then by `spacer`. If `min_arm == 0`, the result is empty.
/// Hint: `iupac_reverse_complement` is available but not needed.
pub fn find_inverted_repeats(seq: &str, min_arm: usize, max_spacer: usize) -> Vec<InvertedRepeat> {
    if min_arm == 0 {
        return Vec::new();
    }

    let bytes = seq.as_bytes();
    let len = bytes.len();
    let mut result = Vec::new();
    // a spacer starting at the sequence length leaves no room for a right arm
    let max_spacer = max_spacer.min(len);

    for p in 0..=len {
        for s in 0..=max_spacer {
            let mut k = 0;
            while p > k && (p + s + k) < len {
                let left_idx = p - k - 1;
                let right_idx = p + s + k;

                let l = bytes[left_idx];
                let r = bytes[right_idx];

                let matches = match l {
                    b'A' => r == b'T',
                    b'T' => r == b'A',
                    b'C' => r == b'G',
                    b'G' => r == b'C',
                    _ => false,
                };

                if matches {
                    k += 1;
                } else {
                    break;
                }
            }

            if k >= min_arm {
                result.push(InvertedRepeat {
                    start: p - k,
                    arm: k,
                    spacer: s,
                });
            }
        }
    }

    result.sort_by(|a, b| a.start.cmp(&b.start).then(a.spacer.cmp(&b.spacer)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(start: usize, end: usize, unit: &str, copies: usize) -> TandemRepeat {
        TandemRepeat {
            start,
            end,
            unit: unit.to_string(),
            copies,
        }
    }

    fn ir(start: usize, arm: usize, spacer: usize) -> InvertedRepeat {
        InvertedRepeat { start, arm, spacer }
    }

    #[test]
    fn tandem_einfach() {
        assert_eq!(
            find_tandem_repeats("ACACACAC", 1, 6, 3),
            vec![t(0, 8, "AC", 4)]
        );
        assert_eq!(
            find_tandem_repeats("AAAAAA", 1, 3, 3),
            vec![t(0, 6, "A", 6)]
        );
        assert!(find_tandem_repeats("ACGT", 1, 3, 2).is_empty());
    }

    #[test]
    fn tandem_mehrere() {
        let s = "GGGTTTTCAGCAGCAGCAGATTTTT";
        assert_eq!(
            find_tandem_repeats(s, 1, 6, 3),
            vec![
                t(0, 3, "G", 3),
                t(3, 7, "T", 4),
                t(7, 19, "CAG", 4),
                t(20, 25, "T", 5)
            ]
        );
        assert_eq!(find_tandem_repeats(s, 2, 6, 2), vec![t(7, 19, "CAG", 4)]);
        assert_eq!(
            find_tandem_repeats("ATATATGCGCGCGC", 1, 4, 3),
            vec![t(0, 6, "AT", 3), t(6, 14, "GC", 4)]
        );
    }

    #[test]
    fn tandem_grenzen() {
        assert!(find_tandem_repeats("AAAA", 0, 3, 2).is_empty());
        assert!(find_tandem_repeats("AAAA", 3, 2, 2).is_empty());
        assert!(find_tandem_repeats("AAAA", 1, 3, 1).is_empty());
        assert!(find_tandem_repeats("", 1, 3, 2).is_empty());
    }

    #[test]
    fn invertiert() {
        assert_eq!(
            find_inverted_repeats("AAGCTTGGGAAGCTT", 3, 0),
            vec![ir(0, 3, 0), ir(9, 3, 0)]
        );
        assert_eq!(find_inverted_repeats("CCCAAAGGG", 3, 3), vec![ir(0, 3, 3)]);
        assert!(find_inverted_repeats("CCCAAAGGG", 3, 0).is_empty());
        assert_eq!(
            find_inverted_repeats("TTGAATTCAA", 2, 2),
            vec![ir(0, 5, 0), ir(0, 2, 1), ir(0, 4, 2), ir(5, 2, 1)]
        );
        assert_eq!(
            find_inverted_repeats("GAATTCAAAGAATTC", 3, 5),
            vec![ir(0, 3, 0), ir(0, 6, 3), ir(0, 5, 5), ir(9, 3, 0)]
        );
        assert_eq!(ir(0, 6, 3).end(), 15);
        assert!(find_inverted_repeats("AAGCTT", 0, 2).is_empty());
    }
}
