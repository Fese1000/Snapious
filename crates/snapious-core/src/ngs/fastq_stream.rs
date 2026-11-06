//! FASTQ als Strom lesen (ein Eintrag nach dem anderen, ohne die Datei ganz zu laden); `.gz` über das Programm `gzip`.

use std::io::BufRead;
use std::path::Path;

use crate::error::{Error, Result};
use crate::io::fastq::FastqRecord;

/// Liest Einträge aus einer Zeilenquelle.
pub struct FastqReader<R: BufRead> {
    reader: R,
    /// Zahl der bisher gelesenen Zeilen (für Fehlermeldungen, 1-basiert die zuletzt gelesene).
    line: usize,
    finished: bool,
}

impl<R: BufRead> FastqReader<R> {
    pub fn new(reader: R) -> Self {
        FastqReader {
            reader,
            line: 0,
            finished: false,
        }
    }

    /// Liest die nächste Zeile ohne `\n`/`\r\n`; `Ok(None)` am Ende der Quelle. Lesefehler als `Error::Io`.
    fn next_line(&mut self) -> Result<Option<String>> {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf) {
            Ok(0) => Ok(None),
            Ok(_) => {
                self.line += 1;
                while buf.ends_with('\n') || buf.ends_with('\r') {
                    buf.pop();
                }
                Ok(Some(buf))
            }
            Err(e) => Err(Error::Io(e.to_string())),
        }
    }
}

impl<R: BufRead> Iterator for FastqReader<R> {
    type Item = Result<FastqRecord>;

