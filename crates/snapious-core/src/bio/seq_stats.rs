//! Statistics on nucleotide sequences: GC profile, GC skew, dinucleotides and CpG islands.

use std::collections::BTreeMap;

use crate::error::{Error, Result};

/// GC fraction in percent per window. Windows of length `window` start at `0, step, 2·step, …` as long as they fit entirely into the
/// sequence. G and C (case-insensitive) are counted and divided by the window length. Result: `(start, percent)`.
/// `window == 0` or `step == 0` → `Error::InvalidInput`. If the sequence is shorter than the window, the result is empty.
pub fn gc_windows(bases: &str, window: usize, step: usize) -> Result<Vec<(usize, f64)>> {
    if window == 0 || step == 0 {
        return Err(Error::InvalidInput(
            "Window size and step must be greater than 0".into(),
        ));
    }
    let mut result = Vec::new();
    let bytes = bases.as_bytes();
    let len = bytes.len();

    if window > len {
        return Ok(result);
    }

    let mut start = 0;
    while start + window <= len {
        let slice = &bytes[start..start + window];
        let gc_count = slice
            .iter()
            .filter(|&&b| b == b'G' || b == b'C' || b == b'g' || b == b'c')
            .count();
        let pct = (gc_count as f64 / window as f64) * 100.0;
        result.push((start, pct));
        start += step;
    }
    Ok(result)
}

/// GC skew `(G − C) / (G + C)` per window (same window rules as `gc_windows`); without G and C the value is 0.0.
pub fn gc_skew(bases: &str, window: usize, step: usize) -> Result<Vec<(usize, f64)>> {
    if window == 0 || step == 0 {
        return Err(Error::InvalidInput(
            "Window size and step must be greater than 0".into(),
        ));
    }
    let mut result = Vec::new();
    let bytes = bases.as_bytes();
    let len = bytes.len();

    if window > len {
        return Ok(result);
    }

    let mut start = 0;
    while start + window <= len {
        let slice = &bytes[start..start + window];
        let mut g_count: usize = 0;
        let mut c_count: usize = 0;

        for &b in slice {
            match b {
                b'G' | b'g' => g_count += 1,
                b'C' | b'c' => c_count += 1,
                _ => {}
            }
        }

        let skew = if g_count + c_count == 0 {
            0.0
        } else {
            (g_count as f64 - c_count as f64) / (g_count as f64 + c_count as f64)
        };
        result.push((start, skew));
        start += step;
    }
    Ok(result)
}

