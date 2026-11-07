//! Read and write FASTQ (four lines per record, quality Phred+33).

use std::path::Path;

use crate::error::{Error, Result};
use crate::model::sequence::{Sequence, SequenceType};

/// A sequencing run record (read).
#[derive(Clone, Debug, PartialEq)]
pub struct FastqRecord {
    pub id: String,
    /// Remainder of the header line after the ID.
    pub comment: String,
    pub sequence: String,
    pub quality: String,
}

impl FastqRecord {
    /// Mean Phred quality (Phred+33); 0 for empty quality.
    pub fn avg_quality(&self) -> f64 {
        if self.quality.is_empty() {
            return 0.0;
        }
        self.quality.bytes().map(|b| b as f64 - 33.0).sum::<f64>() / self.quality.len() as f64
    }

    /// Lowest Phred quality (Phred+33); 0 for empty quality.
    pub fn min_quality(&self) -> i32 {
        self.quality
            .bytes()
            .map(|b| b as i32 - 33)
            .min()
            .unwrap_or(0)
    }

    /// As a DNA sequence (without quality).
    pub fn to_sequence(&self) -> Result<Sequence> {
        Sequence::new(&self.id, &self.id, &self.sequence, SequenceType::Dna)
    }
}

/// Reads all records. Each record has exactly four lines; sequence and quality must have the same length.
pub fn parse(text: &str) -> Result<Vec<FastqRecord>> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }
        let header = lines[i];
        let Some(h) = header.strip_prefix('@') else {
            return Err(Error::Io(format!(
                "Line {}: Header line starting with '@' expected",
                i + 1
            )));
        };
        if i + 3 >= lines.len() {
            return Err(Error::Io(format!(
                "Line {}: Entry is incomplete (4 lines expected)",
                i + 1
            )));
        }
        if !lines[i + 2].starts_with('+') {
            return Err(Error::Io(format!("Line {}: '+' expected", i + 3)));
        }
        let sequence = lines[i + 1].trim().to_ascii_uppercase();
        let quality = lines[i + 3].trim().to_string();
        if sequence.len() != quality.len() {
            return Err(Error::Io(format!(
                "Line {}: Sequence ({}) and quality ({}) have different lengths",
                i + 4,
                sequence.len(),
                quality.len()
            )));
        }
        let mut parts = h.splitn(2, char::is_whitespace);
        let id = parts.next().unwrap_or("").to_string();
        let comment = parts.next().unwrap_or("").trim().to_string();
        out.push(FastqRecord {
            id,
            comment,
            sequence,
            quality,
        });
        i += 4;
    }
    Ok(out)
}

pub fn read(path: &Path) -> Result<Vec<FastqRecord>> {
    parse(&std::fs::read_to_string(path)?)
}

/// FASTQ text.
pub fn format(records: &[FastqRecord]) -> String {
    let mut out = String::new();
    for r in records {
        out.push('@');
        out.push_str(&r.id);
        if !r.comment.is_empty() {
            out.push(' ');
            out.push_str(&r.comment);
        }
        out.push('\n');
        out.push_str(&r.sequence);
        out.push_str("\n+\n");
        out.push_str(&r.quality);
        out.push('\n');
    }
    out
}

pub fn write(records: &[FastqRecord], path: &Path) -> Result<()> {
    std::fs::write(path, format(records))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "@r1 lane1\nACGT\n+\nIIII\n@r2\nTTGA\n+r2\n!5I+\n";

    #[test]
    fn zwei_reads() {
        let r = parse(TEXT).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].id, "r1");
        assert_eq!(r[0].comment, "lane1");
        assert_eq!(r[0].sequence, "ACGT");
        assert_eq!(r[1].comment, "");
        assert_eq!(r[1].quality, "!5I+");
    }

    #[test]
    fn qualitaet() {
        let r = parse(TEXT).unwrap();
        assert_eq!(r[0].avg_quality(), 40.0);
        assert_eq!(r[0].min_quality(), 40);
        // '!' = 0, '5' = 20, 'I' = 40, '+' = 10 -> mean 17.5
        assert_eq!(r[1].avg_quality(), 17.5);
        assert_eq!(r[1].min_quality(), 0);
        assert_eq!(
            FastqRecord {
                id: "x".into(),
                comment: String::new(),
                sequence: String::new(),
                quality: String::new()
            }
            .avg_quality(),
            0.0
        );
    }

    #[test]
    fn round_trip() {
        let r = parse(TEXT).unwrap();
        let again = parse(&format(&r)).unwrap();
        assert_eq!(again, r);
    }

    #[test]
    fn fehler_mit_zeilennummer() {
        assert!(matches!(parse("@r\nACGT\n+\nIII\n"), Err(Error::Io(m)) if m.contains("Line 4")));
        assert!(matches!(parse("r\nACGT\n+\nIIII\n"), Err(Error::Io(m)) if m.contains("Line 1")));
        assert!(matches!(parse("@r\nACGT\n"), Err(Error::Io(m)) if m.contains("incomplete")));
        assert!(matches!(parse("@r\nACGT\nx\nIIII\n"), Err(Error::Io(m)) if m.contains("'+'")));
    }

    #[test]
    fn leere_zeilen_und_windows() {
        let r = parse("\n@r\r\nACGT\r\n+\r\nIIII\r\n\n").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].sequence, "ACGT");
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn kleinbuchstaben_und_sequenz() {
        let r = parse("@r\nacgn\n+\nIIII\n").unwrap();
        assert_eq!(r[0].sequence, "ACGN");
        assert_eq!(r[0].to_sequence().unwrap().bases(), "ACGN");
    }

    #[test]
    fn datei() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.fq");
        write(&parse(TEXT).unwrap(), &p).unwrap();
        assert_eq!(read(&p).unwrap().len(), 2);
    }
}
