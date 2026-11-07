//! Map reads onto a gap-free reference and build a consensus from the coverage.

use crate::cloning::enzymes::iupac_reverse_complement;

/// Location of a mapped read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mapping {
    /// Start (0-based) in the reference.
    pub position: usize,
    /// `true` if the reverse complement of the read matches.
    pub reverse: bool,
    /// Number of mismatches.
    pub mismatches: usize,
}

/// Maps `read` gaplessly onto `reference` (uppercase, A/C/G/T; case-insensitive).
/// All positions are checked for the read forward and for its reverse complement (`iupac_reverse_complement`).
/// The winner is the position with the fewest mismatches, `<= max_mismatches`; on ties: forward before
/// reverse, then the smaller position. `None` if nothing matches, or the read is empty or longer than the reference.
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

/// Maps all reads (the result has the length of `reads`).
pub fn map_reads(reference: &str, reads: &[String], max_mismatches: usize) -> Vec<Option<Mapping>> {
    reads
        .iter()
        .map(|r| map_read(reference, r.as_str(), max_mismatches))
        .collect()
}

/// Consensus and depth: for each reference position, the bases of all mapped reads are counted (with `reverse`
/// it is the reverse complement of the read; bases other than A, C, G, T do not count). Consensus base = most frequent base;
/// on ties the reference base if it is among the tied ones, otherwise the first in the order A, C, G, T.
/// Without coverage the reference base applies. Returns: (consensus in uppercase, depth per position = number of counted bases).
/// `reads` and `mappings` have the same length; `None` entries are skipped.
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
        let b = "GGCTAAACCGG"; // an A instead of T at index 4 (reference position 13)
        let reads = v(&[a, b]);
        let maps = map_reads(REF, &reads, 1);
        let (cons, depth) = pileup_consensus(REF, &reads, &maps);
        assert_eq!(cons, REF); // Tie T/A: reference base T
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
