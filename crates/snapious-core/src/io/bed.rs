//! BED-Dateien lesen und als Features verwenden.

use crate::error::{Error, Result};
use crate::model::feature::{Feature, Interval, Location, Strand};

/// Eine BED-Zeile (mindestens drei Spalten).
#[derive(Clone, Debug, PartialEq)]
pub struct BedRecord {
    pub chrom: String,
    /// 0-basiert, Ende ausgeschlossen (wie in der Datei).
    pub start: usize,
    pub end: usize,
    pub name: Option<String>,
    pub score: Option<f64>,
    pub strand: Strand,
}

/// Liest BED-Text. Spalten sind durch Tabulatoren oder Leerzeichen getrennt. Leere Zeilen und Zeilen, die mit `#`, `track` oder `browser` beginnen,
/// werden übersprungen. Jede andere Zeile braucht mindestens 3 Spalten (`chrom`, `start`, `end`, ganze Zahlen mit `start <= end`), sonst
/// `Error::InvalidInput` mit der 1-basierten Zeilennummer. Optional: Spalte 4 Name (`.` = keiner), Spalte 5 Score (Zahl; `.` = keiner),
/// Spalte 6 Strang (`+`, `-`, `.`; anderes → Fehler); weitere Spalten werden ignoriert. Zeilenenden `\n` und `\r\n`.
pub fn parse_bed(text: &str) -> Result<Vec<BedRecord>> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("track")
            || line.starts_with("browser")
        {
            continue;
        }
        let bad = |what: &str| Error::InvalidInput(format!("Line {}: {}", i + 1, what));
        let cols: Vec<&str> = line.split_whitespace().collect();
        let (chrom, start, end) = match (cols.first(), cols.get(1), cols.get(2)) {
            (Some(c), Some(s), Some(e)) => (*c, *s, *e),
            _ => return Err(bad("fewer than 3 columns")),
        };
        let start: usize = start.parse().map_err(|_| bad("Start is not a number"))?;
        let end: usize = end.parse().map_err(|_| bad("End is not a number"))?;
        if start > end {
            return Err(bad("Start lies beyond the end"));
        }
        let name = cols.get(3).filter(|n| **n != ".").map(|n| n.to_string());
        let score = match cols.get(4) {
            None | Some(&".") => None,
            Some(v) => Some(v.parse::<f64>().map_err(|_| bad("Score is not a number"))?),
        };
        let strand = match cols.get(5) {
            None | Some(&".") => Strand::None,
            Some(&"+") => Strand::Forward,
            Some(&"-") => Strand::Reverse,
            Some(_) => return Err(bad("invalid strand")),
        };
        out.push(BedRecord {
            chrom: chrom.to_string(),
            start,
            end,
            name,
            score,
            strand,
        });
    }
    Ok(out)
}

/// Features für `chrom`: Typ `misc_feature`, Name = Name der Zeile (sonst `{chrom}:{start+1}-{end}`), Intervall `[start, end)`, Strang aus der Zeile.
/// Hat die Zeile einen Score, kommt der Qualifier `("score", [score])` dazu (Text mit `{}`). Zeilen mit `start == end` werden übersprungen.
/// Reihenfolge wie in der Eingabe.
pub fn bed_to_features(records: &[BedRecord], chrom: &str) -> Result<Vec<Feature>> {
    let mut features = Vec::new();
    for r in records
        .iter()
        .filter(|r| r.chrom == chrom && r.start < r.end)
    {
        let name = r
            .name
            .clone()
            .unwrap_or_else(|| format!("{}:{}-{}", r.chrom, r.start + 1, r.end));
        let location = Location::new(vec![Interval::new(r.start, r.end)?], r.strand, false, false)?;
        let qualifiers = r
            .score
            .map(|s| vec![("score".to_string(), vec![format!("{}", s)])])
            .unwrap_or_default();
        features.push(Feature::new("misc_feature", &name, location, qualifiers));
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "track name=x\n# Kommentar\nchr1\t100\t200\tgen1\t500\t+\nchr1 300 350\nchr2\t0\t10\t.\t.\t-\tzusatz\n\n";

    #[test]
    fn lesen() {
        let r = parse_bed(TEXT).unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(
            r[0],
            BedRecord {
                chrom: "chr1".to_string(),
                start: 100,
                end: 200,
                name: Some("gen1".to_string()),
                score: Some(500.0),
                strand: Strand::Forward
            }
        );
        assert_eq!(r[1].name, None);
        assert_eq!(r[1].score, None);
        assert_eq!(r[1].strand, Strand::None);
        assert_eq!((r[1].start, r[1].end), (300, 350));
        assert_eq!(r[2].name, None);
        assert_eq!(r[2].strand, Strand::Reverse);
        assert_eq!(parse_bed(&TEXT.replace('\n', "\r\n")).unwrap(), r);
        assert!(parse_bed("").unwrap().is_empty());
    }

    #[test]
    fn fehler() {
        for schlecht in [
            "chr1\t5\n",
            "chr1\tx\t5\n",
            "chr1\t10\t5\n",
            "chr1\t1\t5\tn\tabc\n",
            "chr1\t1\t5\tn\t1\t*\n",
            "chr1\t-1\t5\n",
        ] {
            match parse_bed(schlecht) {
                Err(Error::InvalidInput(m)) => assert!(m.contains('1'), "{}: {}", schlecht, m),
                other => panic!("unerwartet bei {:?}: {:?}", schlecht, other),
            }
        }
        match parse_bed("# x\nchr1\t5\n") {
            Err(Error::InvalidInput(m)) => assert!(m.contains('2')),
            other => panic!("unerwartet: {:?}", other),
        }
    }

    #[test]
    fn zu_features() {
        let r = parse_bed(
            "chr1\t100\t200\tgen1\t500\t+\nchr1\t300\t350\nchr1\t5\t5\tleer\nchr2\t0\t10\n",
        )
        .unwrap();
        let f = bed_to_features(&r, "chr1").unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].name(), "gen1");
        assert_eq!(f[0].feature_type(), "misc_feature");
        assert_eq!((f[0].location().start(), f[0].location().end()), (100, 200));
        assert_eq!(f[0].location().strand(), Strand::Forward);
        assert_eq!(f[0].qualifier("score"), Some("500"));
        assert_eq!(f[1].name(), "chr1:301-350");
        assert_eq!(f[1].qualifier("score"), None);
        assert!(bed_to_features(&r, "chr9").unwrap().is_empty());
    }
}
