//! Read HyperChem molecule files (HIN): atoms and bonds.

use crate::error::{Error, Result};

/// An atom.
#[derive(Clone, Debug, PartialEq)]
pub struct HinAtom {
    /// Atom number from the file.
    pub number: usize,
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A bond between two atoms (indices into the atom list, 0-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HinBond {
    pub from: usize,
    pub to: usize,
    /// Order: `s` single = 1, `d` double = 2, `t` triple = 3, `a` aromatic = 1.
    pub order: u8,
}

/// Reads all lines that start with `atom ` (other lines are ignored), with the layout
/// `atom <number> - <element> <type> - <x> <y> <z> <count> <neighbor1> <order1> <neighbor2> <order2> ...`
/// (whitespace-separated fields: index 1 number, 3 element, 6/7/8 coordinates, 9 number of bonds, then pairs). A bond
/// is created only once: on the line of the atom with the smaller number (i.e. when the neighbor number is greater than its own).
/// Neighbor numbers are converted into indices of the atom list. `Error::InvalidInput`: too few fields, unreadable
/// numbers, a neighbor that does not exist, an unknown order (not `s`, `d`, `t`, `a`) or no atoms.
pub fn parse_hin(text: &str) -> Result<(Vec<HinAtom>, Vec<HinBond>)> {
    struct Raw {
        number: usize,
        neighbors: Vec<(usize, u8)>,
    }
    let mut atoms: Vec<HinAtom> = Vec::new();
    let mut raws: Vec<Raw> = Vec::new();
    let bad = |what: &str, line: &str| Error::InvalidInput(format!("HIN: {what} in '{line}'"));
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.first() != Some(&"atom") {
            continue;
        }
        if f.len() < 10 {
            return Err(bad("too few fields", line));
        }
        let num = |i: usize| {
            f[i].parse::<usize>()
                .map_err(|_| bad("invalid number", line))
        };
        let flt = |i: usize| {
            f[i].parse::<f64>()
                .map_err(|_| bad("invalid coordinate", line))
        };
        let number = num(1)?;
        let n_bonds = num(9)?;
        let mut neighbors = Vec::new();
        for b in 0..n_bonds {
            let (ni, oi) = (10 + 2 * b, 11 + 2 * b);
            if oi >= f.len() {
                return Err(bad("too few bond fields", line));
            }
            let order = match f[oi] {
                "s" | "a" => 1,
                "d" => 2,
                "t" => 3,
                _ => return Err(bad("unknown bond order", line)),
            };
            neighbors.push((num(ni)?, order));
        }
        atoms.push(HinAtom {
            number,
            element: f[3].to_string(),
            x: flt(6)?,
            y: flt(7)?,
            z: flt(8)?,
        });
        raws.push(Raw { number, neighbors });
    }
    if atoms.is_empty() {
        return Err(Error::InvalidInput(
            "No atoms in the HIN file".to_string(),
        ));
    }
    let index_of = |n: usize| atoms.iter().position(|a| a.number == n);
    let mut bonds = Vec::new();
    for (i, r) in raws.iter().enumerate() {
        for &(nb, order) in &r.neighbors {
            let j = index_of(nb).ok_or_else(|| {
                Error::InvalidInput(format!("HIN: neighbor atom {nb} does not exist"))
            })?;
            if nb > r.number {
                bonds.push(HinBond {
                    from: i,
                    to: j,
                    order,
                });
            }
        }
    }
    Ok((atoms, bonds))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WASSER: &str = "forcefield mm+\nmol 1\natom 1 - O ** - 0.0 0.0 0.0 2 2 s 3 s\natom 2 - H ** - 0.757 0.586 0.0 1 1 s\natom 3 - H ** - -0.757 0.586 0.0 1 1 s\nendmol 1\n";

    #[test]
    fn wasser() {
        let (a, b) = parse_hin(WASSER).unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!((a[0].number, a[0].element.as_str()), (1, "O"));
        assert_eq!((a[1].x, a[1].y, a[1].z), (0.757, 0.586, 0.0));
        assert_eq!(a[2].x, -0.757);
        assert_eq!(
            b,
            vec![
                HinBond {
                    from: 0,
                    to: 1,
                    order: 1
                },
                HinBond {
                    from: 0,
                    to: 2,
                    order: 1
                }
            ]
        );
    }

    #[test]
    fn ordnung_und_fehler() {
        let d = WASSER.replace("2 2 s 3 s", "2 2 d 3 t");
        let (_, b) = parse_hin(&d).unwrap();
        assert_eq!((b[0].order, b[1].order), (2, 3));
        assert_eq!(
            parse_hin(&WASSER.replace("2 2 s 3 s", "2 2 a 3 s"))
                .unwrap()
                .1[0]
                .order,
            1
        );
        assert!(parse_hin("mol 1\nendmol 1\n").is_err());
        assert!(parse_hin("atom 1 - O ** -\n").is_err());
        assert!(parse_hin(&WASSER.replace("2 2 s 3 s", "2 2 s 9 s")).is_err());
        assert!(parse_hin(&WASSER.replace("2 2 s 3 s", "2 2 x 3 s")).is_err());
        assert!(parse_hin(&WASSER.replace("0.757 0.586", "abc 0.586")).is_err());
    }
}
