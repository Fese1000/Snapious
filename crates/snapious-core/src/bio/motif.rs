//! Motivsuche mit IUPAC-Codes auf beiden Strängen (auch über den Nullpunkt zirkulärer Sequenzen).

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};

/// Ein Fundort `[start, end)` auf dem Oberstrang; `reverse` bedeutet, dass das Reverse-Komplement des Motivs dort steht.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotifHit {
    pub start: usize,
    pub end: usize,
    pub reverse: bool,
}

/// Menge der Basen (A, C, G, T als Bits 1, 2, 4, 8) für einen IUPAC-Buchstaben; `None` für unbekannte Zeichen.
fn mask(c: u8) -> Option<u8> {
    Some(match c.to_ascii_uppercase() {
        b'A' => 1,
        b'C' => 2,
        b'G' => 4,
        b'T' | b'U' => 8,
        b'R' => 1 | 4,
        b'Y' => 2 | 8,
        b'S' => 2 | 4,
        b'W' => 1 | 8,
        b'K' => 4 | 8,
        b'M' => 1 | 2,
        b'B' => 2 | 4 | 8,
        b'D' => 1 | 4 | 8,
        b'H' => 1 | 2 | 8,
        b'V' => 1 | 2 | 4,
        b'N' => 15,
        _ => return None,
    })
}

/// Sucht `motif` (IUPAC, Schreibweise egal) in `bases`. Eine Position passt, wenn sich für jedes Zeichenpaar die Basenmengen schneiden
/// (so passt `N` im Motiv zu allem und `R` im Motiv zu `A`, `G` und `R` in der Sequenz). Mit `both_strands` wird zusätzlich das
/// Reverse-Komplement des Motivs gesucht (ist es mit dem Motiv identisch, wird jeder Ort nur einmal als Vorwärtstreffer gemeldet).
/// Bei `circular` zählen auch Treffer über den Nullpunkt: `start` liegt dann in `0..Länge` und `end` kann größer als die Länge sein.
/// Ergebnis sortiert nach `start`, dann `end`, dann Vorwärts vor Rückwärts. Fehler `Error::InvalidInput` bei leerem Motiv oder Zeichen,
/// die kein IUPAC-Code sind. Ist das Motiv länger als die Sequenz, ist das Ergebnis leer.
pub fn find_motif(
    bases: &str,
    motif: &str,
    both_strands: bool,
    circular: bool,
) -> Result<Vec<MotifHit>> {
    let m: Vec<u8> = motif.trim().bytes().collect();
    if m.is_empty() || m.iter().any(|c| mask(*c).is_none()) {
        return Err(Error::InvalidInput(
            "Invalid motif (IUPAC codes are allowed)".to_string(),
        ));
    }
    let seq: Vec<u8> = bases.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let len = seq.len();
    if len == 0 || m.len() > len {
        return Ok(Vec::new());
    }
    let forward: Vec<u8> = m.iter().filter_map(|c| mask(*c)).collect();
    let motif_text = String::from_utf8_lossy(&m).to_ascii_uppercase();
    let rc_text = iupac_reverse_complement(&motif_text);
    let backward: Vec<u8> = rc_text.bytes().filter_map(mask).collect();
    let palindrome = backward == forward;
    let starts = if circular { len } else { len - m.len() + 1 };
    let at = |i: usize| -> Option<u8> { seq.get(i % len).copied().and_then(mask) };
    let fits = |pattern: &[u8], start: usize| -> bool {
        pattern
            .iter()
            .enumerate()
            .all(|(k, p)| at(start + k).is_some_and(|s| s & p != 0))
    };
    let mut hits = Vec::new();
    for start in 0..starts {
        if fits(&forward, start) {
            hits.push(MotifHit {
                start,
                end: start + m.len(),
                reverse: false,
            });
        }
        if both_strands && !palindrome && fits(&backward, start) {
            hits.push(MotifHit {
                start,
                end: start + m.len(),
                reverse: true,
            });
        }
    }
    hits.sort_by_key(|h| (h.start, h.end, h.reverse));
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts(h: &[MotifHit]) -> Vec<(usize, bool)> {
        h.iter().map(|x| (x.start, x.reverse)).collect()
    }

    #[test]
    fn einfache_suche() {
        let h = find_motif("AAGAATTCAAGAATTC", "GAATTC", false, false).unwrap();
        assert_eq!(starts(&h), vec![(2, false), (10, false)]);
        assert_eq!((h[0].start, h[0].end), (2, 8));
        assert!(find_motif("ACGT", "GGG", true, false).unwrap().is_empty());
        assert!(find_motif("AC", "ACGT", true, false).unwrap().is_empty());
        assert!(find_motif("", "A", true, false).unwrap().is_empty());
    }

    #[test]
    fn iupac_und_schreibweise() {
        let h = find_motif("aagaattcaagagttc", "GARTTC", false, false).unwrap();
        assert_eq!(starts(&h), vec![(2, false), (10, false)]);
        let n = find_motif("ACGTA", "ANG", false, false).unwrap();
        assert_eq!(starts(&n), vec![(0, false)]);
        // Mehrdeutigkeit in der Sequenz schneidet sich mit dem Motiv
        let r = find_motif("ARGT", "AGGT", false, false).unwrap();
        assert_eq!(starts(&r), vec![(0, false)]);
        assert_eq!(find_motif("ACGU", "ACGT", false, false).unwrap().len(), 1);
    }

    #[test]
    fn beide_straenge() {
        // GGATCC ist ein Palindrom: nur ein Treffer; AAGCTT ebenfalls
        let p = find_motif("TTGGATCCTT", "GGATCC", true, false).unwrap();
        assert_eq!(starts(&p), vec![(2, false)]);
        // GGTCTC (BsaI) steht rückwärts als GAGACC
        let h = find_motif("TTGAGACCTT", "GGTCTC", true, false).unwrap();
        assert_eq!(starts(&h), vec![(2, true)]);
        let k = find_motif("GGTCTCAAGAGACC", "GGTCTC", true, false).unwrap();
        assert_eq!(starts(&k), vec![(0, false), (8, true)]);
        assert!(find_motif("TTGAGACCTT", "GGTCTC", false, false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn zirkulaer() {
        let h = find_motif("TTCAAGAA", "GAATTC", false, true).unwrap();
        assert_eq!(starts(&h), vec![(5, false)]);
        assert_eq!((h[0].start, h[0].end), (5, 11));
        assert!(find_motif("TTCAAGAA", "GAATTC", false, false)
            .unwrap()
            .is_empty());
        // Motiv genau so lang wie die Sequenz, an jeder Drehung
        assert_eq!(find_motif("ACG", "CGA", false, true).unwrap().len(), 1);
    }

    #[test]
    fn fehler() {
        assert!(matches!(
            find_motif("ACGT", "", true, false),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            find_motif("ACGT", "AXG", true, false),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            find_motif("ACGT", "A G", true, false),
            Err(Error::InvalidInput(_))
        ));
    }
}
