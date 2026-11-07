//! Read SDF files (multiple molecules).

use crate::error::{Error, Result};
use crate::io::mol::{parse_mol, MolAtom, MolBond};

/// A molecule with a name.
#[derive(Clone, Debug, PartialEq)]
pub struct SdfMolecule {
    /// First line of the block (trimmed), may be empty.
    pub name: String,
    pub atoms: Vec<MolAtom>,
    pub bonds: Vec<MolBond>,
}

/// Splits the text at lines that are exactly `$$$$` (trimmed) and reads each non-empty block with `parse_mol`. Blocks that
/// contain only whitespace are dropped. Data after `M  END` (fields `> <...>`) is ignored by `parse_mol`. Errors from `parse_mol` are
/// passed on, prefixed with the number of the molecule (1-based): `Molekül 2: ...` (as `Error::InvalidInput`). No molecules -> error.
pub fn parse_sdf(text: &str) -> Result<Vec<SdfMolecule>> {
    let mut molecules = Vec::new();

    // Go through line by line and split blocks at `$$$$`
    let mut current_block: Vec<&str> = Vec::new();

    for line in text.lines() {
        if line.trim() == "$$$$" {
            // Block end reached
            let block_text = current_block.join("\n");

            if !block_text.is_empty() {
                match parse_mol(&block_text) {
                    Ok((atoms, bonds)) => {
                        // Name is the first line of the block (or empty)
                        let name = current_block
                            .first()
                            .map(|s| s.trim())
                            .unwrap_or("")
                            .to_string();
                        molecules.push(SdfMolecule { name, atoms, bonds });
                    }
                    Err(e) => {
                        // Error with molecule number (1-based; we keep counting internally for successful or failed blocks)
                        // Since parse_mol only returns successful blocks, we have to determine the number of the current block here.
                        // Do we simply count all blocks following `$$$$` (including empty ones) to determine the number correctly?
                        // The task says: "Errors from parse_mol are passed on, prefixed with the number of the molecule (1-based)".
                        // If parse_mol fails, it is the current molecule.
                        // So we need to know which molecule we are currently processing.
                        let mol_num = molecules.len() + 1; // Since successful molecules were already added
                        let wrapped_error =
                            Error::InvalidInput(format!("Molecule {}: {}", mol_num, e));
                        return Err(wrapped_error);
                    }
                }
            }

            // Reset block for the next one
            current_block.clear();
        } else {
            current_block.push(line);
        }
    }

    // If the text does not end with $$$$, we still have a block in memory.
    // In SDF a block must end with $$$$. If it is missing, it is often invalid or incomplete.
    // The task says: "Splits the text at lines that are exactly $$$$ (trimmed)".
    // If there is no terminating $$$$, do we ignore the rest or treat it as an error?
    // Standard SDF expects $$$$ at the end. If it is missing, the last block is incomplete.
    // We check: if current_block is not empty, we have a block without a terminator.
    // Since parse_mol expects a complete structure (including M END), it will probably fail.
    // But should we try anyway, in case parse_mol is tolerant? No, parse_mol expects M END.
    // So: if current_block is not empty, we add it as the last block (without a terminating $$$$).
    if !current_block.is_empty() {
        let block_text = current_block.join("\n");
        // This block has no number in the sense of "completed blocks", but it is the next molecule.
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
