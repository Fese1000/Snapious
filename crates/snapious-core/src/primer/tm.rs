//! Melting temperature (Tm) of oligonucleotides.
//!
//! * Wallace: `Tm = 2*(A+T) + 4*(G+C)` (short oligos).
//! * Marmur/Doty: `Tm = 64.9 + 41*(G+C - 16.4)/N` (longer oligos, without salt and concentration).
//! * Nearest neighbor after SantaLucia, PNAS 95:1460-1465 (1998), with salt correction.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};

/// Tm by the Wallace rule. Only A, C, G, T/U count; other characters are ignored.
pub fn wallace(seq: &str) -> f64 {
    let (mut at, mut gc) = (0u32, 0u32);
    for b in seq.bytes() {
        match b.to_ascii_uppercase() {
            b'A' | b'T' | b'U' => at += 1,
            b'G' | b'C' => gc += 1,
            _ => {}
        }
    }
    (2 * at + 4 * gc) as f64
}

/// GC fraction (0.0 to 1.0) over all characters; 0.0 for empty input.
pub fn gc_fraction(seq: &str) -> f64 {
    if seq.is_empty() {
        return 0.0;
    }
    let gc = seq
        .bytes()
        .filter(|b| matches!(b.to_ascii_uppercase(), b'G' | b'C'))
        .count();
    gc as f64 / seq.len() as f64
}

/// Tm by Marmur/Doty (textbook formula).
pub fn marmur_doty(seq: &str) -> f64 {
    let n = seq.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let gc = seq
        .bytes()
        .filter(|b| matches!(b.to_ascii_uppercase(), b'G' | b'C'))
        .count() as f64;
    64.9 + 41.0 * (gc - 16.4) / n
}

/// (dH in kcal/mol, dS in cal/(mol*K)) for a dinucleotide 5'->3'.
fn pair_params(a: u8, b: u8) -> Option<(f64, f64)> {
    Some(match (a, b) {
        (b'A', b'A') | (b'T', b'T') => (-7.9, -22.2),
        (b'A', b'T') => (-7.2, -20.4),
        (b'T', b'A') => (-7.2, -21.3),
        (b'C', b'A') | (b'T', b'G') => (-8.5, -22.7),
        (b'G', b'T') | (b'A', b'C') => (-8.4, -22.4),
        (b'C', b'T') | (b'A', b'G') => (-7.8, -21.0),
        (b'G', b'A') | (b'T', b'C') => (-8.2, -22.2),
        (b'C', b'G') => (-10.6, -27.2),
        (b'G', b'C') => (-9.8, -24.4),
        (b'G', b'G') | (b'C', b'C') => (-8.0, -19.9),
        _ => return None,
    })
}

