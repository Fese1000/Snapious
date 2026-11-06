//! BLAST-Ergebnisse im Tabellenformat (`-outfmt 6`) lesen.

use crate::error::{Error, Result};

/// Eine Zeile der Tabelle.
#[derive(Clone, Debug, PartialEq)]
pub struct BlastTableRow {
    pub query: String,
    pub subject: String,
    pub identity_percent: f64,
    pub alignment_length: usize,
    pub mismatches: usize,
    pub gap_opens: usize,
    pub query_start: usize,
    pub query_end: usize,
    pub subject_start: usize,
    pub subject_end: usize,
    pub evalue: f64,
    pub bit_score: f64,
}

/// Liest die zwölf Standardspalten (`qseqid sseqid pident length mismatch gapopen qstart qend sstart send evalue bitscore`), durch
/// Tabulator getrennt (weitere Spalten werden ignoriert). Leere Zeilen und Zeilen mit `#` am Anfang werden übersprungen.
/// Fehler `Error::InvalidInput` (mit Zeilennummer): weniger als 12 Felder oder eine nicht lesbare Zahl.
pub fn parse_blast_table(text: &str) -> Result<Vec<BlastTableRow>> {
    let mut rows = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // Leere Zeilen oder Kommentare überspringen
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = trimmed.split('\t').collect();

        // Mindestens 12 Spalten erforderlich
        if fields.len() < 12 {
            return Err(Error::InvalidInput(format!(
                "Zeile {}: Mindestens 12 Spalten erwartet, gefunden {}",
                line_num + 1,
                fields.len()
            )));
        }

        // Parsing der Felder
        let query = fields[0].to_string();
        let subject = fields[1].to_string();

        let identity_percent: f64 = fields[2].parse().map_err(|_| {
            Error::InvalidInput(format!(
                "Zeile {}: Ungültige identity_percent",
                line_num + 1
            ))
        })?;

        let alignment_length: usize = fields[3].parse().map_err(|_| {
            Error::InvalidInput(format!(
                "Zeile {}: Ungültige alignment_length",
                line_num + 1
            ))
        })?;

        let mismatches: usize = fields[4].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige mismatches", line_num + 1))
        })?;

        let gap_opens: usize = fields[5].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige gap_opens", line_num + 1))
        })?;

        let query_start: usize = fields[6].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige query_start", line_num + 1))
        })?;

        let query_end: usize = fields[7].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige query_end", line_num + 1))
        })?;

        let subject_start: usize = fields[8].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige subject_start", line_num + 1))
        })?;

        let subject_end: usize = fields[9].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige subject_end", line_num + 1))
        })?;

        // E-Value Parsing (kann wissenschaftliche Notation enthalten)
        let evalue: f64 = fields[10].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige evalue", line_num + 1))
        })?;

        let bit_score: f64 = fields[11].parse().map_err(|_| {
            Error::InvalidInput(format!("Zeile {}: Ungültige bit_score", line_num + 1))
        })?;

        rows.push(BlastTableRow {
            query,
            subject,
            identity_percent,
            alignment_length,
            mismatches,
            gap_opens,
            query_start,
            query_end,
            subject_start,
            subject_end,
            evalue,
            bit_score,
        });
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeilen() {
        let t = "# BLASTN\nq1\ts1\t99.5\t200\t1\t0\t1\t200\t1000\t1199\t1e-100\t350.5\nq2\ts2\t80.25\t100\t20\t0\t5\t104\t50\t149\t3.5e-10\t60\t99\n";
        let r = parse_blast_table(t).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0],
            BlastTableRow {
                query: "q1".into(),
                subject: "s1".into(),
                identity_percent: 99.5,
                alignment_length: 200,
                mismatches: 1,
                gap_opens: 0,
                query_start: 1,
                query_end: 200,
                subject_start: 1000,
                subject_end: 1199,
                evalue: 1e-100,
                bit_score: 350.5,
            }
        );
        assert_eq!(r[1].evalue, 3.5e-10);
    }

    #[test]
    fn fehler() {
        assert!(parse_blast_table("a\tb\t1\n").is_err());
        assert!(parse_blast_table("q\ts\tx\t1\t1\t0\t1\t1\t1\t1\t1\t1\n").is_err());
        assert!(parse_blast_table("").unwrap().is_empty());
    }
}
