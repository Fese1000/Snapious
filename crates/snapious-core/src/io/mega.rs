//! MEGA-Alignmentdateien (`#mega`) lesen und schreiben.

use crate::error::{Error, Result};

/// Liest ein MEGA-Alignment. Die erste nicht leere Zeile muss `#mega` sein (Groß-/Kleinschreibung egal), sonst
/// `Error::InvalidInput`. Danach: leere Zeilen und Zeilen, die mit `!` beginnen (Kommentare/Titel), werden übersprungen;
/// eine Zeile, die mit `#` beginnt, nennt einen Namen (Rest der Zeile, getrimmt) und beginnt oder setzt fort (bei gleichem Namen,
/// z. B. in verschachtelten Blöcken) den Eintrag; alle anderen Zeilen sind Sequenzzeilen, aus denen Leerraum entfernt und
/// die an den Eintrag angehängt werden (alle anderen Zeichen bleiben, auch `-`). Eine Sequence line before the first name ist ein
/// Fehler. Ergebnis `(Name, Sequenz)` in der Reihenfolge des ersten Auftretens; ohne Einträge → Fehler.
pub fn parse_mega(text: &str) -> Result<Vec<(String, String)>> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    match lines.next() {
        Some(first) if first.eq_ignore_ascii_case("#mega") => {}
        _ => {
            return Err(Error::InvalidInput(
                "Not a MEGA file (#mega missing)".to_string(),
            ))
        }
    }
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut current: Option<usize> = None;
    for line in lines {
        if line.starts_with('!') {
            continue;
        }
        if let Some(name) = line.strip_prefix('#') {
            let name = name.trim();
            current = Some(match rows.iter().position(|(n, _)| n == name) {
                Some(i) => i,
                None => {
                    rows.push((name.to_string(), String::new()));
                    rows.len() - 1
                }
            });
        } else {
            let Some(i) = current else {
                return Err(Error::InvalidInput(
                    "Sequence line before the first name".to_string(),
                ));
            };
            rows[i]
                .1
                .extend(line.chars().filter(|c| !c.is_whitespace()));
        }
    }
    if rows.is_empty() {
        return Err(Error::InvalidInput(
            "The MEGA file contains no sequences".to_string(),
        ));
    }
    Ok(rows)
}

/// Schreibt MEGA: `"#mega\n!Title Snapious;\n\n"`, danach je Eintrag `"#{Name}\n{Sequenz}\n\n"`.
pub fn write_mega(rows: &[(String, String)]) -> String {
    let mut out = String::from("#mega\n!Title Snapious;\n\n");
    for (name, seq) in rows {
        out.push_str(&format!("#{name}\n{seq}\n\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lesen_mit_bloecken() {
        let t = "#mega\n!Title T;\n!Format DataType=DNA;\n\n#a\nACGT\nAC\n#b\nA-GT\n#a\nGG\n";
        assert_eq!(
            parse_mega(t).unwrap(),
            vec![
                ("a".to_string(), "ACGTACGG".to_string()),
                ("b".to_string(), "A-GT".to_string())
            ]
        );
        assert_eq!(
            parse_mega("#MEGA\n#x y\nAC GT\n").unwrap()[0],
            ("x y".to_string(), "ACGT".to_string())
        );
    }

    #[test]
    fn fehler_und_schreiben() {
        assert!(parse_mega("ACGT\n").is_err());
        assert!(parse_mega("#mega\nACGT\n").is_err());
        assert!(parse_mega("#mega\n!Title x;\n").is_err());
        let rows = vec![
            ("a".to_string(), "AC".to_string()),
            ("b".to_string(), "A-".to_string()),
        ];
        assert_eq!(
            write_mega(&rows),
            "#mega\n!Title Snapious;\n\n#a\nAC\n\n#b\nA-\n\n"
        );
        assert_eq!(parse_mega(&write_mega(&rows)).unwrap(), rows);
    }
}
