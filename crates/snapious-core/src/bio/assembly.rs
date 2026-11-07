//! Einfacher Zusammenbau von Reads zu Contigs (gieriger Überlappungsansatz, genaue Überlappungen).

use crate::cloning::enzymes::iupac_reverse_complement;

/// Platzierung eines Reads in einem Contig.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadPlacement {
    /// Beginn (0-basiert) des Reads im Contig.
    pub offset: usize,
    /// `true`, wenn das Reverskomplement des Reads im Contig steht.
    pub reverse: bool,
}

/// Setzt Reads (Uppercase, nur A, C, G, T) zu Contigs zusammen.
///
/// 1. Entferne jede Sequenz, die als Teilstring (vorwärts oder reverskomplementär) in einer anderen Sequenz steckt.
///    Bei gleicher Länge (identisch oder reverskomplementär gleich) bleibt nur die Sequenz mit dem kleineren Index.
/// 2. Wiederhole: Suche unter allen geordneten Paaren `(i, j)` mit `i != j` und beiden Ausrichtungen von `j`
///    (`o = 0` vorwärts, `o = 1` reverskomplementär) die längste Überlappung `L >= min_overlap`, bei der das Ende von `contigs[i]`
///    gleich dem Anfang von `j'` (das ausgerichtete `contigs[j]`) ist, und `L` kleiner als beide Längen ist. Bei gleicher Länge gewinnt
///    das Paar mit kleinerem `i`, dann kleinerem `j`, dann `o = 0`. Ersetze beide durch `contigs[i] + &j'[L..]` (neues Element ans Ende)
///    und wende danach Schritt 1 erneut an. Ohne passendes Paar: Ende.
/// 3. Sortiere das Ergebnis nach Länge absteigend, bei gleicher Länge nach dem Text aufsteigend.
///
/// Leere Eingabe ergibt eine leere Liste. `min_overlap == 0` wird wie 1 behandelt.
pub fn assemble_contigs(reads: &[String], min_overlap: usize) -> Vec<String> {
    let min_ov = min_overlap.max(1);
    let mut contigs = drop_contained(reads.to_vec());
    // (Überlappung, i, j, Ausrichtung)
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

/// Entfernt Sequenzen, die in einer anderen (vorwärts oder reverskomplementär) enthalten sind; bei gleicher Länge bleibt der kleinere Index.
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

/// Längste Überlappung über alle Paare; bei Gleichstand kleinstes `i`, dann `j`, dann vorwärts.
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

/// Sucht jeden Read im Contig: zuerst vorwärts (erstes Vorkommen), sonst als Reverskomplement (erstes Vorkommen).
/// `None`, wenn der Read nicht vorkommt. Die Liste hat die Länge von `reads`.
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
        // TTAC = Reverskomplement von GTAA: steht ab Position 6
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
