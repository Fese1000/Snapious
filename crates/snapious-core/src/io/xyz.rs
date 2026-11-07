//! XYZ-Moleküldateien lesen.

use crate::error::{Error, Result};

/// Ein Atom.
#[derive(Clone, Debug, PartialEq)]
pub struct XyzAtom {
    /// Elementsymbol: erster Buchstabe groß, Rest klein (`o` → `O`, `CL` → `Cl`).
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Liest XYZ: Zeile 1 = Atomzahl `n`, Zeile 2 = Kommentar (beliebig), danach `n` Zeilen `Element x y z` (durch Leerraum getrennt,
/// weitere Spalten werden ignoriert). Fehler `Error::InvalidInput`: `n` keine Zahl, weniger als `n` Atomzeilen, eine Atomzeile mit
/// weniger als vier Feldern oder Koordinaten, die keine Zahlen sind. Mehr Zeilen als `n` werden ignoriert.
pub fn parse_xyz(text: &str) -> Result<Vec<XyzAtom>> {
    let mut lines = text.lines();

    // Zeile 1: Atomzahl
    let atom_count_line = lines
        .next()
        .ok_or_else(|| Error::InvalidInput("File is empty".to_string()))?;

    let n: usize = atom_count_line.trim().parse::<usize>().map_err(|_| {
        Error::InvalidInput(format!("Invalid atom count: '{}'", atom_count_line.trim()))
    })?;

    // Zeile 2: Kommentar (wird ignoriert, muss aber existieren wenn n > 0 oder Datei nicht leer ist)
    // Laut Spezifikation: "Zeile 1 = Atomzahl n, Zeile 2 = Kommentar".
    // Wenn die Datei nur eine Zeile hat und n > 0, fehlt der Kommentar und die Atome.
    // Wenn n == 0, könnte es theoretisch nur eine Zeile geben, aber Standard XYZ hat meist mindestens 2 Zeilen.
    // Wir lesen die Kommentarzeile, falls vorhanden.
    let _comment_line = lines.next();

    let mut atoms = Vec::with_capacity(n);

    for i in 0..n {
        let line = lines.next().ok_or_else(|| {
            Error::InvalidInput(format!(
                "Fewer atom lines than specified (expected {}, found {})",
                n, i
            ))
        })?;

        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 4 {
            return Err(Error::InvalidInput(format!(
                "Atom line {} has fewer than 4 fields: '{}'",
                i + 1,
                line
            )));
        }

        let element_raw = parts[0];

        // Elementsymbol formatieren: Erster Buchstabe groß, Rest klein.
        let mut element_chars = element_raw.chars();
        let first_char = element_chars
            .next()
            .ok_or_else(|| Error::InvalidInput("Empty element symbol".to_string()))?;

        let element = if first_char.is_ascii_alphabetic() {
            let upper_first = first_char.to_ascii_uppercase();
            let rest: String = element_chars.map(|c| c.to_ascii_lowercase()).collect();
            format!("{}{}", upper_first, rest)
        } else {
            // Wenn das erste Zeichen kein Buchstabe ist (z.B. Zahl oder Symbol),
            // versuchen wir es trotzdem zu parsen, aber das ist unüblich für Elemente.
            // Die Aufgabe sagt "Elementsymbol", also gehen wir von Buchstaben aus.
            element_raw.to_string()
        };

        let x: f64 = parts[1]
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput(format!("Invalid X coordinate '{}'", parts[1])))?;

        let y: f64 = parts[2]
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput(format!("Invalid Y coordinate '{}'", parts[2])))?;

        let z: f64 = parts[3]
            .parse::<f64>()
            .map_err(|_| Error::InvalidInput(format!("Invalid Z coordinate '{}'", parts[3])))?;

        atoms.push(XyzAtom { element, x, y, z });
    }

    Ok(atoms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasser() {
        let t = "3\nwater\nO 0.0 0.0 0.0\nH 0.757 0.586 0.0\nH -0.757 0.586 0.0\n";
        let a = parse_xyz(t).unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!(a[0].element, "O");
        assert_eq!((a[1].x, a[1].y, a[1].z), (0.757, 0.586, 0.0));
        assert_eq!(a[2].x, -0.757);
        assert_eq!(
            parse_xyz("1\n\nCL 1 2 3 extra\nZ 9 9 9\n").unwrap()[0].element,
            "Cl"
        );
    }

    #[test]
    fn fehler() {
        assert!(parse_xyz("2\nc\nO 0 0 0\n").is_err());
        assert!(parse_xyz("x\nc\n").is_err());
        assert!(parse_xyz("1\nc\nO 0 0\n").is_err());
        assert!(parse_xyz("1\nc\nO a b c\n").is_err());
        assert!(parse_xyz("").is_err());
    }
}
