//! Häufigkeitsverteilung von Längen (z. B. Read- oder Contig-Längen).

/// Zählt `values` in Klassen der Breite `bin_width` (0 gilt als 1): Klasse `k` umfasst `[k*w, (k+1)*w)`. Ergebnis: alle Klassen
/// von 0 bis zur größten belegten als `(Beginn der Klasse, Anzahl)`, auch leere. Keine Werte → leerer Vektor.
/// Bei mehr als einer Million Klassen werden nur die belegten Klassen (aufsteigend) zurückgegeben.
pub fn histogram(values: &[usize], bin_width: usize) -> Vec<(usize, usize)> {
    if values.is_empty() {
        return vec![];
    }

    let width = if bin_width == 0 { 1 } else { bin_width };

    // Finde den maximalen Wert, um die Größe des Zählers zu bestimmen
    let max_val = *values.iter().max().unwrap_or(&0);

    // Anzahl der benötigten Buckets: max_val / width + 1
    let num_buckets = (max_val / width).saturating_add(1);

    // Sehr weit gestreute Werte: nur belegte Klassen zurückgeben, damit der Speicher nicht explodiert
    if num_buckets > 1_000_000 {
        let mut map = std::collections::BTreeMap::new();
        for &val in values {
            *map.entry(val / width * width).or_insert(0usize) += 1;
        }
        return map.into_iter().collect();
    }

    // Vektor der Größe max+1 (bzw. num_buckets) initialisieren mit 0
    let mut counts = vec![0usize; num_buckets];

    // Werte in die entsprechenden Buckets zählen
    for &val in values {
        let bucket_index = val / width;
        // Da wir num_buckets basierend auf max_val berechnet haben,
        // ist bucket_index immer gültig.
        counts[bucket_index] += 1;
    }

    // Ergebnis als Vec<(usize, usize)> aufbauen: (Startwert der Klasse, Anzahl)
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
