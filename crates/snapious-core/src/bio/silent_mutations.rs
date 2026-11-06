//! Stille (synonyme) Einzelbasenänderungen in einer codierenden Sequenz.

use crate::bio::genetic_code::code_by_id;

/// Eine mögliche stille Änderung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SilentChange {
    /// Position der Base in `cds` (0-basiert).
    pub position: usize,
    pub from: char,
    pub to: char,
}

/// Alle Einzelbasenänderungen, die im Leseraster ab Position 0 dieselbe Aminosäure ergeben (Standardcode, `code_by_id(1)`,
/// Großbuchstaben, `U` wie `T`). Es zählen nur vollständige Codons mit Basen A, C, G, T; ein unvollständiges Schluss-Codon
/// wird ignoriert. Stopp-Codons zählen als eine Aminosäure (`*`). Sortierung: nach `position`, dann `to` alphabetisch.
/// Beispiel: `CTGAAA` → (0 C→T), (2 G→A), (2 G→C), (2 G→T), (5 A→G).
pub fn synonymous_changes(cds: &str) -> Vec<SilentChange> {
    let code = match code_by_id(1) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut changes = Vec::new();
    let len = cds.len();
    let mut i = 0;

    while i + 3 <= len {
        // `get` statt Indexierung: bei Mehrbyte-Zeichen liegt die Grenze nicht auf einem Zeichen, das Codon wird dann übersprungen
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

/// Die stillen Änderungen aus `synonymous_changes`, nach deren Anwendung `motif` (nur A, C, G, T; Großbuchstaben) an einer
/// Stelle steht, die die geänderte Base enthält, und die vorher dort nicht stand. Reihenfolge wie `synonymous_changes`.
/// Beispiel: `cds = GAGTTC`, `motif = GAATTC` → nur (2 G→A).
pub fn changes_creating_motif(cds: &str, motif: &str) -> Vec<SilentChange> {
    synonymous_changes(cds)
        .into_iter()
        .filter(|c| motif_overlaps(cds, c, motif))
        .collect()
}

/// Die stillen Änderungen aus `synonymous_changes`, nach denen ein vorher vorhandenes Vorkommen von `motif`, das die
/// geänderte Base enthält, nicht mehr vorhanden ist (an genau dieser Stelle steht danach etwas anderes). Reihenfolge wie
/// `synonymous_changes`. Beispiel: `cds = GAATTC`, `motif = GAATTC` → (2 A→G) und (5 C→T).
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

        // Prüfe, ob motif im Original an dieser Stelle vorhanden war
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

        // Prüfe, ob motif nach der Mutation an dieser Stelle entsteht
        let mut mut_ok = true;
        for (k, &m_byte) in norm_motif.iter().enumerate() {
            let idx = start + k;
            if idx == pos {
                // Nach Mutation steht hier `change.to` (bereits Großbuchstabe)
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

        // Prüfe, ob motif im Original an dieser Stelle vorhanden war
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

        // Prüfe, ob motif nach der Mutation an dieser Stelle NICHT mehr vorhanden ist
        let mut still_matches = true;
        for (k, &m_byte) in norm_motif.iter().enumerate() {
            let idx = start + k;
            if idx == pos {
                // Nach Mutation steht hier `change.to`. Wenn es dem Motif-Zeichen entspricht, bleibt es erhalten.
                if m_byte == change.to as u8 {
                    // bleibt erhalten -> nicht entfernt
                } else {
                    still_matches = false; // geänderte Base bricht das Motif -> entfernt!
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
        assert!(synonymous_changes("ATG").is_empty()); // Met hat nur ein Codon
        assert!(synonymous_changes("AT").is_empty());
        // Stopp: TAA -> TAG und TGA sind ebenfalls Stopp
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
