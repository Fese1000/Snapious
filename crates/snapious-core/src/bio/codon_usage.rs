//! Codon usage: counting, RSCU, GC3 and Codon Adaptation Index (CAI).

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::sequence::amino_acid_for;

/// Count per codon (uppercase, `T` instead of `U`), sorted ascending by codon.
pub type CodonCounts = BTreeMap<String, usize>;

/// Counts the codons in reading frame from position 0 in steps of three.
/// Case-insensitive, `U` counts as `T`. Codons with characters other than A, C, G, T are skipped
/// (the frame keeps advancing), an incomplete final codon is dropped. Stop codons are counted.
pub fn count_codons(bases: &str) -> CodonCounts {
    let mut counts = CodonCounts::new();
    for codon in valid_codons(bases) {
        *counts.entry(codon).or_insert(0) += 1;
    }
    counts
}

/// All complete codons of A, C, G, T in the frame from position 0.
fn valid_codons(bases: &str) -> Vec<String> {
    let text = bases.to_ascii_uppercase().replace('U', "T");
    text.as_bytes()
        .chunks(3)
        .filter(|c| c.len() == 3 && c.iter().all(|b| matches!(b, b'A' | b'C' | b'G' | b'T')))
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect()
}

/// All 64 codons of the standard code grouped by amino acid (stop = `*`), codons alphabetical.
pub fn codon_families() -> BTreeMap<char, Vec<String>> {
    let letters = ['A', 'C', 'G', 'T'];
    let mut map: BTreeMap<char, Vec<String>> = BTreeMap::new();
    for a in letters {
        for b in letters {
            for c in letters {
                let codon: String = [a, b, c].iter().collect();
                let aa = amino_acid_for(codon.as_bytes()) as char;
                map.entry(aa).or_default().push(codon);
            }
        }
    }
    map
}

/// Relative Synonymous Codon Usage: `count / mean count of the synonymous codons`.
/// Synonymous means: same amino acid according to `amino_acid_for` (stop codons form the family `*`).
/// The family has all its codons of the standard code (not just those in `counts`); missing ones count 0.
/// Families whose sum is 0 are omitted from the result entirely. A codon with count 0 in a family with sum > 0
/// appears with value 0.0. Families with only one codon (M, W) have the value 1.0.
pub fn rscu(counts: &CodonCounts) -> BTreeMap<String, f64> {
    let mut result = BTreeMap::new();
    for codons in codon_families().values() {
        let total: usize = codons
            .iter()
            .map(|c| counts.get(c).copied().unwrap_or(0))
            .sum();
        if total == 0 {
            continue;
        }
        let mean = total as f64 / codons.len() as f64;
        for c in codons {
            let n = counts.get(c).copied().unwrap_or(0) as f64;
            result.insert(c.clone(), n / mean);
        }
    }
    result
}

/// Fraction of G or C at the third position of the complete, valid codons in percent (0 to 100).
/// Valid as in `count_codons`. Without codons: 0.0.
pub fn gc3_percent(bases: &str) -> f64 {
    let codons = valid_codons(bases);
    if codons.is_empty() {
        return 0.0;
    }
    let gc = codons.iter().filter(|c| c.ends_with(['G', 'C'])).count();
    gc as f64 * 100.0 / codons.len() as f64
}

