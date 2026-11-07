//! Variants from an alignment: differences of each row relative to a reference row.

use crate::error::{Error, Result};

/// Kind of variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariantKind {
    /// Different base at the same position (SNP).
    Substitution,
    /// The row has a base where the reference has a gap.
    Insertion,
    /// The row has a gap where the reference has a base.
    Deletion,
}

/// A deviation of a row from the reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// Alignment column (0-based).
    pub column: usize,
    /// Number of reference bases (excluding gaps) before this column: for substitution and deletion the 0-based
    /// position of the reference base, for insertion the position of the next reference base.
    pub reference_position: usize,
    /// Index of the deviating row.
    pub row: usize,
    /// Base of the reference in uppercase, `'-'` for insertion.
    pub reference_base: char,
    /// Base of the row in uppercase, `'-'` for deletion.
    pub alt_base: char,
    pub kind: VariantKind,
}

/// Compares all rows except `reference` column by column with the reference row (case-insensitive).
/// * Both characters `'-'`: no variant.
/// * Reference `'-'`, row base: `Insertion`; reference base, row `'-'`: `Deletion`.
/// * Both bases and different: `Substitution`. If either character is `'N'` (or `'n'`), there is no variant.
///
/// Sorting: by `column`, then by `row`.
/// Error `Error::InvalidInput` if `rows` is empty, the rows differ in length, or `reference >= rows.len()`.
pub fn find_variants(rows: &[String], reference: usize) -> Result<Vec<Variant>> {
    if rows.is_empty() {
        return Err(Error::InvalidInput("Empty sequence".into()));
    }
    if reference >= rows.len() {
        return Err(Error::InvalidInput("Invalid reference index".into()));
    }

    let len = rows[0].len();
    for row in rows.iter().skip(1) {
        if row.len() != len {
            return Err(Error::InvalidInput(
                "Rows have different lengths".into(),
            ));
        }
    }

    // Convert each row to a Vec<char> once for fast indexed access
    let char_rows: Vec<Vec<char>> = rows.iter().map(|s| s.chars().collect()).collect();
    let ref_chars = &char_rows[reference];

    let mut variants = Vec::new();
    // Track the number of non-gap bases in the reference up to the current column
    let mut ref_pos = 0;

    for (c, &ref_ch) in ref_chars.iter().enumerate() {
        let ref_upper = ref_ch.to_ascii_uppercase();

        for (r_idx, row_chars) in char_rows.iter().enumerate() {
            if r_idx == reference {
                continue;
            }

            // Safe retrieval of the character at position c (length was checked)
            let alt_ch = row_chars.get(c).copied().unwrap_or('-');
            let alt_upper = alt_ch.to_ascii_uppercase();

            if ref_ch == '-' && alt_ch == '-' {
                continue;
            }

            // An N (also lowercase n) in the reference or row is never a variant, not even for insertion or deletion.
            if ref_upper == 'N' || alt_upper == 'N' {
                continue;
            }

            let (kind, reference_base, alt_base) = if ref_ch == '-' {
                // Insertion: reference is a gap, row has a base
                (VariantKind::Insertion, '-', alt_upper)
            } else if alt_ch == '-' {
                // Deletion: reference has a base, row is a gap
                (VariantKind::Deletion, ref_upper, '-')
            } else if ref_upper == alt_upper {
                continue;
            } else {
                (VariantKind::Substitution, ref_upper, alt_upper)
            };

            variants.push(Variant {
                column: c,
                reference_position: ref_pos,
                row: r_idx,
                reference_base,
                alt_base,
                kind,
            });
        }

        // Update the reference position for the NEXT column if the current one is not a gap
        if ref_ch != '-' {
            ref_pos += 1;
        }
    }

    // Explicit sorting per the specification (iteration already produces sorted order)
    variants.sort_by_key(|v| (v.column, v.row));
    Ok(variants)
}

/// Counts variants per row: the result has length `rows` (number of rows), the entry of the reference row is 0.
pub fn count_by_row(variants: &[Variant], rows: usize) -> Vec<usize> {
    let mut counts = vec![0usize; rows];
    for v in variants {
        if v.row < rows {
            counts[v.row] += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn substitution_deletion_insertion() {
        let r = rows(&["ACGT-ACGT", "ACCT-ACGT", "AC-TTACGT"]);
        let v = find_variants(&r, 0).unwrap();
        assert_eq!(
            v,
            vec![
                Variant {
                    column: 2,
                    reference_position: 2,
                    row: 1,
                    reference_base: 'G',
                    alt_base: 'C',
                    kind: VariantKind::Substitution
                },
                Variant {
                    column: 2,
                    reference_position: 2,
                    row: 2,
                    reference_base: 'G',
                    alt_base: '-',
                    kind: VariantKind::Deletion
                },
                Variant {
                    column: 4,
                    reference_position: 4,
                    row: 2,
                    reference_base: '-',
                    alt_base: 'T',
                    kind: VariantKind::Insertion
                },
            ]
        );
        assert_eq!(count_by_row(&v, 3), vec![0, 1, 2]);
    }

    #[test]
    fn andere_referenz_und_kleinbuchstaben() {
        let r = rows(&["acgt", "ACGT", "ANGA"]);
        let v = find_variants(&r, 1).unwrap();
        // Row 0 is equal (lowercase), row 2: N is ignored, T->A in column 3
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].column, 3);
        assert_eq!(v[0].row, 2);
        assert_eq!((v[0].reference_base, v[0].alt_base), ('T', 'A'));
        assert_eq!(v[0].reference_position, 3);
    }

    #[test]
    fn lueckenpositionen_zaehlen_nicht() {
        // Reference "A--CG": column 3 is reference position 1, column 4 position 2
        let r = rows(&["A--CG", "A--TG", "AGGCC"]);
        let v = find_variants(&r, 0).unwrap();
        assert_eq!(v.len(), 4);
        assert_eq!(
            (v[0].column, v[0].reference_position, v[0].kind),
            (1, 1, VariantKind::Insertion)
        );
        assert_eq!(
            (v[1].column, v[1].reference_position, v[1].kind),
            (2, 1, VariantKind::Insertion)
        );
        assert_eq!((v[2].column, v[2].reference_position, v[2].row), (3, 1, 1));
        assert_eq!(
            (v[3].column, v[3].reference_position, v[3].alt_base),
            (4, 2, 'C')
        );
    }

    #[test]
    fn n_ist_nie_eine_variante() {
        assert!(find_variants(&rows(&["AC-T", "ACNT"]), 0)
            .unwrap()
            .is_empty());
        assert!(find_variants(&rows(&["ACNT", "AC-T"]), 0)
            .unwrap()
            .is_empty());
        assert!(find_variants(&rows(&["ACGT", "ACnT"]), 0)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn fehlerfaelle() {
        assert!(matches!(find_variants(&[], 0), Err(Error::InvalidInput(_))));
        assert!(matches!(
            find_variants(&rows(&["ACG", "AC"]), 0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            find_variants(&rows(&["ACG"]), 1),
            Err(Error::InvalidInput(_))
        ));
        assert!(find_variants(&rows(&["ACG"]), 0).unwrap().is_empty());
    }
}
