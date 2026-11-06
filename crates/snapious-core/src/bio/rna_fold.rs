//! RNA-Faltung mit ViennaRNA (`RNAfold`): Ausgabe lesen und Basenpaare aus der Punkt-Klammer-Schreibweise bestimmen.

use std::time::Duration;

use crate::error::{Error, Result};
use crate::tools::ToolRegistry;

/// Ergebnis einer Faltung.
#[derive(Clone, Debug, PartialEq)]
pub struct Folding {
    /// Sequenz (Großbuchstaben, `T` als `U`).
    pub sequence: String,
    /// Struktur in Punkt-Klammer-Schreibweise, gleich lang wie die Sequenz.
    pub structure: String,
    /// Freie Energie in kcal/mol.
    pub energy: f64,
}

/// Liest die Ausgabe von `RNAfold --noPS`: erste Zeile die Sequenz, zweite Zeile `STRUKTUR (ENERGIE)`, z. B. `(((...))) ( -3.20)`.
/// Die Energie steht in Klammern am Zeilenende und darf Leerzeichen nach der öffnenden Klammer haben (`( -3.20)`) sowie ein Minuszeichen
/// oder ein Pluszeichen tragen. Leere Zeilen werden übersprungen; ein Kopfzeile mit `>` wird ebenfalls übersprungen. Fehler `Error::InvalidInput`,
/// wenn weniger als zwei Zeilen übrig bleiben, die Struktur nicht nur aus `.`, `(` und `)` besteht, die Energie keine Zahl ist oder
/// Sequenz und Struktur verschieden lang sind. Weitere Zeilen (z. B. Ensemble-Angaben) werden ignoriert.
pub fn parse_rnafold(text: &str) -> Result<Folding> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut valid_lines: Vec<&str> = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('>') {
            valid_lines.push(trimmed);
        }
    }
    if valid_lines.len() < 2 {
        return Err(Error::InvalidInput(
            "Zu wenige Zeilen vorhanden".to_string(),
        ));
    }

    let sequence = valid_lines[0].trim().to_string();
    let line2 = valid_lines[1];

    let structure_end = line2
        .rfind(" (")
        .ok_or_else(|| Error::InvalidInput("Format der Strukturzeile ungültig".to_string()))?;
    let structure = &line2[..structure_end];

    if !structure.chars().all(|c| matches!(c, '.' | '(' | ')')) {
        return Err(Error::InvalidInput(
            "Struktur enthält ungültige Zeichen".to_string(),
        ));
    }

    if sequence.len() != structure.len() {
        return Err(Error::InvalidInput(
            "Sequenz und Struktur haben unterschiedliche Länge".to_string(),
        ));
    }

    let energy_part = &line2[structure_end + 1..];
    let close_paren = energy_part
        .rfind(')')
        .ok_or_else(|| Error::InvalidInput("Energie-Klammer fehlt".to_string()))?;
    let energy_str = &energy_part[1..close_paren];

    let energy: f64 = energy_str
        .trim()
        .parse::<f64>()
        .map_err(|_| Error::InvalidInput("Energie ist keine gültige Zahl".to_string()))?;

    Ok(Folding {
        sequence,
        structure: structure.to_string(),
        energy,
    })
}

/// Basenpaare `(i, j)` mit `i < j` (0-basiert) aus der Punkt-Klammer-Schreibweise, aufsteigend nach `i` sortiert.
/// Fehler `Error::InvalidInput`, wenn eine Klammer nicht geschlossen oder ohne Partner geschlossen wird oder ein anderes Zeichen als `.`,
/// `(` und `)` vorkommt.
pub fn pairs_from_dot_bracket(structure: &str) -> Result<Vec<(usize, usize)>> {
    let mut stack: Vec<usize> = Vec::new();
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    for (i, c) in structure.char_indices() {
        match c {
            '(' => stack.push(i),
            ')' => {
                let open = stack
                    .pop()
                    .ok_or_else(|| Error::InvalidInput("Klammer nicht geschlossen".to_string()))?;
                pairs.push((open, i));
            }
            '.' => {}
            _ => {
                return Err(Error::InvalidInput(
                    "Ungültiges Zeichen in Struktur".to_string(),
                ))
            }
        }
    }

    if !stack.is_empty() {
        return Err(Error::InvalidInput("Offene Klammern vorhanden".to_string()));
    }

    pairs.sort_by_key(|p| p.0);
    Ok(pairs)
}

