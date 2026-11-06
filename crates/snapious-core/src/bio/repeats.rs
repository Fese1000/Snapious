//! Wiederholungen in einer Sequenz: Tandem-Wiederholungen und invertierte Wiederholungen (Haarnadeln).

/// Eine Tandem-Wiederholung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TandemRepeat {
    /// Beginn (0-basiert, inklusive).
    pub start: usize,
    /// Ende (exklusive).
    pub end: usize,
    /// Wiederholte Einheit in Großbuchstaben.
    pub unit: String,
    /// Anzahl der Kopien (`(end - start) / unit.len()`).
    pub copies: usize,
}

/// Sucht Tandem-Wiederholungen in `seq` (Großbuchstaben; nur genaue Wiederholungen).
/// Ablauf: `i = 0`; solange `i < Länge`: für Einheitenlängen `k = min_unit..=max_unit` (aufsteigend; abbrechen, wenn `i + k > Länge`):
/// Einheit `u = seq[i..i+k]`; überspringe `k`, wenn `u` selbst aus einer kürzeren Einheit besteht
/// (z. B. „ACAC“ besteht aus „AC“; es zählt nur die kürzeste Einheit); zähle die aufeinanderfolgenden Kopien `c` von `u` ab `i`;
/// ist `c >= min_copies`, melde `(i, i + k*c, u, c)`, setze `i = i + k*c` und gehe zum nächsten `i` (die übrigen `k` entfallen).
/// Wurde kein `k` gemeldet, ist `i += 1`. `min_unit` muss mindestens 1 sein (kleinere Werte wie 1 behandeln); ist `min_unit == 0`,
/// `max_unit < min_unit` oder `min_copies < 2`, ist das Ergebnis leer.
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
    // Einheiten, die länger als die Sequenz sind, können nie zweimal vorkommen
    let max_unit = max_unit.min(len);

    let mut i = 0;
    while i < len {
        let mut found = false;
        for k in min_unit..=max_unit {
            if k > len - i {
                break;
            }

            let u_bytes = &bytes[i..i + k];

            // Überspringe, wenn die Einheit selbst aus einer kürzeren Einheit besteht
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

            // Zähle aufeinanderfolgende Kopien
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
                found = true; // Übrige k für diesen i entfallen
                break;
            }
        }
        if !found {
            i += 1;
        }
    }

    result
}

/// Eine invertierte Wiederholung: linker Arm, Spacer, rechter Arm (Reverskomplement des linken Arms).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvertedRepeat {
    /// Beginn des linken Arms.
    pub start: usize,
    /// Länge eines Arms.
    pub arm: usize,
    /// Länge des Spacers zwischen den Armen.
    pub spacer: usize,
}

impl InvertedRepeat {
    /// Ende (exklusive) des rechten Arms: `start + 2 * arm + spacer`.
    pub fn end(&self) -> usize {
        self.start + 2 * self.arm + self.spacer
    }
}

/// Sucht invertierte Wiederholungen in `seq` (Großbuchstaben, nur A, C, G, T; andere Zeichen paaren nie).
/// Für jede Position `p` in `0..=Länge` (Ende des linken Arms, exklusiv) und jede Spacerlänge `s` in `0..=max_spacer`:
/// zähle `k`, solange `p - k - 1 >= 0`, `p + s + k < Länge` und `seq[p-k-1]` das Komplement von `seq[p+s+k]` ist (A–T, C–G).
/// Ist `k >= min_arm`, melde `InvertedRepeat { start: p - k, arm: k, spacer: s }`.
/// Sortierung nach `start`, dann nach `spacer`. Ist `min_arm == 0`, ist das Ergebnis leer.
/// Tipp: `iupac_reverse_complement` steht zur Verfügung, wird aber nicht benötigt.
pub fn find_inverted_repeats(seq: &str, min_arm: usize, max_spacer: usize) -> Vec<InvertedRepeat> {
    if min_arm == 0 {
        return Vec::new();
    }

    let bytes = seq.as_bytes();
    let len = bytes.len();
    let mut result = Vec::new();
    // ein Spacer ab der Sequenzlänge lässt keinen rechten Arm mehr zu
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
