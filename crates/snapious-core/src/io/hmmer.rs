//! Ausgabe von `hmmscan --domtblout` (Proteindomänen, z. B. Pfam) lesen.

use crate::error::{Error, Result};
use std::str::FromStr;

/// Ein Domänentreffer.
#[derive(Clone, Debug, PartialEq)]
pub struct DomainHit {
    /// Name der Domäne (Spalte 1, `target name`).
    pub domain: String,
    /// Zugangsnummer der Domäne (Spalte 2, `-` → leer).
    pub accession: String,
    /// Name der Abfragesequenz (Spalte 4, `query name`).
    pub query: String,
    /// E-Wert dieser Domäne (Spalte 13, `i-Evalue`).
    pub evalue: f64,
    /// Wert (Score) dieser Domäne (Spalte 14).
    pub score: f64,
    /// Beginn und Ende des Alignments in der Abfrage, 1-basiert (Spalten 18 und 19, `ali from`, `ali to`).
    pub ali_from: usize,
    pub ali_to: usize,
}

/// Liest die Domänentabelle. Zeilen, die mit `#` beginnen, und leere Zeilen werden übersprungen. Jede andere Zeile hat mindestens
/// 22 durch Leerraum getrennte Felder (der Beschreibungstext am Ende darf Leerzeichen enthalten und wird nicht gelesen).
/// Fehler `Error::InvalidInput`: weniger als 22 Felder oder eine nicht lesbare Zahl (Zeilennummer 1-basiert in der Meldung).
pub fn parse_domtblout(text: &str) -> Result<Vec<DomainHit>> {
    let mut hits = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        let current_line = line_idx + 1;

        // Leere Zeilen oder Kommentare überspringen
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Zeilen nach Index lesen, nie direkt indexieren ohne Längenprüfung.
        let fields: Vec<&str> = line.split_whitespace().collect();

        if fields.len() < 22 {
            return Err(Error::InvalidInput(format!(
                "Zeile {}: Mindestens 22 Felder erwartet, {} gefunden.",
                current_line,
                fields.len()
            )));
        }

        // Spalte 1: target name (domain)
        let domain = fields[0].to_string();

        // Spalte 2: accession (kann '-' sein -> leer)
        let accession = if fields[1] == "-" {
            String::new()
        } else {
            fields[1].to_string()
        };

        // Spalte 4: query name (0-basiert Index 3)
        let query = fields[3].to_string();

        // Spalte 13: i-Evalue (0-basiert Index 12)
        let evalue_str = fields[12];
        let evalue = parse_evalue(evalue_str, current_line)?;

        // Spalte 14: score (0-basiert Index 13)
        let score_str = fields[13];
        let score = parse_float(score_str, "Score", current_line)?;

        // Spalte 18: ali from (0-basiert Index 17)
        let ali_from_str = fields[17];
        let ali_from = parse_usize(ali_from_str, "ali from", current_line)?;

        // Spalte 19: ali to (0-basiert Index 18)
        let ali_to_str = fields[18];
        let ali_to = parse_usize(ali_to_str, "ali to", current_line)?;

        hits.push(DomainHit {
            domain,
            accession,
            query,
            evalue,
            score,
            ali_from,
            ali_to,
        });
    }

    Ok(hits)
}

/// Hilfstfunktion zum Parsen von E-Werten (kann wissenschaftliche Notation enthalten).
fn parse_evalue(s: &str, line: usize) -> Result<f64> {
    // Hmmer E-Werte können z.B. "1.2e-50" sein.
    // std::str::FromStr für f64 unterstützt das standardmäßig.
    f64::from_str(s).map_err(|_| {
        Error::InvalidInput(format!(
            "Zeile {}: Konnte E-Wert '{}' nicht parsen.",
            line, s
        ))
    })
}

/// Hilfstfunktion zum Parsen von Floats (Score).
fn parse_float(s: &str, field_name: &str, line: usize) -> Result<f64> {
    f64::from_str(s).map_err(|_| {
        Error::InvalidInput(format!(
            "Zeile {}: Konnte {} '{}' nicht parsen.",
            line, field_name, s
        ))
    })
}

/// Hilfstfunktion zum Parsen von usize (Indices).
fn parse_usize(s: &str, field_name: &str, line: usize) -> Result<usize> {
    s.parse::<usize>().map_err(|_| {
        Error::InvalidInput(format!(
            "Zeile {}: Konnte {} '{}' nicht parsen.",
            line, field_name, s
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZEILE: &str = "Pkinase PF00069.28 264 sp|P1 - 300 1.2e-50 170.5 0.1 1 1 2.3e-54 4.5e-50 160.2 0.0 1 262 20 275 18 280 0.95 Protein kinase domain";

    #[test]
    fn eine_zeile() {
        let t = format!("# Kommentar\n{ZEILE}\n\n");
        let r = parse_domtblout(&t).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(
            r[0],
            DomainHit {
                domain: "Pkinase".into(),
                accession: "PF00069.28".into(),
                query: "sp|P1".into(),
                evalue: 4.5e-50,
                score: 160.2,
                ali_from: 20,
                ali_to: 275,
            }
        );
    }

    #[test]
    fn leere_accession_und_fehler() {
        let z = ZEILE.replace("PF00069.28", "-");
        assert_eq!(parse_domtblout(&z).unwrap()[0].accession, "");
        assert!(parse_domtblout("a b c").is_err());
        assert!(parse_domtblout(&ZEILE.replace("4.5e-50", "abc")).is_err());
        assert!(parse_domtblout("# nur Kommentar\n").unwrap().is_empty());
        assert!(parse_domtblout("").unwrap().is_empty());
    }
}
