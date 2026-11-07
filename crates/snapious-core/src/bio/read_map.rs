//! Reads auf eine Referenz abbilden (ohne Lücken) und Konsens aus der Abdeckung bilden.

use crate::cloning::enzymes::iupac_reverse_complement;

/// Ort eines abgebildeten Reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mapping {
    /// Beginn (0-basiert) in der Referenz.
    pub position: usize,
    /// `true`, wenn das Reverskomplement des Reads passt.
    pub reverse: bool,
    /// Zahl der Fehlpaarungen.
    pub mismatches: usize,
}

/// Bildet `read` lückenlos auf `reference` ab (Uppercase, A/C/G/T; Groß-/Kleinschreibung egal).
/// Geprüft werden alle Positionen für den Read vorwärts und für sein Reverskomplement (`iupac_reverse_complement`).
/// Gewinner ist die Position mit den wenigsten Fehlpaarungen, `<= max_mismatches`; bei Gleichstand zählt: vorwärts vor
/// rückwärts, dann die kleinere Position. `None`, wenn nichts passt, der Read leer oder länger als die Referenz ist.
pub fn map_read(reference: &str, read: &str, max_mismatches: usize) -> Option<Mapping> {
    if read.is_empty() || read.len() > reference.len() {
        return None;
    }

    let ref_up = reference.to_uppercase();
    let read_up = read.to_uppercase();
    let rc_read = iupac_reverse_complement(&read_up);

    let ref_len = ref_up.len();
    let read_len = read_up.len();
    let max_pos = ref_len - read_len;

    let mut best: Option<(usize, bool, usize)> = None;

    for i in 0..=max_pos {
        let fwd_mismatches = (0..read_len)
            .filter(|&j| read_up.as_bytes()[j] != ref_up.as_bytes()[i + j])
            .count();

        let rev_mismatches = (0..read_len)
            .filter(|&j| rc_read.as_bytes()[j] != ref_up.as_bytes()[i + j])
            .count();

        if fwd_mismatches <= max_mismatches {
            let cand = (fwd_mismatches, false, i);
            if best.is_none_or(|b| cand < b) {
                best = Some(cand);
            }
        }

        if rev_mismatches <= max_mismatches {
            let cand = (rev_mismatches, true, i);
            if best.is_none_or(|b| cand < b) {
                best = Some(cand);
            }
        }
    }

    best.map(|(m, r, p)| Mapping {
        position: p,
        reverse: r,
        mismatches: m,
    })
}

/// Bildet alle Reads ab (Ergebnis hat die Länge von `reads`).
pub fn map_reads(reference: &str, reads: &[String], max_mismatches: usize) -> Vec<Option<Mapping>> {
    reads
        .iter()
        .map(|r| map_read(reference, r.as_str(), max_mismatches))
        .collect()
}