/// Count of each overlapping dinucleotide of A, C, G, T (uppercase as key, `U` as `T`). Pairs with other characters
/// are not counted. Only pairs that occur appear in the result.
pub fn dinucleotide_counts(bases: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    let letters: Vec<char> = bases
        .chars()
        .map(|c| match c.to_ascii_uppercase() {
            'U' => 'T',
            other => other,
        })
        .collect();
    for pair in letters.windows(2) {
        if let [a, b] = pair {
            if matches!(a, 'A' | 'C' | 'G' | 'T') && matches!(b, 'A' | 'C' | 'G' | 'T') {
                *counts.entry(format!("{}{}", a, b)).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// A range `[start, end)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub start: usize,
    pub end: usize,
}

/// Finds CpG islands after Gardiner-Garden and Frommer (1987): all windows of length `min_length` (step 1) whose GC fraction is
/// at least `min_gc` (0 to 1) and whose observed/expected ratio is at least `min_obs_exp` count as hits.
/// Observed/expected = `CpG count · window length / (C count · G count)`; if C or G is absent, the window does not qualify.
/// Overlapping or adjacent hit windows are merged into one region (start of the first to end of the last window).
/// Result sorted ascending. `min_length == 0` → `Error::InvalidInput`. Shorter sequences give an empty list.
pub fn cpg_islands(
    bases: &str,
    min_length: usize,
    min_gc: f64,
    min_obs_exp: f64,
) -> Result<Vec<Region>> {
    if min_length == 0 {
        return Err(Error::InvalidInput(
            "min_length must be greater than 0".into(),
        ));
    }

    let mut regions = Vec::new();
    let bytes = bases.as_bytes();
    let len = bytes.len();

    if min_length > len {
        return Ok(regions);
    }

    let mut region_start: Option<usize> = None;
    let mut region_end: usize = 0;

    let mut start = 0;
    while start + min_length <= len {
        let slice = &bytes[start..start + min_length];

        let mut g_count: usize = 0;
        let mut c_count: usize = 0;

        for &b in slice {
            match b {
                b'G' | b'g' => g_count += 1,
                b'C' | b'c' => c_count += 1,
                _ => {}
            }
        }

        let cpg_count = slice
            .iter()
            .zip(slice.iter().skip(1))
            .filter(|(b1, b2)| b1.eq_ignore_ascii_case(&b'C') && b2.eq_ignore_ascii_case(&b'G'))
            .count();

        let gc_content = (g_count + c_count) as f64 / min_length as f64;
        let is_hit = gc_content >= min_gc && g_count > 0 && c_count > 0;

        let obs_exp = if is_hit {
            (cpg_count as f64 * min_length as f64) / (g_count as f64 * c_count as f64)
        } else {
            0.0
        };

        if is_hit && obs_exp >= min_obs_exp {
            match region_start {
                Some(s) => {
                    if start < region_end {
                        let new_end = start + min_length;
                        if new_end > region_end {
                            region_end = new_end;
                        }
                    } else {
                        regions.push(Region {
                            start: s,
                            end: region_end,
                        });
                        region_start = Some(start);
                        region_end = start + min_length;
                    }
                }
                None => {
                    region_start = Some(start);
                    region_end = start + min_length;
                }
            }
        }

        start += 1;
    }

    if let Some(s) = region_start {
        regions.push(Region {
            start: s,
            end: region_end,
        });
    }

    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn gc_verlauf() {
        let w = gc_windows("GGGGAAAACCCC", 4, 4).unwrap();
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].0, 0);
        assert!(close(w[0].1, 100.0) && close(w[1].1, 0.0) && close(w[2].1, 100.0));
        let v = gc_windows("ACGT", 2, 1).unwrap();
        assert_eq!(v.iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert!(close(v[0].1, 50.0) && close(v[1].1, 100.0) && close(v[2].1, 50.0));
        assert!(gc_windows("AC", 5, 1).unwrap().is_empty());
        assert!(gc_windows("", 1, 1).unwrap().is_empty());
        assert!(matches!(
            gc_windows("ACGT", 0, 1),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            gc_windows("ACGT", 2, 0),
            Err(Error::InvalidInput(_))
        ));
        assert!(close(gc_windows("gcgc", 4, 1).unwrap()[0].1, 100.0));
    }

    #[test]
    fn skew() {
        let s = gc_skew("GGGC", 4, 1).unwrap();
        assert!(close(s[0].1, 0.5));
        let t = gc_skew("CCCCGGAA", 4, 4).unwrap();
        assert!(close(t[0].1, -1.0));
        assert!(close(t[1].1, 1.0));
        let u = gc_skew("AAAA", 2, 2).unwrap();
        assert!(u.iter().all(|x| x.1 == 0.0));
        assert!(gc_skew("AA", 0, 1).is_err());
    }

    #[test]
    fn dinukleotide() {
        let d = dinucleotide_counts("ACGCGT");
        assert_eq!(d.get("AC"), Some(&1));
        assert_eq!(d.get("CG"), Some(&2));
        assert_eq!(d.get("GC"), Some(&1));
        assert_eq!(d.get("GT"), Some(&1));
        assert_eq!(d.len(), 4);
        let e = dinucleotide_counts("aNcg");
        assert_eq!(e.len(), 1);
        assert_eq!(e.get("CG"), Some(&1));
        assert_eq!(dinucleotide_counts("AUA").get("AT"), Some(&1));
        assert!(dinucleotide_counts("A").is_empty());
    }

    #[test]
    fn cpg_inseln() {
        let insel = "CG".repeat(120);
        let flanke = "A".repeat(150);
        let seq = format!("{}{}{}", flanke, insel, flanke);
        let r = cpg_islands(&seq, 200, 0.5, 0.6).unwrap();
        // Windows with up to 100 A (half the window length) and otherwise the island satisfy GC 0.5: start 150 − 100, end 390 + 100
        assert_eq!(
            r,
            vec![Region {
                start: 50,
                end: 490
            }]
        );
        assert!(cpg_islands(&flanke, 100, 0.5, 0.6).unwrap().is_empty());
        assert!(cpg_islands("CGCG", 200, 0.5, 0.6).unwrap().is_empty());
        assert!(matches!(
            cpg_islands("CGCG", 0, 0.5, 0.6),
            Err(Error::InvalidInput(_))
        ));
        // GC rich but without CpG (only alternating GC GCGC… has CpG; here GGGGCCCC… has only one CpG per 8)
        let ohne = "GGGGCCCC".repeat(40);
        assert!(cpg_islands(&ohne, 200, 0.5, 0.6).unwrap().is_empty());
    }
}
