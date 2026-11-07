//! Alignment-Dateiformate: Clustal und PHYLIP lesen und schreiben.
//!
//! Ein Alignment ist eine Liste `(Name, Zeile)` mit gleich langen Zeilen; Lücken sind `-`.

use crate::error::{Error, Result};

/// Prüft Namen und Längen: mindestens eine Zeile, jeder Name nicht leer und ohne Leerraum, alle Zeilen gleich lang
/// (in Zeichen). Fehler `Error::InvalidInput`.
pub fn validate_alignment(rows: &[(String, String)]) -> Result<()> {
    if rows.is_empty() {
        return Err(Error::InvalidInput(
            "An alignment needs at least one row".to_string(),
        ));
    }
    let width = rows.first().map_or(0, |(_, r)| r.chars().count());
    for (name, row) in rows {
        if name.is_empty() || name.chars().any(char::is_whitespace) {
            return Err(Error::InvalidInput(format!(
                "Invalid row name '{}'",
                name
            )));
        }
        if row.chars().count() != width {
            return Err(Error::InvalidInput(format!(
                "Row '{}' has {} instead of {} columns",
                name,
                row.chars().count(),
                width
            )));
        }
    }
    Ok(())
}

/// Liest ein Clustal-Alignment.
/// - Die erste nicht leere Zeile muss mit `CLUSTAL` beginnen, sonst `Error::InvalidInput`.
/// - Danach: leere Zeilen und Konservierungszeilen werden übersprungen. Eine Konservierungszeile beginnt mit einem Leerzeichen
///   oder besteht nur aus den Zeichen `*`, `:`, `.` und Leerzeichen.
/// - Jede andere Zeile hat die Form `Name  Sequenz [Zahl]`: erstes Wort = Name, zweites Wort = Sequenzstück, ein drittes Wort
///   (Restezähler, nur Ziffern) wird ignoriert. Mehr als drei Wörter oder weniger als zwei → `Error::InvalidInput` mit der Zeilennummer.
/// - Die Stücke je Name werden in der Reihenfolge des ersten Auftretens der Namen aneinandergehängt.
/// - Am Ende müssen alle Zeilen gleich lang sein (`validate_alignment`). Zeilenenden `\n` und `\r\n` sind erlaubt.
pub fn parse_clustal(text: &str) -> Result<Vec<(String, String)>> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')));
    let first = lines.by_ref().find(|(_, l)| !l.trim().is_empty());
    match first {
        Some((_, l)) if l.starts_with("CLUSTAL") => {}
        _ => {
            return Err(Error::InvalidInput(
                "The file does not start with CLUSTAL".to_string(),
            ))
        }
    }
    let mut rows: Vec<(String, String)> = Vec::new();
    for (number, line) in lines {
        let only_marks = line.chars().all(|c| matches!(c, '*' | ':' | '.' | ' '));
        if line.trim().is_empty() || line.starts_with(' ') || only_marks {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let counter_ok = words
            .get(2)
            .is_none_or(|w| w.chars().all(|c| c.is_ascii_digit()));
        let (name, piece) = match (words.first(), words.get(1)) {
            (Some(n), Some(p)) if words.len() <= 3 && counter_ok => (*n, *p),
            _ => {
                return Err(Error::InvalidInput(format!(
                    "Line {}: invalid sequence line",
                    number
                )))
            }
        };
        match rows.iter_mut().find(|(n, _)| n == name) {
            Some((_, seq)) => seq.push_str(piece),
            None => rows.push((name.to_string(), piece.to_string())),
        }
    }
    validate_alignment(&rows)?;
    Ok(rows)
}

