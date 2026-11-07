//! Quality report for FASTQ reads (FastQC-like), without storing the reads.

use std::collections::BTreeMap;

use crate::io::fastq::FastqRecord;

/// Result of the analysis. Qualities are Phred+33.
#[derive(Clone, Debug, PartialEq)]
pub struct QcReport {
    pub reads: u64,
    pub min_length: usize,
    pub max_length: usize,
    pub mean_length: f64,
    /// Fraction of G and C among all bases in percent.
    pub gc_percent: f64,
    /// Per position (0 to `max_length − 1`) over all reads that have this position: mean, lower quartile, median, upper quartile.
    pub mean_quality_per_position: Vec<f64>,
    pub q25_per_position: Vec<f64>,
    pub median_per_position: Vec<f64>,
    pub q75_per_position: Vec<f64>,
    /// Length → number of reads.
    pub length_distribution: BTreeMap<usize, u64>,
    /// `100 · (1 − unique / total)` over the reads considered.
    pub duplication_percent: f64,
    /// Fraction of reads (all reads) in percent that contain the first 12 bases of the adapter anywhere.
    pub adapter_percent: f64,
    /// Frequent sequences (first 50 bases) with count: fraction of the reads considered above 0.1 %, at most 10, descending by count,
    /// ties ascending by sequence.
    pub overrepresented: Vec<(String, u64)>,
}

