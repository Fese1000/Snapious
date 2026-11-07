//! Select and remove alignment columns (mask for "remove columns").

/// Columns whose gap fraction is at least `min_gap_fraction` (gap = `-` or `.`). Returns `true` per column if
/// it is selected. Rows are of equal length (otherwise the length of the first row counts; missing characters count as gaps).
/// No rows → empty vector. Example: rows `A-C`, `A-C`, `AGC`, threshold 0.5 → `[false, true, false]`.
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

/// Gap-free columns in which all rows have the same base (case-insensitive). Rows and length as in
/// `columns_by_gaps`. Example: `ACG`, `ATG` → `[true, false, true]`.
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

/// Columns of one codon position: `true` for every column `col >= start` with `(col - start) % 3 == codon_position` (0, 1 or 2),
/// otherwise `false`; length `len`. Example: `len 7`, `start 1`, `codon_position 0` → `[false, true, false, false, true, false, false]`.
pub fn columns_codon_position(len: usize, start: usize, codon_position: usize) -> Vec<bool> {
    (0..len)
        .map(|col| col >= start && (col - start) % 3 == codon_position)
        .collect()
}

/// Removes all columns where `remove[col]` is true (missing entries in `remove` count as `false`).
/// Example: rows `ACGT`, `A-GT`, mask `[false, true, false, false]` → `AGT`, `AGT`.
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
