//! Primers for site-directed mutagenesis (substitution, insertion, deletion), QuikChange style: two fully
//! complementary primers that carry the change in the middle.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::tm::nearest_neighbor;

/// Result of mutagenesis primer planning.
#[derive(Clone, Debug, PartialEq)]
pub struct MutagenesisPrimers {
    /// Forward primer 5'->3': left flank + replacement + right flank (uppercase).
    pub forward: String,
    /// Reverse primer 5'->3': reverse complement of `forward`.
    pub reverse: String,
    /// Length of the left flank in bases.
    pub left_flank: usize,
    /// Length of the right flank in bases.
    pub right_flank: usize,
    /// Nearest-neighbor Tm of `forward` (250 nM oligo, 50 mM Na⁺) in °C.
    pub tm: f64,
}

/// Plans primers that replace the range `[start, end)` in `seq` with `replacement`
/// (`start == end`: insertion; empty `replacement`: deletion).
///
/// * `seq` and `replacement` are converted to uppercase; `replacement` may contain only A, C, G, T (empty is allowed).
/// * Both flanks start with `min_flank` bases, limited to what is available (`start` on the left, `seq.len() - end` on the right).
/// * As long as the Tm of `forward` is below `target_tm` and at least one flank may still grow
///   (limit: `max_flank` and what is available), every flank that may still grow is extended by 1 base.
/// * `forward = seq[start - left .. start] + replacement + seq[end .. end + right]`; `reverse` is its reverse complement
///   (`iupac_reverse_complement`); `tm = nearest_neighbor(forward, 250.0, 50.0)`.
///
/// Errors: `Error::OutOfRange` if `start > end` or `end > seq.len()`; `Error::InvalidInput` if `min_flank == 0`,
/// `max_flank < min_flank`, `replacement` contains other characters, or if there is no base available on either the left or the right
/// (`start == 0 && end == seq.len()`); errors from `nearest_neighbor` are passed on.
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
            "start must not be greater than end".to_string(),
        ));
    }
    if end > seq.len() {
        return Err(Error::OutOfRange(
            "end lies outside the sequence length".to_string(),
        ));
    }
    if min_flank == 0 {
        return Err(Error::InvalidInput(
            "min_flank must be greater than 0".to_string(),
        ));
    }
    if max_flank < min_flank {
        return Err(Error::InvalidInput(
            "max_flank must be at least as large as min_flank".to_string(),
        ));
    }

    let seq = seq.to_uppercase();
    let replacement = replacement.to_uppercase();

    if !replacement.is_empty() && replacement.chars().any(|c| !"ACGT".contains(c)) {
        return Err(Error::InvalidInput(
            "replacement contains invalid characters (only A, C, G, T allowed)".to_string(),
        ));
    }

    if start == 0 && end == seq.len() {
        return Err(Error::InvalidInput(
            "No bases available for flanks".to_string(),
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
        // unreachable target: flanks end at max_flank
        let q = design_mutagenesis(SEQ, 15, 16, "A", 5, 8, 1000.0).unwrap();
        assert_eq!((q.left_flank, q.right_flank), (8, 8));
    }

    #[test]
    fn rand_begrenzt_flanke() {
        // only 3 bases available on the left
        let p = design_mutagenesis(SEQ, 3, 4, "A", 5, 8, 1000.0).unwrap();
        assert_eq!(p.left_flank, 3);
        assert_eq!(p.right_flank, 8);
        assert_eq!(p.forward, "ATGAGTACCTGA");
        // only 7 bases available on the right
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
