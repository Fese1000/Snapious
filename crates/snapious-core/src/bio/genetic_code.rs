//! Genetische Codes (NCBI-Tabellen 1, 2, 4, 5 und 11) und Translation.

use crate::model::sequence::amino_acid_for;

/// Ein genetischer Code: Nummer, Name und Abweichungen vom Standardcode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneticCode {
    pub id: u8,
    pub name: String,
    /// Abweichungen vom Standardcode: Codon (Großbuchstaben, `T` statt `U`) und Aminosäure (`*` für Stopp).
    pub changes: Vec<(String, char)>,
}

/// Liste der unterstützten Codes `(Nummer, Name)`, aufsteigend nach Nummer:
/// 1 `Standard`, 2 `Vertebrate Mitochondrial`, 4 `Mold, Protozoan and Coelenterate Mitochondrial`,
/// 5 `Invertebrate Mitochondrial`, 11 `Bacterial, Archaeal and Plant Plastid`.
pub fn available_codes() -> Vec<(u8, String)> {
    vec![
        (1, "Standard".to_string()),
        (2, "Vertebrate Mitochondrial".to_string()),
        (
            4,
            "Mold, Protozoan and Coelenterate Mitochondrial".to_string(),
        ),
        (5, "Invertebrate Mitochondrial".to_string()),
        (11, "Bacterial, Archaeal and Plant Plastid".to_string()),
    ]
}

/// Der Code mit der Nummer `id` oder `None`. Abweichungen vom Standardcode (`amino_acid_for`):
/// - 1 und 11: keine;
/// - 2: `AGA` → `*`, `AGG` → `*`, `ATA` → `M`, `TGA` → `W`;
/// - 4: `TGA` → `W`;
/// - 5: `AGA` → `S`, `AGG` → `S`, `ATA` → `M`, `TGA` → `W`.
pub fn code_by_id(id: u8) -> Option<GeneticCode> {
    match id {
        1 => Some(GeneticCode {
            id,
            name: "Standard".to_string(),
            changes: Vec::new(),
        }),
        2 => Some(GeneticCode {
            id,
            name: "Vertebrate Mitochondrial".to_string(),
            changes: vec![
                ("AGA".into(), '*'),
                ("AGG".into(), '*'),
                ("ATA".into(), 'M'),
                ("TGA".into(), 'W'),
            ],
        }),
        4 => Some(GeneticCode {
            id,
            name: "Mold, Protozoan and Coelenterate Mitochondrial".to_string(),
            changes: vec![("TGA".into(), 'W')],
        }),
        5 => Some(GeneticCode {
            id,
            name: "Invertebrate Mitochondrial".to_string(),
            changes: vec![
                ("AGA".into(), 'S'),
                ("AGG".into(), 'S'),
                ("ATA".into(), 'M'),
                ("TGA".into(), 'W'),
            ],
        }),
        11 => Some(GeneticCode {
            id,
            name: "Bacterial, Archaeal and Plant Plastid".to_string(),
            changes: Vec::new(),
        }),
        _ => None,
    }
}

impl GeneticCode {
    /// Aminosäure für ein Codon (drei Basen; Schreibweise egal, `U` wie `T`): erst die Abweichungen dieses Codes, sonst
    /// `amino_acid_for`. Codons mit anderen Zeichen als A, C, G, T/U oder mit falscher Länge ergeben `X`.
    pub fn amino_acid(&self, codon: &str) -> char {
        if codon.len() != 3 {
            return 'X';
        }
        let mut normalized = [0u8; 3];
        for (i, b) in codon.bytes().enumerate() {
            if i >= 3 {
                break;
            }
            let up = b.to_ascii_uppercase();
            normalized[i] = if up == b'U' { b'T' } else { up };
        }
        for &b in &normalized {
            if !matches!(b, b'A' | b'C' | b'G' | b'T') {
                return 'X';
            }
        }
        for (code, aa) in &self.changes {
            if code.as_bytes() == normalized.as_slice() {
                return *aa;
            }
        }
        amino_acid_for(&normalized) as char
    }

    /// Übersetzt `bases` im Leseraster ab Position 0 in Dreierschritten; ein unvollständiges Schluss-Codon entfällt.
    pub fn translate(&self, bases: &str) -> String {
        let mut result = String::new();
        let len = bases.len();
        let mut i = 0;
        while i + 3 <= len {
            result.push(self.amino_acid(bases.get(i..i + 3).unwrap_or("")));
            i += 3;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liste() {
        let l = available_codes();
        assert_eq!(
            l.iter().map(|c| c.0).collect::<Vec<_>>(),
            vec![1, 2, 4, 5, 11]
        );
        assert_eq!(l[0].1, "Standard");
        assert_eq!(l[1].1, "Vertebrate Mitochondrial");
        assert!(code_by_id(3).is_none());
        assert!(code_by_id(0).is_none());
        for (id, name) in l {
            let c = code_by_id(id).unwrap();
            assert_eq!(c.id, id);
            assert_eq!(c.name, name);
        }
    }

    #[test]
    fn standard_und_bakterien() {
        let s = code_by_id(1).unwrap();
        assert!(s.changes.is_empty());
        assert_eq!(s.translate("ATGAGATGA"), "MR*");
        assert_eq!(code_by_id(11).unwrap().translate("ATGAGATGA"), "MR*");
        assert_eq!(s.amino_acid("atg"), 'M');
        assert_eq!(s.amino_acid("AUG"), 'M');
    }

    #[test]
    fn mitochondrien() {
        assert_eq!(code_by_id(2).unwrap().translate("ATGAGATGA"), "M*W");
        assert_eq!(code_by_id(2).unwrap().translate("ATAAGG"), "M*");
        assert_eq!(code_by_id(4).unwrap().translate("ATGAGATGA"), "MRW");
        assert_eq!(code_by_id(5).unwrap().translate("ATGAGATGAATA"), "MSWM");
        assert_eq!(code_by_id(1).unwrap().translate("ATA"), "I");
    }

    #[test]
    fn sonderfaelle() {
        let s = code_by_id(1).unwrap();
        assert_eq!(s.translate(""), "");
        assert_eq!(s.translate("AT"), "");
        assert_eq!(s.translate("ATGAA"), "M");
        assert_eq!(s.translate("ANGAAA"), "XK");
        assert_eq!(s.amino_acid("AT"), 'X');
        assert_eq!(s.amino_acid("ATGA"), 'X');
        assert_eq!(s.amino_acid("A-G"), 'X');
    }
}
