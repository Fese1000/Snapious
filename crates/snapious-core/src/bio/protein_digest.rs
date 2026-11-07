//! Proteolytic digest: peptides with masses from a protein sequence.

use crate::bio::protein::molecular_weight;
use crate::error::{Error, Result};

/// Cleavage rule of a protease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protease {
    /// After K or R, not before P.
    Trypsin,
    /// After K.
    LysC,
    /// After R, not before P.
    ArgC,
    /// After E.
    GluC,
    /// After F, W or Y, not before P.
    Chymotrypsin,
    /// After M (cyanogen bromide).
    CyanogenBromide,
}

/// A peptide.
#[derive(Clone, Debug, PartialEq)]
pub struct Peptide {
    pub sequence: String,
    /// Range in the protein `[start, end)`, 0-based.
    pub start: usize,
    pub end: usize,
    /// Number of missed cleavage sites in the peptide.
    pub missed_cleavages: usize,
    /// Average mass in Da (`molecular_weight`).
    pub mass: f64,
}

/// Positions after which cleavage happens: index `i` means "between characters `i − 1` and `i`" (`1 <= i < length`).
/// The rule checks the character before and the character after (uppercase): e.g. trypsin: before K or R and after not P.
/// A cleavage site at the end (after the last character) is not included.
pub fn cleavage_sites(protein: &str, protease: Protease) -> Vec<usize> {
    let upper = protein.to_ascii_uppercase();
    let chars: Vec<char> = upper.chars().collect();
    let mut sites = Vec::new();
    let len = chars.len();

    for i in 0..len.saturating_sub(1) {
        let before = chars[i];
        let after = chars.get(i + 1).copied().unwrap_or('\0');

        let cut = match protease {
            Protease::Trypsin => (before == 'K' || before == 'R') && after != 'P',
            Protease::LysC => before == 'K',
            Protease::ArgC => (before == 'R') && after != 'P',
            Protease::GluC => before == 'E',
            Protease::Chymotrypsin => {
                (before == 'F' || before == 'W' || before == 'Y') && after != 'P'
            }
            Protease::CyanogenBromide => before == 'M',
        };

        if cut {
            sites.push(i + 1);
        }
    }

    sites
}

