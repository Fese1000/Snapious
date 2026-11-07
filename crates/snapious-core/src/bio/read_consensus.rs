//! Consensus from several aligned sequencing reads (e.g. Sanger reads) with quality values.

/// Consensus and quality per consensus base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadConsensus {
    /// Consensus sequence without gaps (uppercase).
    pub bases: String,
    /// Quality per base of `bases` (Phred-like, 0–93).
    pub qualities: Vec<u8>,
}

/// Builds the consensus from aligned rows `rows` (gap = `-`, case-insensitive). `quals[i]` holds, for each **non-gap base**
/// of row `i`, a quality value, in base order (gaps consume no value; if values are missing, quality 0 applies).
///
/// Per column, each base `A`, `C`, `G`, `T` from a row counts with its quality as weight, provided the quality is `>= min_quality`
/// (all other characters and bases below the threshold do not count). The base with the largest weight wins (on ties the first in the order
/// `A`, `C`, `G`, `T`). The quality of the consensus base is `weight(winner) - sum of the weights of the other bases`, at least 0 and at most 93.
/// If the column has no counting base but at least one non-gap base (e.g. `N` or quality too low), the consensus base is `N`
/// with quality 0. Columns in which all rows have a gap are dropped. All rows should have the same length (shorter rows count
/// as gaps at the end). No rows → empty result.
pub fn quality_consensus(rows: &[String], quals: &[Vec<u8>], min_quality: u8) -> ReadConsensus {
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let bytes: Vec<&[u8]> = rows.iter().map(|r| r.as_bytes()).collect();
    // per row: pointer to the next quality value (advances only on non-gap bases)
    let mut next_q = vec![0usize; rows.len()];
    let mut out = ReadConsensus {
        bases: String::new(),
        qualities: Vec::new(),
    };
    for col in 0..width {
        let mut weights = [0u32; 4];
        let mut any_base = false;
        for (r, row) in bytes.iter().enumerate() {
            let Some(&b) = row.get(col) else { continue };
            if b == b'-' {
                continue;
            }
            any_base = true;
            let q = quals
                .get(r)
                .and_then(|v| v.get(next_q[r]))
                .copied()
                .unwrap_or(0);
            next_q[r] += 1;
            if q < min_quality {
                continue;
            }
            if let Some(idx) = b"ACGT".iter().position(|x| *x == b.to_ascii_uppercase()) {
                weights[idx] += q as u32;
            }
        }
        if !any_base {
            continue;
        }
        let total: u32 = weights.iter().sum();
        if total == 0 {
            out.bases.push('N');
            out.qualities.push(0);
            continue;
        }
        // first index with the largest weight (order A, C, G, T)
        let mut best = 0;
        for i in 1..4 {
            if weights[i] > weights[best] {
                best = i;
            }
        }
        out.bases.push(b"ACGT"[best] as char);
        out.qualities
            .push((weights[best] as i64 - (total - weights[best]) as i64).clamp(0, 93) as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn gewichtet() {
        let rows = v(&["ACGT", "ACGT", "ACTT"]);
        let q = vec![
            vec![40, 40, 40, 40],
            vec![30, 30, 30, 30],
            vec![10, 10, 10, 10],
        ];
        let c = quality_consensus(&rows, &q, 0);
        assert_eq!(c.bases, "ACGT");
        assert_eq!(c.qualities, vec![80, 80, 60, 80]);
    }

    #[test]
    fn luecken_und_schwelle() {
        let rows = v(&["AC-T", "ACGT"]);
        let q = vec![vec![40, 40, 40], vec![20, 20, 20, 20]];
        let c = quality_consensus(&rows, &q, 0);
        assert_eq!(c.bases, "ACGT");
        assert_eq!(c.qualities, vec![60, 60, 20, 60]);
        let c = quality_consensus(&rows, &q, 25);
        assert_eq!(c.bases, "ACNT");
        assert_eq!(c.qualities, vec![40, 40, 0, 40]);
    }

    #[test]
    fn randfaelle() {
        assert_eq!(
            quality_consensus(&[], &[], 0),
            ReadConsensus {
                bases: String::new(),
                qualities: vec![]
            }
        );
        // only-gap column is dropped
        let c = quality_consensus(&v(&["A-C", "A-C"]), &[vec![10, 10], vec![10, 10]], 0);
        assert_eq!(c.bases, "AC");
        // tie: A before G
        let c = quality_consensus(&v(&["A", "G"]), &[vec![30], vec![30]], 0);
        assert_eq!((c.bases.as_str(), c.qualities[0]), ("A", 0));
        // quality at most 93
        let c = quality_consensus(&v(&["A", "A", "A"]), &[vec![60], vec![60], vec![60]], 0);
        assert_eq!(c.qualities, vec![93]);
        // missing values count 0, base N does not count
        let c = quality_consensus(&v(&["AN"]), &[vec![40]], 0);
        assert_eq!(c.bases, "AN");
    }
}
