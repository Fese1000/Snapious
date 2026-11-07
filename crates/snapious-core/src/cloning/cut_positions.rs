//! Form fragments from arbitrary cut sites (cut by position or annotation).

use std::collections::BTreeSet;

/// Fragments `(start, end)` (end exclusive) from cut positions. A position `p` is a cut between base `p-1` and
/// base `p`. The positions are used sorted and without duplicates.
/// * linear: positions `0` and `>= len` are ignored; fragments run from 0 across the cuts to `len`; without a cut `[(0, len)]`.
/// * circular: positions `>= len` are ignored, `0` is allowed; fragments run from cut to cut, the last fragment
///   wraps around the end (`end <= start`); a single cut `p` yields `[(p, p)]`; without a cut `[]`.
pub fn fragments_from_cuts(len: usize, positions: &[usize], circular: bool) -> Vec<(usize, usize)> {
    let mut valid_positions = BTreeSet::new();

    for &p in positions {
        if circular && p == 0 {
            valid_positions.insert(0);
        } else if !circular && p == 0 {
            continue;
        }

        if p < len {
            valid_positions.insert(p);
        }
    }

    let sorted_pos: Vec<usize> = valid_positions.into_iter().collect();

    if circular {
        if sorted_pos.is_empty() {
            return Vec::new();
        }

        let mut fragments = Vec::with_capacity(sorted_pos.len());
        for i in 0..sorted_pos.len() {
            let start = sorted_pos[i];
            let end = sorted_pos[(i + 1) % sorted_pos.len()];
            fragments.push((start, end));
        }
        fragments
    } else {
        if sorted_pos.is_empty() {
            return vec![(0, len)];
        }

        let mut fragments = Vec::with_capacity(sorted_pos.len() + 1);
        fragments.push((0, sorted_pos[0]));
        for i in 0..sorted_pos.len() - 1 {
            fragments.push((sorted_pos[i], sorted_pos[i + 1]));
        }
        fragments.push((sorted_pos[sorted_pos.len() - 1], len));
        fragments
    }
}

/// Cut positions from annotation ranges `(start, end)`: with `at_start` each start is added, with `at_end` each end.
/// Result sorted and without duplicates.
pub fn cut_positions_from_spans(
    spans: &[(usize, usize)],
    at_start: bool,
    at_end: bool,
) -> Vec<usize> {
    let mut positions = BTreeSet::new();

    for &(start, end) in spans {
        if at_start {
            positions.insert(start);
        }
        if at_end {
            positions.insert(end);
        }
    }

    positions.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear() {
        assert_eq!(
            fragments_from_cuts(10, &[7, 3, 3, 0, 12], false),
            vec![(0, 3), (3, 7), (7, 10)]
        );
        assert_eq!(fragments_from_cuts(10, &[], false), vec![(0, 10)]);
        assert_eq!(fragments_from_cuts(10, &[0, 10], false), vec![(0, 10)]);
    }

    #[test]
    fn zirkulaer() {
        assert_eq!(fragments_from_cuts(10, &[3, 7], true), vec![(3, 7), (7, 3)]);
        assert_eq!(fragments_from_cuts(10, &[5], true), vec![(5, 5)]);
        assert_eq!(
            fragments_from_cuts(10, &[], true),
            Vec::<(usize, usize)>::new()
        );
        assert_eq!(
            fragments_from_cuts(10, &[0, 4, 10], true),
            vec![(0, 4), (4, 0)]
        );
    }

    #[test]
    fn aus_bereichen() {
        assert_eq!(
            cut_positions_from_spans(&[(2, 5), (5, 8)], true, true),
            vec![2, 5, 8]
        );
        assert_eq!(
            cut_positions_from_spans(&[(2, 5), (5, 8)], true, false),
            vec![2, 5]
        );
        assert_eq!(cut_positions_from_spans(&[(2, 5)], false, true), vec![5]);
        assert!(cut_positions_from_spans(&[(2, 5)], false, false).is_empty());
    }
}
