//! Laborrechner: Stoffmengen, Kopienzahl, Ligationsansatz und Verdünnung für doppelsträngige DNA.

use crate::error::{Error, Result};

/// Mittlere Masse eines Basenpaares in g/mol.
const BP_MASS: f64 = 617.96;
/// Zuschlag für die Enden in g/mol.
const END_MASS: f64 = 36.04;
/// Avogadro-Konstante.
const AVOGADRO: f64 = 6.022_140_76e23;

/// Molekülmasse in g/mol (Da) von dsDNA mit `bp` Basenpaaren: `bp * 617,96 + 36,04`.
/// Fehler `Error::InvalidInput`, wenn `bp` 0 ist.
pub fn dsdna_mass(bp: usize) -> Result<f64> {
    if bp == 0 {
        return Err(Error::InvalidInput("Basenpaare müssen > 0 sein".into()));
    }
    Ok((bp as f64) * BP_MASS + END_MASS)
}

/// Umrechnung ng → pmol: `ng * 1000 / dsdna_mass(bp)`.
/// Fehler `Error::InvalidInput` bei `bp == 0` oder negativem/nicht endlichem `ng`.
pub fn ng_to_pmol(ng: f64, bp: usize) -> Result<f64> {
    if bp == 0 {
        return Err(Error::InvalidInput("Basenpaare müssen > 0 sein".into()));
    }
    if ng < 0.0 || !ng.is_finite() {
        return Err(Error::InvalidInput(
            "ng muss positiv und endlich sein".into(),
        ));
    }
    let mass = dsdna_mass(bp)?;
    Ok(ng * 1000.0 / mass)
}

/// Umrechnung pmol → ng: `pmol * dsdna_mass(bp) / 1000`. Fehler wie bei `ng_to_pmol`.
pub fn pmol_to_ng(pmol: f64, bp: usize) -> Result<f64> {
    if bp == 0 {
        return Err(Error::InvalidInput("Basenpaare müssen > 0 sein".into()));
    }
    if pmol < 0.0 || !pmol.is_finite() {
        return Err(Error::InvalidInput(
            "pmol muss positiv und endlich sein".into(),
        ));
    }
    let mass = dsdna_mass(bp)?;
    Ok(pmol * mass / 1000.0)
}

/// Zahl der Moleküle: `ng * 1e-9 / dsdna_mass(bp) * 6,02214076e23`. Fehler wie bei `ng_to_pmol`.
pub fn copy_number(ng: f64, bp: usize) -> Result<f64> {
    if bp == 0 {
        return Err(Error::InvalidInput("Basenpaare müssen > 0 sein".into()));
    }
    if ng < 0.0 || !ng.is_finite() {
        return Err(Error::InvalidInput(
            "ng muss positiv und endlich sein".into(),
        ));
    }
    let mass = dsdna_mass(bp)?;
    Ok(ng * 1e-9 / mass * AVOGADRO)
}

/// Benötigte Insert-Menge in ng für ein molares Verhältnis `insert : vektor` (z. B. 3.0 für 3:1):
/// `molar_ratio * vector_ng * insert_bp / vector_bp`.
/// Fehler `Error::InvalidInput` bei `vector_bp == 0`, `insert_bp == 0`, `vector_ng < 0` oder `molar_ratio <= 0`.
pub fn ligation_insert_ng(
    vector_ng: f64,
    vector_bp: usize,
    insert_bp: usize,
    molar_ratio: f64,
) -> Result<f64> {
    if vector_bp == 0 || insert_bp == 0 {
        return Err(Error::InvalidInput(
            "Vektor- und Insertlänge müssen > 0 sein".into(),
        ));
    }
    if vector_ng < 0.0 || molar_ratio <= 0.0 {
        return Err(Error::InvalidInput(
            "Vektor-ng muss >= 0 und molar_ratio > 0 sein".into(),
        ));
    }
    Ok(molar_ratio * vector_ng * (insert_bp as f64) / (vector_bp as f64))
}

/// Verdünnung nach `c1 * v1 = c2 * v2`: Volumen der Stammlösung `v1 = c_final * v_final / c_stock`.
/// Fehler `Error::InvalidInput`, wenn `c_stock <= 0`, `c_final < 0`, `v_final < 0` oder `c_final > c_stock`.
pub fn dilution_volume(c_stock: f64, c_final: f64, v_final: f64) -> Result<f64> {
    if c_stock <= 0.0 || c_final < 0.0 || v_final < 0.0 {
        return Err(Error::InvalidInput(
            "Konzentrationen und Endvolumen müssen >= 0 sein, c_stock > 0".into(),
        ));
    }
    if c_final > c_stock {
        return Err(Error::InvalidInput(
            "c_final darf nicht größer als c_stock sein".into(),
        ));
    }
    Ok((c_final * v_final) / c_stock)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn masse_und_stoffmenge() {
        assert!(near(dsdna_mass(1000).unwrap(), 617_996.04));
        assert!(near(
            ng_to_pmol(1000.0, 1000).unwrap(),
            1.618_133_345_967_718_3
        ));
        assert!(near(ng_to_pmol(0.0, 50).unwrap(), 0.0));
        let ng = pmol_to_ng(ng_to_pmol(250.0, 2686).unwrap(), 2686).unwrap();
        assert!(near(ng, 250.0));
    }

    #[test]
    fn kopienzahl() {
        // 1 ng, 1000 bp: 1e-9 / 617996.04 * 6.02214076e23
        let c = copy_number(1.0, 1000).unwrap();
        assert!((c / 9.744_5e8 - 1.0).abs() < 1e-3, "{c}");
    }

    #[test]
    fn ligation_und_verduennung() {
        // 50 ng Vektor (3000 bp), Insert 1000 bp, 3:1 -> 3 * 50 * 1000 / 3000 = 50 ng
        assert!(near(
            ligation_insert_ng(50.0, 3000, 1000, 3.0).unwrap(),
            50.0
        ));
        assert!(near(
            ligation_insert_ng(100.0, 4000, 500, 1.0).unwrap(),
            12.5
        ));
        // 10x-Stamm auf 1x in 50 µl -> 5 µl
        assert!(near(dilution_volume(10.0, 1.0, 50.0).unwrap(), 5.0));
        assert!(near(dilution_volume(5.0, 5.0, 20.0).unwrap(), 20.0));
    }

    #[test]
    fn fehlerfaelle() {
        assert!(matches!(dsdna_mass(0), Err(Error::InvalidInput(_))));
        assert!(matches!(ng_to_pmol(1.0, 0), Err(Error::InvalidInput(_))));
        assert!(matches!(ng_to_pmol(-1.0, 10), Err(Error::InvalidInput(_))));
        assert!(matches!(
            ng_to_pmol(f64::NAN, 10),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(copy_number(1.0, 0), Err(Error::InvalidInput(_))));
        assert!(matches!(
            ligation_insert_ng(50.0, 0, 100, 3.0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            ligation_insert_ng(50.0, 100, 100, 0.0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            dilution_volume(1.0, 2.0, 10.0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            dilution_volume(0.0, 0.0, 10.0),
            Err(Error::InvalidInput(_))
        ));
    }
}
