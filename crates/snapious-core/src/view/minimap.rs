//! Density strips for the overview (minimap) of a sequence: fraction of each section covered by features.

/// Splits `0..total_len` into `bins` equally wide sections (bounds as floating-point numbers: section `i` runs from
/// `i * total_len / bins` to `(i + 1) * total_len / bins`) and returns the covered fraction (0.0–1.0) for each section.
/// `intervals` are `(start, end)` with exclusive end; entries with `start >= end` are ignored, ends beyond
/// `total_len` are clipped; overlapping intervals count only once (union).
/// `total_len == 0` or `bins == 0` → empty vector.
pub fn density_bins(total_len: usize, intervals: &[(usize, usize)], bins: usize) -> Vec<f64> {
    if total_len == 0 || bins == 0 {
        return Vec::new();
    }

    let mut valid: Vec<(usize, usize)> = Vec::with_capacity(intervals.len());
    for &(s, e) in intervals {
        if s >= e {
            continue;
        }
        let start = s.min(total_len);
        let end = e.min(total_len);
        if start < end {
            valid.push((start, end));
        }
    }

    valid.sort_by_key(|&(s, _)| s);

    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(valid.len());
    for &(s, e) in &valid {
        if let Some(last) = merged.last_mut() {
            if s < last.1 {
                last.1 = last.1.max(e);
            } else {
                merged.push((s, e));
            }
        } else {
            merged.push((s, e));
        }
    }

    let bin_width = total_len as f64 / bins as f64;
    let mut result = Vec::with_capacity(bins);

    for i in 0..bins {
        let bin_start = (i as f64) * bin_width;
        let bin_end = ((i + 1) as f64) * bin_width;

        let mut covered = 0.0;
        for &(u_start, u_end) in &merged {
            let inter_start = bin_start.max(u_start as f64);
            let inter_end = bin_end.min(u_end as f64);
            if inter_start < inter_end {
                covered += inter_end - inter_start;
            }
        }

        result.push(covered / bin_width);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 1e-9, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn einfach() {
        close(&density_bins(100, &[(0, 25)], 4), &[1.0, 0.0, 0.0, 0.0]);
        close(&density_bins(100, &[], 4), &[0.0; 4]);
    }

    #[test]
    fn vereinigung_und_teilbedeckung() {
        // 10..40 covered; sections 0..25 and 25..50: 15/25 each
        close(
            &density_bins(100, &[(10, 30), (20, 40)], 4),
            &[0.6, 0.6, 0.0, 0.0],
        );
    }

    #[test]
    fn kommagrenzen() {
        // length 10, 4 sections of 2.5
        close(&density_bins(10, &[(5, 10)], 4), &[0.0, 0.0, 1.0, 1.0]);
        close(&density_bins(10, &[(0, 10)], 4), &[1.0; 4]);
    }

    #[test]
    fn randfaelle() {
        assert!(density_bins(0, &[(0, 5)], 4).is_empty());
        assert!(density_bins(10, &[(0, 5)], 0).is_empty());
        close(&density_bins(10, &[(5, 5), (7, 3)], 2), &[0.0, 0.0]);
        close(&density_bins(10, &[(5, 99)], 2), &[0.0, 1.0]);
    }
}
