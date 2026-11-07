//! Read GFF3 annotations and convert them to features.

use crate::error::{Error, Result};
use crate::model::feature::{Feature, Interval, Location, Strand};

/// A line of a GFF3 file.
#[derive(Clone, Debug, PartialEq)]
pub struct Gff3Record {
    pub seqid: String,
    pub source: String,
    pub feature_type: String,
    /// 1-based, both bounds inclusive (as in the file).
    pub start: usize,
    pub end: usize,
    pub score: Option<f64>,
    pub strand: Strand,
    pub phase: Option<u8>,
    /// Attributes in file order, values percent-decoded.
    pub attributes: Vec<(String, String)>,
}

/// Decodes `%XX` sequences (UTF-8). Invalid sequences (`%` without two hex digits) are left unchanged.
pub fn percent_decode(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = bytes[i + 1];
            let h2 = bytes[i + 2];
            if is_hex(h1) && is_hex(h2) {
                let val = (hex_val(h1) << 4) | hex_val(h2);
                out.push(val);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

fn is_hex(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

/// Reads GFF3 text.
/// - Empty lines and lines starting with `#` are skipped; a line `##FASTA` ends reading.
/// - Every other line must have exactly 9 tab-separated columns, otherwise
///   `Error::InvalidInput` with the 1-based line number in the text.
/// - Columns 4 and 5 are integers with `start >= 1` and `end >= start`, otherwise `Error::InvalidInput`.
/// - `.` in score, strand and phase means "not given" (score/phase: `None`, strand: `Strand::None`).
///   Strand `+` = `Forward`, `-` = `Reverse`, `?` = `None`; anything else -> `Error::InvalidInput`.
///   Phase is 0, 1 or 2, otherwise `Error::InvalidInput`.
/// - Column 9 consists of `key=value` pairs separated by `;`. Empty parts are skipped, a
///   part without `=` yields `Error::InvalidInput`. Key and value are decoded with `percent_decode`;
///   `.` as the whole of column 9 means no attributes.
/// - Line endings `\n` and `\r\n` are allowed.
pub fn parse_gff3(text: &str) -> Result<Vec<Gff3Record>> {
    let mut records = Vec::new();
    for (line_num, line) in text.lines().enumerate() {
        let num = line_num + 1;
        let trimmed = line.trim_end();

        if trimmed == "##FASTA" {
            break;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = trimmed.split('\t').collect();
        if cols.len() != 9 {
            return Err(Error::InvalidInput(format!(
                "Line {} has {} instead of 9 columns",
                num,
                cols.len()
            )));
        }

        let seqid = cols[0].to_string();
        let source = cols[1].to_string();
        let feature_type = cols[2].to_string();

        let start: usize = parse_u32(cols[3], num)?;
        if start < 1 {
            return Err(Error::InvalidInput(format!(
                "Line {}: Start must be >= 1",
                num
            )));
        }

        let end: usize = parse_u32(cols[4], num)?;
        if end < start {
            return Err(Error::InvalidInput(format!(
                "Line {}: End must be >= Start",
                num
            )));
        }

        let score = parse_score(cols[5], num)?;
        let strand = parse_strand(cols[6], num)?;
        let phase = parse_phase(cols[7], num)?;
        let attributes = parse_attributes(cols[8], num)?;

        records.push(Gff3Record {
            seqid,
            source,
            feature_type,
            start,
            end,
            score,
            strand,
            phase,
            attributes,
        });
    }
    Ok(records)
}

fn parse_u32(s: &str, line: usize) -> Result<usize> {
    s.parse::<u32>()
        .map(|v| v as usize)
        .map_err(|_| Error::InvalidInput(format!("Line {}: invalid position", line)))
}

fn parse_score(s: &str, line: usize) -> Result<Option<f64>> {
    if s == "." {
        Ok(None)
    } else {
        Some(
            s.parse::<f64>()
                .map_err(|_| Error::InvalidInput(format!("Line {}: invalid score", line))),
        )
        .transpose()
    }
}

fn parse_strand(s: &str, line: usize) -> Result<Strand> {
    match s {
        "." => Ok(Strand::None),
        "+" => Ok(Strand::Forward),
        "-" => Ok(Strand::Reverse),
        "?" => Ok(Strand::None),
        _ => Err(Error::InvalidInput(format!(
            "Line {}: invalid strand",
            line
        ))),
    }
}

fn parse_phase(s: &str, line: usize) -> Result<Option<u8>> {
    if s == "." {
        Ok(None)
    } else {
        let p: u8 = s
            .parse()
            .map_err(|_| Error::InvalidInput(format!("Line {}: invalid phase", line)))?;
        if p > 2 {
            Err(Error::InvalidInput(format!(
                "Line {}: Phase must be 0, 1 or 2",
                line
            )))
        } else {
            Ok(Some(p))
        }
    }
}

fn parse_attributes(s: &str, line: usize) -> Result<Vec<(String, String)>> {
    if s == "." {
        return Ok(Vec::new());
    }
    let mut attrs = Vec::new();
    for part in s.split(';') {
        if part.is_empty() {
            continue;
        }
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() != 2 {
            return Err(Error::InvalidInput(format!(
                "Line {}: invalid attribute '{}'",
                line, part
            )));
        }
        let k = percent_decode(kv[0]);
        let v = percent_decode(kv[1]);
        attrs.push((k, v));
    }
    Ok(attrs)
}

/// Converts the lines with `seqid == seqid` into features (order as in the input).
/// - Interval 0-based with exclusive end: `start - 1 .. end`.
/// - Type = `feature_type`; name = attribute `Name`, otherwise `ID`, otherwise the type.
/// - Qualifiers: all attributes in file order, each `(key, vec![value])`, except `Name`.
///   If the line has a source other than `.` and non-empty, `("source", [source])` is added as the first qualifier.
pub fn gff3_to_features(records: &[Gff3Record], seqid: &str) -> Result<Vec<Feature>> {
    let mut features = Vec::new();
    for rec in records.iter() {
        if rec.seqid != seqid {
            continue;
        }

        let interval = Interval::new(rec.start.saturating_sub(1), rec.end)?;
        let location = Location::new(vec![interval], rec.strand, false, false)?;

        let name = rec
            .attributes
            .iter()
            .find(|(k, _)| k == "Name")
            .map(|(_, v)| v.clone())
            .or_else(|| {
                rec.attributes
                    .iter()
                    .find(|(k, _)| k == "ID")
                    .map(|(_, v)| v.clone())
            })
            .unwrap_or_else(|| rec.feature_type.clone());

        let mut qualifiers = Vec::new();
        if !rec.source.is_empty() && rec.source != "." {
            qualifiers.push(("source".to_string(), vec![rec.source.clone()]));
        }

        for (k, v) in &rec.attributes {
            if k != "Name" {
                qualifiers.push((k.clone(), vec![v.clone()]));
            }
        }

        features.push(Feature::new(&rec.feature_type, &name, location, qualifiers));
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "##gff-version 3\n\
# Kommentar\n\
\n\
ctg1\tsrc\tgene\t1\t9\t.\t+\t.\tID=gene1;Name=abc\n\
ctg1\t.\tCDS\t3\t8\t5.5\t-\t2\tID=cds1;Note=a%3Bb%3Dc\n\
ctg2\t.\texon\t10\t20\t.\t.\t.\t.\n";

    #[test]
    fn prozent_dekodierung() {
        assert_eq!(percent_decode("a%3Bb%3Dc"), "a;b=c");
        assert_eq!(percent_decode("100%25"), "100%");
        assert_eq!(percent_decode("%C3%A4"), "ä");
        assert_eq!(percent_decode("50%"), "50%");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode(""), "");
    }

    #[test]
    fn zeilen_lesen() {
        let r = parse_gff3(SAMPLE).unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].seqid, "ctg1");
        assert_eq!(r[0].source, "src");
        assert_eq!(r[0].feature_type, "gene");
        assert_eq!((r[0].start, r[0].end), (1, 9));
        assert_eq!(r[0].score, None);
        assert_eq!(r[0].strand, Strand::Forward);
        assert_eq!(r[0].phase, None);
        assert_eq!(
            r[0].attributes,
            vec![
                ("ID".to_string(), "gene1".to_string()),
                ("Name".to_string(), "abc".to_string())
            ]
        );
        assert_eq!(r[1].score, Some(5.5));
        assert_eq!(r[1].strand, Strand::Reverse);
        assert_eq!(r[1].phase, Some(2));
        assert_eq!(
            r[1].attributes[1],
            ("Note".to_string(), "a;b=c".to_string())
        );
        assert_eq!(r[2].strand, Strand::None);
        assert!(r[2].attributes.is_empty());
    }

    #[test]
    fn fasta_abschnitt_und_windows_zeilenenden() {
        let t = "ctg1\t.\tgene\t1\t3\t.\t+\t.\tID=a\r\n##FASTA\r\n>ctg1\r\nACGT\r\n";
        let r = parse_gff3(t).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].attributes[0].1, "a");
        assert!(parse_gff3("").unwrap().is_empty());
        assert!(parse_gff3("##gff-version 3\n").unwrap().is_empty());
    }

    #[test]
    fn fehler() {
        let falsche_spalten = parse_gff3("# x\nctg1\t.\tgene\t1\t3\t.\t+\t.\n");
        match falsche_spalten {
            Err(Error::InvalidInput(m)) => assert!(m.contains('2'), "{}", m),
            other => panic!("unerwartet: {:?}", other),
        }
        let schlecht = [
            "c\t.\tg\t0\t3\t.\t+\t.\tID=a",
            "c\t.\tg\t5\t3\t.\t+\t.\tID=a",
            "c\t.\tg\tx\t3\t.\t+\t.\tID=a",
            "c\t.\tg\t1\t3\t.\t*\t.\tID=a",
            "c\t.\tg\t1\t3\t.\t+\t3\tID=a",
            "c\t.\tg\t1\t3\tabc\t+\t.\tID=a",
            "c\t.\tg\t1\t3\t.\t+\t.\tkeinWert",
        ];
        for z in schlecht {
            assert!(
                matches!(parse_gff3(z), Err(Error::InvalidInput(_))),
                "{}",
                z
            );
        }
    }

    #[test]
    fn attribute_leere_teile() {
        let r = parse_gff3("c\t.\tg\t1\t3\t.\t+\t.\tID=a;;Name=b;\n").unwrap();
        assert_eq!(r[0].attributes.len(), 2);
    }

    #[test]
    fn zu_features() {
        let r = parse_gff3(SAMPLE).unwrap();
        let f = gff3_to_features(&r, "ctg1").unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].feature_type(), "gene");
        assert_eq!(f[0].name(), "abc");
        assert_eq!((f[0].location().start(), f[0].location().end()), (0, 9));
        assert_eq!(f[0].location().strand(), Strand::Forward);
        assert_eq!(f[0].qualifier("source"), Some("src"));
        assert_eq!(f[0].qualifier("ID"), Some("gene1"));
        assert_eq!(f[0].qualifier("Name"), None);
        assert_eq!(f[1].name(), "cds1");
        assert_eq!((f[1].location().start(), f[1].location().end()), (2, 8));
        assert_eq!(f[1].location().strand(), Strand::Reverse);
        assert_eq!(f[1].qualifier("source"), None);
        assert_eq!(f[1].qualifier("Note"), Some("a;b=c"));
        let g = gff3_to_features(&r, "ctg2").unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].name(), "exon");
        assert!(g[0].qualifiers().is_empty());
        assert!(gff3_to_features(&r, "gibtsnicht").unwrap().is_empty());
    }
}
