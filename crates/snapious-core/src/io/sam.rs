//! SAM-Dateien (Textformat) lesen. BAM folgt mit Phase 5.

use std::path::Path;

use crate::error::{Error, Result};

/// Ein Alignment-Eintrag (11 Pflichtspalten).
#[derive(Clone, Debug, PartialEq)]
pub struct SamRecord {
    pub read_name: String,
    pub flag: u16,
    /// Name der Referenz oder `*`.
    pub reference: String,
    /// 1-basiert wie in der Datei; 0 = nicht ausgerichtet.
    pub position: usize,
    pub mapq: u8,
    /// CIGAR-Text oder `*`.
    pub cigar: String,
    pub mate_reference: String,
    pub mate_position: usize,
    pub template_length: i64,
    pub sequence: String,
    pub quality: String,
}

impl SamRecord {
    pub fn is_paired(&self) -> bool {
        self.flag & 0x1 != 0
    }

    pub fn is_unmapped(&self) -> bool {
        self.flag & 0x4 != 0
    }

    pub fn is_reverse(&self) -> bool {
        self.flag & 0x10 != 0
    }

    pub fn is_secondary(&self) -> bool {
        self.flag & 0x100 != 0
    }

    pub fn is_duplicate(&self) -> bool {
        self.flag & 0x400 != 0
    }

    /// Startposition 0-basiert (None, wenn nicht ausgerichtet).
    pub fn position0(&self) -> Option<usize> {
        (!self.is_unmapped() && self.position > 0).then(|| self.position - 1)
    }

    /// CIGAR als Liste `(Länge, Operation)`.
    pub fn cigar_ops(&self) -> Result<Vec<(usize, char)>> {
        parse_cigar(&self.cigar)
    }

    /// Länge auf der Referenz (Summe von M, D, N, =, X).
    pub fn reference_length(&self) -> Result<usize> {
        Ok(self
            .cigar_ops()?
            .iter()
            .filter(|(_, op)| matches!(op, 'M' | 'D' | 'N' | '=' | 'X'))
            .map(|(n, _)| *n)
            .sum())
    }
}

/// Zerlegt einen CIGAR-Text (`3S4M2D5M`). `*` ergibt eine leere Liste.
pub fn parse_cigar(cigar: &str) -> Result<Vec<(usize, char)>> {
    if cigar == "*" || cigar.is_empty() {
        return Ok(Vec::new());
    }
    let mut ops = Vec::new();
    let mut num = String::new();
    for c in cigar.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else if "MIDNSHP=X".contains(c) {
            let n: usize = num
                .parse()
                .map_err(|_| Error::Io(format!("Ungültiger CIGAR '{}'", cigar)))?;
            ops.push((n, c));
            num.clear();
        } else {
            return Err(Error::Io(format!("Ungültiger CIGAR '{}'", cigar)));
        }
    }
    if !num.is_empty() {
        return Err(Error::Io(format!("Ungültiger CIGAR '{}'", cigar)));
    }
    Ok(ops)
}

/// Inhalt einer SAM-Datei.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SamFile {
    /// Referenzen aus `@SQ`-Zeilen: (Name, Länge).
    pub references: Vec<(String, usize)>,
    pub records: Vec<SamRecord>,
}

fn num<T: std::str::FromStr>(s: &str, what: &str, line: usize) -> Result<T> {
    s.parse::<T>().map_err(|_| {
        Error::Io(format!(
            "Zeile {}: Ungültiger Wert für {}: '{}'",
            line, what, s
        ))
    })
}

