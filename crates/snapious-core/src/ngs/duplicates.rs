//! PCR-Duplikate nach Position markieren.

use std::collections::BTreeMap;

/// Ein Read für die Duplikatsuche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadKey {
    pub reference: String,
    /// Startposition.
    pub position: usize,
    pub reverse: bool,
    /// Summe der Qualitätswerte (höher = besser).
    pub quality: u64,
}

/// Markiert Duplikate: Reads mit gleicher Referenz, Position und Richtung sind eine Gruppe. In jeder Gruppe bleibt der Read mit der
/// höchsten `quality` unmarkiert (bei Gleichstand der mit dem kleinsten Index), alle anderen sind Duplikate. Ergebnis: je Read `true`, wenn er ein
/// Duplikat ist (gleiche Länge und Reihenfolge wie `reads`).
pub fn mark_duplicates(reads: &[ReadKey]) -> Vec<bool> {
    if reads.is_empty() {
        return vec![];
    }

    // Gruppieren der Indizes nach (reference, position, reverse)
    let mut groups: BTreeMap<(String, usize, bool), Vec<usize>> = BTreeMap::new();

    for (i, read) in reads.iter().enumerate() {
        let key = (read.reference.clone(), read.position, read.reverse);
        groups.entry(key).or_default().push(i);
    }

    // Ergebnis-Array initialisieren (alle false)
    let mut result = vec![false; reads.len()];

    // Durch jede Gruppe gehen und das beste Read markieren (false), Rest true
    for indices in groups.values() {
        // Finde den Index des besten Reads in dieser Gruppe
        let mut best_idx = indices[0];

        for &idx in indices.iter().skip(1) {
            let current_quality = reads[idx].quality;
            let best_quality = reads[best_idx].quality;

            if current_quality > best_quality {
                // Höhere Qualität -> neues Bestes
                best_idx = idx;
            } else if current_quality == best_quality {
                // Gleiche Qualität -> kleinerer Index gewinnt (bereits im Ergebnis als false markiert)
                // Da wir die Indizes in aufsteigender Reihenfolge durchgehen (0, 1, 2...),
                // bleibt der erste gefundene Index (kleinster Index) der Gewinner.
                // Wir müssen also nichts tun, wenn current_quality <= best_quality.
            }
        }

        // Alle Indizes in der Gruppe sind Duplikate, außer dem besten
        for &idx in indices.iter() {
            if idx != best_idx {
                result[idx] = true;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(reference: &str, position: usize, reverse: bool, quality: u64) -> ReadKey {
        ReadKey {
            reference: reference.into(),
            position,
            reverse,
            quality,
        }
    }

    #[test]
    fn gruppen() {
        let reads = vec![
            k("c1", 10, false, 30),
            k("c1", 10, false, 50),
            k("c1", 10, true, 20),
            k("c2", 10, false, 20),
            k("c1", 10, false, 50),
            k("c1", 11, false, 10),
        ];
        assert_eq!(
            mark_duplicates(&reads),
            vec![true, false, false, false, true, false]
        );
    }

    #[test]
    fn leer_und_einzeln() {
        assert!(mark_duplicates(&[]).is_empty());
        assert_eq!(mark_duplicates(&[k("a", 1, false, 5)]), vec![false]);
        assert_eq!(
            mark_duplicates(&[
                k("a", 1, false, 5),
                k("a", 1, false, 5),
                k("a", 1, false, 5)
            ]),
            vec![false, true, true]
        );
    }
}
