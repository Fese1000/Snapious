//! FASTA-Index im samtools-Format (`.fai`): bauen, lesen, schreiben und Teilstücke direkt aus der Datei holen.

use std::path::Path;

use crate::error::{Error, Result};

/// Ein Indexeintrag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaiEntry {
    /// Name = erstes Wort der Kopfzeile nach `>`.
    pub name: String,
    /// Zahl der Basen.
    pub length: u64,
    /// Byteposition der ersten Base in der Datei.
    pub offset: u64,
    /// Basen je Zeile (Länge der ersten Sequenzzeile).
    pub bases_per_line: u64,
    /// Bytes je Zeile einschließlich Zeilenende (`bases_per_line + 1` bei `\n`, `+ 2` bei `\r\n`).
    pub bytes_per_line: u64,
}

/// Baut den Index aus dem Bytestrom einer FASTA-Datei. Kopfzeilen beginnen mit `>`; leere Zeilen vor dem ersten Eintrag werden
/// ignoriert. `offset` = Position des ersten Bytes hinter der Kopfzeile (einschließlich ihres Zeilenendes). `bases_per_line` ist die
/// Länge der ersten Sequenzzeile (ohne Zeilenende), `bytes_per_line` ihre Länge mit Zeilenende (`\n` oder `\r\n`, erkannt an der ersten
/// Sequenzzeile). Fehler `Error::InvalidInput`: Sequenzzeile vor der ersten Kopfzeile; eine Zeile eines Eintrags ist länger als die erste
/// oder eine Zeile außer der letzten ist kürzer (Meldung enthält den Namen); doppelter Name. Ein Eintrag ohne Sequenz hat `length 0`,
/// `offset` hinter der Kopfzeile und `bases_per_line = bytes_per_line = 0`. Die letzte Zeile darf ohne Zeilenende enden.
pub fn build_fai(fasta: &[u8]) -> Result<Vec<FaiEntry>> {
    let mut entries: Vec<FaiEntry> = Vec::new();
    // laufender Eintrag: (Name, Offset, erste Zeilenlänge, Bytes je Zeile, Länge, letzte Zeile war kürzer)
    struct Cur {
        entry: FaiEntry,
        short_seen: bool,
    }
    let mut cur: Option<Cur> = None;
    let finish = |c: Option<Cur>, entries: &mut Vec<FaiEntry>| -> Result<()> {
        if let Some(c) = c {
            if entries.iter().any(|e| e.name == c.entry.name) {
                return Err(Error::InvalidInput(format!(
                    "Doppelter Name '{}'",
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
                    "Sequenzzeile vor der ersten Kopfzeile".to_string(),
                ));
            };
            let len = content.len() as u64;
            if c.entry.bases_per_line == 0 {
                c.entry.bases_per_line = len;
                c.entry.bytes_per_line = len + eol as u64;
            } else if c.short_seen || len > c.entry.bases_per_line {
                return Err(Error::InvalidInput(format!(
                    "Unregelmäßige Zeilenlänge im Eintrag '{}'",
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

/// Schreibt Einträge im `.fai`-Format: je Zeile `name\tlength\toffset\tbases_per_line\tbytes_per_line\n`.
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

/// Liest `.fai`-Text (fünf tabgetrennte Spalten, leere Zeilen ignoriert). Fehler `Error::InvalidInput` bei falscher Spaltenzahl
/// oder Zahlen, die sich nicht lesen lassen (Meldung mit Zeilennummer).
pub fn parse_fai(text: &str) -> Result<Vec<FaiEntry>> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 5 {
            return Err(Error::InvalidInput(format!(
                "Zeile {}: erwartet 5 Spalten, gefunden {}",
                n + 1,
                cols.len()
            )));
        }
        let num = |i: usize| -> Result<u64> {
            cols[i].trim().parse::<u64>().map_err(|_| {
                Error::InvalidInput(format!("Zeile {}: ungültige Zahl '{}'", n + 1, cols[i]))
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

/// Holt die Basen `[start, end)` (0-basiert) eines Eintrags aus dem Bytestrom der FASTA-Datei: Basis `i` liegt bei
/// `offset + (i / bases_per_line) * bytes_per_line + i % bases_per_line`. Fehler `Error::OutOfRange` bei `start > end`,
/// `end > length` oder wenn die Bytes außerhalb der Daten liegen. Ein leerer Bereich ergibt `""`.
pub fn fetch_from_bytes(data: &[u8], entry: &FaiEntry, start: u64, end: u64) -> Result<String> {
    if start > end || end > entry.length {
        return Err(Error::OutOfRange(format!(
            "Bereich {}..{} liegt nicht in 0..{}",
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
            .ok_or_else(|| Error::OutOfRange(format!("Byte {} liegt außerhalb der Daten", at)))?;
        out.push(*b as char);
    }
    Ok(out)
}

/// Wie `fetch_from_bytes`, liest aber nur die nötigen Bytes aus der Datei (`Seek` + `Read`, nicht die ganze Datei). Dateifehler als `Error::Io`.
pub fn fetch_from_file(path: &Path, entry: &FaiEntry, start: u64, end: u64) -> Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    if start > end || end > entry.length {
        return Err(Error::OutOfRange(format!(
            "Bereich {}..{} liegt nicht in 0..{}",
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
        // Windows-Zeilenenden, letzte Zeile ohne Zeilenende, leerer Eintrag
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
        // Fehler
        assert!(build_fai(b"ACGT\n>a\nAC\n").is_err());
        assert!(build_fai(b">a\nAC\n>a\nGG\n").is_err());
        assert!(build_fai(b">a\nACGT\nACGTAC\n").is_err());
        assert!(build_fai(b">a\nAC\nACGT\nAC\n").is_err());
    }
}
