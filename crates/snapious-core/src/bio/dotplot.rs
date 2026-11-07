//! Punktdiagramm (Dotplot) zum Vergleich zweier Sequenzen.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};

/// Berechnet die Punkte eines Dotplots. Ein Punkt `(i, j)` gehört dazu, wenn das Fenster `a[i..i+window]`
/// und das Fenster `b[j..j+window]` an mindestens `min_matches` Stellen (Stelle für Stelle verglichen) übereinstimmen.
/// * Beide Sequenzen werden in Uppercase verglichen.
/// * `reverse`: `b` wird vorher durch sein Reverskomplement (`iupac_reverse_complement`) ersetzt; `j` zählt dann im
///   Reverskomplement.
/// * Sortierung zeilenweise: erst nach `i`, dann nach `j`. Ist eine Sequenz kürzer als `window`, ist das Ergebnis leer.
///
/// Fehler `Error::InvalidInput`, wenn `window == 0`, `min_matches == 0` oder `min_matches > window`.
pub fn dotplot_points(
    a: &str,
    b: &str,
    window: usize,
    min_matches: usize,
    reverse: bool,
) -> Result<Vec<(usize, usize)>> {
    if window == 0 || min_matches == 0 || min_matches > window {
        return Err(Error::InvalidInput(
            "Window size or minimum match invalid".into(),
        ));
    }

    let a_upper = a.to_uppercase();
    let b_upper = b.to_uppercase();

    let b_processed: String = if reverse {
        iupac_reverse_complement(&b_upper)
    } else {
        b_upper
    };

    let a_bytes = a_upper.as_bytes();
    let b_bytes = b_processed.as_bytes();

    let len_a = a_bytes.len();
    let len_b = b_bytes.len();

    if len_a < window || len_b < window {
        return Ok(vec![]);
    }

    let mut points = Vec::new();
    for i in 0..=(len_a - window) {
        for j in 0..=(len_b - window) {
            let mut matches = 0;
            for k in 0..window {
                if a_bytes[i + k] == b_bytes[j + k] {
                    matches += 1;
                }
            }
            if matches >= min_matches {
                points.push((i, j));
            }
        }
    }

    Ok(points)
}

/// Gruppiert die Punkte zu Diagonalen: aufeinanderfolgende Punkte `(i, j)`, `(i+1, j+1)`, … bilden eine Diagonale
/// `(start_i, start_j, länge)`. Sortiert nach `start_i`, dann `start_j`. Eingabe darf in beliebiger Reihenfolge sein.
pub fn diagonals(points: &[(usize, usize)]) -> Vec<(usize, usize, usize)> {
    if points.is_empty() {
        return vec![];
    }

    let mut sorted_pts = points.to_vec();
    sorted_pts.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut result = Vec::new();
    let mut start_i = sorted_pts[0].0;
    let mut start_j = sorted_pts[0].1;
    let mut length: usize = 1;

    for k in 1..sorted_pts.len() {
        let (i, j) = sorted_pts[k];
        let (prev_i, prev_j) = (sorted_pts[k - 1].0, sorted_pts[k - 1].1);

        if i == prev_i + 1 && j == prev_j + 1 {
            length += 1;
        } else {
            result.push((start_i, start_j, length));
            start_i = i;
            start_j = j;
            length = 1;
        }
    }

    result.push((start_i, start_j, length));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identische_sequenz() {
        let p = dotplot_points("ACGTAC", "ACGTAC", 3, 3, false).unwrap();
        assert_eq!(p, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
        assert_eq!(diagonals(&p), vec![(0, 0, 4)]);
    }

    #[test]
    fn mit_fehlpaarung() {
        let p = dotplot_points("ACGTA", "ACGTT", 3, 2, false).unwrap();
        assert_eq!(p, vec![(0, 0), (1, 1), (2, 2)]);
        let q = dotplot_points("acgta", "ACGTT", 3, 3, false).unwrap();
        assert_eq!(q, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn reverskomplement() {
        let p = dotplot_points("AAACCC", "GGGTTT", 3, 3, true).unwrap();
        assert_eq!(p, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
        assert!(dotplot_points("AAACCC", "GGGTTT", 3, 3, false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn diagonalen_gruppieren() {
        let pts = vec![(5, 1), (0, 0), (1, 1), (2, 2), (6, 2), (9, 0)];
        assert_eq!(diagonals(&pts), vec![(0, 0, 3), (5, 1, 2), (9, 0, 1)]);
        assert!(diagonals(&[]).is_empty());
    }

    #[test]
    fn grenzen_und_fehler() {
        assert!(dotplot_points("AC", "ACGT", 3, 3, false)
            .unwrap()
            .is_empty());
        assert!(matches!(
            dotplot_points("ACGT", "ACGT", 0, 1, false),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            dotplot_points("ACGT", "ACGT", 3, 0, false),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            dotplot_points("ACGT", "ACGT", 3, 4, false),
            Err(Error::InvalidInput(_))
        ));
    }
}
