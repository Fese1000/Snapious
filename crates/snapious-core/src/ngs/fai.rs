//! FASTA index in samtools format (`.fai`): build, read, write and fetch subsequences directly from the file.

use std::path::Path;

use crate::error::{Error, Result};

/// An index entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaiEntry {
    /// Name = first word of the header line after `>`.
    pub name: String,
    /// Number of bases.
    pub length: u64,
    /// Byte position of the first base in the file.
    pub offset: u64,
    /// Bases per line (length of the first sequence line).
    pub bases_per_line: u64,
    /// Bytes per line including the line ending (`bases_per_line + 1` for `\n`, `+ 2` for `\r\n`).
    pub bytes_per_line: u64,
}

/// Builds the index from the byte stream of a FASTA file. Header lines start with `>`; empty lines before the first entry are
/// ignored. `offset` = position of the first byte after the header line (including its line ending). `bases_per_line` is the
/// length of the first sequence line (without line ending), `bytes_per_line` its length with line ending (`\n` or `\r\n`, detected from the first
/// sequence line). Error `Error::InvalidInput`: Sequence line before the first header line; a line of an entry is longer than the first
/// or a line other than the last is shorter (message contains the name); duplicate name. An entry without sequence has `length 0`,
/// `offset` after the header line and `bases_per_line = bytes_per_line = 0`. The last line may end without a line ending.
pub fn build_fai(fasta: &[u8]) -> Result<Vec<FaiEntry>> {
    let mut entries: Vec<FaiEntry> = Vec::new();
    // current entry: (name, offset, first line length, bytes per line, length, last line was shorter)
    struct Cur {
        entry: FaiEntry,
        short_seen: bool,
    }
    let mut cur: Option<Cur> = None;
    let finish = |c: Option<Cur>, entries: &mut Vec<FaiEntry>| -> Result<()> {
        if let Some(c) = c {
            if entries.iter().any(|e| e.name == c.entry.name) {
                return Err(Error::InvalidInput(format!(
                    "Duplicate name '{}'",
                    c.entry.name
                )));
            }
            entries.push(c.entry);
        }
        Ok(())
    };
    let mut pos = 0usize;
    while pos < fasta.len() {
        let end = fasta[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(fasta.len(), |p| pos + p);
        let next = (end + 1).min(fasta.len());
        let mut content = &fasta[pos..end];
        let mut eol = next - end;
        if content.ends_with(b"\r") {
            content = &content[..content.len() - 1];
            eol += 1;
        }
        if content.starts_with(b">") {
            finish(cur.take(), &mut entries)?;
            let header = String::from_utf8_lossy(&content[1..]).to_string();
            let name = header.split_whitespace().next().unwrap_or("").to_string();
            cur = Some(Cur {
                entry: FaiEntry {
                    name,
                    length: 0,
                    offset: next as u64,
                    bases_per_line: 0,
                    bytes_per_line: 0,
                },
                short_seen: false,
            });
        } else if !content.is_empty() {
            let Some(c) = cur.as_mut() else {
                return Err(Error::InvalidInput(
                    "Sequence line before the first header line".to_string(),
                ));
            };
            let len = content.len() as u64;
            if c.entry.bases_per_line == 0 {
                c.entry.bases_per_line = len;
                c.entry.bytes_per_line = len + eol as u64;
            } else if c.short_seen || len > c.entry.bases_per_line {
                return Err(Error::InvalidInput(format!(
                    "Irregular line length in entry '{}'",
                    c.entry.name
                )));
            } else if len < c.entry.bases_per_line {
                c.short_seen = true;
            }
            c.entry.length += len;
        }
        pos = next;
    }
    finish(cur.take(), &mut entries)?;
    Ok(entries)
}

/// Writes entries in `.fai` format: per line `name\tlength\toffset\tbases_per_line\tbytes_per_line\n`.
pub fn format_fai(entries: &[FaiEntry]) -> String {
    entries
        .iter()
        .map(|e| {
            format!(
                "{}\t{}\t{}\t{}\t{}\n",
                e.name, e.length, e.offset, e.bases_per_line, e.bytes_per_line
            )
        })
        .collect()
}

/// Reads `.fai` text (five tab-separated columns, empty lines ignored). Error `Error::InvalidInput` for a wrong number of columns
/// or numbers that cannot be parsed (message with line number).
pub fn parse_fai(text: &str) -> Result<Vec<FaiEntry>> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 5 {
            return Err(Error::InvalidInput(format!(
                "Line {}: expected 5 columns, found {}",
                n + 1,
                cols.len()
            )));
        }
        let num = |i: usize| -> Result<u64> {
            cols[i].trim().parse::<u64>().map_err(|_| {
                Error::InvalidInput(format!("Line {}: invalid number '{}'", n + 1, cols[i]))
            })
        };
        out.push(FaiEntry {
            name: cols[0].to_string(),
            length: num(1)?,
            offset: num(2)?,
            bases_per_line: num(3)?,
            bytes_per_line: num(4)?,
        });
    }
    Ok(out)
}

/// Fetches the bases `[start, end)` (0-based) of an entry from the byte stream of the FASTA file: base `i` is located at
/// `offset + (i / bases_per_line) * bytes_per_line + i % bases_per_line`. Error `Error::OutOfRange` for `start > end`,
/// `end > length` or if the bytes lie outside the data. An empty range yields `""`.
pub fn fetch_from_bytes(data: &[u8], entry: &FaiEntry, start: u64, end: u64) -> Result<String> {
    if start > end || end > entry.length {
        return Err(Error::OutOfRange(format!(
            "Range {}..{} is not within 0..{}",
            start, end, entry.length
        )));
    }
    let mut out = String::with_capacity((end - start) as usize);
    for i in start..end {
        let at = entry.offset
            + (i / entry.bases_per_line) * entry.bytes_per_line
            + i % entry.bases_per_line;
        let b = data
            .get(at as usize)
            .ok_or_else(|| Error::OutOfRange(format!("Byte {} lies outside the data", at)))?;
        out.push(*b as char);
    }
    Ok(out)
}

