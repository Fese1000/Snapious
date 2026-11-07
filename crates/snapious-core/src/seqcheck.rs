//! Sequencing check ("Align to Reference"): align a read against a reference and report differences.

use crate::bio::align::Aligner;
use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};

/// Kind of difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DifferenceType {
    Mismatch,
    Insertion,
    Deletion,
    LowQuality,
}

/// A difference between read and reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    pub kind: DifferenceType,
    /// Position in the reference (0-based). For an insertion the position of the next reference base; for a circular reference modulo length.
    pub reference_position: usize,
    /// Position in the aligned read (in the reverse-complemented read when `reversed`), 0-based. For a deletion
    /// the index of the next read base.
    pub read_position: usize,
    /// Reference base, empty for an insertion.
    pub expected: String,
    /// Read base, empty for a deletion.
    pub observed: String,
    /// Quality of the read base (40 for a deletion).
    pub quality: u8,
}

/// Result of the check.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckResult {
    pub read_name: String,
    /// `true` if the reverse-complemented read fits better.
    pub reversed: bool,
    /// Aligned reference range: start and end (exclusive). For a circular reference `reference_end < reference_start` is possible
    /// (range across the origin); if the end lies exactly at the length, it is the length.
    pub reference_start: usize,
    pub reference_end: usize,
    /// Fraction of columns with identical bases among all alignment columns, in percent.
    pub identity: f64,
    /// Sorted by `reference_position`, then `read_position`, then kind (order of the variants of `DifferenceType`).
    pub differences: Vec<Difference>,
    pub aligned_read: String,
    pub aligned_reference: String,
}

/// Setting: bases with quality below `min_quality` are additionally reported as `LowQuality`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequencingCheck {
    pub min_quality: u8,
}

impl Default for SequencingCheck {
    /// `min_quality` 20.
    fn default() -> Self {
        SequencingCheck { min_quality: 20 }
    }
}

impl SequencingCheck {
    pub fn new(min_quality: u8) -> SequencingCheck {
        SequencingCheck { min_quality }
    }

