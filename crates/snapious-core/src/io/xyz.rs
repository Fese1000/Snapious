//! Read XYZ molecule files.

use crate::error::{Error, Result};

/// An atom.
#[derive(Clone, Debug, PartialEq)]
pub struct XyzAtom {
    /// Element symbol: first letter uppercase, rest lowercase (`o` -> `O`, `CL` -> `Cl`).
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Reads XYZ: line 1 = atom count `n`, line 2 = comment (arbitrary), then `n` lines `element x y z` (whitespace-separated,
/// further columns are ignored). `Error::InvalidInput`: `n` not a number, fewer than `n` atom lines, an atom line with
/// fewer than four fields, or coordinates that are not numbers. Lines beyond `n` are ignored.
pub fn parse_xyz(text: &str) -> Result<Vec<XyzAtom>> {
    let mut lines = text.lines();

    // Line 1: atom count
    let atom_count_line = lines
        .next()
        .ok_or_else(|| Error::InvalidInput("File is empty".to_string()))?;

    let n: usize = atom_count_line.trim().parse::<usize>().map_err(|_| {
        Error::InvalidInput(format!("Invalid atom count: '{}'", atom_count_line.trim()))
    })?;

    // Line 2: comment (ignored, but must exist if n > 0 or the file is not empty)
    // According to the specification: "line 1 = atom count n, line 2 = comment".
    // If the file has only one line and n > 0, the comment and the atoms are missing.
    // If n == 0 there could theoretically be only one line, but standard XYZ usually has at least 2 lines.
    // We read the comment line if present.
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

        // Format element symbol: first letter uppercase, rest lowercase.
        let mut element_chars = element_raw.chars();
        let first_char = element_chars
            .next()
            .ok_or_else(|| Error::InvalidInput("Empty element symbol".to_string()))?;

        let element = if first_char.is_ascii_alphabetic() {
            let upper_first = first_char.to_ascii_uppercase();
            let rest: String = element_chars.map(|c| c.to_ascii_lowercase()).collect();
            format!("{}{}", upper_first, rest)
        } else {
            // If the first character is not a letter (e.g. a digit or symbol),
            // we try to parse it anyway, but that is unusual for elements.
            // The task says "element symbol", so we assume letters.
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