/// Faltet eine RNA- oder DNA-Sequenz (`T` wird zu `U`) mit dem Programm `RNAfold --noPS` (Eingabe über stdin, höchstens 120 Sekunden).
/// Fehler `Error::InvalidInput` bei leerer Sequenz oder Zeichen außer A, C, G, U/T, N; `Error::Io`, wenn das Programm fehlt oder scheitert.
pub fn fold_rna(sequence: &str) -> Result<Folding> {
    let seq: String = sequence.trim().to_ascii_uppercase().replace('T', "U");
    if seq.is_empty()
        || !seq
            .chars()
            .all(|c| matches!(c, 'A' | 'C' | 'G' | 'U' | 'N'))
    {
        return Err(Error::InvalidInput("Ungültige RNA-Sequenz".to_string()));
    }
    let reg = ToolRegistry::defaults();
    if !reg.is_available("RNAfold") {
        return Err(Error::Io(
            "Programm 'RNAfold' ist nicht installiert".to_string(),
        ));
    }
    let r = reg.run(
        "RNAfold",
        &["--noPS"],
        None,
        Some(&format!("{}\n", seq)),
        Duration::from_secs(120),
    )?;
    if !r.ok() {
        return Err(Error::Io(format!(
            "'RNAfold' meldete einen Fehler: {}",
            r.stderr.trim()
        )));
    }
    parse_rnafold(&r.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::is_available;

    #[test]
    fn ausgabe_lesen() {
        let f = parse_rnafold("GGGAAAUCCC\n(((....))) ( -3.20)\n").unwrap();
        assert_eq!(f.sequence, "GGGAAAUCCC");
        assert_eq!(f.structure, "(((....)))");
        assert!((f.energy + 3.2).abs() < 1e-9);
        let g = parse_rnafold(">name\nGGGAAAUCCC\n(((....))) (-12.50)\n frei: -13.0\n").unwrap();
        assert!((g.energy + 12.5).abs() < 1e-9);
        let h = parse_rnafold("AAAA\n.... (  0.00)\r\n").unwrap();
        assert_eq!(h.structure, "....");
        assert_eq!(h.energy, 0.0);
        let p = parse_rnafold("AAAA\n.... ( +1.50)\n").unwrap();
        assert!((p.energy - 1.5).abs() < 1e-9);
        let l = parse_rnafold("\n\nGGGAAAUCCC\n\n(((....))) ( -3.20)\n\n").unwrap();
        assert_eq!(l.sequence, "GGGAAAUCCC");
    }

    #[test]
    fn ausgabe_fehler() {
        assert!(matches!(parse_rnafold(""), Err(Error::InvalidInput(_))));
        assert!(matches!(
            parse_rnafold("GGG\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_rnafold("GGG\n((( (abc)\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_rnafold("GGG\n(((x (-1.0)\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_rnafold("GGGA\n((( (-1.0)\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_rnafold("GGG\n((( -1.0\n"),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn paare() {
        assert_eq!(
            pairs_from_dot_bracket("(((....)))").unwrap(),
            vec![(0, 9), (1, 8), (2, 7)]
        );
        assert_eq!(
            pairs_from_dot_bracket("((..))((..))").unwrap(),
            vec![(0, 5), (1, 4), (6, 11), (7, 10)]
        );
        assert_eq!(
            pairs_from_dot_bracket("(.(.).)").unwrap(),
            vec![(0, 6), (2, 4)]
        );
        assert!(pairs_from_dot_bracket("....").unwrap().is_empty());
        assert!(pairs_from_dot_bracket("").unwrap().is_empty());
        assert!(matches!(
            pairs_from_dot_bracket("(()"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            pairs_from_dot_bracket("())"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            pairs_from_dot_bracket("(.x.)"),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn faltung_mit_programm() {
        if !is_available("RNAfold") {
            eprintln!("übersprungen: RNAfold nicht installiert");
            return;
        }
        let f = fold_rna("GGGGAAAACCCC").unwrap();
        assert_eq!(f.sequence, "GGGGAAAACCCC");
        assert_eq!(f.structure.len(), 12);
        assert!(f.energy <= 0.0);
        let p = pairs_from_dot_bracket(&f.structure).unwrap();
        assert!(!p.is_empty());
        // DNA wird als RNA gelesen
        assert_eq!(
            fold_rna("ggggaaaacccc").unwrap().sequence,
            "GGGGAAAACCCC".to_string()
        );
        assert_eq!(fold_rna("GGGGTTTTCCCC").unwrap().sequence, "GGGGUUUUCCCC");
        assert!(matches!(fold_rna(""), Err(Error::InvalidInput(_))));
        assert!(matches!(fold_rna("GGGX"), Err(Error::InvalidInput(_))));
    }
}
