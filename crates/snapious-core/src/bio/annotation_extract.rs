//! Sequenzabschnitte von Annotationen ausschneiden (mit Rand, auf Wunsch revers-komplementiert).

use crate::cloning::enzymes::iupac_reverse_complement;

/// Schneidet je Bereich `(start, ende, reverse)` die Basen aus: `bases[start - flank .. ende + flank]`, an den Rändern der Sequenz
/// begrenzt (`start.saturating_sub(flank)` und `min(ende + flank, len)`). Bei `reverse` wird das Reverskomplement
/// (`iupac_reverse_complement`) geliefert. Ist `start >= ende` oder `start >= len`, ist der Eintrag ein leerer Text. Das Ergebnis hat
/// so viele Einträge wie `spans`. Zugriff mit `str::get`, damit Sonderzeichen nicht abstürzen (dann leerer Text).
/// Beispiel `AACCGGTT`: `(2, 4, false)` ohne Rand → `CC`, mit Rand 1 → `ACCG`, `(2, 4, true)` → `GG`.
pub fn extract_sequences(bases: &str, spans: &[(usize, usize, bool)], flank: usize) -> Vec<String> {
    let len = bases.len();

    spans
        .iter()
        .map(|&(start, end, reverse)| {
            // Ist `start >= ende` oder `start >= len`, ist der Eintrag ein leerer Text.
            if start >= end || start >= len {
                return String::new();
            }

            // Begrenzung an den Rändern der Sequenz
            let actual_start = start.saturating_sub(flank);
            let actual_end = (end + flank).min(len);

            // Zugriff mit `str::get`, damit Sonderzeichen nicht abstürzen (dann leerer Text).
            match bases.get(actual_start..actual_end) {
                Some(seq) => {
                    if reverse {
                        iupac_reverse_complement(seq)
                    } else {
                        seq.to_string()
                    }
                }
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ausschneiden() {
        let b = "AACCGGTT";
        assert_eq!(extract_sequences(b, &[(2, 4, false)], 0), vec!["CC"]);
        assert_eq!(extract_sequences(b, &[(2, 4, false)], 1), vec!["ACCG"]);
        assert_eq!(extract_sequences(b, &[(2, 4, true)], 0), vec!["GG"]);
        assert_eq!(extract_sequences(b, &[(6, 8, false)], 3), vec!["CGGTT"]);
        assert_eq!(extract_sequences(b, &[(0, 2, true)], 2), vec!["GGTT"]);
    }

    #[test]
    fn randfaelle() {
        let b = "AACCGGTT";
        assert_eq!(
            extract_sequences(b, &[(5, 3, false), (3, 3, false), (20, 30, false)], 0),
            vec!["", "", ""]
        );
        assert_eq!(extract_sequences(b, &[(6, 99, false)], 0), vec!["TT"]);
        assert!(extract_sequences(b, &[], 1).is_empty());
        assert_eq!(extract_sequences("ÄCGT", &[(0, 2, false)], 0).len(), 1);
    }
}