/// Schreibt ein Clustal-Alignment (nach `validate_alignment`).
/// - Erste Zeile `CLUSTAL W (1.83) multiple sequence alignment`, danach zwei Zeilenumbrüche (eine Leerzeile).
/// - Blöcke zu höchstens 60 Spalten. Jede Sequenzzeile: Name, aufgefüllt mit Leerzeichen auf (längster Name + 6) Zeichen, dann das Stück.
/// - Danach eine Konservierungszeile: so viele Leerzeichen wie die Namensbreite (längster Name + 6), dann je Spalte `*`, wenn alle
///   Zeilen in der Spalte denselben Buchstaben haben (keine Lücke; Groß-/Kleinschreibung zählt als verschieden), sonst ein Leerzeichen;
///   Leerzeichen am Zeilenende bleiben stehen. Danach eine Leerzeile.
/// - Alle Zeilen enden mit `\n`.
pub fn write_clustal(rows: &[(String, String)]) -> Result<String> {
    validate_alignment(rows)?;
    let name_width = rows
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(0)
        + 6;
    let columns: Vec<Vec<char>> = rows.iter().map(|(_, r)| r.chars().collect()).collect();
    let width = columns.first().map_or(0, Vec::len);
    let mut out = String::from("CLUSTAL W (1.83) multiple sequence alignment\n\n");
    let mut start = 0;
    while start < width {
        let end = (start + 60).min(width);
        for ((name, _), chars) in rows.iter().zip(&columns) {
            let piece: String = chars.iter().skip(start).take(end - start).collect();
            out.push_str(&format!("{:<w$}{}\n", name, piece, w = name_width));
        }
        let mut marks = " ".repeat(name_width);
        for col in start..end {
            let first = columns.first().and_then(|c| c.get(col)).copied();
            let same = first.is_some_and(|f| {
                f != '-' && f != '.' && columns.iter().all(|c| c.get(col) == Some(&f))
            });
            marks.push(if same { '*' } else { ' ' });
        }
        out.push_str(&marks);
        out.push_str("\n\n");
        start = end;
    }
    Ok(out)
}

/// Liest ein PHYLIP-Alignment (relaxiertes Format, Namen ohne Leerraum).
/// - Erste nicht leere Zeile: zwei ganze Zahlen `ntax nchar`, sonst `Error::InvalidInput`.
/// - Die nächsten `ntax` nicht leeren Zeilen: erstes Wort = Name, der Rest der Zeile (Leerraum entfernt) = erstes Stück der Zeile.
/// - Weitere nicht leere Zeilen (interleaved) gehören der Reihe nach zu den Zeilen 1, 2, …, `ntax`, 1, 2, …; Leerraum wird entfernt.
/// - Am Ende muss es genau `ntax` Zeilen mit je `nchar` Zeichen geben, sonst `Error::InvalidInput`. Mehrzeilige sequentielle
///   Formate werden nicht unterstützt.
pub fn parse_phylip(text: &str) -> Result<Vec<(String, String)>> {
    let mut lines = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let header = lines
        .next()
        .ok_or_else(|| Error::InvalidInput("Empty PHYLIP file".to_string()))?;
    let numbers: Vec<&str> = header.split_whitespace().collect();
    let (ntax, nchar) = match (
        numbers.first().and_then(|a| a.parse::<usize>().ok()),
        numbers.get(1).and_then(|b| b.parse::<usize>().ok()),
    ) {
        (Some(a), Some(b)) if numbers.len() == 2 => (a, b),
        _ => {
            return Err(Error::InvalidInput(
                "Invalid PHYLIP header line".to_string(),
            ))
        }
    };
    let mut rows: Vec<(String, String)> = Vec::new();
    for _ in 0..ntax {
        let line = lines
            .next()
            .ok_or_else(|| Error::InvalidInput("Too few sequence lines".to_string()))?;
        let mut parts = line.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").to_string();
        let rest: String = parts
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        rows.push((name, rest));
    }
    for (k, line) in lines.enumerate() {
        let cleaned: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if let Some((_, seq)) = rows.get_mut(k % ntax.max(1)) {
            seq.push_str(&cleaned);
        }
    }
    if rows.iter().any(|(_, s)| s.chars().count() != nchar) {
        return Err(Error::InvalidInput(format!(
            "Not every row has {} characters",
            nchar
        )));
    }
    validate_alignment(&rows)?;
    Ok(rows)
}