/// Evaluates the reads.
/// * Quartile rule ("lower value"): for the sorted qualities `v` of a position (n values) the quantile `q` (0.25, 0.5, 0.75) is the
///   value at index `(n − 1) · q` rounded down; with integers `(n − 1) · 25 / 100`, `/ 2`, `(n − 1) · 75 / 100`. The median of `[0, 0, 40, 40]` is therefore 0.
///   Not all values should be stored: one count array per position for the Phred values 0 to 93 is enough.
/// * Quality character `c` gives Phred `c − 33` (clamped to 0 to 93).
/// * Duplication and overrepresented sequences: key = the first 50 bases (or the whole read if shorter) of the first
///   `max_dup_reads` reads; `unique` = number of distinct keys, `total` = number of these reads; 0 without reads.
/// * Adapter: the first 12 characters of `adapter` (or all if shorter); a read counts if its bases (uppercase) contain this word.
///   If `adapter` is empty, the fraction is 0.
/// * Without reads: everything 0, empty lists and distributions, `min_length = max_length = 0`; no division by 0.
pub fn analyze_reads<I: IntoIterator<Item = FastqRecord>>(
    reads: I,
    adapter: &str,
    max_dup_reads: usize,
) -> QcReport {
    let mut count = 0u64;
    let mut total_len = 0u64;
    let (mut min_len, mut max_len) = (usize::MAX, 0usize);
    let mut gc = 0u64;
    let mut bases = 0u64;
    let mut hist: Vec<[u64; 94]> = Vec::new();
    let mut lengths: BTreeMap<usize, u64> = BTreeMap::new();
    let mut dup_total = 0u64;
    let mut keys: BTreeMap<String, u64> = BTreeMap::new();
    let adapter_word: String = adapter.to_ascii_uppercase().chars().take(12).collect();
    let mut adapter_hits = 0u64;
    for r in reads {
        count += 1;
        let len = r.sequence.len();
        total_len += len as u64;
        min_len = min_len.min(len);
        max_len = max_len.max(len);
        *lengths.entry(len).or_insert(0) += 1;
        let upper = r.sequence.to_ascii_uppercase();
        gc += upper.bytes().filter(|b| matches!(b, b'G' | b'C')).count() as u64;
        bases += len as u64;
        if hist.len() < r.quality.len() {
            hist.resize(r.quality.len(), [0u64; 94]);
        }
        for (i, b) in r.quality.bytes().enumerate() {
            let q = (b as i32 - 33).clamp(0, 93) as usize;
            hist[i][q] += 1;
        }
        if (count as usize) <= max_dup_reads {
            dup_total += 1;
            let key: String = upper.chars().take(50).collect();
            *keys.entry(key).or_insert(0) += 1;
        }
        if !adapter_word.is_empty() && upper.contains(&adapter_word) {
            adapter_hits += 1;
        }
    }
    if count == 0 {
        return QcReport {
            reads: 0,
            min_length: 0,
            max_length: 0,
            mean_length: 0.0,
            gc_percent: 0.0,
            mean_quality_per_position: Vec::new(),
            q25_per_position: Vec::new(),
            median_per_position: Vec::new(),
            q75_per_position: Vec::new(),
            length_distribution: BTreeMap::new(),
            duplication_percent: 0.0,
            adapter_percent: 0.0,
            overrepresented: Vec::new(),
        };
    }
    let mut mean = Vec::new();
    let (mut q25, mut q50, mut q75) = (Vec::new(), Vec::new(), Vec::new());
    for h in &hist {
        let n: u64 = h.iter().sum();
        let sum: u64 = h.iter().enumerate().map(|(q, c)| q as u64 * c).sum();
        mean.push(if n == 0 { 0.0 } else { sum as f64 / n as f64 });
        let pick = |num: u64, den: u64| -> f64 {
            if n == 0 {
                return 0.0;
            }
            let idx = (n - 1) * num / den;
            let mut seen = 0u64;
            for (q, c) in h.iter().enumerate() {
                seen += c;
                if seen > idx {
                    return q as f64;
                }
            }
            0.0
        };
        q25.push(pick(25, 100));
        q50.push(pick(1, 2));
        q75.push(pick(75, 100));
    }
    let unique = keys.len() as f64;
    let duplication = if dup_total == 0 {
        0.0
    } else {
        100.0 * (1.0 - unique / dup_total as f64)
    };
    let mut over: Vec<(String, u64)> = keys
        .into_iter()
        .filter(|(_, c)| *c >= 2 && *c as f64 / dup_total as f64 > 0.001)
        .collect();
    over.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    over.truncate(10);
    QcReport {
        reads: count,
        min_length: min_len,
        max_length: max_len,
        mean_length: total_len as f64 / count as f64,
        gc_percent: if bases == 0 {
            0.0
        } else {
            100.0 * gc as f64 / bases as f64
        },
        mean_quality_per_position: mean,
        q25_per_position: q25,
        median_per_position: q50,
        q75_per_position: q75,
        length_distribution: lengths,
        duplication_percent: duplication,
        adapter_percent: 100.0 * adapter_hits as f64 / count as f64,
        overrepresented: over,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(seq: &str, qual: &str) -> FastqRecord {
        FastqRecord {
            id: "r".to_string(),
            comment: String::new(),
            sequence: seq.to_string(),
            quality: qual.to_string(),
        }
    }

    const AD: &str = "AGATCGGAAGAGC";

    #[test]
    fn einfache_kennzahlen() {
        let reads = vec![rec("ACGT", "IIII"); 4];
        let r = analyze_reads(reads, AD, 100_000);
        assert_eq!(r.reads, 4);
        assert_eq!((r.min_length, r.max_length), (4, 4));
        assert_eq!(r.mean_length, 4.0);
        assert_eq!(r.gc_percent, 50.0);
        assert_eq!(r.mean_quality_per_position, vec![40.0; 4]);
        assert_eq!(r.median_per_position, vec![40.0; 4]);
        assert_eq!(r.length_distribution.get(&4), Some(&4));
    }

    #[test]
    fn quartile_unterer_wert() {
        let reads = vec![rec("A", "!"), rec("A", "!"), rec("A", "I"), rec("A", "I")];
        let r = analyze_reads(reads, AD, 10);
        assert_eq!(r.mean_quality_per_position, vec![20.0]);
        assert_eq!(r.q25_per_position, vec![0.0]);
        assert_eq!(r.median_per_position, vec![0.0]);
        assert_eq!(r.q75_per_position, vec![40.0]);
    }

    #[test]
    fn laengen_und_positionen() {
        let reads = vec![
            rec("ACGT", "IIII"),
            rec("ACGT", "IIII"),
            rec("ACGT", "IIII"),
            rec("ACGTA", "IIII5"),
        ];
        let r = analyze_reads(reads, AD, 10);
        assert_eq!((r.min_length, r.max_length), (4, 5));
        assert_eq!(r.mean_length, 4.25);
        assert_eq!(r.length_distribution.get(&4), Some(&3));
        assert_eq!(r.length_distribution.get(&5), Some(&1));
        assert_eq!(r.mean_quality_per_position.len(), 5);
        // position 4 has only one value: '5' = 20
        assert_eq!(r.mean_quality_per_position[4], 20.0);
        assert_eq!(r.q25_per_position[4], 20.0);
    }

    #[test]
    fn duplikation_und_ueberrepraesentiert() {
        let reads = vec![rec("ACGTACGT", "IIIIIIII"); 10];
        let r = analyze_reads(reads, AD, 100);
        assert!((r.duplication_percent - 90.0).abs() < 1e-9);
        assert_eq!(r.overrepresented, vec![("ACGTACGT".to_string(), 10)]);
        // only the first 3 reads count
        let mut mixed = vec![rec("AAAA", "IIII"); 2];
        mixed.push(rec("CCCC", "IIII"));
        mixed.push(rec("GGGG", "IIII"));
        let r2 = analyze_reads(mixed, AD, 3);
        assert!((r2.duplication_percent - 100.0 * (1.0 - 2.0 / 3.0)).abs() < 1e-9);
        // with many different reads there is no overrepresented sequence
        let many: Vec<FastqRecord> = (0..2000)
            .map(|i| {
                let s: String = (0..12)
                    .map(|k| ['A', 'C', 'G', 'T'][(i / 4usize.pow(k % 6) + k as usize) % 4])
                    .collect();
                rec(&s, "IIIIIIIIIIII")
            })
            .collect();
        let r3 = analyze_reads(many, AD, 100_000);
        assert!(r3
            .overrepresented
            .iter()
            .all(|(_, n)| *n as f64 / 2000.0 > 0.001));
    }

    #[test]
    fn adapteranteil() {
        let mut reads = vec![rec("ACGTACGTACGT", "IIIIIIIIIIII"); 8];
        reads.push(rec("ACGTAGATCGGAAGAGCC", "IIIIIIIIIIIIIIIIII"));
        reads.push(rec("AGATCGGAAGAG", "IIIIIIIIIIII"));
        let r = analyze_reads(reads, AD, 100);
        assert!((r.adapter_percent - 20.0).abs() < 1e-9);
        let none = analyze_reads(vec![rec("ACGT", "IIII")], "", 10);
        assert_eq!(none.adapter_percent, 0.0);
    }

    #[test]
    fn leere_eingabe() {
        let r = analyze_reads(Vec::<FastqRecord>::new(), AD, 10);
        assert_eq!(r.reads, 0);
        assert_eq!((r.min_length, r.max_length), (0, 0));
        assert_eq!(r.mean_length, 0.0);
        assert_eq!(r.gc_percent, 0.0);
        assert_eq!(r.duplication_percent, 0.0);
        assert_eq!(r.adapter_percent, 0.0);
        assert!(r.mean_quality_per_position.is_empty());
        assert!(r.length_distribution.is_empty());
        assert!(r.overrepresented.is_empty());
    }
}
