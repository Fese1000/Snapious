//! Read PDB files (atom coordinates): atoms, chains and sequence.

use crate::error::{Error, Result};

/// An atom from an `ATOM` or `HETATM` line.
#[derive(Clone, Debug, PartialEq)]
pub struct Atom {
    /// Atom name (columns 13-16, without whitespace).
    pub name: String,
    /// Residue name (columns 18-20, without whitespace).
    pub residue: String,
    /// Chain (column 22).
    pub chain: char,
    /// Residue number (columns 23-26).
    pub number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Element (columns 77-78, without whitespace; empty if the line is shorter).
    pub element: String,
    /// `true` for `HETATM`.
    pub hetero: bool,
}

/// A parsed structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Pdb {
    pub atoms: Vec<Atom>,
}

/// Three-letter code -> one-letter code of the 20 standard amino acids (uppercase, e.g. `GLY` -> `G`); `MSE` counts as `M`,
/// everything else is `X`.
pub fn one_letter(residue: &str) -> char {
    let upper = residue.trim().to_uppercase();
    match upper.as_str() {
        "ALA" => 'A',
        "CYS" => 'C',
        "ASP" => 'D',
        "GLU" => 'E',
        "PHE" => 'F',
        "GLY" => 'G',
        "HIS" => 'H',
        "ILE" => 'I',
        "LYS" => 'K',
        "LEU" => 'L',
        "MET" => 'M',
        "ASN" => 'N',
        "PRO" => 'P',
        "SER" => 'S',
        "THR" => 'T',
        "VAL" => 'V',
        "TRP" => 'W',
        "TYR" => 'Y',
        "MSE" => 'M',
        _ => 'X',
    }
}

