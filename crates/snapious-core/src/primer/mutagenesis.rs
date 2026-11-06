//! Primer für gezielte Mutagenese (Austausch, Einfügung, Deletion), Art QuikChange: zwei vollständig
//! komplementäre Primer, die die Änderung in der Mitte tragen.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::tm::nearest_neighbor;

/// Ergebnis der Mutagenese-Primer-Planung.
#[derive(Clone, Debug, PartialEq)]
pub struct MutagenesisPrimers {
    /// Vorwärtsprimer 5'→3': linke Flanke + Ersatz + rechte Flanke (Großbuchstaben).
    pub forward: String,
    /// Rückwärtsprimer 5'→3': Reverskomplement von `forward`.
    pub reverse: String,
    /// Länge der linken Flanke in Basen.
    pub left_flank: usize,
    /// Länge der rechten Flanke in Basen.
    pub right_flank: usize,
    /// Nearest-Neighbor-Tm von `forward` (250 nM Oligo, 50 mM Na⁺) in °C.
    pub tm: f64,
}

/// Plant Primer, die in `seq` den Bereich `[start, end)` durch `replacement` ersetzen
/// (`start == end`: Einfügung; leeres `replacement`: Deletion).
///
/// * `seq` und `replacement` werden in Großbuchstaben umgewandelt; `replacement` darf nur A, C, G, T enthalten (leer ist erlaubt).
/// * Beide Flanken beginnen mit `min_flank` Basen, begrenzt auf das Vorhandene (`start` links, `seq.len() - end` rechts).
/// * Solange der Tm von `forward` kleiner als `target_tm` ist und mindestens eine Flanke noch wachsen darf
///   (Grenze: `max_flank` und das Vorhandene), wird jede Flanke, die noch wachsen darf, um 1 Base verlängert.
/// * `forward = seq[start - left .. start] + replacement + seq[end .. end + right]`; `reverse` ist sein Reverskomplement
///   (`iupac_reverse_complement`); `tm = nearest_neighbor(forward, 250.0, 50.0)`.
///
/// Fehler: `Error::OutOfRange`, wenn `start > end` oder `end > seq.len()`; `Error::InvalidInput`, wenn `min_flank == 0`,
/// `max_flank < min_flank`, `replacement` andere Zeichen enthält, oder wenn weder links noch rechts eine Base
/// vorhanden ist (`start == 0 && end == seq.len()`); Fehler von `nearest_neighbor` werden weitergegeben.
pub fn design_mutagenesis(
    seq: &str,
    start: usize,
    end: usize,
    replacement: &str,
    min_flank: usize,
    max_flank: usize,
    target_tm: f64,
) -> Result<MutagenesisPrimers> {
    if start > end {
        return Err(Error::OutOfRange(
            "start darf nicht größer als end sein".to_string(),
        ));
    }
    if end > seq.len() {
        return Err(Error::OutOfRange(
            "end liegt außerhalb der Sequenzlänge".to_string(),
        ));
    }
    if min_flank == 0 {
        return Err(Error::InvalidInput(
            "min_flank muss größer als 0 sein".to_string(),
        ));
    }
    if max_flank < min_flank {
        return Err(Error::InvalidInput(
            "max_flank muss mindestens so groß wie min_flank sein".to_string(),
        ));
    }

    let seq = seq.to_uppercase();
    let replacement = replacement.to_uppercase();

    if !replacement.is_empty() && replacement.chars().any(|c| !"ACGT".contains(c)) {
        return Err(Error::InvalidInput(
            "replacement enthält ungültige Zeichen (nur A, C, G, T erlaubt)".to_string(),
        ));
    }

    if start == 0 && end == seq.len() {
        return Err(Error::InvalidInput(
            "Keine Basen für Flanken verfügbar".to_string(),
        ));
    }

    let available_left = start;
    let available_right = seq.len() - end;

    let mut left = min_flank.min(available_left);
    let mut right = min_flank.min(available_right);

    loop {
        let forward = format!(
            "{}{}{}",
            &seq[start - left..start],
            replacement,
            &seq[end..end + right]
        );
        let tm = nearest_neighbor(&forward, 250.0, 50.0)?;

        if tm >= target_tm {
            break;
        }

        let can_left = left < max_flank && left < available_left;
        let can_right = right < max_flank && right < available_right;

        if !can_left && !can_right {
            break;
        }

        if can_left {
            left += 1;
        }
        if can_right {
            right += 1;
        }
    }

    let forward = format!(
        "{}{}{}",
        &seq[start - left..start],
        replacement,
        &seq[end..end + right]
    );
    let reverse = iupac_reverse_complement(&forward);
    let tm = nearest_neighbor(&forward, 250.0, 50.0)?;

    Ok(MutagenesisPrimers {
        forward,
        reverse,
        left_flank: left,
        right_flank: right,
        tm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEQ: &str = "ATGCGTACCTGAAGTCCGATTAGCTAGGCTTACGATCG";

    #[test]
    fn austausch_mit_kleinem_ziel() {
        let p = design_mutagenesis(SEQ, 15, 16, "a", 5, 20, 0.0).unwrap();
        assert_eq!(p.forward, "GAAGTACGATT");
        assert_eq!(p.reverse, "AATCGTACTTC");
        assert_eq!((p.left_flank, p.right_flank), (5, 5));
        assert_eq!(p.tm, nearest_neighbor("GAAGTACGATT", 250.0, 50.0).unwrap());
    }

    #[test]
    fn deletion_und_einfuegung() {
        let d = design_mutagenesis(SEQ, 15, 18, "", 4, 4, 0.0).unwrap();
        assert_eq!(d.forward, "AAGTATTA");
        let i = design_mutagenesis(SEQ, 15, 15, "TTT", 4, 4, 0.0).unwrap();
        assert_eq!(i.forward, "AAGTTTTCCGA");
        assert_eq!(i.reverse, iupac_reverse_complement("AAGTTTTCCGA"));
    }

    #[test]
    fn flanken_wachsen_bis_zum_ziel() {
        let base = design_mutagenesis(SEQ, 15, 16, "A", 5, 12, 0.0).unwrap();
        let target = base.tm + 0.5;
        let p = design_mutagenesis(SEQ, 15, 16, "A", 5, 12, target).unwrap();
        assert!(p.left_flank > 5 && p.right_flank > 5);
        assert!(p.left_flank <= 12 && p.right_flank <= 12);
        assert_eq!(p.left_flank, p.right_flank);
        assert!(p.tm >= target || p.left_flank == 12);
        let expected = format!(
            "{}A{}",
            &SEQ[15 - p.left_flank..15],
            &SEQ[16..16 + p.right_flank]
        );
        assert_eq!(p.forward, expected);
        // unerreichbares Ziel: Flanken enden bei max_flank
        let q = design_mutagenesis(SEQ, 15, 16, "A", 5, 8, 1000.0).unwrap();
        assert_eq!((q.left_flank, q.right_flank), (8, 8));
    }

    #[test]
    fn rand_begrenzt_flanke() {
        // links nur 3 Basen vorhanden
        let p = design_mutagenesis(SEQ, 3, 4, "A", 5, 8, 1000.0).unwrap();
        assert_eq!(p.left_flank, 3);
        assert_eq!(p.right_flank, 8);
        assert_eq!(p.forward, "ATGAGTACCTGA");
        // rechts nur 7 Basen vorhanden
        let q = design_mutagenesis(SEQ, 30, 31, "T", 5, 8, 1000.0).unwrap();
        assert_eq!((q.left_flank, q.right_flank), (8, 7));
        assert_eq!(q.forward, "GCTAGGCTTACGATCG");
    }

    #[test]
    fn fehlerfaelle() {
        assert!(matches!(
            design_mutagenesis(SEQ, 5, 4, "A", 5, 8, 0.0),
            Err(Error::OutOfRange(_))
        ));
        assert!(matches!(
            design_mutagenesis(SEQ, 5, 99, "A", 5, 8, 0.0),
            Err(Error::OutOfRange(_))
        ));
        assert!(matches!(
            design_mutagenesis(SEQ, 5, 6, "A", 0, 8, 0.0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            design_mutagenesis(SEQ, 5, 6, "A", 6, 5, 0.0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            design_mutagenesis(SEQ, 5, 6, "X", 5, 8, 0.0),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            design_mutagenesis("ACGTACGT", 0, 8, "A", 2, 4, 0.0),
            Err(Error::InvalidInput(_))
        ));
    }
}
