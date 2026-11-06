//! Paired-End-Reads anhand der Namen zusammenführen.

use std::collections::BTreeMap;

/// Ergebnis der Paarbildung (Indizes in der Namensliste).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairResult {
    /// `(Vorwärtsread, Rückwärtsread)`, aufsteigend nach dem Index des Vorwärtsreads.
    pub pairs: Vec<(usize, usize)>,
    /// Reads ohne Partner oder ohne erkennbare Paarinformation, aufsteigend.
    pub unpaired: Vec<usize>,
}

/// Erkennt Paare: Ein Name endet mit `/1` oder `/2` (Schlüssel = Name ohne diese zwei Zeichen), oder er enthält ein Leerzeichen und
/// der Teil danach beginnt mit `1:` oder `2:` (Schlüssel = Teil vor dem ersten Leerzeichen). Reads mit gleichem Schlüssel und den
/// Nummern 1 und 2 bilden ein Paar (kommt eine Nummer mehrfach vor, zählt das erste Vorkommen, die weiteren sind ungepaart).
/// Alle anderen Reads sind ungepaart.
pub fn pair_reads(names: &[String]) -> PairResult {
    // Map für gefundene Paare: Schlüssel -> (Index_von_1, Index_von_2)
    // Wir nutzen BTreeMap, um deterministisch zu sein.
    let mut pairs_map: BTreeMap<String, (Option<usize>, Option<usize>)> = BTreeMap::new();
    // Liste der ungepaarten Reads (Indizes)
    let mut unpaired_indices: Vec<usize> = Vec::new();

    for (idx, name) in names.iter().enumerate() {
        let (key, num) = match parse_pair_info(name) {
            Some(info) => info,
            None => {
                unpaired_indices.push(idx);
                continue;
            }
        };

        // Hole oder erstelle Entry für den Schlüssel
        let entry = pairs_map.entry(key).or_insert((None, None));

        match num {
            1 => {
                if entry.0.is_none() {
                    // Erster /1 für diesen Schlüssel gefunden
                    entry.0 = Some(idx);
                } else {
                    // Duplikat von /1 -> ungepaart
                    unpaired_indices.push(idx);
                }
            }
            2 => {
                if entry.1.is_none() {
                    // Erster /2 für diesen Schlüssel gefunden
                    entry.1 = Some(idx);
                } else {
                    // Duplikat von /2 -> ungepaart
                    unpaired_indices.push(idx);
                }
            }
            _ => {} // Sollte nicht passieren durch parse_pair_info
        }
    }

    // Paare extrahieren
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    for (_, (first_opt, second_opt)) in pairs_map {
        if let (Some(first), Some(second)) = (first_opt, second_opt) {
            pairs.push((first, second));
        } else {
            // Wenn nur einer vorhanden ist (z.B. nur /1 aber kein /2), sind beide ungepaart?
            // Die Aufgabenstellung sagt: "Reads mit gleichem Schlüssel und den Nummern 1 und 2 bilden ein Paar".
            // "Alle anderen Reads sind ungepaart."
            // Wenn wir also einen /1 haben, aber kein passendes /2 (weil der Schlüssel anders ist oder fehlt),
            // dann ist dieser /1 ungepaart. Gleiches für /2 ohne /1.
            if let Some(first) = first_opt {
                unpaired_indices.push(first);
            }
            if let Some(second) = second_opt {
                unpaired_indices.push(second);
            }
        }
    }

    // Sortiere Paare nach dem Index des Vorwärtsreads (erster Wert im Tuple)
    pairs.sort_by_key(|&(f, _)| f);

    // Unpaired-Liste sortieren (aufsteigend nach Index)
    unpaired_indices.sort();

    PairResult {
        pairs,
        unpaired: unpaired_indices,
    }
}

/// Parst einen Read-Namen auf Paar-Information.
/// Gibt `(Schlüssel, Nummer)` zurück, wobei Nummer 1 oder 2 ist.
fn parse_pair_info(name: &str) -> Option<(String, u8)> {
    // Regel 1: Endet mit `/1` oder `/2`
    if let Some(stripped) = name.strip_suffix("/1") {
        return Some((stripped.to_string(), 1));
    }
    if let Some(stripped) = name.strip_suffix("/2") {
        return Some((stripped.to_string(), 2));
    }

    // Regel 2: Enthält Leerzeichen, Teil danach beginnt mit `1:` oder `2:`
    if let Some(space_pos) = name.find(' ') {
        let suffix = &name[space_pos + 1..];
        if suffix.starts_with("1:") {
            // Schlüssel ist Teil vor dem ersten Leerzeichen
            return Some((name[..space_pos].to_string(), 1));
        }
        if suffix.starts_with("2:") {
            return Some((name[..space_pos].to_string(), 2));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn schraegstrich_und_illumina() {
        let names = v(&["r1/1", "r2/1", "r1/2", "r3 2:N:0", "r3 1:N:0", "x", "r4/2"]);
        let r = pair_reads(&names);
        assert_eq!(r.pairs, vec![(0, 2), (4, 3)]);
        assert_eq!(r.unpaired, vec![1, 5, 6]);
    }

    #[test]
    fn doppelte_und_leer() {
        let r = pair_reads(&v(&["a/1", "a/1", "a/2"]));
        assert_eq!(r.pairs, vec![(0, 2)]);
        assert_eq!(r.unpaired, vec![1]);
        let leer = pair_reads(&[]);
        assert!(leer.pairs.is_empty() && leer.unpaired.is_empty());
    }
}
