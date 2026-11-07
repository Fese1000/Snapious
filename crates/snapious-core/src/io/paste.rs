//! Eingefügten Sequenztext (roh, FASTA oder mit Zeilennummern) säubern und die Art der Sequenz erkennen.

/// Art der Sequenz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dna,
    Rna,
    Protein,
}

/// Ein Eintrag aus dem Text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PastedRecord {
    /// Name aus der FASTA-Kopfzeile (ohne `>`, getrimmt); leer ohne Kopfzeile.
    pub name: String,
    /// Großgeschriebene Buchstaben.
    pub residues: String,
}

/// Zerlegt den Text in Einträge. Beginnt der Text (nach führendem Leerraum) mit `>`, ist es FASTA: jede Zeile, die mit `>`
/// beginnt, startet einen Eintrag (Name = Rest der Zeile getrimmt); die folgenden Zeilen gehören dazu. Sonst ist der ganze Text
/// ein einziger Eintrag mit leerem Namen. Aus den Sequenzzeilen werden nur ASCII-Buchstaben behalten (Leerzeichen, Ziffern,
/// `*`, `-`, `\r` usw. fallen weg) und in Uppercase gewandelt. Einträge ohne Buchstaben entfallen; leerer Text → leere Liste.
pub fn parse_pasted(text: &str) -> Vec<PastedRecord> {
    fn letters(line: &str) -> String {
        line.chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_uppercase())
            .collect()
    }

    let mut records = Vec::new();
    if text.trim_start().starts_with('>') {
        let mut name: Option<String> = None;
        let mut residues = String::new();
        for line in text.lines() {
            if let Some(header) = line.trim_start().strip_prefix('>') {
                if let Some(n) = name.take() {
                    records.push(PastedRecord {
                        name: n,
                        residues: std::mem::take(&mut residues),
                    });
                }
                name = Some(header.trim().to_string());
            } else {
                residues.push_str(&letters(line));
            }
        }
        if let Some(n) = name {
            records.push(PastedRecord { name: n, residues });
        }
    } else {
        records.push(PastedRecord {
            name: String::new(),
            residues: text.lines().map(letters).collect(),
        });
    }
    records.retain(|r| !r.residues.is_empty());
    records
}

/// Erkennt die Art: Zähle die Buchstaben außer Leerraum. Leer → `Dna`. Gehören mindestens 80 % zu `A C G T U N`, ist es
/// Nukleinsäure: enthält die Sequenz `U` und kein `T` → `Rna`; enthält sie `U` und `T` → `Protein`; sonst `Dna`. Alles andere → `Protein`.
/// Groß-/Kleinschreibung egal.
pub fn detect_kind(residues: &str) -> Kind {
    let upper: Vec<char> = residues
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if upper.is_empty() {
        return Kind::Dna;
    }
    let nucleic = upper.iter().filter(|c| "ACGTUN".contains(**c)).count();
    if nucleic * 5 < upper.len() * 4 {
        return Kind::Protein;
    }
    match (upper.contains(&'T'), upper.contains(&'U')) {
        (true, true) => Kind::Protein,
        (false, true) => Kind::Rna,
        _ => Kind::Dna,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(name: &str, residues: &str) -> PastedRecord {
        PastedRecord {
            name: name.to_string(),
            residues: residues.to_string(),
        }
    }

    #[test]
    fn fasta_und_rohtext() {
        assert_eq!(
            parse_pasted(">seq1 first\nACGT\nAC GT 10\n>seq2\nggcc\n"),
            vec![rec("seq1 first", "ACGTACGT"), rec("seq2", "GGCC")]
        );
        assert_eq!(
            parse_pasted("1 acgtacgt\n9 acgt\n"),
            vec![rec("", "ACGTACGTACGT")]
        );
        assert_eq!(parse_pasted(">empty\n>x\nAC\n"), vec![rec("x", "AC")]);
        assert_eq!(parse_pasted(">a\r\nAC\r\nGT\r\n"), vec![rec("a", "ACGT")]);
        assert_eq!(parse_pasted("  \n>b\nM K*\n"), vec![rec("b", "MK")]);
        assert!(parse_pasted("").is_empty());
        assert!(parse_pasted("123 456\n").is_empty());
    }

    #[test]
    fn art() {
        assert_eq!(detect_kind("ACGTACGTAC"), Kind::Dna);
        assert_eq!(detect_kind("acgu"), Kind::Rna);
        assert_eq!(detect_kind("ACGTU"), Kind::Protein);
        assert_eq!(detect_kind("MKVLAAGIVGLLLAQ"), Kind::Protein);
        assert_eq!(detect_kind("ACGTNNNNNN"), Kind::Dna);
        assert_eq!(detect_kind("ACGTRYACGT"), Kind::Dna); // 8 von 10
        assert_eq!(detect_kind(""), Kind::Dna);
    }
}
