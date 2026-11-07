//! Read InterProScan output in TSV format.

use crate::error::{Error, Result};

/// A hit of a signature.
#[derive(Clone, Debug, PartialEq)]
pub struct InterProHit {
    /// Accession/name of the protein (column 1).
    pub protein: String,
    /// Analysis, e.g. `Pfam` (column 4).
    pub analysis: String,
    /// Signature accession (column 5).
    pub signature: String,
    /// Description of the signature (column 6, `-` -> empty).
    pub description: String,
    /// Start and end (columns 7 and 8), 1-based.
    pub start: usize,
    pub end: usize,
    /// Score (column 9); `-` -> `None`.
    pub score: Option<f64>,
    /// InterPro accession (column 12); `-` or missing -> `None`.
    pub interpro: Option<String>,
}

/// Reads the TSV output (tab-separated fields). Empty lines and lines starting with `#` are skipped. Each line
/// needs at least 9 fields. `Error::InvalidInput`: too few fields or start/end not integers (with line number).
pub fn parse_interpro_tsv(text: &str) -> Result<Vec<InterProHit>> {
    let mut hits = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines or comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = trimmed.split('\t').collect();

        // At least 9 fields required (indices 0 to 8)
        if fields.len() < 9 {
            return Err(Error::InvalidInput(format!(
                "Line {}: Too few fields ({} present, 9 needed)",
                line_num + 1,
                fields.len()
            )));
        }

        // Column 1: protein (index 0)
        let protein = fields[0].to_string();

        // Column 4: analysis (index 3)
        let analysis = fields[3].to_string();

        // Column 5: signature (index 4)
        let signature = fields[4].to_string();

        // Column 6: description (index 5), '-' becomes an empty string
        let description = if fields[5] == "-" {
            String::new()
        } else {
            fields[5].to_string()
        };

        // Column 7: start (index 6), must be an integer
        let start = fields[6].parse::<usize>().map_err(|_| {
            Error::InvalidInput(format!(
                "Line {}: Start '{}' is not a valid number",
                line_num + 1,
                fields[6]
            ))
        })?;

        // Column 8: end (index 7), must be an integer
        let end = fields[7].parse::<usize>().map_err(|_| {
            Error::InvalidInput(format!(
                "Line {}: End '{}' is not a valid number",
                line_num + 1,
                fields[7]
            ))
        })?;

        // Column 9: score (index 8), '-' or invalid number -> None
        let score = if fields[8] == "-" {
            None
        } else {
            // Try to parse the score as f64.
            // InterProScan often uses scientific notation (e.g. 1.5E-50).
            // FromStr for f64 supports this by default.
            fields[8].parse::<f64>().ok()
        };

        // Column 12: InterPro (index 11)
        // If fewer than 12 fields are present, it is None.
        // If present and '-', then None. Otherwise a string.
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