    /// Aligns `read` against `reference`.
    ///
    /// 1. Error `Error::InvalidInput` for an empty reference, an empty read, or if `quality` is neither empty nor as long as `read`.
    ///    An empty `quality` counts as "40 everywhere". All bases are read in uppercase with `T` instead of `U`.
    /// 2. The read and its reverse complement (`iupac_reverse_complement`, qualities reversed) are aligned with
    ///    `Aligner::new(2, -3, -5).semi_global(read, reference)` (result: `seq1_aln` = read, `seq2_aln` = reference;
    ///    range in the reference: `seq2_start..seq2_end`). For a circular reference `reference + reference` is used. The
    ///    direction with the higher `score` wins (forward on a tie); `reversed` shows whether it was the reverse-complemented read.
    /// 3. Circular: `reference_start = seq2_start % length`, `reference_end = seq2_end − length` if `seq2_end > length`, otherwise `seq2_end`
    ///    (an end exactly at the length stays the length); positions of the differences are given modulo length.
    ///    Linear: `reference_start = seq2_start`, `reference_end = seq2_end`.
    /// 4. Column by column through the alignment (tracking reference index and read index):
    ///    - both characters present and different → `Mismatch` (`expected` reference base, `observed` read base);
    ///    - gap in the reference, read base present → `Insertion` (`reference_position` = index of the next reference base);
    ///    - gap in the read, reference base present → `Deletion` (`read_position` = index of the next read base, `quality` 40).
    ///
    ///    Additionally, for every read base with quality `< min_quality` (also on a match and on insertions) a
    ///    `LowQuality` entry (`expected` reference base or empty for an insertion, `observed` read base, `quality` of the base).
    ///
    /// 5. `identity` = columns with identical characters ÷ all columns × 100.
    pub fn check(
        &self,
        reference: &str,
        circular: bool,
        read_name: &str,
        read: &str,
        quality: &[u8],
    ) -> Result<CheckResult> {
        if reference.is_empty() || read.is_empty() {
            return Err(Error::InvalidInput(
                "Reference or read must not be empty".into(),
            ));
        }
        if !quality.is_empty() && quality.len() != read.len() {
            return Err(Error::InvalidInput(
                "Quality vector must be empty or the same length as the read".into(),
            ));
        }

        let ref_len = reference.len();
        let read_norm = read.to_uppercase().replace('U', "T");
        let ref_norm = reference.to_uppercase().replace('U', "T");

        // normalize qualities: empty means 40 for all
        let quals: Vec<u8> = if quality.is_empty() {
            vec![40u8; read.len()]
        } else {
            quality.to_vec()
        };

        // prepare reverse qualities (in case the reverse-complemented read wins)
        let rc_quals: Vec<u8> = quals.iter().rev().cloned().collect();

        let aligner = Aligner::new(2, -3, -5);

        // forward alignment
        let aln_ref = if circular {
            format!("{}{}", ref_norm, ref_norm)
        } else {
            ref_norm.clone()
        };

        let fwd = aligner.semi_global(&read_norm, &aln_ref)?;

        // reverse alignment: the read is reverse-complemented, qualities are reversed
        let rc_read = iupac_reverse_complement(&read_norm);
        let bwd = aligner.semi_global(&rc_read, &aln_ref)?;

        // decide which alignment is better (higher score wins, forward on a tie)
        let (best_fwd, reversed) = if fwd.score >= bwd.score {
            (fwd, false)
        } else {
            (bwd, true)
        };

        let seq1_aln = best_fwd.seq1_aln;
        let seq2_aln = best_fwd.seq2_aln;

        // compute the reference range
        let (reference_start, reference_end) = if circular {
            let s = best_fwd.seq2_start;
            let e = best_fwd.seq2_end;
            (s % ref_len, if e > ref_len { e - ref_len } else { e })
        } else {
            (best_fwd.seq2_start, best_fwd.seq2_end)
        };

        let mut differences: Vec<Difference> = Vec::new();
        let mut matching_cols = 0usize;
        // total alignment length (number of columns)
        let total_cols = seq1_aln.len();

        // indices into the original sequences (before alignment)
        let mut orig_read_idx = 0usize;
        let mut orig_ref_idx = best_fwd.seq2_start;

        // the relevant qualities based on the chosen direction
        let processing_quals = if reversed { &rc_quals } else { &quals };

        // iterate through the alignment
        for (c1, c2) in seq1_aln.chars().zip(seq2_aln.chars()) {
            let is_gap1 = c1 == '-';
            let is_gap2 = c2 == '-';

            if !is_gap1 && !is_gap2 {
                // both characters present (match or mismatch)
                if c1 == c2 {
                    matching_cols += 1;
                } else {
                    let ref_pos = if circular {
                        orig_ref_idx % ref_len
                    } else {
                        orig_ref_idx
                    };
                    let q = processing_quals.get(orig_read_idx).copied().unwrap_or(40);
                    differences.push(Difference {
                        kind: DifferenceType::Mismatch,
                        reference_position: ref_pos,
                        read_position: orig_read_idx,
                        expected: c2.to_string(),
                        observed: c1.to_string(),
                        quality: q,
                    });
                }

                // quality check for the LowQuality entry
                let q = processing_quals.get(orig_read_idx).copied().unwrap_or(40);
                if q < self.min_quality {
                    differences.push(Difference {
                        kind: DifferenceType::LowQuality,
                        reference_position: if circular {
                            orig_ref_idx % ref_len
                        } else {
                            orig_ref_idx
                        },
                        read_position: orig_read_idx,
                        expected: c2.to_string(),
                        observed: c1.to_string(),
                        quality: q,
                    });
                }

                orig_read_idx += 1;
                orig_ref_idx += 1;
            } else if is_gap1 && !is_gap2 {
                // gap in the read (deletion relative to the read, or insertion in the read?)
                // Per the specification: "gap in the read, reference base present → Deletion"
                // That means the reference has a base, the read does not.
                let ref_pos = if circular {
                    orig_ref_idx % ref_len
                } else {
                    orig_ref_idx
                };
                differences.push(Difference {
                    kind: DifferenceType::Deletion,
                    reference_position: ref_pos,
                    read_position: orig_read_idx, // index of the next read base (gap in the read)
                    expected: c2.to_string(),     // reference base
                    observed: String::new(),      // no read base
                    quality: 40,                  // default value for deletions
                });
                orig_ref_idx += 1;
            } else if !is_gap1 && is_gap2 {
                // gap in the reference (insertion in the read)
                let ref_pos = if circular {
                    orig_ref_idx % ref_len
                } else {
                    orig_ref_idx
                };
                let q = processing_quals.get(orig_read_idx).copied().unwrap_or(40);
                differences.push(Difference {
                    kind: DifferenceType::Insertion,
                    reference_position: ref_pos, // position of the next reference base (or the current one, since gap in ref)
                    read_position: orig_read_idx,
                    expected: String::new(),  // no reference base
                    observed: c1.to_string(), // read base
                    quality: q,
                });

                // quality check for LowQuality on an insertion
                if q < self.min_quality {
                    differences.push(Difference {
                        kind: DifferenceType::LowQuality,
                        reference_position: ref_pos,
                        read_position: orig_read_idx,
                        expected: String::new(), // no reference base on an insertion
                        observed: c1.to_string(), // read base
                        quality: q,
                    });
                }

                orig_read_idx += 1;
            } else {
                // both gaps (should not occur in a sensible alignment, but just to be safe)
                orig_read_idx += 1;
                orig_ref_idx += 1;
            }
        }

        // sort the differences per the specification
        differences.sort_by(|a, b| {
            a.reference_position
                .cmp(&b.reference_position)
                .then_with(|| a.read_position.cmp(&b.read_position))
                .then_with(|| a.kind.cmp(&b.kind))
        });

        // compute identity
        let identity = if total_cols == 0 {
            100.0
        } else {
            (matching_cols as f64 / total_cols as f64) * 100.0
        };

        Ok(CheckResult {
            read_name: read_name.to_string(),
            reversed,
            reference_start,
            reference_end,
            identity,
            differences,
            aligned_read: seq1_aln,
            aligned_reference: seq2_aln,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic, non-repeating test sequence.
    fn seq(n: usize, seed: u64) -> String {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ['A', 'C', 'G', 'T'][((s >> 33) % 4) as usize]
            })
            .collect()
    }

    fn other_base(not: &[char]) -> char {
        ['A', 'C', 'G', 'T']
            .into_iter()
            .find(|b| !not.contains(b))
            .unwrap()
    }

    fn rc(s: &str) -> String {
        iupac_reverse_complement(s)
    }

    fn reference() -> String {
        seq(100, 11)
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn exakter_read() {
        let r = reference();
        let read = r[10..40].to_string();
        let res = SequencingCheck::default()
            .check(&r, false, "r1", &read, &[])
            .unwrap();
        assert_eq!(res.read_name, "r1");
        assert!(!res.reversed);
        assert_eq!((res.reference_start, res.reference_end), (10, 40));
        assert!((res.identity - 100.0).abs() < 1e-9);
        assert!(res.differences.is_empty());
        assert_eq!(res.aligned_read, read);
        assert_eq!(res.aligned_reference, read);
    }

    #[test]
    fn austausch_und_rueckwaerts() {
        let r = reference();
        let mut read = chars(&r[10..40]);
        let alt = other_base(&[read[8]]);
        let old = read[8];
        read[8] = alt;
        let read: String = read.into_iter().collect();
        let res = SequencingCheck::default()
            .check(&r, false, "r", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::Mismatch);
        assert_eq!((d.reference_position, d.read_position), (18, 8));
        assert_eq!(d.expected, old.to_string());
        assert_eq!(d.observed, alt.to_string());
        assert!((res.identity - 29.0 / 30.0 * 100.0).abs() < 1e-9);
        // Reverse: the same difference, read position in the aligned (reverse-complemented) read
        let rev = SequencingCheck::default()
            .check(&r, false, "r", &rc(&read), &[])
            .unwrap();
        assert!(rev.reversed);
        assert_eq!(rev.differences, res.differences);
        assert_eq!((rev.reference_start, rev.reference_end), (10, 40));
    }

    #[test]
    fn einfuegung_und_loeschung() {
        let r = reference();
        let rc_chars = chars(&r);
        // Insertion between reference bases 24 and 25 (read index 15): a base that is neither of the neighbors
        let ins = other_base(&[rc_chars[24], rc_chars[25]]);
        let read = format!("{}{}{}", &r[10..25], ins, &r[25..40]);
        let res = SequencingCheck::default()
            .check(&r, false, "ins", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::Insertion);
        assert_eq!((d.reference_position, d.read_position), (25, 15));
        assert_eq!(
            (d.expected.as_str(), d.observed.as_str()),
            ("", ins.to_string().as_str())
        );
        assert!((res.identity - 30.0 / 31.0 * 100.0).abs() < 1e-9);
        // Deletion: a reference base that differs from its neighbors
        let p = (30..60)
            .find(|&p| rc_chars[p] != rc_chars[p - 1] && rc_chars[p] != rc_chars[p + 1])
            .unwrap();
        let read = format!("{}{}", &r[20..p], &r[p + 1..p + 20]);
        let res = SequencingCheck::default()
            .check(&r, false, "del", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::Deletion);
        assert_eq!((d.reference_position, d.read_position), (p, p - 20));
        assert_eq!(
            (d.expected.clone(), d.observed.clone()),
            (rc_chars[p].to_string(), String::new())
        );
        assert_eq!(d.quality, 40);
    }

    #[test]
    fn niedrige_qualitaet() {
        let r = reference();
        let read = r[10..40].to_string();
        let mut q = vec![40u8; 30];
        q[3] = 10;
        let res = SequencingCheck::default()
            .check(&r, false, "q", &read, &q)
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::LowQuality);
        assert_eq!(
            (d.reference_position, d.read_position, d.quality),
            (13, 3, 10)
        );
        assert_eq!(d.observed, r[13..14].to_string());
        assert_eq!(d.expected, r[13..14].to_string());
        // own threshold: 10 is not "below 10"
        assert!(SequencingCheck::new(10)
            .check(&r, false, "q", &read, &q)
            .unwrap()
            .differences
            .is_empty());
        // reverse: qualities are flipped too
        let mut qr = vec![40u8; 30];
        qr[29 - 3] = 10;
        let rev = SequencingCheck::default()
            .check(&r, false, "q", &rc(&read), &qr)
            .unwrap();
        assert!(rev.reversed);
        assert_eq!(rev.differences, res.differences);
    }

    #[test]
    fn austausch_mit_niedriger_qualitaet_gibt_zwei_eintraege() {
        let r = reference();
        let mut read = chars(&r[10..40]);
        read[8] = other_base(&[read[8]]);
        let read: String = read.into_iter().collect();
        let mut q = vec![40u8; 30];
        q[8] = 5;
        let res = SequencingCheck::default()
            .check(&r, false, "m", &read, &q)
            .unwrap();
        let kinds: Vec<DifferenceType> = res.differences.iter().map(|d| d.kind).collect();
        assert_eq!(
            kinds,
            vec![DifferenceType::Mismatch, DifferenceType::LowQuality]
        );
        assert!(res
            .differences
            .iter()
            .all(|d| d.reference_position == 18 && d.read_position == 8));
    }

    #[test]
    fn zirkulaere_referenz() {
        let r = seq(60, 5);
        let doubled = format!("{}{}", r, r);
        let read = doubled[50..80].to_string();
        let res = SequencingCheck::default()
            .check(&r, true, "c", &read, &[])
            .unwrap();
        assert!(!res.reversed);
        assert_eq!((res.reference_start, res.reference_end), (50, 20));
        assert!(res.differences.is_empty());
        assert!((res.identity - 100.0).abs() < 1e-9);
        // Difference after the origin: position modulo length
        let mut v = chars(&read);
        v[15] = other_base(&[v[15]]);
        let read: String = v.into_iter().collect();
        let res = SequencingCheck::default()
            .check(&r, true, "c", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        assert_eq!(res.differences[0].reference_position, (50 + 15) % 60);
        assert_eq!(res.differences[0].read_position, 15);
    }

    #[test]
    fn fehler() {
        let r = reference();
        let c = SequencingCheck::default();
        assert!(matches!(
            c.check("", false, "x", "ACGT", &[]),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            c.check(&r, false, "x", "", &[]),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            c.check(&r, false, "x", "ACGTACGT", &[30, 30]),
            Err(Error::InvalidInput(_))
        ));
        assert_eq!(SequencingCheck::default().min_quality, 20);
    }
}
