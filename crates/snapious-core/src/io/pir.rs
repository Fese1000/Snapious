//! Read PIR/NBRF sequence files.

use crate::error::{Error, Result};

/// A PIR record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PirRecord {
    /// Two-letter code before the semicolon, e.g. `P1` (protein) or `DL` (DNA linear).
    pub kind: String,
    pub id: String,
    /// The line after the header line (trimmed, may be empty).
    pub description: String,
    /// Sequence without whitespace, digits and `*`.
    pub sequence: String,
}

/// Reads all records. A header line starts with `>` and contains a semicolon: `kind` is between `>` and `;`, `id` after it
/// (both trimmed). The next line is the description (even if empty). All further lines up to the next header line are
/// sequence: whitespace, digits and `*` are removed, the remaining characters stay. Lines before the first header line are
/// ignored. `Error::InvalidInput`: a header line without semicolon or no records.
pub fn parse_pir(text: &str) -> Result<Vec<PirRecord>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut records = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if line.starts_with('>') {
            // Header line i
            let semi_pos = match line.find(';') {
                Some(pos) => pos,
                None => return Err(Error::InvalidInput("Header line without semicolon".into())),
            };

            let kind = line[1..semi_pos].trim().to_string();
            let id = line[semi_pos + 1..].trim().to_string();

            if kind.is_empty() || id.is_empty() {
                return Err(Error::InvalidInput("Invalid header line".into()));
            }

            // Next line (i+1) is the description, if present
            let description = if i + 1 < lines.len() {
                lines[i + 1].trim().to_string()
            } else {
                String::new()
            };

            // All lines from i+2 up to the next header are sequence
            let mut seq_builder = String::new();
            let mut j = i + 2;

            while j < lines.len() {
                let next_line = lines[j];
                // Check whether the next line is a new header
                if next_line.starts_with('>') {
                    break;
                }

                // Process sequence line: remove whitespace, digits and *
                for c in next_line.chars() {
                    if !c.is_whitespace() && !c.is_ascii_digit() && c != '*' {
                        seq_builder.push(c);
                    }
                }
                j += 1;
            }

            records.push(PirRecord {
                kind,
                id,
                description,
                sequence: seq_builder,
            });

            // Jump to the next header or the end
            i = j;
        } else {
            // Ignore line before the first header
            i += 1;
        }
    }

    if records.is_empty() {
        return Err(Error::InvalidInput(
            "No valid entries found".into(),
        ));
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zwei_eintraege() {
        let t = ">P1;SEQ1\nTest protein\nMKV LAA\nGG*\n>DL;SEQ2\n\nACGT\n";
        let r = parse_pir(t).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0],
            PirRecord {
                kind: "P1".into(),
                id: "SEQ1".into(),
                description: "Test protein".into(),
                sequence: "MKVLAAGG".into()
            }
        );
        assert_eq!(
            (
                r[1].kind.as_str(),
                r[1].id.as_str(),
                r[1].description.as_str(),
                r[1].sequence.as_str()
            ),
            ("DL", "SEQ2", "", "ACGT")
        );
    }

    #[test]
    fn fehler() {
        assert!(parse_pir(">ohne semikolon\nx\nACGT\n").is_err());
        assert!(parse_pir("nur text\n").is_err());
        assert!(parse_pir("").is_err());
        assert_eq!(
            parse_pir("vorspann\n>P1;a\nd\nAC12 GT\n").unwrap()[0].sequence,
            "ACGT"
        );
    }
}
