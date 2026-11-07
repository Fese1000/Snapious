//! Fragment lengths (insert size) from paired alignments.

use crate::io::sam::SamRecord;

/// Summary of the lengths.
#[derive(Clone, Debug, PartialEq)]
pub struct InsertStats {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    /// Sample standard deviation (denominator `n - 1`); `0.0` for fewer than two values.
    pub std_dev: f64,
}

/// Fragment lengths: for every entry that is paired (flag `0x1`), is the first read of the pair (flag `0x40`), is aligned
/// (flag `0x4` not set), whose mate is aligned (flag `0x8` not set), is not a secondary entry
/// (flag `0x100` not set), has `mate_reference == "="` or equal to `reference`, and has `template_length != 0`,
/// `|template_length|` counts. Order as in the input.
pub fn insert_sizes(records: &[SamRecord]) -> Vec<u64> {
    records
        .iter()
        .filter(|r| {
            r.flag & 0x1 != 0
                && r.flag & 0x40 != 0
                && r.flag & 0x4 == 0
                && r.flag & 0x8 == 0
                && r.flag & 0x100 == 0
                && (r.mate_reference == "=" || r.mate_reference == r.reference)
                && r.template_length != 0
        })
        .map(|r| r.template_length.unsigned_abs())
        .collect()
}

/// Mean, median (for an even count the mean of the two middle values) and standard deviation. No values → all `0`.
pub fn insert_stats(sizes: &[u64]) -> InsertStats {
    if sizes.is_empty() {
        return InsertStats {
            count: 0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
        };
    }
    let n = sizes.len();
    let mean = sizes.iter().sum::<u64>() as f64 / n as f64;
    let mut sorted = sizes.to_vec();
    sorted.sort_unstable();
    let median = if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
    };
    let std_dev = if n < 2 {
        0.0
    } else {
        (sizes
            .iter()
            .map(|&s| (s as f64 - mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64)
            .sqrt()
    };
    InsertStats {
        count: n,
        mean,
        median,
        std_dev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(flag: u16, reference: &str, mate: &str, tlen: i64) -> SamRecord {
        SamRecord {
            read_name: "r".into(),
            flag,
            reference: reference.into(),
            position: 100,
            mapq: 60,
            cigar: "10M".into(),
            mate_reference: mate.into(),
            mate_position: 300,
            template_length: tlen,
            sequence: "ACGTACGTAC".into(),
            quality: "IIIIIIIIII".into(),
        }
    }

    #[test]
    fn auswahl() {
        let r = vec![
            rec(99, "chr1", "=", 300),
            rec(147, "chr1", "=", -300),
            rec(65, "chr1", "=", 0),
            rec(73, "chr1", "=", 250),
            rec(97, "chr1", "chr2", 500),
            rec(97, "chr1", "chr1", -420),
            rec(321, "chr1", "=", 200),
            rec(69, "chr1", "=", 150),
            rec(1, "chr1", "=", 700),
        ];
        assert_eq!(insert_sizes(&r), vec![300, 420]);
        assert!(insert_sizes(&[]).is_empty());
    }

    #[test]
    fn kennzahlen() {
        let s = insert_stats(&[100, 200, 300, 400]);
        assert_eq!(s.count, 4);
        assert!((s.mean - 250.0).abs() < 1e-9);
        assert!((s.median - 250.0).abs() < 1e-9);
        assert!((s.std_dev - 129.09944487358056).abs() < 1e-6);
        let e = insert_stats(&[]);
        assert_eq!((e.count, e.mean, e.median, e.std_dev), (0, 0.0, 0.0, 0.0));
        assert_eq!(insert_stats(&[7]).std_dev, 0.0);
        assert_eq!(insert_stats(&[5, 1, 9]).median, 5.0);
    }
}
