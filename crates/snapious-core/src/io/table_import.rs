//! Sequenzen aus einer Tabelle (CSV/TSV, bereits in Zeilen und Felder zerlegt) übernehmen.

/// Liest `(Name, Sequenz)` aus den Spalten `name_col` und `seq_col` (0-basiert). Ist `has_header` wahr, wird die erste Zeile
/// übersprungen. Die Sequenz wird von Leerraum befreit und in Uppercase gewandelt. Zeilen, in denen eine der Spalten fehlt
/// oder die Sequenz danach leer ist, entfallen. Ist der Name (getrimmt) leer, heißt der Eintrag `Zeile {n}` mit `n` = Index der
/// Zeile in `rows` plus 1 (die Kopfzeile zählt mit).
pub fn sequences_from_table(
    rows: &[Vec<String>],
    name_col: usize,
    seq_col: usize,
    has_header: bool,
) -> Vec<(String, String)> {
    let mut result = Vec::new();

    // Startindex festlegen: Wenn Header vorhanden ist, wird Zeile 0 übersprungen.
    let start_index = if has_header { 1 } else { 0 };

    for (i, row) in rows.iter().enumerate() {
        // Header-Zeile überspringen, falls aktiviert
        if i < start_index {
            continue;
        }

        // Prüfen, ob die benötigten Spalten in der Zeile existieren
        let name_raw = match row.get(name_col) {
            Some(n) => n,
            None => continue, // Spalte fehlt -> Zeile überspringen
        };

        let seq_raw = match row.get(seq_col) {
            Some(s) => s,
            None => continue, // Spalte fehlt -> Zeile überspringen
        };

        // Name verarbeiten: Trimmen. Wenn leer, Standardname verwenden.
        let name = name_raw.trim().to_string();
        let final_name = if name.is_empty() {
            // Index in `rows` plus 1 (da "Kopfzeile zählt mit" und i der Index in rows ist)
            format!("Row {}", i + 1)
        } else {
            name
        };

        // Sequenz verarbeiten: Trimmen, Leerzeichen entfernen und Uppercase.
        // "Leerraum befreien" bedeutet hier, alle Whitespaces zu entfernen (nicht nur Trimmen),
        // da "AC GT" zu "ACGT" werden soll.
        let seq: String = seq_raw.chars().filter(|c| !c.is_whitespace()).collect();

        if seq.is_empty() {
            continue; // Leere Sequenz nach Bereinigung überspringen
        }

        result.push((final_name, seq.to_uppercase()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|x| x.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[test]
    fn mit_kopfzeile() {
        let rows = r(&[
            &["Name", "Seq"],
            &["a", " ac gt "],
            &["", "TT"],
            &["c", ""],
            &["d"],
        ]);
        assert_eq!(
            sequences_from_table(&rows, 0, 1, true),
            vec![
                ("a".to_string(), "ACGT".to_string()),
                ("Row 3".to_string(), "TT".to_string())
            ]
        );
    }

    #[test]
    fn andere_spalten_ohne_kopf() {
        let rows = r(&[&["x", "q1", "acg"], &["y", " q2 ", "T"]]);
        assert_eq!(
            sequences_from_table(&rows, 1, 2, false),
            vec![
                ("q1".to_string(), "ACG".to_string()),
                ("q2".to_string(), "T".to_string())
            ]
        );
        assert!(sequences_from_table(&rows, 0, 9, false).is_empty());
        assert!(sequences_from_table(&[], 0, 1, true).is_empty());
    }
}