/// Tm in °C using the nearest-neighbor model.
///
/// * `oligo_nm`: oligo concentration in nmol/l (e.g. 250)
/// * `na_mm`: sodium concentration in mmol/l (e.g. 50)
///
/// Only A, C, G, T (U is read as T); at least 2 bases.
pub fn nearest_neighbor(seq: &str, oligo_nm: f64, na_mm: f64) -> Result<f64> {
    let s: Vec<u8> = seq
        .bytes()
        .map(|b| match b.to_ascii_uppercase() {
            b'U' => b'T',
            x => x,
        })
        .collect();
    if s.len() < 2 {
        return Err(Error::InvalidInput(
            "At least 2 bases are needed for the Tm calculation".to_string(),
        ));
    }
    if oligo_nm <= 0.0 || na_mm <= 0.0 {
        return Err(Error::InvalidInput(
            "Concentrations must be greater than 0".to_string(),
        ));
    }
    let (mut dh, mut ds) = (0.0f64, 0.0f64);
    for w in s.windows(2) {
        let (h, e) = pair_params(w[0], w[1]).ok_or_else(|| {
            Error::InvalidInput(format!(
                "Invalid bases '{}{}' for the Tm calculation",
                w[0] as char, w[1] as char
            ))
        })?;
        dh += h;
        ds += e;
    }
    for end in [s[0], s[s.len() - 1]] {
        if end == b'G' || end == b'C' {
            dh += 0.1;
            ds += -2.8;
        } else {
            dh += 2.3;
            ds += 4.1;
        }
    }
    let rc = iupac_reverse_complement(&String::from_utf8_lossy(&s));
    let self_complementary = rc.as_bytes() == s.as_slice();
    if self_complementary {
        ds += -1.4;
    }
    ds += 0.368 * (s.len() as f64 - 1.0) * (na_mm / 1000.0).ln();
    let ct = oligo_nm * 1e-9;
    let x = if self_complementary { 1.0 } else { 4.0 };
    const R: f64 = 1.987;
    Ok(dh * 1000.0 / (ds + R * (ct / x).ln()) - 273.15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallace_beispiele() {
        assert_eq!(wallace("ATGC"), 12.0);
        assert_eq!(wallace(""), 0.0);
        assert_eq!(wallace("GGGG"), 16.0);
        assert_eq!(wallace("atgc"), 12.0);
    }

    #[test]
    fn gc_anteil() {
        assert_eq!(gc_fraction("ATGC"), 0.5);
        assert_eq!(gc_fraction(""), 0.0);
        assert_eq!(gc_fraction("GGGG"), 1.0);
    }

    #[test]
    fn marmur_doty_formel() {
        // 20 G: 64.9 + 41 * (20 - 16.4) / 20 = 72.28
        assert!((marmur_doty(&"G".repeat(20)) - 72.28).abs() < 1e-9);
        assert_eq!(marmur_doty(""), 0.0);
    }

    #[test]
    fn nearest_neighbor_referenzwert() {
        // Computed with an independent Python implementation of the same formula: 51.7108 °C
        let tm = nearest_neighbor("ATGCGTACCTAGGATCCTAG", 250.0, 50.0).unwrap();
        assert!((tm - 51.7108).abs() < 0.01, "{}", tm);
    }

    #[test]
    fn monotonie_laenge_gc_und_salz() {
        let kurz = nearest_neighbor("ATGCGTACCTAGG", 250.0, 50.0).unwrap();
        let lang = nearest_neighbor("ATGCGTACCTAGGATCCTAG", 250.0, 50.0).unwrap();
        assert!(lang > kurz);
        let gc = nearest_neighbor(&"GC".repeat(10), 250.0, 50.0).unwrap();
        let at = nearest_neighbor(&"AT".repeat(10), 250.0, 50.0).unwrap();
        assert!(gc > at);
        let wenig = nearest_neighbor("ATGCGTACCTAGGATCCTAG", 250.0, 20.0).unwrap();
        let viel = nearest_neighbor("ATGCGTACCTAGGATCCTAG", 250.0, 200.0).unwrap();
        assert!(viel > wenig);
        let niedrig = nearest_neighbor("ATGCGTACCTAGGATCCTAG", 50.0, 50.0).unwrap();
        let hoch = nearest_neighbor("ATGCGTACCTAGGATCCTAG", 1000.0, 50.0).unwrap();
        assert!(hoch > niedrig);
    }

    #[test]
    fn symmetrie_sequenz_und_reverse_komplement() {
        let s = "ATGCGTACCTAGGATCCTAG";
        let rc = iupac_reverse_complement(s);
        let a = nearest_neighbor(s, 250.0, 50.0).unwrap();
        let b = nearest_neighbor(&rc, 250.0, 50.0).unwrap();
        assert!((a - b).abs() < 0.01, "{} {}", a, b);
    }

    #[test]
    fn fehlerfaelle() {
        assert!(nearest_neighbor("A", 250.0, 50.0).is_err());
        assert!(nearest_neighbor("", 250.0, 50.0).is_err());
        assert!(nearest_neighbor("ACNT", 250.0, 50.0).is_err());
        assert!(nearest_neighbor("ACGT", 0.0, 50.0).is_err());
        assert!(nearest_neighbor("ACGT", 250.0, -1.0).is_err());
    }

    #[test]
    fn rna_wird_wie_dna_gelesen() {
        let a = nearest_neighbor("ACGUACGUACGUACGU", 250.0, 50.0).unwrap();
        let b = nearest_neighbor("ACGTACGTACGTACGT", 250.0, 50.0).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn selbstkomplementaer_hat_anderen_konzentrationsterm() {
        let pal = nearest_neighbor("GGATCCGGATCC", 250.0, 50.0).unwrap();
        assert!(pal.is_finite());
    }
}
