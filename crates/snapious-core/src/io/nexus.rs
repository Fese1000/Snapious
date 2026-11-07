//! NEXUS-Alignmentformat (DATA- oder CHARACTERS-Block) lesen und schreiben.
//!
//! Ein Alignment ist eine Liste `(Name, Zeile)` mit gleich langen Zeilen; Lücken sind `-`.

use crate::error::{Error, Result};
use crate::io::alignment_formats::validate_alignment;

/// Liest ein NEXUS-Alignment.
/// * Die erste nicht leere Zeile muss mit `#NEXUS` beginnen (Groß-/Kleinschreibung egal), sonst `Error::InvalidInput`.
/// * Kommentare in eckigen Klammern `[ ... ]` werden überall entfernt (auch mitten in einer Zeile).
/// * Gesucht wird ein Block `BEGIN DATA;` oder `BEGIN CHARACTERS;` (Schlüsselwörter ohne Rücksicht auf Groß-/Kleinschreibung);
///   darin der Abschnitt `MATRIX` bis zum nächsten Semikolon. Jede Zeile dazwischen ist `Name Zeile` (durch Leerraum getrennt);
///   Leerraum innerhalb der Sequenz wird entfernt. Kommt ein Name in mehreren Zeilen vor (interleaved), werden die Teile in
///   der Reihenfolge des Auftretens angehängt. Die Reihenfolge der Namen ist die des ersten Auftretens.
/// * Fehlt der Block oder die Matrix, oder ist die Matrix leer: `Error::InvalidInput`.
/// * Das Ergebnis wird mit `validate_alignment` geprüft (dessen Fehler werden weitergegeben).
pub fn parse_nexus(text: &str) -> Result<Vec<(String, String)>> {
    let bad = |m: &str| Error::InvalidInput(m.to_string());
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    if !first.trim().to_ascii_lowercase().starts_with("#nexus") {
        return Err(bad("Not a NEXUS file (#NEXUS missing)"));
    }
    // Kommentare entfernen
    let mut clean = String::with_capacity(text.len());
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => clean.push(c),
            _ => {}
        }
    }
    let lower = clean.to_ascii_lowercase();
    let mut matrix: Option<&str> = None;
    let mut pos = 0;
    while let Some(i) = lower[pos..].find("begin") {
        let start = pos + i + 5;
        let Some(semi) = lower[start..].find(';') else {
            break;
        };
        let name = lower[start..start + semi].trim();
        pos = start + semi + 1;
        if name == "data" || name == "characters" {
            let rest = &lower[pos..];
            let end = rest.find("end;").unwrap_or(rest.len());
            if let Some(m) = rest[..end].find("matrix") {
                let from = pos + m + 6;
                let to = lower[from..].find(';').map_or(clean.len(), |s| from + s);
                matrix = Some(&clean[from..to]);
            }
            break;
        }
    }
    let matrix = matrix.ok_or_else(|| bad("No DATA/CHARACTERS block with MATRIX found"))?;
    let mut rows: Vec<(String, String)> = Vec::new();
    for line in matrix.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let seq: String = parts.collect();
        match rows.iter_mut().find(|(n, _)| n == name) {
            Some((_, s)) => s.push_str(&seq),
            None => rows.push((name.to_string(), seq)),
        }
    }
    if rows.is_empty() {
        return Err(bad("The MATRIX is empty"));
    }
    validate_alignment(&rows)?;
    Ok(rows)
}

/// Schreibt ein Alignment als NEXUS-Text. Zuerst `validate_alignment`.
pub fn write_nexus(rows: &[(String, String)]) -> Result<String> {
    validate_alignment(rows)?;
    let width = rows.first().map_or(0, |(_, r)| r.chars().count());
    let protein = rows.iter().any(|(_, r)| {
        r.chars()
            .any(|c| c.is_alphabetic() && !"ACGTUNacgtun".contains(c))
    });
    let name_w = rows
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = String::from("#NEXUS\nBEGIN DATA;\n");
    out.push_str(&format!(
        "  DIMENSIONS NTAX={} NCHAR={};\n",
        rows.len(),
        width
    ));
    out.push_str(&format!(
        "  FORMAT DATATYPE={} GAP=- MISSING=?;\n  MATRIX\n",
        if protein { "PROTEIN" } else { "DNA" }
    ));
    for (n, r) in rows {
        out.push_str(&format!("    {:<name_w$}  {}\n", n, r));
    }
    out.push_str("  ;\nEND;\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "#NEXUS
[ein Kommentar]
BEGIN DATA;
  DIMENSIONS NTAX=3 NCHAR=8;
  FORMAT DATATYPE=DNA GAP=- MISSING=?;
  MATRIX
    seqA  ACGT-ACG
    seqB  ACGTTACG
    seqC  AC[x]GTTACG
  ;
END;
";

    fn rows(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn einfach_lesen() {
        let r = parse_nexus(SAMPLE).unwrap();
        assert_eq!(
            r,
            rows(&[
                ("seqA", "ACGT-ACG"),
                ("seqB", "ACGTTACG"),
                ("seqC", "ACGTTACG")
            ])
        );
    }

    #[test]
    fn interleaved_und_kleinschreibung() {
        let t = "#nexus\nbegin characters;\ndimensions ntax=2 nchar=8;\nmatrix\nA ACGT\nB TTTT\n\nA ACGG\nB TTTA\n;\nend;\n";
        let r = parse_nexus(t).unwrap();
        assert_eq!(r, rows(&[("A", "ACGTACGG"), ("B", "TTTTTTTA")]));
    }

    #[test]
    fn schreiben_und_rundlauf() {
        let r = rows(&[
            ("seqA", "ACGT-ACG"),
            ("seqB", "ACGTTACG"),
            ("kurz", "ACGTTACG"),
        ]);
        let t = write_nexus(&r).unwrap();
        assert!(t.starts_with("#NEXUS\nBEGIN DATA;\n"));
        assert!(t.contains("  DIMENSIONS NTAX=3 NCHAR=8;\n"));
        assert!(t.contains("  FORMAT DATATYPE=DNA GAP=- MISSING=?;\n"));
        assert!(t.contains("    seqA  ACGT-ACG\n"));
        assert!(t.contains("    kurz  ACGTTACG\n"));
        assert!(t.ends_with("  ;\nEND;\n"));
        assert_eq!(parse_nexus(&t).unwrap(), r);
    }

    #[test]
    fn protein_erkannt() {
        let t = write_nexus(&rows(&[("p1", "MKV-L"), ("p2", "MKVAL")])).unwrap();
        assert!(t.contains("DATATYPE=PROTEIN"));
    }

    #[test]
    fn fehlerfaelle() {
        assert!(matches!(
            parse_nexus(">a\nACGT\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_nexus("#NEXUS\nBEGIN TREES;\nEND;\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_nexus("#NEXUS\nBEGIN DATA;\nMATRIX\n;\nEND;\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_nexus("#NEXUS\nBEGIN DATA;\nMATRIX\nA ACGT\nB AC\n;\nEND;\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(write_nexus(&[]).is_err());
    }
}