/// Codon Adaptation Index after Sharp & Li (1987): geometric mean of the weights `w` of all codons of the sequence.
/// - `w = max(count, 0.5) / largest count in the family` with the counts from `reference`.
/// - Not considered: ATG, TGG, stop codons and codons whose family has sum 0 in `reference`.
/// - Computed via the sum of logarithms (`ln`), finally `exp(sum / n)`.
///
/// Error `Error::InvalidInput` if no codon was considered.
pub fn codon_adaptation_index(bases: &str, reference: &CodonCounts) -> Result<f64> {
    let mut max_per_codon: BTreeMap<String, usize> = BTreeMap::new();
    for codons in codon_families().values() {
        let max = codons
            .iter()
            .map(|c| reference.get(c).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        for c in codons {
            max_per_codon.insert(c.clone(), max);
        }
    }
    let mut log_sum = 0.0;
    let mut used = 0usize;
    for codon in valid_codons(bases) {
        if codon == "ATG" || codon == "TGG" || amino_acid_for(codon.as_bytes()) == b'*' {
            continue;
        }
        let max = max_per_codon.get(&codon).copied().unwrap_or(0);
        if max == 0 {
            continue;
        }
        let count = reference.get(&codon).copied().unwrap_or(0) as f64;
        log_sum += (count.max(0.5) / max as f64).ln();
        used += 1;
    }
    if used == 0 {
        return Err(Error::InvalidInput(
            "No codons usable for the CAI".to_string(),
        ));
    }
    Ok((log_sum / used as f64).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn counts(pairs: &[(&str, usize)]) -> CodonCounts {
        pairs.iter().map(|(c, n)| (c.to_string(), *n)).collect()
    }

    #[test]
    fn codons_zaehlen() {
        let c = count_codons("ATGGCTGCCGCTTAA");
        assert_eq!(c, counts(&[("ATG", 1), ("GCT", 2), ("GCC", 1), ("TAA", 1)]));
    }

    #[test]
    fn zaehlen_sonderfaelle() {
        // lowercase, U, incomplete remainder
        assert_eq!(count_codons("augaaaGC"), counts(&[("ATG", 1), ("AAA", 1)]));
        // invalid codon in the middle is skipped, frame stays
        assert_eq!(count_codons("ATGNNNAAA"), counts(&[("ATG", 1), ("AAA", 1)]));
        assert!(count_codons("").is_empty());
        assert!(count_codons("AT").is_empty());
    }

    #[test]
    fn rscu_werte() {
        let r = rscu(&counts(&[("GCT", 3), ("GCC", 1), ("ATG", 5)]));
        assert!(close(r["GCT"], 3.0));
        assert!(close(r["GCC"], 1.0));
        assert!(close(r["GCA"], 0.0));
        assert!(close(r["GCG"], 0.0));
        assert!(close(r["ATG"], 1.0));
        assert!(!r.contains_key("AAA"));
        assert!(!r.contains_key("TGG"));
    }

    #[test]
    fn rscu_zwei_codon_familie_und_stopp() {
        let r = rscu(&counts(&[("AAA", 3), ("AAG", 1), ("TAA", 2)]));
        assert!(close(r["AAA"], 1.5));
        assert!(close(r["AAG"], 0.5));
        // Family * has 3 codons, sum 2 → mean 2/3
        assert!(close(r["TAA"], 3.0));
        assert!(close(r["TAG"], 0.0));
        assert!(close(r["TGA"], 0.0));
        assert!(rscu(&CodonCounts::new()).is_empty());
    }

    #[test]
    fn gc3() {
        assert!(close(gc3_percent("ATGGCC"), 100.0));
        assert!(close(gc3_percent("ATTGCA"), 0.0));
        assert!(close(gc3_percent("ATGAAA"), 50.0));
        assert!(close(gc3_percent(""), 0.0));
        assert!(close(gc3_percent("ATGNNNGCC"), 100.0));
    }

    #[test]
    fn cai_werte() {
        let r = counts(&[("GCT", 3), ("GCC", 1)]);
        assert!(close(codon_adaptation_index("GCTGCT", &r).unwrap(), 1.0));
        assert!(close(
            codon_adaptation_index("GCCGCC", &r).unwrap(),
            1.0 / 3.0
        ));
        assert!(close(
            codon_adaptation_index("GCTGCC", &r).unwrap(),
            (1.0f64 / 3.0).sqrt()
        ));
        // Codon with count 0 gets 0.5 / 3
        assert!(close(codon_adaptation_index("GCA", &r).unwrap(), 0.5 / 3.0));
    }

    #[test]
    fn cai_ausnahmen() {
        let r = counts(&[("GCT", 3), ("GCC", 1)]);
        // ATG, TGG and stop do not count
        assert!(close(
            codon_adaptation_index("ATGTGGGCTTAA", &r).unwrap(),
            1.0
        ));
        // Family K is missing in the reference → AAA is skipped
        assert!(close(codon_adaptation_index("AAAGCT", &r).unwrap(), 1.0));
        assert!(matches!(
            codon_adaptation_index("ATGTGGTAA", &r),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            codon_adaptation_index("AAA", &r),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            codon_adaptation_index("", &r),
            Err(Error::InvalidInput(_))
        ));
    }
}