    /// Nächster Eintrag. Leere Zeilen vor einer Kopfzeile werden übersprungen; am Ende der Quelle `None`.
    /// Ein Eintrag besteht aus vier Zeilen: Kopfzeile mit `@` (ID = erstes Wort nach `@`, `comment` = der Rest der Zeile ohne
    /// führenden Leerraum, sonst leer), Sequenz, eine Zeile, die mit `+` beginnt, und Qualität.
    /// Fehler (`Error::InvalidInput`, Meldung beginnt mit `Zeile {n}:` mit der Nummer der fehlerhaften Zeile): Kopfzeile ohne `@`;
    /// dritte Zeile ohne `+`; Qualität und Sequenz verschieden lang; Datei endet mitten im Eintrag. Nach einem Fehler liefert der Iterator
    /// danach nur noch `None`.
    fn next(&mut self) -> Option<Result<FastqRecord>> {
        if self.finished {
            return None;
        }
        match read_record(self) {
            Ok(Some(r)) => Some(Ok(r)),
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

/// Öffnet eine FASTQ-Datei zum Streamen. Endet der Name auf `.gz`, wird `gzip -dc <datei>` gestartet und dessen Ausgabe gelesen
/// (Fehler beim Start → `Error::Io`); sonst wird die Datei direkt gelesen (`Error::Io`, wenn sie fehlt).
pub fn open_fastq(path: &Path) -> Result<FastqReader<Box<dyn BufRead>>> {
    let name = path.to_string_lossy();
    if name.ends_with(".gz") {
        let mut child = std::process::Command::new("gzip")
            .arg("-dc")
            .arg(path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| Error::Io(format!("gzip konnte nicht gestartet werden: {}", e)))?;
        let out = child
            .stdout
            .take()
            .ok_or_else(|| Error::Io("gzip lieferte keinen Ausgabestrom".to_string()))?;
        Ok(FastqReader::new(Box::new(std::io::BufReader::new(out))))
    } else {
        let f = std::fs::File::open(path)
            .map_err(|e| Error::Io(format!("{}: {}", path.display(), e)))?;
        Ok(FastqReader::new(Box::new(std::io::BufReader::new(f))))
    }
}

fn read_record<R: BufRead>(r: &mut FastqReader<R>) -> Result<Option<FastqRecord>> {
    let bad = |line: usize, msg: &str| Error::InvalidInput(format!("Zeile {}: {}", line, msg));
    // Kopfzeile, leere Zeilen davor überspringen
    let header = loop {
        match r.next_line()? {
            None => return Ok(None),
            Some(l) if l.trim().is_empty() => continue,
            Some(l) => break l,
        }
    };
    let Some(rest) = header.strip_prefix('@') else {
        return Err(bad(r.line, "Kopfzeile beginnt nicht mit '@'"));
    };
    let (id, comment) = match rest.split_once(char::is_whitespace) {
        Some((i, c)) => (i.to_string(), c.trim_start().to_string()),
        None => (rest.to_string(), String::new()),
    };
    let eof = |line: usize| bad(line, "Datei endet mitten im Eintrag");
    let sequence = r.next_line()?.ok_or_else(|| eof(r.line))?;
    let plus = r.next_line()?.ok_or_else(|| eof(r.line))?;
    if !plus.starts_with('+') {
        return Err(bad(r.line, "dritte Zeile beginnt nicht mit '+'"));
    }
    let quality = r.next_line()?.ok_or_else(|| eof(r.line))?;
    if quality.len() != sequence.len() {
        return Err(bad(r.line, "Qualität und Sequenz sind verschieden lang"));
    }
    Ok(Some(FastqRecord {
        id,
        comment,
        sequence,
        quality,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn reader(text: &str) -> FastqReader<Cursor<Vec<u8>>> {
        FastqReader::new(Cursor::new(text.as_bytes().to_vec()))
    }

    #[test]
    fn drei_eintraege() {
        let text =
            "@r1 erster Read\nACGT\n+\nIIII\n\n@r2\nTTGA\n+r2\n!!II\r\n@r3 x\tcomment\nGG\n+\nII\n";
        let v: Vec<FastqRecord> = reader(text).map(|r| r.unwrap()).collect();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].id, "r1");
        assert_eq!(v[0].comment, "erster Read");
        assert_eq!(v[0].sequence, "ACGT");
        assert_eq!(v[0].quality, "IIII");
        assert_eq!(v[1].id, "r2");
        assert_eq!(v[1].comment, "");
        assert_eq!(v[1].quality, "!!II");
        assert_eq!(v[2].comment, "x\tcomment");
    }

    #[test]
    fn fehler_mit_zeilennummer() {
        let text = "@r1\nACGT\n+\nIIII\n@r2\nACGT\n+\nIII\n@r3\nAC\n+\nII\n";
        let mut r = reader(text);
        assert!(r.next().unwrap().is_ok());
        let e = r.next().unwrap().unwrap_err();
        assert!(e.to_string().contains("Zeile 8"), "{}", e);
        assert!(r.next().is_none());
        let e = reader("r1\nACGT\n+\nIIII\n").next().unwrap().unwrap_err();
        assert!(e.to_string().contains("Zeile 1"), "{}", e);
        let e = reader("@r1\nACGT\nIIII\nIIII\n")
            .next()
            .unwrap()
            .unwrap_err();
        assert!(e.to_string().contains("Zeile 3"), "{}", e);
        let e = reader("@r1\nACGT\n").next().unwrap().unwrap_err();
        assert!(matches!(e, Error::InvalidInput(_)));
        assert!(reader("").next().is_none());
        assert!(reader("\n\n").next().is_none());
    }

    #[test]
    fn hunderttausend_eintraege_als_strom() {
        let mut text = String::new();
        for i in 0..100_000 {
            text.push_str(&format!("@r{}\nACGTACGT\n+\nIIIIIIII\n", i));
        }
        let mut count = 0usize;
        for r in reader(&text) {
            let r = r.unwrap();
            assert_eq!(r.sequence.len(), 8);
            count += 1;
        }
        assert_eq!(count, 100_000);
    }

    #[test]
    fn datei_und_gzip() {
        let dir = std::env::temp_dir().join(format!("snapious-fq-{}", crate::util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("a.fastq");
        std::fs::write(&plain, "@x\nAC\n+\nII\n@y\nGT\n+\nII\n").unwrap();
        let n = open_fastq(&plain).unwrap().filter(|r| r.is_ok()).count();
        assert_eq!(n, 2);
        assert!(open_fastq(&dir.join("fehlt.fastq")).is_err());
        // gzip nur, wenn das Programm vorhanden ist
        let zipped = std::process::Command::new("gzip")
            .arg("-k")
            .arg(&plain)
            .status();
        if let Ok(s) = zipped {
            if s.success() {
                let gz = dir.join("a.fastq.gz");
                let ids: Vec<String> = open_fastq(&gz).unwrap().map(|r| r.unwrap().id).collect();
                assert_eq!(ids, vec!["x", "y"]);
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