/// Digests `protein` (case-insensitive; a trailing stop `*` is cut off). Without a cleavage site the whole protein results.
/// Splitting at the cleavage sites into base peptides `p1, p2, …` yields all peptides made of 1 to `missed_cleavages + 1`
/// consecutive base peptides (`missed_cleavages` of the peptide = number of joined base peptides − 1).
/// Sorting: by `start`, then by `end`. `min_length` (at least 1) filters out peptides that are shorter.
/// Error `Error::InvalidInput` for an empty protein or characters that are not letters.
pub fn digest_protein(
    protein: &str,
    protease: Protease,
    missed_cleavages: usize,
    min_length: usize,
) -> Result<Vec<Peptide>> {
    // Scope the mutation to avoid the Clippy warning "variable does not need to be mutable".
    let clean = {
        let mut s = protein.to_ascii_uppercase();
        if s.ends_with('*') {
            s.pop();
        }
        s
    };

    // Validate after removing '*' so that "aakpr*" is valid.
    if clean.is_empty() || clean.chars().any(|c| !c.is_ascii_alphabetic()) {
        return Err(Error::InvalidInput(
            "Protein sequence must not be empty and may only contain letters".into(),
        ));
    }

    let sites = cleavage_sites(&clean, protease);

    let mut fragments = Vec::new();
    let mut start = 0;
    for &site in &sites {
        fragments.push((start, site));
        start = site;
    }
    if !clean.is_empty() {
        fragments.push((start, clean.len()));
    }

    let mut peptides = Vec::new();
    let max_fragments = missed_cleavages + 1;

    for i in 0..fragments.len() {
        let current_start = fragments[i].0;
        for j in 1..=max_fragments {
            if i + j > fragments.len() {
                break;
            }
            let current_end = fragments[i + j - 1].1;

            if current_end - current_start < min_length {
                continue;
            }

            let seq = &clean[current_start..current_end];
            peptides.push(Peptide {
                sequence: seq.to_string(),
                start: current_start,
                end: current_end,
                missed_cleavages: j - 1,
                mass: molecular_weight(seq),
            });
        }
    }

    peptides.sort_by_key(|p| (p.start, p.end));

    Ok(peptides)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seqs(p: &[Peptide]) -> Vec<&str> {
        p.iter().map(|x| x.sequence.as_str()).collect()
    }

    #[test]
    fn schnittstellen() {
        assert_eq!(cleavage_sites("AKRAKP", Protease::Trypsin), vec![2, 3]);
        assert_eq!(cleavage_sites("AKRAKP", Protease::LysC), vec![2, 5]);
        assert_eq!(cleavage_sites("AKRAKP", Protease::ArgC), vec![3]);
        assert_eq!(cleavage_sites("AEEA", Protease::GluC), vec![2, 3]);
        assert_eq!(cleavage_sites("AFPFW", Protease::Chymotrypsin), vec![4]);
        assert_eq!(cleavage_sites("AMAM", Protease::CyanogenBromide), vec![2]);
        assert!(cleavage_sites("AAAA", Protease::Trypsin).is_empty());
        assert!(cleavage_sites("K", Protease::Trypsin).is_empty());
        assert_eq!(cleavage_sites("akra", Protease::Trypsin), vec![2, 3]);
    }

    #[test]
    fn trypsinverdau() {
        let p = digest_protein("MKWVTFISLLFLFSSAYSRGVFRR", Protease::Trypsin, 0, 1).unwrap();
        assert_eq!(seqs(&p), vec!["MK", "WVTFISLLFLFSSAYSR", "GVFR", "R"]);
        assert_eq!((p[0].start, p[0].end), (0, 2));
        assert_eq!((p[1].start, p[1].end), (2, 19));
        assert_eq!(p[0].missed_cleavages, 0);
        assert!((p[0].mass - molecular_weight("MK")).abs() < 1e-9);
        let q = digest_protein("MKWVTFISLLFLFSSAYSRGVFRR", Protease::Trypsin, 0, 3).unwrap();
        assert_eq!(seqs(&q), vec!["WVTFISLLFLFSSAYSR", "GVFR"]);
    }

    #[test]
    fn ausgelassene_schnitte() {
        let p = digest_protein("AKBKC", Protease::Trypsin, 1, 1).unwrap();
        assert_eq!(seqs(&p), vec!["AK", "AKBK", "BK", "BKC", "C"]);
        assert_eq!(p[1].missed_cleavages, 1);
        assert_eq!((p[1].start, p[1].end), (0, 4));
        let q = digest_protein("AKBKC", Protease::Trypsin, 5, 1).unwrap();
        assert_eq!(seqs(&q), vec!["AK", "AKBK", "AKBKC", "BK", "BKC", "C"]);
    }

    #[test]
    fn sonderfaelle() {
        let p = digest_protein("aakpr*", Protease::Trypsin, 0, 1).unwrap();
        assert_eq!(seqs(&p), vec!["AAKPR"]);
        assert_eq!(
            digest_protein("GGG", Protease::Trypsin, 0, 1)
                .unwrap()
                .len(),
            1
        );
        assert!(matches!(
            digest_protein("", Protease::Trypsin, 0, 1),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            digest_protein("*", Protease::Trypsin, 0, 1),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            digest_protein("AK1", Protease::Trypsin, 0, 1),
            Err(Error::InvalidInput(_))
        ));
        assert_eq!(
            digest_protein("AKBK", Protease::Trypsin, 0, 0)
                .unwrap()
                .len(),
            2
        );
    }
}
