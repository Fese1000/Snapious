//! Frequency distribution of lengths (e.g. read or contig lengths).

/// Counts `values` into bins of width `bin_width` (0 counts as 1): bin `k` covers `[k*w, (k+1)*w)`. Result: all bins
/// from 0 up to the largest occupied one as `(start of the bin, count)`, including empty ones. No values → empty vector.
/// With more than one million bins only the occupied bins (ascending) are returned.
pub fn histogram(values: &[usize], bin_width: usize) -> Vec<(usize, usize)> {
    if values.is_empty() {
        return vec![];
    }

    let width = if bin_width == 0 { 1 } else { bin_width };

    // Find the maximum value to determine the size of the counter
    let max_val = *values.iter().max().unwrap_or(&0);

    // Number of buckets needed: max_val / width + 1
    let num_buckets = (max_val / width).saturating_add(1);

    // Very widely scattered values: return only occupied bins so that memory does not explode
    if num_buckets > 1_000_000 {
        let mut map = std::collections::BTreeMap::new();
        for &val in values {
            *map.entry(val / width * width).or_insert(0usize) += 1;
        }
        return map.into_iter().collect();
    }

    // Initialize a vector of size max+1 (or num_buckets) with 0
    let mut counts = vec![0usize; num_buckets];

    // Count values into the corresponding buckets
    for &val in values {
        let bucket_index = val / width;
        // Since we computed num_buckets based on max_val,
        // bucket_index is always valid.
        counts[bucket_index] += 1;
    }

    // Build the result as Vec<(usize, usize)>: (start value of the bin, count)
    counts
        .into_iter()
        .enumerate()
        .map(|(k, count)| (k * width, count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn klassen() {
        assert_eq!(
            histogram(&[1, 2, 3, 10, 11, 25], 10),
            vec![(0, 3), (10, 2), (20, 1)]
        );
        assert_eq!(histogram(&[5, 5], 5), vec![(0, 0), (5, 2)]);
        assert_eq!(
            histogram(&[0, 1, 1, 3], 0),
            vec![(0, 1), (1, 2), (2, 0), (3, 1)]
        );
        assert_eq!(histogram(&[7], 100), vec![(0, 1)]);
        assert!(histogram(&[], 10).is_empty());
    }

    #[test]
    fn weit_gestreut() {
        let h = histogram(&[1, usize::MAX], 1);
        assert_eq!(h, vec![(1, 1), (usize::MAX, 1)]);
    }

    #[test]
    fn gross() {
        let h = histogram(&[1000, 1999, 2000], 1000);
        assert_eq!(h, vec![(0, 0), (1000, 2), (2000, 1)]);
    }
}
