//! Varianten auf eine Referenzsequenz anwenden (neue Sequenz aus Alignment-Varianten).

use crate::bio::variants::{Variant, VariantKind};
use crate::error::{Error, Result};

/// Wendet alle Varianten, deren `row == row` ist, auf `reference` (Basen ohne Lücken, Uppercase) an und gibt die neue
/// Sequenz zurück. `reference_position` ist die 0-basierte Position in `reference`:
/// * `Substitution`: die Base dort wird durch `alt_base` ersetzt,
/// * `Deletion`: die Base dort entfällt,
/// * `Insertion`: `alt_base` wird vor der Base an `reference_position` eingefügt (gleich `reference.len()` = am Ende);
///   mehrere Einsertionen an derselben Position kommen in der Reihenfolge ihrer `column` (aufsteigend).
///
/// Fehler `Error::InvalidInput`, wenn bei Substitution/Deletion `reference_position >= reference.len()`, wenn bei
/// Substitution/Deletion `reference_base` nicht zur Referenz an dieser Position passt oder bei Insertion
/// `reference_position > reference.len()`. Varianten anderer Zeilen werden ignoriert.
pub fn apply_variants(reference: &str, variants: &[Variant], row: usize) -> Result<String> {
    let mut filtered = variants
        .iter()
        .filter(|v| v.row == row)
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        return Ok(reference.to_string());
    }

    // Sortierung nach reference_position, dann column (aufsteigend).
    // Verarbeitung in umgekehrter Reihenfolge vermeidet Indexverschiebungen für niedrigere Positionen.
    filtered.sort_by_key(|v| (v.reference_position, v.column));

    let mut chars: Vec<char> = reference.chars().collect();

    for v in filtered.into_iter().rev() {
        match v.kind {
            VariantKind::Substitution => {
                if v.reference_position >= chars.len() {
                    return Err(Error::InvalidInput(format!(
                        "Substitution outside the valid range: {}",
                        v.reference_position
                    )));
                }
                if chars[v.reference_position] != v.reference_base {
                    return Err(Error::InvalidInput(format!(
                        "Reference base does not match: expected {}, found {}",
                        v.reference_base, chars[v.reference_position]
                    )));
                }
                chars[v.reference_position] = v.alt_base;
            }
            VariantKind::Deletion => {
                if v.reference_position >= chars.len() {
                    return Err(Error::InvalidInput(format!(
                        "Deletion outside the valid range: {}",
                        v.reference_position
                    )));
                }
                if chars[v.reference_position] != v.reference_base {
                    return Err(Error::InvalidInput(format!(
                        "Reference base does not match: expected {}, found {}",
                        v.reference_base, chars[v.reference_position]
                    )));
                }
                chars.remove(v.reference_position);
            }
            VariantKind::Insertion => {
                if v.reference_position > chars.len() {
                    return Err(Error::InvalidInput(format!(
                        "Insertion outside the valid range: {}",
                        v.reference_position
                    )));
                }
                chars.insert(v.reference_position, v.alt_base);
            }
        }
    }

    Ok(chars.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(
        column: usize,
        pos: usize,
        ref_base: char,
        alt: char,
        kind: VariantKind,
        row: usize,
    ) -> Variant {
        Variant {
            column,
            reference_position: pos,
            row,
            reference_base: ref_base,
            alt_base: alt,
            kind,
        }
    }

    #[test]
    fn einzeln() {
        let sub = var(1, 1, 'C', 'T', VariantKind::Substitution, 1);
        assert_eq!(apply_variants("ACGT", &[sub], 1).unwrap(), "ATGT");
        let del = var(2, 2, 'G', '-', VariantKind::Deletion, 1);
        assert_eq!(apply_variants("ACGT", &[del], 1).unwrap(), "ACT");
        let ins = var(3, 3, '-', 'A', VariantKind::Insertion, 1);
        assert_eq!(apply_variants("ACGT", &[ins], 1).unwrap(), "ACGAT");
        let ende = var(4, 4, '-', 'C', VariantKind::Insertion, 1);
        assert_eq!(apply_variants("ACGT", &[ende], 1).unwrap(), "ACGTC");
    }

    #[test]
    fn mehrere_und_fehler() {
        let list = vec![
            var(0, 0, 'A', 'G', VariantKind::Substitution, 1),
            var(2, 2, '-', 'T', VariantKind::Insertion, 1),
            var(4, 3, 'T', '-', VariantKind::Deletion, 1),
            var(1, 1, 'C', 'A', VariantKind::Substitution, 2), // andere Zeile
        ];
        assert_eq!(apply_variants("ACGT", &list, 1).unwrap(), "GCTG");
        let zwei = vec![
            var(5, 2, '-', 'T', VariantKind::Insertion, 1),
            var(4, 2, '-', 'A', VariantKind::Insertion, 1),
        ];
        assert_eq!(apply_variants("ACGT", &zwei, 1).unwrap(), "ACATGT");
        let falsch = var(1, 1, 'G', 'T', VariantKind::Substitution, 1);
        assert!(apply_variants("ACGT", &[falsch], 1).is_err());
        let weit = var(9, 9, 'A', 'C', VariantKind::Substitution, 1);
        assert!(apply_variants("ACGT", &[weit], 1).is_err());
        assert_eq!(apply_variants("ACGT", &[], 1).unwrap(), "ACGT");
    }
}
