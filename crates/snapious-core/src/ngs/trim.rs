//! Read trimming: adapter at the 3' end and quality window (Phred+33).

use crate::io::fastq::FastqRecord;

/// Settings.
#[derive(Clone, Debug, PartialEq)]
pub struct TrimOptions {
    pub window_size: usize,
    pub window_mean_quality: f64,
    pub min_length: usize,
    pub adapter: String,
    pub adapter_min_overlap: usize,
    pub max_adapter_mismatches: usize,
}

impl TrimOptions {
    /// Default: window 4, mean 15, minimum length 36, adapter `AGATCGGAAGAG`, overlap 8, one mismatch.
    pub fn standard() -> TrimOptions {
        TrimOptions {
            window_size: 4,
            window_mean_quality: 15.0,
            min_length: 36,
            adapter: "AGATCGGAAGAG".to_string(),
            adapter_min_overlap: 8,
            max_adapter_mismatches: 1,
        }
    }
}

/// Summary of a run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrimStats {
    pub input_reads: u64,
    pub output_reads: u64,
    pub bases_removed: u64,
    pub adapters_found: u64,
}

/// Trims a read and returns `(result, adapter found)`; the result is `None` if the remainder is shorter than `min_length`.
/// 1. Adapter: for `p` from 0 on (earliest position first): `m = min(adapter length, read length − p)`; if `m < adapter_min_overlap`, the search stops
///    (no further position can match). Otherwise count Hamming mismatches between `bases[p..p+m]` and `adapter[..m]`
///    (uppercase, case-insensitive); `<= max_adapter_mismatches` → the read is cut at `p`,
///    adapter found. An empty adapter finds nothing.
/// 2. Quality: Phred = character − 33 (not below 0). Window of length `window_size` (at most read length; `window_size == 0` disables the step)
///    from the left: if the first window with mean `< window_mean_quality` starts at index `i`, the read is cut at `i`.
///    For a shorter read the whole read counts as one window.
/// 3. The remainder (sequence and quality cut to the same length) must be at least `min_length` long.
pub fn trim_record(r: &FastqRecord, o: &TrimOptions) -> (Option<FastqRecord>, bool) {
    let seq = r.sequence.to_uppercase();
    let qual = &r.quality;
    let read_len = seq.len();

    if read_len == 0 {
        return (None, false);
    }

    let mut cut_pos = read_len;
    let mut adapter_found = false;

    // 1. Adapter trimming
    if !o.adapter.is_empty() {
        let adapter = o.adapter.to_uppercase();
        let adapter_len = adapter.len();
        for p in 0..read_len {
            let m = std::cmp::min(adapter_len, read_len - p);
            if m < o.adapter_min_overlap {
                break;
            }
            let mut mismatches = 0usize;
            for k in 0..m {
                if seq.as_bytes().get(p + k).copied() != adapter.as_bytes().get(k).copied() {
                    mismatches += 1;
                }
            }
            if mismatches <= o.max_adapter_mismatches {
                cut_pos = p;
                adapter_found = true;
                break;
            }
        }
    }

    let mut seq_slice = &seq[..cut_pos];
    let mut qual_slice = &qual[..cut_pos];

    // 2. Quality trimming
    if o.window_size > 0 {
        let window = o.window_size;
        // For a shorter read the whole read counts as one window.
        let max_start = if seq_slice.len() < window {
            0
        } else {
            seq_slice.len() - window
        };
        for i in 0..=max_start {
            // Make sure the slice does not go past the end
            let end = std::cmp::min(i + window, qual_slice.len());
            let window_qual = &qual_slice[i..end];

            let mut sum_phred = 0.0;
            for b in window_qual.bytes() {
                sum_phred += (b as i32).saturating_sub(33) as f64;
            }
            // Mean over the actual length of the window (for shortened reads)
            let mean = sum_phred / window_qual.len() as f64;

            if mean < o.window_mean_quality {
                cut_pos = i;
                seq_slice = &seq[..cut_pos];
                qual_slice = &qual[..cut_pos];
                break;
            }
        }
    }

    // 3. Length check
    if seq_slice.len() < o.min_length {
        return (None, adapter_found);
    }

    (
        Some(FastqRecord {
            id: r.id.clone(),
            comment: r.comment.clone(),
            sequence: seq_slice.to_string(),
            quality: qual_slice.to_string(),
        }),
        adapter_found,
    )
}

