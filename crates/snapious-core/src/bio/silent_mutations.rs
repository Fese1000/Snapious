//! Silent (synonymous) single-base changes in a coding sequence.

use crate::bio::genetic_code::code_by_id;

/// One possible silent change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SilentChange {
    /// Position of the base in `cds` (0-based).
    pub position: usize,
    pub from: char,
    pub to: char,
}

/// All single-base changes that yield the same amino acid in the reading frame from position 0 (standard code, `code_by_id(1)`,
/// uppercase, `U` as `T`). Only complete codons with bases A, C, G, T count; an incomplete final codon
/// is ignored. Stop codons count as one amino acid (`*`). Sorting: by `position`, then `to` alphabetically.
/// Example: `CTGAAA` → (0 C→T), (2 G→A), (2 G→C), (2 G→T), (5 A→G).
pub fn synonymous_changes(cds: &str) -> Vec<SilentChange> {
    let code = match code_by_id(1) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut changes = Vec::new();
    let len = cds.len();
    let mut i = 0;

    while i + 3 <= len {
        // `get` instead of indexing: with multi-byte characters the boundary does not fall on a character, and the codon is then skipped
        let Some(codon_str) = cds.get(i..i + 3) else {
            i += 3;
            continue;
        };
        let mut norm = [0u8; 3];
        let mut valid = true;

        for (j, b) in codon_str.bytes().enumerate() {
            let up = b.to_ascii_uppercase();
            if matches!(up, b'A' | b'C' | b'G' | b'T') {
                norm[j] = if up == b'U' { b'T' } else { up };
            } else {
                valid = false;
                break;
            }
        }

        if !valid {
            i += 3;
            continue;
        }

        let orig_aa = code.amino_acid(&String::from_utf8_lossy(&norm));

        for pos_in_codon in 0..3 {
            let orig_base = norm[pos_in_codon];
            for &new_base in b"ACGT" {
                if new_base == orig_base {
                    continue;
                }

                let mut mutated = norm;
                mutated[pos_in_codon] = new_base;

                if code.amino_acid(&String::from_utf8_lossy(&mutated)) == orig_aa {
                    changes.push(SilentChange {
                        position: i + pos_in_codon,
                        from: orig_base as char,
                        to: new_base as char,
                    });
                }
            }
        }

        i += 3;
    }

    changes.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.to.cmp(&b.to)));
    changes
}

/// The silent changes from `synonymous_changes` after which `motif` (only A, C, G, T; uppercase) is present at a
/// site containing the changed base and was not present there before. Order as in `synonymous_changes`.
/// Example: `cds = GAGTTC`, `motif = GAATTC` → only (2 G→A).
pub fn changes_creating_motif(cds: &str, motif: &str) -> Vec<SilentChange> {
    synonymous_changes(cds)
        .into_iter()
        .filter(|c| motif_overlaps(cds, c, motif))
        .collect()
}

/// The silent changes from `synonymous_changes` after which a previously present occurrence of `motif` containing the
/// changed base is no longer present (something else is located at exactly that site afterwards). Order as in
/// `synonymous_changes`. Example: `cds = GAATTC`, `motif = GAATTC` → (2 A→G) and (5 C→T).
pub fn changes_removing_motif(cds: &str, motif: &str) -> Vec<SilentChange> {
    synonymous_changes(cds)
        .into_iter()
        .filter(|c| motif_removed(cds, c, motif))
        .collect()
}

fn is_valid_base(b: u8) -> bool {
    matches!(b.to_ascii_uppercase(), b'A' | b'C' | b'G' | b'T')
}
fn norm_base(b: u8) -> u8 {
    let up = b.to_ascii_uppercase();
    if up == b'U' {
        b'T'
    } else {
        up
    }
}