/// Schreibt ein PHYLIP-Alignment (nach `validate_alignment`): Kopfzeile `{ntax} {nchar}`, dann je Zeile der Name, aufgefüllt
/// mit Leerzeichen auf (längster Name + 2) Zeichen, und die ganze Sequenz in einer Zeile. Alle Zeilen enden mit `\n`.
pub fn write_phylip(rows: &[(String, String)]) -> Result<String> {
    validate_alignment(rows)?;
    let width = rows
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let nchar = rows.first().map_or(0, |(_, r)| r.chars().count());
    let mut out = format!("{} {}\n", rows.len(), nchar);
    for (name, row) in rows {
        out.push_str(&format!("{:<w$}{}\n", name, row, w = width));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(r: &[(&str, &str)]) -> Vec<(String, String)> {
        r.iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn pruefung() {
        assert!(validate_alignment(&rows(&[("a", "AC-T"), ("b", "ACGT")])).is_ok());
        assert!(matches!(
            validate_alignment(&[]),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            validate_alignment(&rows(&[("a", "AC"), ("b", "ACG")])),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            validate_alignment(&rows(&[("", "AC")])),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            validate_alignment(&rows(&[("a b", "AC")])),
            Err(Error::InvalidInput(_))
        ));
    }

    const CLUSTAL: &str = "CLUSTAL W (1.83) multiple sequence alignment\n\
\n\
seq1            ACGTACGT-A  9\n\
seq2            ACGTTCGTCA  10\n\
                ****.***.*\n\
\n\
seq1            GG\n\
seq2            GA\n\
                *.\n\
\n";

    #[test]
    fn clustal_lesen() {
        let r = parse_clustal(CLUSTAL).unwrap();
        assert_eq!(
            r,
            rows(&[("seq1", "ACGTACGT-AGG"), ("seq2", "ACGTTCGTCAGA")])
        );
        let w = parse_clustal(&CLUSTAL.replace('\n', "\r\n")).unwrap();
        assert_eq!(w, r);
    }

    #[test]
    fn clustal_fehler() {
        assert!(matches!(
            parse_clustal("kein Clustal\nseq1 ACGT\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(parse_clustal(""), Err(Error::InvalidInput(_))));
        let ungleich = "CLUSTAL\n\na ACGT\nb ACG\n";
        assert!(matches!(
            parse_clustal(ungleich),
            Err(Error::InvalidInput(_))
        ));
        match parse_clustal("CLUSTAL\n\nnurname\n") {
            Err(Error::InvalidInput(m)) => assert!(m.contains('3'), "{}", m),
            other => panic!("unerwartet: {:?}", other),
        }
    }

    #[test]
    fn clustal_schreiben() {
        let r = rows(&[("seq1", "ACGT-A"), ("seq2", "ACGA-C")]);
        let t = write_clustal(&r).unwrap();
        let lines: Vec<&str> = t.lines().collect();
        assert_eq!(lines[0], "CLUSTAL W (1.83) multiple sequence alignment");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "seq1      ACGT-A");
        assert_eq!(lines[3], "seq2      ACGA-C");
        assert_eq!(lines[4], "          ***   ");
        assert_eq!(lines[5], "");
        assert!(t.ends_with('\n'));
        // Rundreise
        assert_eq!(parse_clustal(&t).unwrap(), r);
    }

    #[test]
    fn clustal_bloecke() {
        let a = "A".repeat(130);
        let b = format!("{}C", "A".repeat(129));
        let r = rows(&[("x", &a), ("y", &b)]);
        let t = write_clustal(&r).unwrap();
        // 3 Blöcke (60, 60, 10): 3 × (2 Zeilen + Konservierung + Leerzeile) + Kopf (2 Zeilen)
        assert_eq!(t.lines().count(), 2 + 3 * 4);
        assert_eq!(parse_clustal(&t).unwrap(), r);
        assert!(write_clustal(&rows(&[("x", "AC"), ("y", "A")])).is_err());
    }

    #[test]
    fn phylip_lesen() {
        let t = "3 8\nalpha ACGT ACGT\nbeta  ACGA ACGA\ngamma ACGG ACGG\n";
        let r = parse_phylip(t).unwrap();
        assert_eq!(
            r,
            rows(&[
                ("alpha", "ACGTACGT"),
                ("beta", "ACGAACGA"),
                ("gamma", "ACGGACGG")
            ])
        );
        // interleaved
        let i = "2 8\na ACGT\nb TTTT\n\nAAAA\nCCCC\n";
        assert_eq!(
            parse_phylip(i).unwrap(),
            rows(&[("a", "ACGTAAAA"), ("b", "TTTTCCCC")])
        );
    }

    #[test]
    fn phylip_fehler() {
        assert!(matches!(parse_phylip(""), Err(Error::InvalidInput(_))));
        assert!(matches!(
            parse_phylip("x y\na ACGT\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_phylip("2 4\na ACGT\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_phylip("1 5\na ACGT\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_phylip("1 3\na ACGT\n"),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn phylip_schreiben() {
        let r = rows(&[("alpha", "ACGT"), ("be", "AC-T")]);
        let t = write_phylip(&r).unwrap();
        assert_eq!(t, "2 4\nalpha  ACGT\nbe     AC-T\n");
        assert_eq!(parse_phylip(&t).unwrap(), r);
        assert!(write_phylip(&[]).is_err());
    }
}
