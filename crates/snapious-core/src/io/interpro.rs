//! InterProScan-Ausgabe im TSV-Format lesen.

use crate::error::{Error, Result};

/// Ein Treffer einer Signatur.
#[derive(Clone, Debug, PartialEq)]
pub struct InterProHit {
    /// Zugangsnummer/Name des Proteins (Spalte 1).
    pub protein: String,
    /// Analyse, z. B. `Pfam` (Spalte 4).
    pub analysis: String,
    /// Signatur-Zugangsnummer (Spalte 5).
    pub signature: String,
    /// Beschreibung der Signatur (Spalte 6, `-` → leer).
    pub description: String,
    /// Beginn und Ende (Spalten 7 und 8), 1-basiert.
    pub start: usize,
    pub end: usize,
    /// Wert (Spalte 9); `-` → `None`.
    pub score: Option<f64>,
    /// InterPro-Zugangsnummer (Spalte 12); `-` oder fehlend → `None`.
    pub interpro: Option<String>,
}

/// Liest die TSV-Ausgabe (Felder durch Tabulator getrennt). Leere Zeilen und Zeilen mit `#` am Anfang werden übersprungen. Jede Zeile
/// braucht mindestens 9 Felder. Fehler `Error::InvalidInput`: zu wenige Felder oder Beginn/Ende keine ganzen Zahlen (mit Zeilennummer).
pub fn parse_interpro_tsv(text: &str) -> Result<Vec<InterProHit>> {
    let mut hits = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // Leere Zeilen oder Kommentare überspringen
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = trimmed.split('\t').collect();

        // Mindestens 9 Felder erforderlich (Indizes 0 bis 8)
        if fields.len() < 9 {
            return Err(Error::InvalidInput(format!(
                "Zeile {}: Zu wenige Felder ({} vorhanden, 9 benötigt)",
                line_num + 1,
                fields.len()
            )));
        }

        // Spalte 1: Protein (Index 0)
        let protein = fields[0].to_string();

        // Spalte 4: Analyse (Index 3)
        let analysis = fields[3].to_string();

        // Spalte 5: Signatur (Index 4)
        let signature = fields[4].to_string();

        // Spalte 6: Beschreibung (Index 5), '-' wird zu leerem String
        let description = if fields[5] == "-" {
            String::new()
        } else {
            fields[5].to_string()
        };

        // Spalte 7: Start (Index 6), muss eine ganze Zahl sein
        let start = fields[6].parse::<usize>().map_err(|_| {
            Error::InvalidInput(format!(
                "Zeile {}: Start '{}' ist keine gültige Zahl",
                line_num + 1,
                fields[6]
            ))
        })?;

        // Spalte 8: Ende (Index 7), muss eine ganze Zahl sein
        let end = fields[7].parse::<usize>().map_err(|_| {
            Error::InvalidInput(format!(
                "Zeile {}: Ende '{}' ist keine gültige Zahl",
                line_num + 1,
                fields[7]
            ))
        })?;

        // Spalte 9: Score (Index 8), '-' oder ungültige Zahl -> None
        let score = if fields[8] == "-" {
            None
        } else {
            // Versuche, den Score als f64 zu parsen.
            // InterProScan nutzt oft wissenschaftliche Notation (z.B. 1.5E-50).
            // VonStr für f64 unterstützt das standardmäßig.
            fields[8].parse::<f64>().ok()
        };

        // Spalte 12: InterPro (Index 11)
        // Falls weniger als 12 Felder vorhanden sind, ist es None.
        // Wenn vorhanden und '-', dann None. Sonst String.
        let interpro = if fields.len() > 11 {
            let val = fields[11];
            if val == "-" || val.is_empty() {
                None
            } else {
                Some(val.to_string())
            }
        } else {
            None
        };

        hits.push(InterProHit {
            protein,
            analysis,
            signature,
            description,
            start,
            end,
            score,
            interpro,
        });
    }

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zwei_zeilen() {
        let t = "P1\tabc123\t300\tPfam\tPF00069\tProtein kinase domain\t20\t275\t1.5E-50\tT\t01-01-2025\tIPR000719\tProtein kinase\tGO:0004672\t-\nP1\tabc123\t300\tSMART\tSM00220\t-\t10\t280\t-\tT\t01-01-2025\t-\t-\n";
        let r = parse_interpro_tsv(t).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0],
            InterProHit {
                protein: "P1".into(),
                analysis: "Pfam".into(),
                signature: "PF00069".into(),
                description: "Protein kinase domain".into(),
                start: 20,
                end: 275,
                score: Some(1.5e-50),
                interpro: Some("IPR000719".into()),
            }
        );
        assert_eq!(
            (r[1].description.as_str(), r[1].score, r[1].interpro.clone()),
            ("", None, None)
        );
    }

    #[test]
    fn fehler() {
        assert!(parse_interpro_tsv("a\tb\tc\n").is_err());
        assert!(parse_interpro_tsv("P\tm\t1\tX\tY\tZ\tx\t5\t-\n").is_err());
        assert!(parse_interpro_tsv("# k\n\n").unwrap().is_empty());
        let kurz = "P\tm\t1\tX\tY\tZ\t1\t5\t-";
        assert_eq!(parse_interpro_tsv(kurz).unwrap()[0].interpro, None);
    }
}
