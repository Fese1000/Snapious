//! Import sequences from a table (CSV/TSV, already split into rows and fields).

/// Reads `(name, sequence)` from columns `name_col` and `seq_col` (0-based). If `has_header` is true, the first row is
/// skipped. The sequence is stripped of whitespace and converted to uppercase. Rows in which one of the columns is missing
/// or the sequence is empty afterwards are dropped. If the (trimmed) name is empty, the entry is named `Zeile {n}` with `n` = index of the
/// row in `rows` plus 1 (the header row counts).
pub fn sequences_from_table(
    rows: &[Vec<String>],
    name_col: usize,
    seq_col: usize,
    has_header: bool,
) -> Vec<(String, String)> {
    let mut result = Vec::new();

    // Determine start index: if a header is present, row 0 is skipped.
    let start_index = if has_header { 1 } else { 0 };

    for (i, row) in rows.iter().enumerate() {
        // Skip header row if enabled
        if i < start_index {
            continue;
        }

        // Check that the required columns exist in the row
        let name_raw = match row.get(name_col) {
            Some(n) => n,
            None => continue, // Column missing -> skip row
        };

        let seq_raw = match row.get(seq_col) {
            Some(s) => s,
            None => continue, // Column missing -> skip row
        };

        // Process name: trim. If empty, use default name.
        let name = name_raw.trim().to_string();
        let final_name = if name.is_empty() {
            // Index in `rows` plus 1 (since "the header row counts" and i is the index in rows)
            format!("Row {}", i + 1)
        } else {
            name
        };

        // Process sequence: trim, remove spaces and uppercase.
        // "Stripped of whitespace" here means removing all whitespace (not just trimming),
        // since "AC GT" should become "ACGT".
        let seq: String = seq_raw.chars().filter(|c| !c.is_whitespace()).collect();

        if seq.is_empty() {
            continue; // Skip empty sequence after cleanup
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
