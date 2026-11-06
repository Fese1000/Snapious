//! Etiketten ohne Überlappung auf Zeilen verteilen (Enzymnamen an der Karte, Beschriftungen an Spuren).

/// Gewünschte Etikette: Mitte und Breite in beliebiger gleicher Einheit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelRequest {
    pub center: f64,
    pub width: f64,
}

/// Platzierte Etikette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedLabel {
    /// Index in der übergebenen Liste.
    pub index: usize,
    /// Zeile (0 = nächste an der Achse).
    pub row: usize,
    /// Linke Kante (`center - width / 2`).
    pub left: f64,
}

/// Verteilt Etiketten auf Zeilen. Regeln:
/// 1. Bearbeitet wird nach `center` aufsteigend, bei Gleichstand nach Index (`f64::total_cmp`).
/// 2. Jede Etikette bekommt die kleinste Zeile, deren zuletzt platzierte Etikette (rechte Kante `left + width`)
///    mindestens `gap` links von `left` endet, also `rechte_kante + gap <= left`. Eine leere Zeile passt immer.
/// 3. Ergebnis: eine `PlacedLabel` je Etikette, sortiert nach `index`.
pub fn place_labels(labels: &[LabelRequest], gap: f64) -> Vec<PlacedLabel> {
    if labels.is_empty() {
        return Vec::new();
    }

    let mut indexed: Vec<(usize, &LabelRequest)> = labels.iter().enumerate().collect();
    indexed.sort_by(|a, b| {
        a.1.center
            .total_cmp(&b.1.center)
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut row_right_edges: Vec<f64> = Vec::with_capacity(labels.len());
    let mut results: Vec<(usize, usize, f64)> = Vec::with_capacity(labels.len());

    for (orig_idx, label) in indexed {
        let left = label.center - label.width / 2.0;

        let target_row = (0..row_right_edges.len()).find(|&r| row_right_edges[r] + gap <= left);

        let row_idx = match target_row {
            Some(r) => r,
            None => {
                row_right_edges.push(left + label.width);
                row_right_edges.len() - 1
            }
        };

        if let Some(r) = target_row {
            row_right_edges[r] = left + label.width;
        }

        results.push((orig_idx, row_idx, left));
    }

    results.sort_by_key(|(idx, _, _)| *idx);
    results
        .into_iter()
        .map(|(index, row, left)| PlacedLabel { index, row, left })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(center: f64, width: f64) -> LabelRequest {
        LabelRequest { center, width }
    }

    #[test]
    fn ueberlappende_in_zweite_zeile() {
        let placed = place_labels(&[req(10.0, 10.0), req(14.0, 10.0), req(40.0, 10.0)], 2.0);
        assert_eq!(
            placed,
            vec![
                PlacedLabel {
                    index: 0,
                    row: 0,
                    left: 5.0
                },
                PlacedLabel {
                    index: 1,
                    row: 1,
                    left: 9.0
                },
                PlacedLabel {
                    index: 2,
                    row: 0,
                    left: 35.0
                },
            ]
        );
    }

    #[test]
    fn beruehrung_und_reihenfolge() {
        // 0..10 und 10..20 mit gap 0: gleiche Zeile; Eingabe unsortiert
        let placed = place_labels(&[req(15.0, 10.0), req(5.0, 10.0)], 0.0);
        assert_eq!(
            placed,
            vec![
                PlacedLabel {
                    index: 0,
                    row: 0,
                    left: 10.0
                },
                PlacedLabel {
                    index: 1,
                    row: 0,
                    left: 0.0
                },
            ]
        );
        // mit gap 1 reicht die Berührung nicht mehr
        let placed = place_labels(&[req(15.0, 10.0), req(5.0, 10.0)], 1.0);
        assert_eq!(placed[0].row, 1);
        assert_eq!(placed[1].row, 0);
    }

    #[test]
    fn drei_uebereinander_und_leer() {
        let placed = place_labels(&[req(10.0, 10.0), req(10.0, 10.0), req(10.0, 10.0)], 0.0);
        let rows: Vec<usize> = placed.iter().map(|p| p.row).collect();
        assert_eq!(rows, vec![0, 1, 2]);
        assert!(place_labels(&[], 1.0).is_empty());
    }
}
