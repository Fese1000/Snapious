//! Read the output of `hmmscan --domtblout` (protein domains, e.g. Pfam).

use crate::error::{Error, Result};
use std::str::FromStr;

/// A domain hit.
#[derive(Clone, Debug, PartialEq)]
pub struct DomainHit {
    /// Name of the domain (column 1, `target name`).
    pub domain: String,
    /// Accession of the domain (column 2, `-` -> empty).
    pub accession: String,
    /// Name of the query sequence (column 4, `query name`).
    pub query: String,
    /// E-value of this domain (column 13, `i-Evalue`).
    pub evalue: f64,
    /// Score of this domain (column 14).
    pub score: f64,
    /// Start and end of the alignment in the query, 1-based (columns 18 and 19, `ali from`, `ali to`).
    pub ali_from: usize,
    pub ali_to: usize,
}

/// Reads the domain table. Lines starting with `#` and empty lines are skipped. Every other line has at least
/// 22 whitespace-separated fields (the description text at the end may contain spaces and is not read).
/// `Error::InvalidInput`: fewer than 22 fields or an unreadable number (line number 1-based in the message).
pub fn parse_domtblout(text: &str) -> Result<Vec<DomainHit>> {
    let mut hits = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        let current_line = line_idx + 1;

        // Skip empty lines or comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Read lines by index, never index directly without a length check.
        let fields: Vec<&str> = line.split_whitespace().collect();

        if fields.len() < 22 {
            return Err(Error::InvalidInput(format!(
                "Line {}: At least 22 fields expected, {} found.",
                current_line,
                fields.len()
            )));
        }

        // Column 1: target name (domain)
        let domain = fields[0].to_string();

        // Column 2: accession (may be '-' -> empty)
        let accession = if fields[1] == "-" {
            String::new()
        } else {
            fields[1].to_string()
        };

        // Column 4: query name (0-based index 3)
        let query = fields[3].to_string();

        // Column 13: i-Evalue (0-based index 12)
        let evalue_str = fields[12];
        let evalue = parse_evalue(evalue_str, current_line)?;

        // Column 14: score (0-based index 13)
        let score_str = fields[13];
        let score = parse_float(score_str, "Score", current_line)?;

        // Column 18: ali from (0-based index 17)
        let ali_from_str = fields[17];
        let ali_from = parse_usize(ali_from_str, "ali from", current_line)?;

        // Column 19: ali to (0-based index 18)
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

/// Helper function for parsing E-values (may contain scientific notation).
fn parse_evalue(s: &str, line: usize) -> Result<f64> {
    // Hmmer E-values can be e.g. "1.2e-50".
    // std::str::FromStr for f64 supports this by default.
    f64::from_str(s).map_err(|_| {
        Error::InvalidInput(format!(
            "Line {}: Could not parse E-value '{}'.",
            line, s
        ))
    })
}

/// Helper function for parsing floats (score).
fn parse_float(s: &str, field_name: &str, line: usize) -> Result<f64> {
    f64::from_str(s).map_err(|_| {
        Error::InvalidInput(format!(
            "Line {}: Could not parse {} '{}'.",
            line, field_name, s
        ))
    })
}

/// Helper function for parsing usize (indices).
fn parse_usize(s: &str, field_name: &str, line: usize) -> Result<usize> {
    s.parse::<usize>().map_err(|_| {
        Error::InvalidInput(format!(
            "Line {}: Could not parse {} '{}'.",
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
