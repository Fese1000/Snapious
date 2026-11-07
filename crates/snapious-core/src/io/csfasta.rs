//! SOLiD-Farbraumdaten (csfasta und QUAL) lesen und in Basen umrechnen.

use crate::error::{Error, Result};

/// Liest csfasta: Zeilen mit `#` am Anfang und leere Zeilen werden übersprungen; eine Zeile `>Name` beginnt einen Eintrag
/// (Name = Rest getrimmt), die folgenden Zeilen (Leerraum entfernt) werden zur Farbfolge zusammengesetzt, z. B. `T0123`
/// (erste Base, dann Farben). Ergebnis `(Name, Farbfolge)`. Fehler `Error::InvalidInput`, wenn Zeichen vor der ersten
/// `>`-Zeile stehen, die nicht übersprungen werden, oder wenn es keine Einträge gibt.
pub fn parse_csfasta(text: &str) -> Result<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('>') {
            out.push((name.trim().to_string(), String::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.extend(line.chars().filter(|c| !c.is_whitespace()));
        } else {
            return Err(Error::InvalidInput(
                "Color sequence before the first header line".to_string(),
            ));
        }
    }
    if out.is_empty() {
        return Err(Error::InvalidInput(
            "No entries in the csfasta file".to_string(),
        ));
    }
    Ok(out)
}

/// Wandelt eine Farbfolge (erste Base `A`, `C`, `G` oder `T`, dann Farben `0`–`3`) in Basen um (Groß-/Kleinschreibung egal).
/// Die Ausgabe hat die Länge der Eingabe (Startbase plus eine Base je Farbe). Nächste Base aus der vorigen und der Farbe:
/// 0: gleiche Base; 1: A↔C und G↔T; 2: A↔G und C↔T; 3: A↔T und C↔G.
/// Eine Farbe, die keine Ziffer 0–3 ist (z. B. `.` oder `4`), ergibt `N`, und alle folgenden Basen sind ebenfalls `N`.
/// Fehler `Error::InvalidInput`: leere Eingabe oder erste Base nicht A, C, G, T. Beispiel: `T0123` → `TTGAT`.
pub fn decode_color_space(colors: &str) -> Result<String> {
    let mut chars = colors.chars();
    let first = chars
        .next()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| "ACGT".contains(*c))
        .ok_or_else(|| {
            Error::InvalidInput("The first base must be A, C, G or T".to_string())
        })?;
    let mut out = String::with_capacity(colors.len());
    out.push(first);
    let mut current = Some(first);
    for c in chars {
        current = current.and_then(|b| match (b, c) {
            (_, '0') => Some(b),
            ('A', '1') => Some('C'),
            ('C', '1') => Some('A'),
            ('G', '1') => Some('T'),
            ('T', '1') => Some('G'),
            ('A', '2') => Some('G'),
            ('G', '2') => Some('A'),
            ('C', '2') => Some('T'),
            ('T', '2') => Some('C'),
            ('A', '3') => Some('T'),
            ('T', '3') => Some('A'),
            ('C', '3') => Some('G'),
            ('G', '3') => Some('C'),
            _ => None,
        });
        out.push(current.unwrap_or('N'));
    }
    Ok(out)
}

/// Liest SOLiD-QUAL: wie bei csfasta (`#`-Zeilen und leere Zeilen übersprungen, `>Name` beginnt einen Eintrag), die Zeilen danach
/// enthalten durch Leerraum getrennte Zahlen (0–255), die zu einem Vektor zusammengesetzt werden. Fehler `Error::InvalidInput`,
/// wenn ein Wert keine Zahl 0–255 ist oder Zahlen vor der ersten `>`-Zeile stehen.
pub fn parse_solid_qual(text: &str) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('>') {
            out.push((name.trim().to_string(), Vec::new()));
        } else if let Some(last) = out.last_mut() {
            for tok in line.split_whitespace() {
                let v = tok.parse::<u8>().map_err(|_| {
                    Error::InvalidInput(format!("Invalid quality value '{tok}'"))
                })?;
                last.1.push(v);
            }
        } else {
            return Err(Error::InvalidInput(
                "Quality values before the first header line".to_string(),
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csfasta_lesen() {
        let t = "# Titel\n>r1_1\nT0123\n>r2\nA01\n23\n\n";
        assert_eq!(
            parse_csfasta(t).unwrap(),
            vec![
                ("r1_1".to_string(), "T0123".to_string()),
                ("r2".to_string(), "A0123".to_string())
            ]
        );
        assert!(parse_csfasta("T0123\n").is_err());
        assert!(parse_csfasta("# nur Kommentar\n").is_err());
    }

    #[test]
    fn farbraum() {
        assert_eq!(decode_color_space("T0123").unwrap(), "TTGAT");
        assert_eq!(decode_color_space("a0.1").unwrap(), "AANN");
        assert_eq!(decode_color_space("G").unwrap(), "G");
        assert_eq!(decode_color_space("C3").unwrap(), "CG");
        assert_eq!(decode_color_space("A1").unwrap(), "AC");
        assert!(decode_color_space("").is_err());
        assert!(decode_color_space("N01").is_err());
    }

    #[test]
    fn qual_lesen() {
        let t = ">r1\n30 20\n10\n>r2\n5\n";
        assert_eq!(
            parse_solid_qual(t).unwrap(),
            vec![
                ("r1".to_string(), vec![30, 20, 10]),
                ("r2".to_string(), vec![5])
            ]
        );
        assert!(parse_solid_qual(">r\n300\n").is_err());
        assert!(parse_solid_qual(">r\nabc\n").is_err());
        assert!(parse_solid_qual("1 2\n").is_err());
    }
}