/// Konsens und Tiefe: Für jede Referenzposition werden die Basen aller abgebildeten Reads gezählt (bei `reverse`
/// ist es das Reverskomplement des Reads; Basen außer A, C, G, T zählen nicht). Konsensbase = häufigste Base;
/// bei Gleichstand die Referenzbase, falls sie unter den Gleichständen ist, sonst die in der Reihenfolge A, C, G, T erste.
/// Ohne Abdeckung gilt die Referenzbase. Rückgabe: (Konsens in Uppercase, Tiefe je Position = Zahl der gezählten Basen).
/// `reads` und `mappings` sind gleich lang; Einträge `None` werden übersprungen.
pub fn pileup_consensus(
    reference: &str,
    reads: &[String],
    mappings: &[Option<Mapping>],
) -> (String, Vec<usize>) {
    const BASES: [u8; 4] = *b"ACGT";
    let reference = reference.to_ascii_uppercase();
    let ref_bytes = reference.as_bytes();
    let mut counts = vec![[0usize; 4]; ref_bytes.len()];
    for (read, mapping) in reads.iter().zip(mappings) {
        let Some(m) = mapping else { continue };
        let oriented = if m.reverse {
            iupac_reverse_complement(read)
        } else {
            read.clone()
        };
        for (k, b) in oriented.to_ascii_uppercase().bytes().enumerate() {
            let Some(slot) = counts.get_mut(m.position + k) else {
                break;
            };
            if let Some(idx) = BASES.iter().position(|&x| x == b) {
                slot[idx] += 1;
            }
        }
    }
    let mut consensus = String::with_capacity(ref_bytes.len());
    let mut depth = Vec::with_capacity(ref_bytes.len());
    for (pos, c) in counts.iter().enumerate() {
        let total: usize = c.iter().sum();
        depth.push(total);
        let max = c.iter().copied().max().unwrap_or(0);
        let ref_base = ref_bytes[pos];
        let ch = if total == 0 {
            ref_base
        } else {
            let tied: Vec<u8> = (0..4).filter(|&i| c[i] == max).map(|i| BASES[i]).collect();
            if tied.contains(&ref_base) {
                ref_base
            } else {
                tied[0]
            }
        };
        consensus.push(ch as char);
    }
    (consensus, depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REF: &str = "ACGTTGCAAGGCTTAACCGGATATCGCGATTAGC";

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn genau_vorwaerts_und_rueckwaerts() {
        let m = map_read(REF, "GGCTTAACCGG", 0).unwrap();
        assert_eq!(
            m,
            Mapping {
                position: 9,
                reverse: false,
                mismatches: 0
            }
        );
        let r = iupac_reverse_complement("GGCTTAACCGG");
        let m = map_read(REF, &r, 0).unwrap();
        assert_eq!(
            m,
            Mapping {
                position: 9,
                reverse: true,
                mismatches: 0
            }
        );
        assert_eq!(map_read(REF, "ggcttaaccgg", 0).unwrap().position, 9);
    }

    #[test]
    fn fehlpaarungen_und_nichts() {
        let m = map_read(REF, "GGCTAAACCGG", 1).unwrap();
        assert_eq!(
            m,
            Mapping {
                position: 9,
                reverse: false,
                mismatches: 1
            }
        );
        assert_eq!(map_read(REF, "GGCTAAACCGG", 0), None);
        assert_eq!(map_read(REF, "TTTTTTTT", 2), None);
        assert_eq!(map_read(REF, "", 2), None);
        assert_eq!(map_read("ACG", "ACGTT", 5), None);
    }

    #[test]
    fn mehrere_reads() {
        let reads = v(&["GGCTTAACCGG", "TTTTTTTT", "ATCGCGATTAGC"]);
        let m = map_reads(REF, &reads, 0);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].unwrap().position, 9);
        assert_eq!(m[1], None);
        assert_eq!(m[2].unwrap().position, 22);
    }

    #[test]
    fn konsens_und_tiefe() {
        let a = "GGCTTAACCGG";
        let b = "GGCTAAACCGG"; // an Index 4 ein A statt T (Referenzposition 13)
        let reads = v(&[a, b]);
        let maps = map_reads(REF, &reads, 1);
        let (cons, depth) = pileup_consensus(REF, &reads, &maps);
        assert_eq!(cons, REF); // Gleichstand T/A: Referenzbase T
        assert_eq!(depth[8], 0);
        assert_eq!(depth[9], 2);
        assert_eq!(depth[19], 2);
        assert_eq!(depth[20], 0);
        assert_eq!(depth.iter().sum::<usize>(), 22);

        let reads = v(&[b, b, a]);
        let maps = map_reads(REF, &reads, 1);
        let (cons, depth) = pileup_consensus(REF, &reads, &maps);
        assert_eq!(&cons[9..20], "GGCTAAACCGG");
        assert_eq!(&cons[..9], &REF[..9]);
        assert_eq!(depth[13], 3);
    }

    #[test]
    fn konsens_mit_reverskomplement_und_luecke() {
        let rev = iupac_reverse_complement("GGCTAAACCGG");
        let reads = v(&[&rev, "XX"]);
        let maps = vec![
            Some(Mapping {
                position: 9,
                reverse: true,
                mismatches: 1,
            }),
            None,
        ];
        let (cons, depth) = pileup_consensus(REF, &reads, &maps);
        assert_eq!(&cons[9..20], "GGCTAAACCGG");
        assert_eq!(depth.len(), REF.len());
        assert_eq!(depth[9], 1);
    }
}