/// Applies `trim_record` to all reads and returns the kept reads and the statistics
/// (`bases_removed` = removed bases of all reads; discarded reads count with their full length).
pub fn trim_all<I: IntoIterator<Item = FastqRecord>>(
    reads: I,
    o: &TrimOptions,
) -> (Vec<FastqRecord>, TrimStats) {
    let mut kept = Vec::new();
    let mut stats = TrimStats::default();

    for r in reads {
        stats.input_reads += 1;
        let original_len = r.sequence.len();

        match trim_record(&r, o) {
            (Some(rec), found) => {
                stats.output_reads += 1;
                if found {
                    stats.adapters_found += 1;
                }
                let removed = original_len - rec.sequence.len();
                stats.bases_removed += removed as u64;
                kept.push(rec);
            }
            (None, _) => {
                stats.bases_removed += original_len as u64;
            }
        }
    }

    (kept, stats)
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

    fn opts() -> TrimOptions {
        TrimOptions {
            min_length: 5,
            ..TrimOptions::standard()
        }
    }

    #[test]
    fn adapter_wird_abgeschnitten() {
        let r = rec(&format!("ACGTACGTACGT{}", "AGATCGGAAGAG"), &"I".repeat(24));
        let (out, found) = trim_record(&r, &opts());
        assert!(found);
        let out = out.unwrap();
        assert_eq!(out.sequence, "ACGTACGTACGT");
        assert_eq!(out.quality, "I".repeat(12));
    }

    #[test]
    fn mismatches_beim_adapter() {
        let one = rec("ACGTACGTACGTAGATCGCAAGAG", &"I".repeat(24));
        let two = rec("ACGTACGTACGTAGTTCGCAAGAG", &"I".repeat(24));
        let (a, f1) = trim_record(&one, &opts());
        assert!(f1);
        assert_eq!(a.unwrap().sequence, "ACGTACGTACGT");
        let (b, f2) = trim_record(&two, &opts());
        assert!(!f2);
        assert_eq!(b.unwrap().sequence.len(), 24);
    }

    #[test]
    fn qualitaetsfenster() {
        let r = rec("ACGTACGTACGT", "IIIIIIII!!!!");
        let (out, found) = trim_record(&r, &opts());
        assert!(!found);
        let out = out.unwrap();
        assert_eq!(out.sequence, "ACGTACG");
        assert_eq!(out.quality, "IIIIIII");
        // good quality: nothing changes
        let good = rec("ACGTACGTACGT", "IIIIIIIIIIII");
        assert_eq!(trim_record(&good, &opts()).0.unwrap(), good);
    }

    #[test]
    fn zu_kurz_wird_verworfen() {
        let r = rec("ACGTACGTACGT", "IIIIIIII!!!!");
        let o = TrimOptions {
            min_length: 8,
            ..opts()
        };
        assert!(trim_record(&r, &o).0.is_none());
        // short read: the whole read is one window
        let short = rec("ACG", "!!!");
        let o2 = TrimOptions {
            min_length: 1,
            ..opts()
        };
        assert!(trim_record(&short, &o2).0.is_none());
        let short_ok = rec("ACG", "III");
        assert_eq!(trim_record(&short_ok, &o2).0.unwrap().sequence, "ACG");
    }

    #[test]
    fn lauf_mit_statistik() {
        let reads = vec![
            rec(&format!("ACGTACGTACGT{}", "AGATCGGAAGAG"), &"I".repeat(24)),
            rec("ACGTACGTACGT", "IIIIIIII!!!!"),
            rec("ACGT", "IIII"),
            rec("ACGTACGTAC", "IIIIIIIIII"),
        ];
        let (kept, stats) = trim_all(reads, &opts());
        assert_eq!(kept.len(), 3);
        assert_eq!(stats.input_reads, 4);
        assert_eq!(stats.output_reads, 3);
        assert_eq!(stats.adapters_found, 1);
        // 12 + 5 + 4 (discarded) + 0
        assert_eq!(stats.bases_removed, 21);
    }
}
