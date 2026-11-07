//! SDF-Dateien (mehrere Moleküle) lesen.

use crate::error::{Error, Result};
use crate::io::mol::{parse_mol, MolAtom, MolBond};

/// Ein Molekül mit Namen.
#[derive(Clone, Debug, PartialEq)]
pub struct SdfMolecule {
    /// Erste Zeile des Blocks (getrimmt), kann leer sein.
    pub name: String,
    pub atoms: Vec<MolAtom>,
    pub bonds: Vec<MolBond>,
}

/// Teilt den Text an Zeilen, die (getrimmt) genau `$$$$` lauten, und liest jeden nicht leeren Block mit `parse_mol`. Blöcke, die
/// nur Leerraum enthalten, entfallen. Daten nach `M  END` (Felder `> <…>`) werden von `parse_mol` nicht beachtet. Fehler aus `parse_mol` werden
/// weitergegeben, mit der Nummer des Moleküls (1-basiert) vorangestellt: `Molekül 2: …` (als `Error::InvalidInput`). Keine Moleküle → Fehler.
pub fn parse_sdf(text: &str) -> Result<Vec<SdfMolecule>> {
    let mut molecules = Vec::new();

    // Zeilenweise durchgehen und Blöcke an `$$$$` trennen
    let mut current_block: Vec<&str> = Vec::new();

    for line in text.lines() {
        if line.trim() == "$$$$" {
            // Blockende erreicht
            let block_text = current_block.join("\n");

            if !block_text.is_empty() {
                match parse_mol(&block_text) {
                    Ok((atoms, bonds)) => {
                        // Name ist die erste Zeile des Blocks (oder leer)
                        let name = current_block
                            .first()
                            .map(|s| s.trim())
                            .unwrap_or("")
                            .to_string();
                        molecules.push(SdfMolecule { name, atoms, bonds });
                    }
                    Err(e) => {
                        // Fehler mit Molekül-Nummer (1-basiert, nur für erfolgreiche oder fehlerhafte Blöcke zählen wir intern weiter)
                        // Da parse_mol nur erfolgreiche Blöcke zurückgibt, müssen wir hier die Nummer des aktuellen Blocks ermitteln.
                        // Wir zählen einfach alle Blöcke, die auf `$$$$` folgen (auch leere), um die Nummer korrekt zu bestimmen?
                        // Die Aufgabe sagt: "Fehler aus parse_mol werden weitergegeben, mit der Nummer des Moleküls (1-basiert) vorangestellt".
                        // Wenn parse_mol fehlschlägt, ist es das aktuelle Molekül.
                        // Wir müssen also wissen, welches Molekül wir gerade verarbeiten.
                        let mol_num = molecules.len() + 1; // Da erfolgreiche Moleküle schon addiert wurden
                        let wrapped_error =
                            Error::InvalidInput(format!("Molecule {}: {}", mol_num, e));
                        return Err(wrapped_error);
                    }
                }
            }

            // Block zurücksetzen für den nächsten
            current_block.clear();
        } else {
            current_block.push(line);
        }
    }

    // Wenn der Text nicht mit $$$$ endet, haben wir noch einen Block im Speicher.
    // In SDF muss ein Block mit $$$$ enden. Wenn er fehlt, ist es oft ungültig oder unvollständig.
    // Die Aufgabe sagt: "Teilt den Text an Zeilen, die (getrimmt) genau $$$$ lauten".
    // Wenn kein abschließendes $$$$ da ist, ignorieren wir den Rest oder behandeln ihn als Fehler?
    // Standard SDF erwartet $$$$ am Ende. Wenn es fehlt, ist der letzte Block unvollständig.
    // Wir prüfen: Wenn current_block nicht leer ist, haben wir einen Block ohne Abschluss.
    // Da parse_mol eine vollständige Struktur erwartet (inklusive M END), wird er wahrscheinlich fehlschlagen.
    // Aber wir müssen ihn trotzdem versuchen, falls parse_mol tolerant ist? Nein, parse_mol erwartet M END.
    // Also: Wenn current_block nicht leer ist, fügen wir ihn als letzten Block hinzu (ohne abschließendes $$$$).
    if !current_block.is_empty() {
        let block_text = current_block.join("\n");
        // Dieser Block hat keine Nummer im Sinne von "abgeschlossenen Blöcken", aber er ist das nächste Molekül.
        let mol_num = molecules.len() + 1;

        match parse_mol(&block_text) {
            Ok((atoms, bonds)) => {
                let name = current_block
                    .first()
                    .map(|s| s.trim())
                    .unwrap_or("")
                    .to_string();
                molecules.push(SdfMolecule { name, atoms, bonds });
            }
            Err(e) => {
                let wrapped_error = Error::InvalidInput(format!("Molecule {}: {}", mol_num, e));
                return Err(wrapped_error);
            }
        }
    }

    if molecules.is_empty() {
        return Err(Error::InvalidInput("No molecules found".into()));
    }

    Ok(molecules)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "water\n  prog\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n    0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n   -0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  1  3  1  0\nM  END\n> <ID>\nW1\n\n";

    #[test]
    fn zwei_molekuele() {
        let t = format!("{BLOCK}$$$$\n{}$$$$\n", BLOCK.replace("water", "wasser2"));
        let m = parse_sdf(&t).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].name, "water");
        assert_eq!(m[1].name, "wasser2");
        assert_eq!((m[0].atoms.len(), m[0].bonds.len()), (3, 2));
    }

    #[test]
    fn fehler_und_leer() {
        assert!(parse_sdf("").is_err());
        assert!(parse_sdf("$$$$\n\n$$$$\n").is_err());
        let t = format!("{BLOCK}$$$$\nkaputt\nx\ny\nz\n$$$$\n");
        let e = parse_sdf(&t).unwrap_err().to_string();
        assert!(e.contains("Molecule 2"), "{e}");
        assert_eq!(parse_sdf(&format!("{BLOCK}$$$$")).unwrap().len(), 1);
    }
}
