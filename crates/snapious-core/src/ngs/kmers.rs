//! k-mer-Häufigkeiten (z. B. zum Erkennen von Adaptern oder Verunreinigungen).

use std::collections::BTreeMap;

/// Zählt alle k-mer der Länge `k` in allen Sequenzen (Uppercase; Groß-/Kleinschreibung egal). k-mer, die ein anderes Zeichen
/// als A, C, G, T enthalten, werden nicht gezählt. `k == 0` oder Sequenzen kürzer als `k` tragen nichts bei.
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

/// Die `n` häufigsten k-mer als `(k-mer, Anzahl)`, absteigend nach Anzahl, bei Gleichstand alphabetisch aufsteigend.
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
