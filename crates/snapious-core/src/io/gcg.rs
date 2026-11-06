//! GCG-Sequenzdateien (einzelne Sequenz) lesen.

use crate::error::{Error, Result};

/// Liest eine GCG-Sequenz. Gesucht wird die erste Zeile, die (getrimmt) mit `..` endet; der Name ist ihr erstes durch Leerraum
/// getrenntes Wort. Alle Zeilen danach sind Sequenz: nur Buchstaben bleiben (Zeilennummern, Leerraum usw. entfallen), in
/// Großbuchstaben. Ergebnis `(Name, Sequenz)`. Fehler `Error::InvalidInput`, wenn es keine solche Zeile gibt oder die Sequenz leer ist.
pub fn parse_gcg(text: &str) -> Result<(String, String)> {
    let mut found_marker = false;
    let mut name = String::new();
    let mut sequence = Vec::new();

    for line in text.lines() {
        if !found_marker {
            let trimmed = line.trim();
            if trimmed.ends_with("..") {
                // Name ist das erste durch Leerraum getrennte Wort
                if let Some(first_word) = trimmed.split_whitespace().next() {
                    name = first_word.to_string();
                } else {
                    return Err(Error::InvalidInput("Kein Name in Marker-Zeile".to_string()));
                }
                found_marker = true;
            }
        } else {
            // Nur Buchstaben behalten, alles andere ignorieren (Zahlen, Leerzeichen etc.)
            for c in line.chars() {
                if c.is_ascii_alphabetic() {
                    sequence.push(c.to_ascii_uppercase());
                }
            }
        }
    }

    if !found_marker {
        return Err(Error::InvalidInput(
            "Keine GCG-Marker-Zeile (.. am Ende) gefunden".to_string(),
        ));
    }

    if sequence.is_empty() {
        return Err(Error::InvalidInput("Sequenz ist leer".to_string()));
    }

    Ok((name, sequence.into_iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lesen() {
        let t = "!!NA_SEQUENCE 1.0\n\nsome description\n\n  seq1  Length: 12  Type: N  Check: 123  ..\n\n        1 ACGTACGTAC GT\n";
        assert_eq!(
            parse_gcg(t).unwrap(),
            ("seq1".to_string(), "ACGTACGTACGT".to_string())
        );
        let t2 = "x ..\n 1 acg\n 4 tt\n";
        assert_eq!(
            parse_gcg(t2).unwrap(),
            ("x".to_string(), "ACGTT".to_string())
        );
    }

    #[test]
    fn fehler() {
        assert!(parse_gcg("ACGT\n").is_err());
        assert!(parse_gcg("name ..\n   1 \n").is_err());
        assert!(parse_gcg("").is_err());
    }
}
