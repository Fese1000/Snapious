//! Simple assembly of reads into contigs (greedy overlap approach, exact overlaps).

use crate::cloning::enzymes::iupac_reverse_complement;

/// Placement of a read in a contig.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadPlacement {
    /// Start (0-based) of the read in the contig.
    pub offset: usize,
    /// `true` if the reverse complement of the read lies in the contig.
    pub reverse: bool,
}

/// Assembles reads (uppercase, only A, C, G, T) into contigs.
///
/// 1. Remove every sequence that is a substring (forward or reverse-complement) of another sequence.
///    For equal length (identical or equal as reverse complement) only the sequence with the smaller index stays.
/// 2. Repeat: among all ordered pairs `(i, j)` with `i != j` and both orientations of `j`
///    (`o = 0` forward, `o = 1` reverse complement), find the longest overlap `L >= min_overlap` where the end of `contigs[i]`
///    equals the start of `j'` (the oriented `contigs[j]`) and `L` is smaller than both lengths. On equal length the
///    pair with the smaller `i` wins, then the smaller `j`, then `o = 0`. Replace both by `contigs[i] + &j'[L..]` (new element at the end)
///    and then apply step 1 again. With no matching pair: done.
/// 3. Sort the result by length descending, for equal length by text ascending.
///
/// Empty input gives an empty list. `min_overlap == 0` is treated as 1.
pub fn assemble_contigs(reads: &[String], min_overlap: usize) -> Vec<String> {
    let min_ov = min_overlap.max(1);
    let mut contigs = drop_contained(reads.to_vec());
    // (overlap, i, j, orientation)
    while let Some((len, i, j, rev)) = best_overlap(&contigs, min_ov) {
        let other = if rev {
            iupac_reverse_complement(&contigs[j])
        } else {
            contigs[j].clone()
        };
        let merged = format!("{}{}", contigs[i], &other[len..]);
        let mut next: Vec<String> = contigs
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != i && *k != j)
            .map(|(_, c)| c.clone())
            .collect();
        next.push(merged);
        contigs = drop_contained(next);
    }
    contigs.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    contigs
}

/// Removes sequences contained in another (forward or reverse-complement); for equal length the smaller index stays.
fn drop_contained(items: Vec<String>) -> Vec<String> {
    let mut keep = Vec::new();
    for (i, c) in items.iter().enumerate() {
        let rc = iupac_reverse_complement(c);
        let contained = items.iter().enumerate().any(|(j, d)| {
            i != j
                && (d.contains(c.as_str()) || d.contains(rc.as_str()))
                && (c.len() < d.len() || (c.len() == d.len() && j < i))
        });
        if !contained {
            keep.push(c.clone());
        }
    }
    keep
}

/// Longest overlap over all pairs; on ties the smallest `i`, then `j`, then forward.
fn best_overlap(contigs: &[String], min_ov: usize) -> Option<(usize, usize, usize, bool)> {
    let mut best: Option<(usize, usize, usize, bool)> = None;
    for (i, a) in contigs.iter().enumerate() {
        for (j, b) in contigs.iter().enumerate() {
            if i == j {
                continue;
            }
            let rc = iupac_reverse_complement(b);
            for (rev, other) in [(false, b), (true, &rc)] {
                let max = a.len().min(other.len()).saturating_sub(1);
                let found = (min_ov..=max).rev().find(|&l| a.ends_with(&other[..l]));
                if let Some(l) = found {
                    if best.is_none_or(|(bl, ..)| l > bl) {
                        best = Some((l, i, j, rev));
                    }
                }
            }
        }
    }
    best
}

/// Looks for each read in the contig: first forward (first occurrence), otherwise as reverse complement (first occurrence).
/// `None` if the read does not occur. The list has the length of `reads`.
pub fn place_reads(contig: &str, reads: &[String]) -> Vec<Option<ReadPlacement>> {
    reads
        .iter()
        .map(|r| {
            if let Some(offset) = contig.find(r.as_str()) {
                return Some(ReadPlacement {
                    offset,
                    reverse: false,
                });
            }
            contig
                .find(iupac_reverse_complement(r).as_str())
                .map(|offset| ReadPlacement {
                    offset,
                    reverse: true,
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: &str = "ATGGCTAGCTAGGATCCGATCGTACGTTAGCAAGCTTGCATGCCTGCAGGTCGACTCTAGAGGATCC";

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn kette_vorwaerts() {
        let reads = v(&[&G[0..25], &G[15..40], &G[30..55], &G[45..]]);
        assert_eq!(assemble_contigs(&reads, 8), vec![G.to_string()]);
    }

    #[test]
    fn read_reverskomplementaer() {
        let rev = iupac_reverse_complement(&G[15..40]);
        let reads = v(&[&G[0..25], &rev, &G[30..55], &G[45..]]);
        assert_eq!(assemble_contigs(&reads, 8), vec![G.to_string()]);
        let placed = place_reads(G, &reads);
        assert_eq!(
            placed,
            vec![
                Some(ReadPlacement {
                    offset: 0,
                    reverse: false
                }),
                Some(ReadPlacement {
                    offset: 15,
                    reverse: true
                }),
                Some(ReadPlacement {
                    offset: 30,
                    reverse: false
                }),
                Some(ReadPlacement {
                    offset: 45,
                    reverse: false
                }),
            ]
        );
    }

    #[test]
    fn enthaltene_reads_und_duplikate() {
        let reads = v(&[&G[0..25], &G[5..20], &G[15..40]]);
        assert_eq!(assemble_contigs(&reads, 8), vec![G[0..40].to_string()]);
        let dup = v(&[&G[..20], &G[..20], &iupac_reverse_complement(&G[..20])]);
        assert_eq!(assemble_contigs(&dup, 5), vec![G[..20].to_string()]);
    }

    #[test]
    fn zwei_contigs_und_zu_grosse_ueberlappung() {
        let reads = v(&[
            &G[0..25],
            &G[15..40],
            "CCCCCCCCCCGGGGGGGGGGAAAAATTTTT",
            "GGGGGGGGGGAAAAATTTTTCCCCAA",
        ]);
        assert_eq!(
            assemble_contigs(&reads, 8),
            vec![
                G[0..40].to_string(),
                "CCCCCCCCCCGGGGGGGGGGAAAAATTTTTCCCCAA".to_string()
            ]
        );
        let chain = v(&[&G[0..25], &G[15..40], &G[30..55], &G[45..]]);
        assert_eq!(
            assemble_contigs(&chain, 30),
            vec![
                G[0..25].to_string(),
                G[30..55].to_string(),
                G[15..40].to_string(),
                G[45..].to_string()
            ]
        );
        assert!(assemble_contigs(&[], 5).is_empty());
    }

    #[test]
    fn platzierung_fehlt() {
        let p = place_reads("ACGTACGTAA", &v(&["CGTA", "TTAC", "GGGG"]));
        assert_eq!(
            p[0],
            Some(ReadPlacement {
                offset: 1,
                reverse: false
            })
        );
        // TTAC = reverse complement of GTAA: located from position 6
        assert_eq!(
            p[1],
            Some(ReadPlacement {
                offset: 6,
                reverse: true
            })
        );
        assert_eq!(p[2], None);
    }
}
