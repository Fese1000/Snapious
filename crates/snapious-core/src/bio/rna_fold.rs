//! RNA folding with ViennaRNA (`RNAfold`): read output and determine base pairs from dot-bracket notation.

use std::time::Duration;

use crate::error::{Error, Result};
use crate::tools::ToolRegistry;

/// Result of a folding.
#[derive(Clone, Debug, PartialEq)]
pub struct Folding {
    /// Sequence (uppercase, `T` as `U`).
    pub sequence: String,
    /// Structure in dot-bracket notation, same length as the sequence.
    pub structure: String,
    /// Free energy in kcal/mol.
    pub energy: f64,
}

/// Reads the output of `RNAfold --noPS`: first line the sequence, second line `STRUCTURE (ENERGY)`, e.g. `(((...))) ( -3.20)`.
/// The energy is in parentheses at the end of the line and may have spaces after the opening parenthesis (`( -3.20)`) and may carry a minus sign
/// or a plus sign. Empty lines are skipped; a header line with `>` is skipped as well. Error `Error::InvalidInput`
/// if fewer than two lines remain, the structure does not consist solely of `.`, `(` and `)`, the energy is not a number, or
/// sequence and structure differ in length. Further lines (e.g. ensemble information) are ignored.
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
            "Too few lines present".to_string(),
        ));
    }

    let sequence = valid_lines[0].trim().to_string();
    let line2 = valid_lines[1];

    let structure_end = line2
        .rfind(" (")
        .ok_or_else(|| Error::InvalidInput("Invalid structure line format".to_string()))?;
    let structure = &line2[..structure_end];

    if !structure.chars().all(|c| matches!(c, '.' | '(' | ')')) {
        return Err(Error::InvalidInput(
            "Structure contains invalid characters".to_string(),
        ));
    }

    if sequence.len() != structure.len() {
        return Err(Error::InvalidInput(
            "Sequence and structure have different lengths".to_string(),
        ));
    }

    let energy_part = &line2[structure_end + 1..];
    let close_paren = energy_part
        .rfind(')')
        .ok_or_else(|| Error::InvalidInput("Energy bracket is missing".to_string()))?;
    let energy_str = &energy_part[1..close_paren];

    let energy: f64 = energy_str
        .trim()
        .parse::<f64>()
        .map_err(|_| Error::InvalidInput("Energy is not a valid number".to_string()))?;

    Ok(Folding {
        sequence,
        structure: structure.to_string(),
        energy,
    })
}

/// Base pairs `(i, j)` with `i < j` (0-based) from the dot-bracket notation, sorted ascending by `i`.
/// Error `Error::InvalidInput` if a bracket is not closed or is closed without a partner, or a character other than `.`,
/// `(` and `)` occurs.
pub fn pairs_from_dot_bracket(structure: &str) -> Result<Vec<(usize, usize)>> {
    let mut stack: Vec<usize> = Vec::new();
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    for (i, c) in structure.char_indices() {
        match c {
            '(' => stack.push(i),
            ')' => {
                let open = stack
                    .pop()
                    .ok_or_else(|| Error::InvalidInput("Bracket not closed".to_string()))?;
                pairs.push((open, i));
            }
            '.' => {}
            _ => {
                return Err(Error::InvalidInput(
                    "Invalid character in structure".to_string(),
                ))
            }
        }
    }

    if !stack.is_empty() {
        return Err(Error::InvalidInput("Open brackets present".to_string()));
    }

    pairs.sort_by_key(|p| p.0);
    Ok(pairs)
}

/// Folds an RNA or DNA sequence (`T` becomes `U`) with the program `RNAfold --noPS` (input via stdin, at most 120 seconds).
/// Error `Error::InvalidInput` for an empty sequence or characters other than A, C, G, U/T, N; `Error::Io` if the program is missing or fails.
pub fn fold_rna(sequence: &str) -> Result<Folding> {
    let seq: String = sequence.trim().to_ascii_uppercase().replace('T', "U");
    if seq.is_empty()
        || !seq
            .chars()
            .all(|c| matches!(c, 'A' | 'C' | 'G' | 'U' | 'N'))
    {
        return Err(Error::InvalidInput("Invalid RNA sequence".to_string()));
    }
    let reg = ToolRegistry::defaults();
    if !reg.is_available("RNAfold") {
        return Err(Error::Io(
            "Program 'RNAfold' is not installed".to_string(),
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
            "'RNAfold' reported an error: {}",
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
            eprintln!("skipped: RNAfold not installed");
            return;
        }
        let f = fold_rna("GGGGAAAACCCC").unwrap();
        assert_eq!(f.sequence, "GGGGAAAACCCC");
        assert_eq!(f.structure.len(), 12);
        assert!(f.energy <= 0.0);
        let p = pairs_from_dot_bracket(&f.structure).unwrap();
        assert!(!p.is_empty());
        // DNA is read as RNA
        assert_eq!(
            fold_rna("ggggaaaacccc").unwrap().sequence,
            "GGGGAAAACCCC".to_string()
        );
        assert_eq!(fold_rna("GGGGTTTTCCCC").unwrap().sequence, "GGGGUUUUCCCC");
        assert!(matches!(fold_rna(""), Err(Error::InvalidInput(_))));
        assert!(matches!(fold_rna("GGGX"), Err(Error::InvalidInput(_))));
    }
}