/// Like `fetch_from_bytes`, but reads only the necessary bytes from the file (`Seek` + `Read`, not the whole file). File errors as `Error::Io`.
pub fn fetch_from_file(path: &Path, entry: &FaiEntry, start: u64, end: u64) -> Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    if start > end || end > entry.length {
        return Err(Error::OutOfRange(format!(
            "Range {}..{} is not within 0..{}",
            start, end, entry.length
        )));
    }
    if start == end {
        return Ok(String::new());
    }
    let first = entry.offset
        + (start / entry.bases_per_line) * entry.bytes_per_line
        + start % entry.bases_per_line;
    let last = entry.offset
        + ((end - 1) / entry.bases_per_line) * entry.bytes_per_line
        + (end - 1) % entry.bases_per_line;
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(first))?;
    let mut buf = vec![0u8; (last - first + 1) as usize];
    f.read_exact(&mut buf)?;
    let mut out = String::new();
    for i in start..end {
        let at = entry.offset
            + (i / entry.bases_per_line) * entry.bytes_per_line
            + i % entry.bases_per_line
            - first;
        out.push(buf[at as usize] as char);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(s: &str, w: usize) -> String {
        s.as_bytes()
            .chunks(w)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn sample() -> (String, String, String) {
        let s1 = "ACGT".repeat(25);
        let s2 = "TTGCA".repeat(14);
        let fa = format!(">seq1 erste\n{}\n>seq2\n{}\n", wrap(&s1, 60), wrap(&s2, 60));
        (fa, s1, s2)
    }

    #[test]
    fn index_bauen() {
        let (fa, _, _) = sample();
        let e = build_fai(fa.as_bytes()).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(
            e[0],
            FaiEntry {
                name: "seq1".to_string(),
                length: 100,
                offset: 12,
                bases_per_line: 60,
                bytes_per_line: 61
            }
        );
        assert_eq!(e[1].name, "seq2");
        assert_eq!(e[1].length, 70);
        assert_eq!(e[1].offset, 120);
        assert_eq!((e[1].bases_per_line, e[1].bytes_per_line), (60, 61));
    }

    #[test]
    fn schreiben_und_lesen() {
        let (fa, _, _) = sample();
        let e = build_fai(fa.as_bytes()).unwrap();
        let text = format_fai(&e);
        assert_eq!(text, "seq1\t100\t12\t60\t61\nseq2\t70\t120\t60\t61\n");
        assert_eq!(parse_fai(&text).unwrap(), e);
        assert!(parse_fai("a\t1\t2\n").is_err());
        assert!(parse_fai("a\tx\t2\t3\t4\n").is_err());
    }

    #[test]
    fn teilstueck_ueber_den_zeilenumbruch() {
        let (fa, s1, s2) = sample();
        let e = build_fai(fa.as_bytes()).unwrap();
        let got = fetch_from_bytes(fa.as_bytes(), &e[1], 58, 64).unwrap();
        assert_eq!(got, &s2[58..64]);
        assert_eq!(got, "CATTGC");
        assert_eq!(fetch_from_bytes(fa.as_bytes(), &e[0], 0, 100).unwrap(), s1);
        assert_eq!(fetch_from_bytes(fa.as_bytes(), &e[0], 5, 5).unwrap(), "");
        assert!(matches!(
            fetch_from_bytes(fa.as_bytes(), &e[0], 0, 101),
            Err(Error::OutOfRange(_))
        ));
        assert!(matches!(
            fetch_from_bytes(fa.as_bytes(), &e[0], 7, 3),
            Err(Error::OutOfRange(_))
        ));
    }

    #[test]
    fn datei_und_sonderfaelle() {
        let (fa, _, s2) = sample();
        let dir = std::env::temp_dir().join(format!("snapious-fai-{}", crate::util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.fa");
        std::fs::write(&p, &fa).unwrap();
        let e = build_fai(fa.as_bytes()).unwrap();
        assert_eq!(fetch_from_file(&p, &e[1], 58, 64).unwrap(), &s2[58..64]);
        assert!(fetch_from_file(&dir.join("fehlt.fa"), &e[1], 0, 1).is_err());
        let _ = std::fs::remove_dir_all(dir);
        // Windows line endings, last line without line ending, empty entry
        let crlf = ">a\r\nACGTAC\r\nGT";
        let e = build_fai(crlf.as_bytes()).unwrap();
        assert_eq!(e[0].length, 8);
        assert_eq!((e[0].bases_per_line, e[0].bytes_per_line), (6, 8));
        assert_eq!(
            fetch_from_bytes(crlf.as_bytes(), &e[0], 4, 8).unwrap(),
            "ACGT"
        );
        let leer = build_fai(b">x\n>y\nAC\n").unwrap();
        assert_eq!(leer[0].length, 0);
        assert_eq!(leer[1].offset, 6);
        // Errors
        assert!(build_fai(b"ACGT\n>a\nAC\n").is_err());
        assert!(build_fai(b">a\nAC\n>a\nGG\n").is_err());
        assert!(build_fai(b">a\nACGT\nACGTAC\n").is_err());
        assert!(build_fai(b">a\nAC\nACGT\nAC\n").is_err());
    }
}
