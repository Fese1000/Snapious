//! Varianten aus einem Alignment: Unterschiede jeder Zeile gegenüber einer Referenzzeile.

use crate::error::{Error, Result};

/// Art einer Variante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariantKind {
    /// Andere Base an derselben Stelle (SNP).
    Substitution,
    /// Die Zeile hat eine Base, wo die Referenz eine Lücke hat.
    Insertion,
    /// Die Zeile hat eine Lücke, wo die Referenz eine Base hat.
    Deletion,
}

/// Eine Abweichung einer Zeile von der Referenz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// Alignment-Spalte (0-basiert).
    pub column: usize,
    /// Anzahl der Referenzbasen (ohne Lücken) vor dieser Spalte: bei Substitution und Deletion die 0-basierte
    /// Position der Referenzbase, bei Insertion die Position der nächsten Referenzbase.
    pub reference_position: usize,
    /// Index der abweichenden Zeile.
    pub row: usize,
    /// Base der Referenz in Großbuchstaben, `'-'` bei Insertion.
    pub reference_base: char,
    /// Base der Zeile in Großbuchstaben, `'-'` bei Deletion.
    pub alt_base: char,
    pub kind: VariantKind,
}

/// Vergleicht alle Zeilen außer `reference` Spalte für Spalte mit der Referenzzeile (Groß-/Kleinschreibung egal).
/// * Beide Zeichen `'-'`: keine Variante.
/// * Referenz `'-'`, Zeile Base: `Insertion`; Referenz Base, Zeile `'-'`: `Deletion`.
/// * Beide Basen und verschieden: `Substitution`. Ist eines der beiden Zeichen `'N'` (oder `'n'`), gibt es keine Variante.
///
/// Sortierung: nach `column`, dann nach `row`.
/// Fehler `Error::InvalidInput`, wenn `rows` leer ist, die Zeilen verschieden lang sind oder `reference >= rows.len()`.
pub fn find_variants(rows: &[String], reference: usize) -> Result<Vec<Variant>> {
    if rows.is_empty() {
        return Err(Error::InvalidInput("Leere Sequenz".into()));
    }
    if reference >= rows.len() {
        return Err(Error::InvalidInput("Ungültiger Referenzindex".into()));
    }

    let len = rows[0].len();
    for row in rows.iter().skip(1) {
        if row.len() != len {
            return Err(Error::InvalidInput(
                "Zeilen haben verschiedene Längen".into(),
            ));
        }
    }

    // Jede Zeile einmal in Vec<char> umwandeln für schnellen Indexzugriff
    let char_rows: Vec<Vec<char>> = rows.iter().map(|s| s.chars().collect()).collect();
    let ref_chars = &char_rows[reference];

    let mut variants = Vec::new();
    // Verfolge die Anzahl der Nicht-Lücken-Basen in der Referenz bis zur aktuellen Spalte
    let mut ref_pos = 0;

    for (c, &ref_ch) in ref_chars.iter().enumerate() {
        let ref_upper = ref_ch.to_ascii_uppercase();

        for (r_idx, row_chars) in char_rows.iter().enumerate() {
            if r_idx == reference {
                continue;
            }

            // Sicheres Abrufen des Zeichens an Position c (Länge wurde geprüft)
            let alt_ch = row_chars.get(c).copied().unwrap_or('-');
            let alt_upper = alt_ch.to_ascii_uppercase();

            if ref_ch == '-' && alt_ch == '-' {
                continue;
            }

            // Ein N (auch kleines n) bei Referenz oder Zeile ist nie eine Variante, auch nicht bei Insertion oder Deletion.
            if ref_upper == 'N' || alt_upper == 'N' {
                continue;
            }

            let (kind, reference_base, alt_base) = if ref_ch == '-' {
                // Insertion: Referenz ist Lücke, Zeile hat Base
                (VariantKind::Insertion, '-', alt_upper)
            } else if alt_ch == '-' {
                // Deletion: Referenz hat Base, Zeile ist Lücke
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

        // Aktualisiere die Referenzposition für die NÄCHSTE Spalte, falls aktuelle keine Lücke ist
        if ref_ch != '-' {
            ref_pos += 1;
        }
    }

    // Explizite Sortierung gemäß Spezifikation (Iteration erzeugt bereits sortierte Reihenfolge)
    variants.sort_by_key(|v| (v.column, v.row));
    Ok(variants)
}

/// Zählt Varianten je Zeile: Ergebnis hat `rows`-Länge (Anzahl Zeilen), der Eintrag der Referenzzeile ist 0.
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
        // Zeile 0 ist gleich (Kleinbuchstaben), Zeile 2: N wird ignoriert, T->A in Spalte 3
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].column, 3);
        assert_eq!(v[0].row, 2);
        assert_eq!((v[0].reference_base, v[0].alt_base), ('T', 'A'));
        assert_eq!(v[0].reference_position, 3);
    }

    #[test]
    fn lueckenpositionen_zaehlen_nicht() {
        // Referenz "A--CG": Spalte 3 ist Referenzposition 1, Spalte 4 Position 2
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
