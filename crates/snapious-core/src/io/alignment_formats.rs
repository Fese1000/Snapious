//! Alignment file formats: read and write Clustal and PHYLIP.
//!
//! An alignment is a list of `(name, row)` with rows of equal length; gaps are `-`.

use crate::error::{Error, Result};

/// Checks names and lengths: at least one row, every name non-empty and without whitespace, all rows of equal length
/// (in characters). `Error::InvalidInput`.
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

/// Reads a Clustal alignment.
/// - The first non-empty line must start with `CLUSTAL`, otherwise `Error::InvalidInput`.
/// - After that: empty lines and conservation lines are skipped. A conservation line starts with a space
///   or consists only of the characters `*`, `:`, `.` and spaces.
/// - Every other line has the form `name  sequence [number]`: first word = name, second word = sequence chunk, a third word
///   (residue counter, digits only) is ignored. More than three words or fewer than two -> `Error::InvalidInput` with the line number.
/// - The chunks per name are concatenated in the order in which the names first appear.
/// - At the end all rows must have the same length (`validate_alignment`). Line endings `\n` and `\r\n` are allowed.
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

/// Writes a Clustal alignment (after `validate_alignment`).
/// - First line `CLUSTAL W (1.83) multiple sequence alignment`, followed by two line breaks (one blank line).
/// - Blocks of at most 60 columns. Each sequence line: name, padded with spaces to (longest name + 6) characters, then the chunk.
/// - Then a conservation line: as many spaces as the name width (longest name + 6), then per column `*` if all
///   rows have the same letter in the column (no gap; upper/lower case counts as different), otherwise a space;
///   trailing spaces at the end of the line are kept. Then a blank line.
/// - All lines end with `\n`.
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

/// Reads a PHYLIP alignment (relaxed format, names without whitespace).
/// - First non-empty line: two integers `ntax nchar`, otherwise `Error::InvalidInput`.
/// - The next `ntax` non-empty lines: first word = name, the rest of the line (whitespace removed) = first chunk of the row.
/// - Further non-empty lines (interleaved) belong in turn to rows 1, 2, ..., `ntax`, 1, 2, ...; whitespace is removed.
/// - At the end there must be exactly `ntax` rows with `nchar` characters each, otherwise `Error::InvalidInput`. Multi-line sequential
///   formats are not supported.
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

/// Writes a PHYLIP alignment (after `validate_alignment`): header line `{ntax} {nchar}`, then per row the name, padded
/// with spaces to (longest name + 2) characters, and the whole sequence on one line. All lines end with `\n`.
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
        // Round trip
        assert_eq!(parse_clustal(&t).unwrap(), r);
    }

    #[test]
    fn clustal_bloecke() {
        let a = "A".repeat(130);
        let b = format!("{}C", "A".repeat(129));
        let r = rows(&[("x", &a), ("y", &b)]);
        let t = write_clustal(&r).unwrap();
        // 3 blocks (60, 60, 10): 3 x (2 lines + conservation + blank line) + header (2 lines)
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
