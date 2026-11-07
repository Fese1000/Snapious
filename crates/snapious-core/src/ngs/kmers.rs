//! k-mer frequencies (e.g. for detecting adapters or contaminants).

use std::collections::BTreeMap;

/// Counts all k-mers of length `k` in all sequences (uppercase; case-insensitive). k-mers containing a character
/// other than A, C, G, T are not counted. `k == 0` or sequences shorter than `k` contribute nothing.
pub fn count_kmers(seqs: &[String], k: usize) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    if k == 0 {
        return out;
    }
    for seq in seqs {
        let up = seq.to_ascii_uppercase();
        for w in up.as_bytes().windows(k) {
            if w.iter().all(|b| matches!(b, b'A' | b'C' | b'G' | b'T')) {
                *out.entry(String::from_utf8_lossy(w).to_string())
                    .or_insert(0) += 1;
            }
        }
    }
    out
}

/// The `n` most frequent k-mers as `(k-mer, count)`, descending by count, ties alphabetically ascending.
pub fn top_kmers(counts: &BTreeMap<String, usize>, n: usize) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = counts.iter().map(|(k, c)| (k.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn zaehlen() {
        let c = count_kmers(&v(&["ACGT", "CGTA"]), 2);
        assert_eq!(c.get("AC"), Some(&1));
        assert_eq!(c.get("CG"), Some(&2));
        assert_eq!(c.get("GT"), Some(&2));
        assert_eq!(c.get("TA"), Some(&1));
        assert_eq!(c.len(), 4);
        assert_eq!(
            top_kmers(&c, 2),
            vec![("CG".to_string(), 2), ("GT".to_string(), 2)]
        );
        assert_eq!(top_kmers(&c, 10).len(), 4);
    }

    #[test]
    fn randfaelle() {
        assert!(count_kmers(&v(&["ACGT"]), 0).is_empty());
        assert!(count_kmers(&v(&["AC"]), 3).is_empty());
        let n = count_kmers(&v(&["ACNGT"]), 2);
        assert_eq!(n.len(), 2);
        assert_eq!(n.get("AC"), Some(&1));
        assert_eq!(n.get("GT"), Some(&1));
        let k = count_kmers(&v(&["acgt"]), 4);
        assert_eq!(k.get("ACGT"), Some(&1));
        assert!(top_kmers(&BTreeMap::new(), 3).is_empty());
    }
}
