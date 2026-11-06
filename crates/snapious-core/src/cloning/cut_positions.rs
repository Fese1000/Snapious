//! Fragmente aus beliebigen Schnittstellen bilden (Schnitt nach Position oder Annotation).

use std::collections::BTreeSet;

/// Fragmente `(Beginn, Ende)` (Ende exklusiv) aus Schnittpositionen. Eine Position `p` ist ein Schnitt zwischen Base `p-1` und
/// Base `p`. Die Positionen werden sortiert und ohne Doppelte verwendet.
/// * linear: Positionen `0` und `>= len` werden ignoriert; Fragmente reichen von 0 über die Schnitte bis `len`; ohne Schnitt `[(0, len)]`.
/// * zirkulär: Positionen `>= len` werden ignoriert, `0` ist erlaubt; Fragmente laufen von Schnitt zu Schnitt, das letzte Fragment
///   umläuft das Ende (`Ende <= Beginn`); ein einziger Schnitt `p` ergibt `[(p, p)]`; ohne Schnitt `[]`.
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

/// Schnittpositionen aus Annotationsbereichen `(Beginn, Ende)`: mit `at_start` kommt jeder Beginn dazu, mit `at_end` jedes Ende.
/// Ergebnis sortiert und ohne Doppelte.
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
