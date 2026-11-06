//! MOL-Dateien (MDL V2000) lesen: Atome und Bindungen.

use crate::error::{Error, Result};

/// Ein Atom.
#[derive(Clone, Debug, PartialEq)]
pub struct MolAtom {
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Eine Bindung zwischen zwei Atomen (0-basierte Indizes) mit Ordnung (1 einfach, 2 doppelt, 3 dreifach …).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MolBond {
    pub from: usize,
    pub to: usize,
    pub order: u8,
}

/// Liest eine MOL-Datei (V2000). Die ersten drei Zeilen sind Kopf (Name, Programm, Kommentar). Zeile 4 ist die Zählzeile: die
/// ersten drei Zeichen (getrimmt) sind die Atomzahl, die nächsten drei die Bindungszahl. Danach folgen die Atomzeilen: `x` in den
/// Zeichen 0–10, `y` in 10–20, `z` in 20–30 (jeweils getrimmt), das Elementsymbol in 31–34 (getrimmt). Danach die Bindungszeilen:
/// Atom 1 in den Zeichen 0–3, Atom 2 in 3–6 (1-basiert in der Datei), Ordnung in 6–9. Spaltenzugriffe mit `str::get`, damit
/// Sonderzeichen nicht abstürzen. Fehler `Error::InvalidInput` bei zu kurzer Datei, unlesbaren Zahlen oder Bindungsindizes
/// außerhalb der Atomzahl.
pub fn parse_mol(text: &str) -> Result<(Vec<MolAtom>, Vec<MolBond>)> {
    let mut lines = text.lines();

    // Zeilen 0, 1, 2 überspringen (Name, Programm, Kommentar)
    if lines.next().is_none() {
        return Err(Error::InvalidInput("Zu wenige Zeilen".into()));
    }
    if lines.next().is_none() {
        return Err(Error::InvalidInput("Zu wenige Zeilen".into()));
    }
    if lines.next().is_none() {
        return Err(Error::InvalidInput("Zu wenige Zeilen".into()));
    }

    // Zeile 3: Zählzeile
    let count_line = lines
        .next()
        .ok_or_else(|| Error::InvalidInput("Zu wenige Zeilen".into()))?;

    // Atomzahl (Zeichen 0-3) und Bindungszahl (Zeichen 3-6) parsen
    let atom_count_str = count_line.get(0..3).unwrap_or("").trim();
    let bond_count_str = count_line.get(3..6).unwrap_or("").trim();

    let atom_count: usize = atom_count_str
        .parse::<usize>()
        .map_err(|_| Error::InvalidInput("Ungültige Atomzahl".into()))?;
    let _bond_count: usize = bond_count_str
        .parse::<usize>()
        .map_err(|_| Error::InvalidInput("Ungültige Bindungszahl".into()))?;

    let mut atoms = Vec::with_capacity(atom_count);

    // Atomzeilen parsen (nächste atom_count Zeilen)
    for _ in 0..atom_count {
        let line = lines
            .next()
            .ok_or_else(|| Error::InvalidInput("Zu wenige Atomzeilen".into()))?;

        // x: 0-10, y: 10-20, z: 20-30
        let x_str = line.get(0..10).unwrap_or("").trim();
        let y_str = line.get(10..20).unwrap_or("").trim();
        let z_str = line.get(20..30).unwrap_or("").trim();

        let x: f64 = x_str
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput("Ungültige X-Koordinate".into()))?;
        let y: f64 = y_str
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput("Ungültige Y-Koordinate".into()))?;
        let z: f64 = z_str
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput("Ungültige Z-Koordinate".into()))?;

        // Element: 31-34
        let element = line.get(31..34).unwrap_or("").trim().to_string();

        atoms.push(MolAtom { element, x, y, z });
    }

    let mut bonds = Vec::new();

    // Bindungszeilen parsen (bis zum Ende oder bis M  END)
    // genau so viele Zeilen, wie die Zählzeile angibt (danach folgen Eigenschaftszeilen wie `M  CHG` und `M  END`)
    for line in lines.take(_bond_count) {
        // Atom 1: 0-3, Atom 2: 3-6, Ordnung: 6-9
        let atom1_str = line.get(0..3).unwrap_or("").trim();
        let atom2_str = line.get(3..6).unwrap_or("").trim();
        let order_str = line.get(6..9).unwrap_or("").trim();

        let atom1: usize = match atom1_str.parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::InvalidInput(
                    "Ungültiger Atomindex in Bindung".into(),
                ))
            }
        };

        let atom2: usize = match atom2_str.parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::InvalidInput(
                    "Ungültiger Atomindex in Bindung".into(),
                ))
            }
        };

        let order: u8 = match order_str.parse::<u8>() {
            Ok(v) => v,
            Err(_) => return Err(Error::InvalidInput("Ungültige Bindungsordnung".into())),
        };

        // 1-basiert in Datei -> 0-basiert im Vektor
        let from = atom1
            .checked_sub(1)
            .ok_or_else(|| Error::InvalidInput("Atomindex zu klein".into()))?;
        let to = atom2
            .checked_sub(1)
            .ok_or_else(|| Error::InvalidInput("Atomindex zu klein".into()))?;

        if from >= atom_count || to >= atom_count {
            return Err(Error::InvalidInput(
                "Bindungsindex außerhalb der Atomzahl".into(),
            ));
        }

        bonds.push(MolBond { from, to, order });
    }

    // Wenn weniger Bindungen gefunden wurden als in der Zählzeile angegeben, ist die Datei ungültig.
    if bonds.len() != _bond_count {
        return Err(Error::InvalidInput(
            "Anzahl der Bindungen stimmt nicht mit Zählzeile überein".into(),
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
