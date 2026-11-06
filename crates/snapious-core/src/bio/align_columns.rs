//! Spalten eines Alignments auswählen und entfernen (Maske für „Spalten entfernen“).

/// Spalten, deren Lückenanteil mindestens `min_gap_fraction` beträgt (Lücke = `-` oder `.`). Rückgabe: je Spalte `true`, wenn
/// sie ausgewählt ist. Zeilen sind gleich lang (sonst zählt die Länge der ersten Zeile; fehlende Zeichen zählen als Lücke).
/// Keine Zeilen → leerer Vektor. Beispiel: Zeilen `A-C`, `A-C`, `AGC`, Schwelle 0.5 → `[false, true, false]`.
pub fn columns_by_gaps(rows: &[String], min_gap_fraction: f64) -> Vec<bool> {
    let Some(first) = rows.first() else {
        return Vec::new();
    };
    (0..first.len())
        .map(|col| {
            let gaps = rows
                .iter()
                .filter(|r| {
                    r.as_bytes()
                        .get(col)
                        .is_none_or(|b| matches!(b, b'-' | b'.'))
                })
                .count();
            gaps as f64 / rows.len() as f64 >= min_gap_fraction
        })
        .collect()
}

/// Spalten ohne Lücke, in denen alle Zeilen dieselbe Base haben (Groß-/Kleinschreibung egal). Zeilen und Länge wie bei
/// `columns_by_gaps`. Beispiel: `ACG`, `ATG` → `[true, false, true]`.
pub fn columns_constant(rows: &[String]) -> Vec<bool> {
    let Some(first) = rows.first() else {
        return Vec::new();
    };
    (0..first.len())
        .map(|col| {
            let base = first.as_bytes()[col].to_ascii_uppercase();
            !matches!(base, b'-' | b'.')
                && rows.iter().all(|r| {
                    r.as_bytes()
                        .get(col)
                        .is_some_and(|b| b.to_ascii_uppercase() == base)
                })
        })
        .collect()
}

/// Spalten einer Codonposition: `true` für jede Spalte `col >= start` mit `(col - start) % 3 == codon_position` (0, 1 oder 2),
/// sonst `false`; Länge `len`. Beispiel: `len 7`, `start 1`, `codon_position 0` → `[false, true, false, false, true, false, false]`.
pub fn columns_codon_position(len: usize, start: usize, codon_position: usize) -> Vec<bool> {
    (0..len)
        .map(|col| col >= start && (col - start) % 3 == codon_position)
        .collect()
}

/// Entfernt alle Spalten, bei denen `remove[col]` wahr ist (fehlende Einträge in `remove` zählen als `false`).
/// Beispiel: Zeilen `ACGT`, `A-GT`, Maske `[false, true, false, false]` → `AGT`, `AGT`.
pub fn remove_columns(rows: &[String], remove: &[bool]) -> Vec<String> {
    rows.iter()
        .map(|r| {
            r.chars()
                .enumerate()
                .filter(|(i, _)| !remove.get(*i).copied().unwrap_or(false))
                .map(|(_, c)| c)
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn luecken_und_konstant() {
        assert_eq!(
            columns_by_gaps(&v(&["A-C", "A-C", "AGC"]), 0.5),
            vec![false, true, false]
        );
        assert_eq!(
            columns_by_gaps(&v(&["A-C", "A-C", "AGC"]), 1.0),
            vec![false, false, false]
        );
        assert_eq!(
            columns_by_gaps(&v(&["A.C", "A.C"]), 1.0),
            vec![false, true, false]
        );
        assert!(columns_by_gaps(&[], 0.5).is_empty());
        assert_eq!(
            columns_constant(&v(&["ACG", "ATG"])),
            vec![true, false, true]
        );
        assert_eq!(
            columns_constant(&v(&["ac-", "AC-"])),
            vec![true, true, false]
        );
    }

    #[test]
    fn codon_und_entfernen() {
        assert_eq!(
            columns_codon_position(7, 1, 0),
            vec![false, true, false, false, true, false, false]
        );
        assert_eq!(
            columns_codon_position(6, 0, 2),
            vec![false, false, true, false, false, true]
        );
        assert_eq!(
            remove_columns(&v(&["ACGT", "A-GT"]), &[false, true, false, false]),
            v(&["AGT", "AGT"])
        );
        assert_eq!(remove_columns(&v(&["ACGT"]), &[]), v(&["ACGT"]));
    }
}
