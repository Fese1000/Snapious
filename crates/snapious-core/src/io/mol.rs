//! Read MOL files (MDL V2000): atoms and bonds.

use crate::error::{Error, Result};

/// An atom.
#[derive(Clone, Debug, PartialEq)]
pub struct MolAtom {
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A bond between two atoms (0-based indices) with order (1 single, 2 double, 3 triple ...).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MolBond {
    pub from: usize,
    pub to: usize,
    pub order: u8,
}

/// Reads a MOL file (V2000). The first three lines are the header (name, program, comment). Line 4 is the counts line: the
/// first three characters (trimmed) are the atom count, the next three the bond count. The atom lines follow: `x` in
/// characters 0-10, `y` in 10-20, `z` in 20-30 (each trimmed), the element symbol in 31-34 (trimmed). Then the bond lines:
/// atom 1 in characters 0-3, atom 2 in 3-6 (1-based in the file), order in 6-9. Column access uses `str::get` so that
/// special characters do not panic. `Error::InvalidInput` for a file that is too short, unreadable numbers or bond indices
/// outside the atom count.
pub fn parse_mol(text: &str) -> Result<(Vec<MolAtom>, Vec<MolBond>)> {
    let mut lines = text.lines();

    // Skip lines 0, 1, 2 (name, program, comment)
    if lines.next().is_none() {
        return Err(Error::InvalidInput("Too few lines".into()));
    }
    if lines.next().is_none() {
        return Err(Error::InvalidInput("Too few lines".into()));
    }
    if lines.next().is_none() {
        return Err(Error::InvalidInput("Too few lines".into()));
    }

    // Line 3: counts line
    let count_line = lines
        .next()
        .ok_or_else(|| Error::InvalidInput("Too few lines".into()))?;

    // Parse atom count (characters 0-3) and bond count (characters 3-6)
    let atom_count_str = count_line.get(0..3).unwrap_or("").trim();
    let bond_count_str = count_line.get(3..6).unwrap_or("").trim();

    let atom_count: usize = atom_count_str
        .parse::<usize>()
        .map_err(|_| Error::InvalidInput("Invalid atom count".into()))?;
    let _bond_count: usize = bond_count_str
        .parse::<usize>()
        .map_err(|_| Error::InvalidInput("Invalid bond count".into()))?;

    let mut atoms = Vec::with_capacity(atom_count);

    // Parse atom lines (next atom_count lines)
    for _ in 0..atom_count {
        let line = lines
            .next()
            .ok_or_else(|| Error::InvalidInput("Too few atom lines".into()))?;

        // x: 0-10, y: 10-20, z: 20-30
        let x_str = line.get(0..10).unwrap_or("").trim();
        let y_str = line.get(10..20).unwrap_or("").trim();
        let z_str = line.get(20..30).unwrap_or("").trim();

        let x: f64 = x_str
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput("Invalid X coordinate".into()))?;
        let y: f64 = y_str
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput("Invalid Y coordinate".into()))?;
        let z: f64 = z_str
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput("Invalid Z coordinate".into()))?;

        // Element: 31-34
        let element = line.get(31..34).unwrap_or("").trim().to_string();

        atoms.push(MolAtom { element, x, y, z });
    }

    let mut bonds = Vec::new();

    // Parse bond lines (to the end or until M  END)
    // exactly as many lines as the counts line states (property lines such as `M  CHG` and `M  END` follow)
    for line in lines.take(_bond_count) {
        // Atom 1: 0-3, atom 2: 3-6, order: 6-9
        let atom1_str = line.get(0..3).unwrap_or("").trim();
        let atom2_str = line.get(3..6).unwrap_or("").trim();
        let order_str = line.get(6..9).unwrap_or("").trim();

        let atom1: usize = match atom1_str.parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::InvalidInput(
                    "Invalid atom index in bond".into(),
                ))
            }
        };

        let atom2: usize = match atom2_str.parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::InvalidInput(
                    "Invalid atom index in bond".into(),
                ))
            }
        };

        let order: u8 = match order_str.parse::<u8>() {
            Ok(v) => v,
            Err(_) => return Err(Error::InvalidInput("Invalid bond order".into())),
        };

        // 1-based in file -> 0-based in the vector
        let from = atom1
            .checked_sub(1)
            .ok_or_else(|| Error::InvalidInput("Atom index too small".into()))?;
        let to = atom2
            .checked_sub(1)
            .ok_or_else(|| Error::InvalidInput("Atom index too small".into()))?;

        if from >= atom_count || to >= atom_count {
            return Err(Error::InvalidInput(
                "Bond index outside the atom count".into(),
            ));
        }

        bonds.push(MolBond { from, to, order });
    }

    // If fewer bonds were found than stated in the counts line, the file is invalid.
    if bonds.len() != _bond_count {
        return Err(Error::InvalidInput(
            "Number of bonds does not match the counts line".into(),
        ));
    }

    Ok((atoms, bonds))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER: &str = "water\n  prog\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n    0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n   -0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  1  3  1  0\nM  END\n";

    #[test]
    fn wasser() {
        let (atoms, bonds) = parse_mol(WATER).unwrap();
        assert_eq!(atoms.len(), 3);
        assert_eq!(atoms[0].element, "O");
        assert_eq!((atoms[1].x, atoms[1].y), (0.757, 0.586));
        assert_eq!(atoms[2].x, -0.757);
        assert_eq!(
            bonds,
            vec![
                MolBond {
                    from: 0,
                    to: 1,
                    order: 1
                },
                MolBond {
                    from: 0,
                    to: 2,
                    order: 1
                }
            ]
        );
    }

    #[test]
    fn eigenschaftszeilen_nach_den_bindungen() {
        let mit = WATER.replace("M  END", "M  CHG  1   1   1\nM  END");
        let (atoms, bonds) = parse_mol(&mit).unwrap();
        assert_eq!((atoms.len(), bonds.len()), (3, 2));
    }

    #[test]
    fn fehler() {
        assert!(parse_mol("").is_err());
        assert!(parse_mol("a\nb\nc\n").is_err());
        let kurz = WATER.replace("  1  3  1  0\n", "");
        assert!(parse_mol(&kurz).is_err());
        let falsch = WATER.replace("  1  3  1  0", "  1  9  1  0");
        assert!(parse_mol(&falsch).is_err());
    }
}