fn motif_overlaps(cds: &str, change: &SilentChange, motif: &str) -> bool {
    let norm_motif: Vec<u8> = motif
        .bytes()
        .filter(|&b| is_valid_base(b))
        .map(norm_base)
        .collect();
    if norm_motif.is_empty() || cds.len() < motif.len() {
        return false;
    }

    let pos = change.position;
    for start in 0..=pos.min(cds.len() - motif.len()) {
        let end = start + motif.len();
        if !(start <= pos && pos < end) {
            continue;
        }

        // Check whether the motif was present at this site in the original
        let mut orig_ok = true;
        for (k, &m_byte) in norm_motif.iter().enumerate() {
            let b = cds.as_bytes()[start + k];
            if !is_valid_base(b) || norm_base(b) != m_byte {
                orig_ok = false;
                break;
            }
        }
        if orig_ok {
            continue;
        }

        // Check whether the motif arises at this site after the mutation
        let mut mut_ok = true;
        for (k, &m_byte) in norm_motif.iter().enumerate() {
            let idx = start + k;
            if idx == pos {
                // After the mutation, `change.to` is here (already uppercase)
                if m_byte != change.to as u8 {
                    mut_ok = false;
                    break;
                }
            } else {
                let b = cds.as_bytes()[idx];
                if !is_valid_base(b) || norm_base(b) != m_byte {
                    mut_ok = false;
                    break;
                }
            }
        }

        if mut_ok {
            return true;
        }
    }
    false
}

fn motif_removed(cds: &str, change: &SilentChange, motif: &str) -> bool {
    let norm_motif: Vec<u8> = motif
        .bytes()
        .filter(|&b| is_valid_base(b))
        .map(norm_base)
        .collect();
    if norm_motif.is_empty() || cds.len() < motif.len() {
        return false;
    }

    let pos = change.position;
    for start in 0..=pos.min(cds.len() - motif.len()) {
        let end = start + motif.len();
        if !(start <= pos && pos < end) {
            continue;
        }

        // Check whether the motif was present at this site in the original
        let mut orig_matches = true;
        for (k, &m_byte) in norm_motif.iter().enumerate() {
            let b = cds.as_bytes()[start + k];
            if !is_valid_base(b) || norm_base(b) != m_byte {
                orig_matches = false;
                break;
            }
        }
        if !orig_matches {
            continue;
        }

        // Check whether the motif is NOT present at this site after the mutation
        let mut still_matches = true;
        for (k, &m_byte) in norm_motif.iter().enumerate() {
            let idx = start + k;
            if idx == pos {
                // After the mutation, `change.to` is here. If it matches the motif character, it is retained.
                if m_byte == change.to as u8 {
                    // retained -> not removed
                } else {
                    still_matches = false; // changed base breaks the motif -> removed!
                }
            } else {
                let b = cds.as_bytes()[idx];
                if !is_valid_base(b) || norm_base(b) != m_byte {
                    still_matches = false;
                    break;
                }
            }
        }

        if !still_matches {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(position: usize, from: char, to: char) -> SilentChange {
        SilentChange { position, from, to }
    }

    #[test]
    fn alle_stillen() {
        assert_eq!(
            synonymous_changes("CTGAAA"),
            vec![
                c(0, 'C', 'T'),
                c(2, 'G', 'A'),
                c(2, 'G', 'C'),
                c(2, 'G', 'T'),
                c(5, 'A', 'G')
            ]
        );
        assert!(synonymous_changes("ATG").is_empty()); // Met has only one codon
        assert!(synonymous_changes("AT").is_empty());
        // Stop: TAA -> TAG and TGA are also stop
        assert_eq!(
            synonymous_changes("TAA"),
            vec![c(1, 'A', 'G'), c(2, 'A', 'G')]
        );
    }

    #[test]
    fn motiv_erzeugen_und_entfernen() {
        assert_eq!(
            changes_creating_motif("GAGTTC", "GAATTC"),
            vec![c(2, 'G', 'A')]
        );
        assert_eq!(
            changes_removing_motif("GAATTC", "GAATTC"),
            vec![c(2, 'A', 'G'), c(5, 'C', 'T')]
        );
        assert!(changes_creating_motif("ATGATG", "GAATTC").is_empty());
        assert!(changes_removing_motif("ATGATG", "GAATTC").is_empty());
    }
}