/// Liest SAM-Text. Zeilen mit weniger als 11 Spalten sind Fehler.
pub fn parse(text: &str) -> Result<SamFile> {
    let mut file = SamFile::default();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@SQ") {
            let mut name = None;
            let mut len = None;
            for f in rest.split('\t') {
                if let Some(v) = f.strip_prefix("SN:") {
                    name = Some(v.to_string());
                } else if let Some(v) = f.strip_prefix("LN:") {
                    len = Some(num::<usize>(v, "LN", n)?);
                }
            }
            if let (Some(a), Some(b)) = (name, len) {
                file.references.push((a, b));
            }
            continue;
        }
        if line.starts_with('@') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 11 {
            return Err(Error::Io(format!(
                "Zeile {}: {} Spalten gefunden, mindestens 11 erwartet",
                n,
                f.len()
            )));
        }
        file.records.push(SamRecord {
            read_name: f[0].to_string(),
            flag: num(f[1], "FLAG", n)?,
            reference: f[2].to_string(),
            position: num(f[3], "POS", n)?,
            mapq: num(f[4], "MAPQ", n)?,
            cigar: f[5].to_string(),
            mate_reference: f[6].to_string(),
            mate_position: num(f[7], "PNEXT", n)?,
            template_length: num(f[8], "TLEN", n)?,
            sequence: f[9].to_ascii_uppercase(),
            quality: f[10].to_string(),
        });
    }
    Ok(file)
}

pub fn read(path: &Path) -> Result<SamFile> {
    parse(&std::fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:1000\n@SQ\tSN:chr2\tLN:500\n\
r1\t0\tchr1\t5\t60\t4M\t*\t0\t0\tACGT\tIIII\n\
r2\t16\tchr1\t10\t30\t3S4M2D5M\t=\t100\t-50\tacgtacgtacgt\t!!!!!!!!!!!!\n\
r3\t4\t*\t0\t0\t*\t*\t0\t0\tACGT\tIIII\n";

    #[test]
    fn referenzen_und_eintraege() {
        let f = parse(TEXT).unwrap();
        assert_eq!(
            f.references,
            vec![("chr1".to_string(), 1000), ("chr2".to_string(), 500)]
        );
        assert_eq!(f.records.len(), 3);
        let r = &f.records[0];
        assert_eq!(r.read_name, "r1");
        assert_eq!(r.position, 5);
        assert_eq!(r.position0(), Some(4));
        assert_eq!(r.mapq, 60);
        assert_eq!(r.sequence, "ACGT");
    }

    #[test]
    fn flags() {
        let f = parse(TEXT).unwrap();
        assert!(f.records[1].is_reverse());
        assert!(!f.records[0].is_reverse());
        assert!(f.records[2].is_unmapped());
        assert_eq!(f.records[2].position0(), None);
        assert!(!f.records[1].is_paired());
        let paired = SamRecord {
            flag: 0x1 | 0x100 | 0x400,
            ..f.records[0].clone()
        };
        assert!(paired.is_paired() && paired.is_secondary() && paired.is_duplicate());
    }

    #[test]
    fn cigar() {
        let f = parse(TEXT).unwrap();
        assert_eq!(f.records[0].reference_length().unwrap(), 4);
        assert_eq!(f.records[1].reference_length().unwrap(), 4 + 2 + 5);
        assert_eq!(
            f.records[1].cigar_ops().unwrap(),
            vec![(3, 'S'), (4, 'M'), (2, 'D'), (5, 'M')]
        );
        assert_eq!(f.records[2].reference_length().unwrap(), 0);
        assert!(parse_cigar("4Q").is_err());
        assert!(parse_cigar("M4").is_err());
        assert!(parse_cigar("4").is_err());
        assert_eq!(parse_cigar("12=1X").unwrap(), vec![(12, '='), (1, 'X')]);
    }

    #[test]
    fn fehler_mit_zeilennummer() {
        assert!(matches!(parse("r1\t0\tchr1\n"), Err(Error::Io(m)) if m.contains("Zeile 1")));
        assert!(matches!(
            parse("r1\tX\tchr1\t1\t60\t4M\t*\t0\t0\tACGT\tIIII\n"),
            Err(Error::Io(m)) if m.contains("FLAG")
        ));
    }

    #[test]
    fn leere_datei_und_windows() {
        assert_eq!(parse("").unwrap(), SamFile::default());
        let f = parse("@SQ\tSN:a\tLN:5\r\nr\t0\ta\t1\t1\t5M\t*\t0\t0\tACGTA\tIIIII\r\n").unwrap();
        assert_eq!(f.records.len(), 1);
        assert_eq!(f.references[0].1, 5);
    }

    #[test]
    fn datei() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.sam");
        std::fs::write(&p, TEXT).unwrap();
        assert_eq!(read(&p).unwrap().records.len(), 3);
    }
}