/// Reads `ATOM` and `HETATM` lines (columns as in the PDB specification, 1-based: name 13-16, residue 18-20, chain 22,
/// number 23-26, x 31-38, y 39-46, z 47-54, element 77-78). All other lines are ignored; lines that are too short for the
/// coordinates (fewer than 54 characters) or whose numbers cannot be parsed are skipped. Line endings `\n` and `\r\n`.
/// There are no errors; a file without atoms yields an empty structure.
pub fn parse_pdb(text: &str) -> Pdb {
    let mut atoms = Vec::new();
    for line in text.lines() {
        if !(line.starts_with("ATOM") || line.starts_with("HETATM")) {
            continue;
        }
        if line.len() < 54 {
            continue;
        }

        // `get` instead of indexing: with multi-byte characters in a column the boundary is not on a character, and the line is skipped
        let col = |from: usize, to: usize| line.get(from..to).map(str::trim);
        let (Some(name), Some(residue), Some(num), Some(xs), Some(ys), Some(zs)) = (
            col(12, 16),
            col(17, 20),
            col(22, 26),
            col(30, 38),
            col(38, 46),
            col(46, 54),
        ) else {
            continue;
        };
        let chain = line.chars().nth(21).unwrap_or(' ');
        let (Ok(number), Ok(x), Ok(y), Ok(z)) = (
            num.parse::<i32>(),
            xs.parse::<f64>(),
            ys.parse::<f64>(),
            zs.parse::<f64>(),
        ) else {
            continue;
        };
        let name = name.to_string();
        let residue = residue.to_string();
        let element = if line.len() >= 78 {
            line.get(76..78)
                .map(|e| e.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let hetero = line.starts_with("HETATM");

        atoms.push(Atom {
            name,
            residue,
            chain,
            number,
            x,
            y,
            z,
            element,
            hetero,
        });
    }
    Pdb { atoms }
}

impl Pdb {
    /// Chains in order of first appearance (only atoms with `hetero == false` count; chain ` ` also counts as a character).
    pub fn chains(&self) -> Vec<char> {
        let mut seen = Vec::new();
        for a in &self.atoms {
            if !a.hetero && !seen.contains(&a.chain) {
                seen.push(a.chain);
            }
        }
        seen
    }

    /// Sequence of a chain from the non-HETATM atoms: one letter per new residue (change of residue number relative to the
    /// previous atom of the same chain) using `one_letter`. Unknown chain -> empty text.
    pub fn sequence(&self, chain: char) -> String {
        let mut seq = String::new();
        let mut last_num: Option<i32> = None;

        for a in &self.atoms {
            if a.chain == chain && !a.hetero {
                match last_num {
                    Some(n) if n == a.number => continue,
                    _ => {
                        seq.push(one_letter(&a.residue));
                        last_num = Some(a.number);
                    }
                }
            }
        }
        seq
    }

    /// Number of residues in the chain (as in `sequence`).
    pub fn residue_count(&self, chain: char) -> usize {
        let mut count = 0;
        let mut last_num: Option<i32> = None;

        for a in &self.atoms {
            if a.chain == chain && !a.hetero {
                match last_num {
                    Some(n) if n == a.number => continue,
                    _ => {
                        count += 1;
                        last_num = Some(a.number);
                    }
                }
            }
        }
        count
    }

    /// Centroid of all atoms (mean of x, y, z); `Error::InvalidInput` without atoms.
    pub fn center(&self) -> Result<(f64, f64, f64)> {
        if self.atoms.is_empty() {
            return Err(Error::InvalidInput("No atoms present".into()));
        }
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;
        for a in &self.atoms {
            sum_x += a.x;
            sum_y += a.y;
            sum_z += a.z;
        }
        let n = self.atoms.len() as f64;
        Ok((sum_x / n, sum_y / n, sum_z / n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDB: &str = "HEADER    TESTSTRUKTUR
ATOM      1 N    GLY A   1      10.000  11.000  12.000  1.00 20.00           N
ATOM      2 CA   GLY A   1      11.500  11.000  12.000  1.00 20.00           C
ATOM      3 N    ALA A   2      12.000  12.500  13.250  1.00 20.00           N
ATOM      4 CA   ALA A   2      13.000  13.000  14.000  1.00 20.00           C
ATOM      5 N    SER A   3      14.000  14.000  15.000  1.00 20.00           N
ATOM      6 CA   SER A   3      15.000  15.500  16.000  1.00 20.00           C
HETATM    7 O    HOH A 101       1.000   2.000   3.000  1.00 20.00           O
ATOM      8 N    VAL B   1      20.000  21.000  22.000  1.00 20.00           N
TER
END
";

    #[test]
    fn einbuchstabencode() {
        assert_eq!(one_letter("GLY"), 'G');
        assert_eq!(one_letter("ala"), 'A');
        assert_eq!(one_letter("TRP"), 'W');
        assert_eq!(one_letter("MSE"), 'M');
        assert_eq!(one_letter("HOH"), 'X');
    }

    #[test]
    fn atome_lesen() {
        let p = parse_pdb(PDB);
        assert_eq!(p.atoms.len(), 8);
        let a = &p.atoms[2];
        assert_eq!(a.name, "N");
        assert_eq!(a.residue, "ALA");
        assert_eq!(a.chain, 'A');
        assert_eq!(a.number, 2);
        assert_eq!((a.x, a.y, a.z), (12.0, 12.5, 13.25));
        assert_eq!(a.element, "N");
        assert!(!a.hetero);
        assert!(p.atoms[6].hetero);
        assert_eq!(p.atoms[6].number, 101);
    }

    #[test]
    fn ketten_und_sequenz() {
        let p = parse_pdb(PDB);
        assert_eq!(p.chains(), vec!['A', 'B']);
        assert_eq!(p.sequence('A'), "GAS");
        assert_eq!(p.residue_count('A'), 3);
        assert_eq!(p.sequence('B'), "V");
        assert_eq!(p.sequence('Z'), "");
    }

    #[test]
    fn mittelpunkt_und_robustheit() {
        let p = parse_pdb("ATOM      1 N    GLY A   1      10.000  11.000  12.000\r\nATOM      2 CA   GLY A   1      12.000  13.000  14.000\r\nATOM zu kurz\r\nHETATM garbage\n");
        assert_eq!(p.atoms.len(), 2);
        assert_eq!(p.atoms[0].element, "");
        assert_eq!(p.center().unwrap(), (11.0, 12.0, 13.0));
        assert!(parse_pdb("").center().is_err());
        assert!(parse_pdb("").chains().is_empty());
    }
}
