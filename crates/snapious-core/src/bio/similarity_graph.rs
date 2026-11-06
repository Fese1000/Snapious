//! Ähnlichkeitsverlauf entlang eines Alignments (Werte je Spalte und geglättet).

/// Mittlere paarweise Identität je Spalte: Für jedes Paar von Zeilen `i < j`, in dem in dieser Spalte beide Zeichen keine Lücke
/// (`-` oder `.`) sind, wird gezählt, ob sie gleich sind (Groß-/Kleinschreibung egal). Wert = gleiche Paare / gültige Paare;
/// Spalten ohne gültiges Paar haben `0.0`. Die Länge ist die der ersten Zeile (fehlende Zeichen in kürzeren Zeilen zählen als Lücke);
/// ohne Zeilen leer. Beispiel: `ACGT`, `ACGA`, `TCGT` → `[1/3, 1, 1, 1/3]`.
pub fn column_identity(rows: &[String]) -> Vec<f64> {
    let Some(first) = rows.first() else {
        return Vec::new();
    };
    let is_gap = |b: u8| b == b'-' || b == b'.';
    (0..first.len())
        .map(|col| {
            let cells: Vec<u8> = rows
                .iter()
                .filter_map(|r| r.as_bytes().get(col).copied())
                .filter(|b| !is_gap(*b))
                .map(|b| b.to_ascii_uppercase())
                .collect();
            let mut pairs = 0usize;
            let mut equal = 0usize;
            for i in 0..cells.len() {
                for j in i + 1..cells.len() {
                    pairs += 1;
                    if cells[i] == cells[j] {
                        equal += 1;
                    }
                }
            }
            if pairs == 0 {
                0.0
            } else {
                equal as f64 / pairs as f64
            }
        })
        .collect()
}

/// Glättet Werte mit einem zentrierten Fenster der Breite `window`: Wert `i` ist der Mittelwert von `values[lo..=hi]` mit
/// `half = window / 2`, `lo = i.saturating_sub(half)` und `hi = min(i + half, len - 1)`. `window` 0 oder 1 lässt die Werte unverändert.
/// Beispiel: `[1/3, 1, 1, 1/3]` mit Fenster 3 → `[2/3, 7/9, 7/9, 2/3]`. Leere Eingabe → leer.
pub fn window_mean(values: &[f64], window: usize) -> Vec<f64> {
    let half = window / 2;
    (0..values.len())
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half).min(values.len() - 1);
            values[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn close(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 1e-9, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn spalten() {
        close(
            &column_identity(&v(&["ACGT", "ACGA", "TCGT"])),
            &[1.0 / 3.0, 1.0, 1.0, 1.0 / 3.0],
        );
        close(&column_identity(&v(&["A-", "a-", "AC"])), &[1.0, 0.0]);
        close(&column_identity(&v(&["AC"])), &[0.0, 0.0]);
        assert!(column_identity(&[]).is_empty());
    }

    #[test]
    fn glaetten() {
        let w = [1.0 / 3.0, 1.0, 1.0, 1.0 / 3.0];
        close(
            &window_mean(&w, 3),
            &[2.0 / 3.0, 7.0 / 9.0, 7.0 / 9.0, 2.0 / 3.0],
        );
        close(&window_mean(&w, 1), &w);
        close(&window_mean(&w, 0), &w);
        close(&window_mean(&[2.0, 4.0], 5), &[3.0, 3.0]);
        assert!(window_mean(&[], 3).is_empty());
    }
}
