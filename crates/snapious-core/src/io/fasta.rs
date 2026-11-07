//! Read and write FASTA.

use std::path::Path;

use crate::error::{Error, Result};
use crate::model::sequence::Sequence;

/// ID from a header line: first word after `>`.
pub fn extract_id(header: &str) -> String {
    header
        .trim_start_matches('>')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// Reads all records. Text before the first `>` is ignored; empty lines and `\r` are allowed.
/// The sequence type is detected (only ACGTN -> DNA, only ACGUN -> RNA, otherwise protein).
pub fn parse(text: &str) -> Result<Vec<Sequence>> {
    let mut out = Vec::new();
    let mut header: Option<(usize, String)> = None;
    let mut body = String::new();
    let flush = |header: &mut Option<(usize, String)>,
                 body: &mut String,
                 out: &mut Vec<Sequence>|
     -> Result<()> {
        if let Some((line, h)) = header.take() {
            let s = Sequence::from_fasta(&extract_id(&h), &h, body)
                .map_err(|e| Error::Io(format!("Line {}: {}", line, e)))?;
            out.push(s);
        }
        body.clear();
        Ok(())
    };
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if let Some(h) = line.strip_prefix('>') {
            flush(&mut header, &mut body, &mut out)?;
            header = Some((i + 1, format!(">{}", h.trim())));
        } else if header.is_some() {
            body.push_str(line.trim());
            body.push('\n');
        }
    }
    flush(&mut header, &mut body, &mut out)?;
    Ok(out)
}

pub fn read(path: &Path) -> Result<Vec<Sequence>> {
    parse(&std::fs::read_to_string(path)?)
}

/// FASTA text (60 characters per line, each record ending with a line break).
pub fn format(sequences: &[Sequence]) -> String {
    let mut out = String::new();
    for s in sequences {
        out.push_str(&s.to_fasta());
        out.push('\n');
    }
    out
}

pub fn write(sequences: &[Sequence], path: &Path) -> Result<()> {
    std::fs::write(path, format(sequences))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::SequenceType;

    #[test]
    fn zwei_eintraege() {
        let s = parse(">seq1 erste\nACGT\nACGT\n>seq2\nTTTT\n").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id(), "seq1");
        assert_eq!(s[0].name(), "seq1 erste");
        assert_eq!(s[0].bases(), "ACGTACGT");
        assert_eq!(s[1].id(), "seq2");
    }

    #[test]
    fn windows_zeilenenden_leerzeilen_und_text_davor() {
        let s = parse("Kommentar\r\n\r\n>a\r\nAC\r\n\r\nGT\r\n").unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].bases(), "ACGT");
    }

    #[test]
    fn leerer_eintrag_und_leere_eingabe() {
        let s = parse(">leer\n>voll\nAC\n").unwrap();
        assert_eq!(s[0].bases(), "");
        assert_eq!(s[1].bases(), "AC");
        assert!(parse("").unwrap().is_empty());
        assert!(parse("ACGT\n").unwrap().is_empty());
    }

    #[test]
    fn typen_werden_erkannt() {
        let s = parse(">d\nACGT\n>r\nACGU\n>p\nMKVL\n").unwrap();
        assert_eq!(s[0].seq_type(), SequenceType::Dna);
        assert_eq!(s[1].seq_type(), SequenceType::Rna);
        assert_eq!(s[2].seq_type(), SequenceType::Protein);
    }

    #[test]
    fn ungueltiges_zeichen_meldet_zeile() {
        match parse(">ok\nACGT\n>kaputt\nAC1T\n") {
            Err(Error::Io(m)) => assert!(m.contains("Line 3"), "{}", m),
            other => panic!("unerwartet: {:?}", other.map(|v| v.len())),
        }
    }

    #[test]
    fn schreiben_und_wieder_lesen() {
        let seqs = parse(">a x\nACGTACGTAC\n>b\nGGGG\n").unwrap();
        let text = format(&seqs);
        let back = parse(&text).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].bases(), seqs[0].bases());
        assert_eq!(back[1].name(), "b");
    }

    #[test]
    fn lange_sequenz_wird_umbrochen() {
        let s = Sequence::new("l", "l", &"A".repeat(130), SequenceType::Dna).unwrap();
        let text = format(&[s]);
        let lens: Vec<usize> = text.lines().skip(1).map(str::len).collect();
        assert_eq!(lens, vec![60, 60, 10]);
    }

    #[test]
    fn datei_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.fa");
        let seqs = parse(">a\nACGT\n").unwrap();
        write(&seqs, &p).unwrap();
        assert_eq!(read(&p).unwrap()[0].bases(), "ACGT");
        assert!(read(&dir.path().join("fehlt.fa")).is_err());
    }

    #[test]
    fn id_aus_kopfzeile() {
        assert_eq!(extract_id(">abc def"), "abc");
        assert_eq!(extract_id("abc"), "abc");
        assert_eq!(extract_id(">"), "");
    }
}
