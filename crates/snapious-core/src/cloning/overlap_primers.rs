//! Primers for overlap assembly: overhang to the vector plus a binding part on the insert.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::tm::nearest_neighbor;

/// A primer pair with overhangs.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlapPrimers {
    /// Forward primer 5'→3': vector overhang + binding part.
    pub forward: String,
    /// Reverse primer 5'→3': reverse complement of the vector overhang to the right of the cut site + binding part.
    pub reverse: String,
    pub overlap_length: usize,
    /// Length of the binding part of the forward primer and of the reverse primer.
    pub forward_binding_length: usize,
    pub reverse_binding_length: usize,
}

/// Chooses the length `L` (18 to 30) of a binding part. `tm_of(L)` returns the melting temperature for length `L`
/// (`None` if not computable; such lengths do not count). The smallest length with `58.0 <= tm <= 62.0` is chosen;
/// if there is none, the length with the smallest `|tm − 60|` (ties: the smaller one). If no length is computable at all: `None`.
pub fn choose_binding_length(tm_of: &dyn Fn(usize) -> Option<f64>) -> Option<usize> {
    let mut best_len: Option<usize> = None;
    let mut min_diff = f64::MAX;

    for l in 18..=30 {
        if let Some(tm) = tm_of(l) {
            if (58.0..=62.0).contains(&tm) {
                return Some(l);
            }
            let diff = (tm - 60.0).abs();
            if diff < min_diff {
                min_diff = diff;
                best_len = Some(l);
            }
        }
    }
    best_len
}

/// Designs primers so that `insert[insert_start..insert_end]` can be inserted at `cut_position` (between bases of the vector).
///
/// - All texts are read in uppercase with `T` instead of `U`.
/// - Forward primer = `vector[cut − overlap_length .. cut]` + `insert[insert_start .. insert_start + L]`, where `L` is chosen with
///   `choose_binding_length` from the Tm (`nearest_neighbor(part, 250.0, 50.0)`) of the insert start.
/// - Reverse primer = `iupac_reverse_complement(vector[cut .. cut + overlap_length])` + `iupac_reverse_complement(insert[insert_end − L' .. insert_end])`,
///   `L'` likewise with `choose_binding_length` (Tm of the binding part of the reverse primer, i.e. of the reverse complement of the insert end).
///   As a result `iupac_reverse_complement(reverse)` ends with `vector[cut .. cut + overlap_length]`.
/// - Errors `Error::InvalidInput`: `overlap_length == 0`; `overlap_length > cut_position`; `cut_position + overlap_length > vector.len()`;
///   `insert_start >= insert_end` or `insert_end > insert.len()`; insert range shorter than 30 bases (otherwise `L` cannot be chosen freely);
///   no computable length for one of the two binding parts.
///
/// Example: vector `…ACGT|TTGG…` with `cut_position` between `T` and `T`, `overlap_length 4`: forward primer starts with `ACGT`
/// and continues with the insert start; reverse primer starts with `CCAA` (reverse complement of `TTGG`).
pub fn design_overlap_primers(
    insert: &str,
    insert_start: usize,
    insert_end: usize,
    vector: &str,
    cut_position: usize,
    overlap_length: usize,
) -> Result<OverlapPrimers> {
    if overlap_length == 0 {
        return Err(Error::InvalidInput(
            "overlap_length must be greater than 0".into(),
        ));
    }
    if overlap_length > cut_position {
        return Err(Error::InvalidInput(
            "overlap_length must not be greater than cut_position".into(),
        ));
    }
    if (cut_position + overlap_length) > vector.len() {
        return Err(Error::InvalidInput(
            "Cut site + overhang lies outside the vector".into(),
        ));
    }
    if insert_start >= insert_end {
        return Err(Error::InvalidInput(
            "insert_start must be less than insert_end".into(),
        ));
    }
    if insert_end > insert.len() {
        return Err(Error::InvalidInput(
            "insert_end lies outside the insert sequence".into(),
        ));
    }

    let insert_len = insert_end - insert_start;
    if insert_len < 30 {
        return Err(Error::InvalidInput(
            "Insert region must be at least 30 bases long".into(),
        ));
    }

    let forward_overlap = &vector[cut_position - overlap_length..cut_position];

    // Helper function to find the length for a sequence section
    let find_binding_length = |seq: &str| -> Option<usize> {
        let tm_of = |l: usize| nearest_neighbor(&seq[..l], 250.0, 50.0).ok();
        choose_binding_length(&tm_of)
    };

    let forward_l = find_binding_length(&insert[insert_start..]).ok_or_else(|| {
        Error::InvalidInput("No computable length for the forward primer".into())
    })?;

    let reverse_overlap =
        iupac_reverse_complement(&vector[cut_position..cut_position + overlap_length]);

    let reverse_l = {
        let tm_of = |l: usize| {
            let slice = &insert[insert_end - l..insert_end];
            nearest_neighbor(&iupac_reverse_complement(slice), 250.0, 50.0).ok()
        };
        choose_binding_length(&tm_of).ok_or_else(|| {
            Error::InvalidInput("No computable length for the reverse primer".into())
        })?
    };

    let forward_binding = &insert[insert_start..insert_start + forward_l];
    let reverse_binding_rc = iupac_reverse_complement(&insert[insert_end - reverse_l..insert_end]);

    let forward = format!("{}{}", forward_overlap, forward_binding);
    let reverse = format!("{}{}", reverse_overlap, reverse_binding_rc);

    Ok(OverlapPrimers {
        forward,
        reverse,
        overlap_length,
        forward_binding_length: forward_l,
        reverse_binding_length: reverse_l,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic test sequence with mixed composition.
    fn seq(n: usize, seed: u64) -> String {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ['A', 'C', 'G', 'T'][((s >> 33) % 4) as usize]
            })
            .collect()
    }

    fn rc(s: &str) -> String {
        iupac_reverse_complement(s)
    }

    #[test]
    fn laengenwahl() {
        // Tm rises with length: 18 → 50, +1.0 per base → 58 at L = 26
        let f = |l: usize| Some(50.0 + (l as f64 - 18.0));
        assert_eq!(choose_binding_length(&f), Some(26));
        // never in the window, always too cold → closest to 60 is the longest
        let kalt = |l: usize| Some(30.0 + 0.1 * l as f64);
        assert_eq!(choose_binding_length(&kalt), Some(30));
        // always too hot → shortest
        let heiss = |l: usize| Some(80.0 + 0.1 * l as f64);
        assert_eq!(choose_binding_length(&heiss), Some(18));
        // not computable for any
        assert_eq!(choose_binding_length(&|_| None), None);
        // only some lengths computable
        let teils = |l: usize| if l == 20 { Some(59.0) } else { None };
        assert_eq!(choose_binding_length(&teils), Some(20));
    }

    #[test]
    fn primeraufbau() {
        let vector = seq(80, 1);
        let insert = seq(120, 2);
        let p = design_overlap_primers(&insert, 10, 100, &vector, 40, 25).unwrap();
        assert_eq!(p.overlap_length, 25);
        assert!((18..=30).contains(&p.forward_binding_length));
        assert!((18..=30).contains(&p.reverse_binding_length));
        assert!(p.forward.starts_with(&vector[15..40]));
        assert_eq!(p.forward.len(), 25 + p.forward_binding_length);
        assert_eq!(&p.forward[25..], &insert[10..10 + p.forward_binding_length]);
        assert_eq!(p.reverse.len(), 25 + p.reverse_binding_length);
        assert!(p.reverse.starts_with(&rc(&vector[40..65])));
        assert_eq!(
            &p.reverse[25..],
            rc(&insert[100 - p.reverse_binding_length..100])
        );
        assert!(rc(&p.reverse).ends_with(&vector[40..65]));
    }

    #[test]
    fn gewaehlte_laenge_folgt_der_regel() {
        let vector = seq(80, 3);
        let insert = seq(150, 4);
        let p = design_overlap_primers(&insert, 5, 140, &vector, 30, 25).unwrap();
        let tm_vorn = |l: usize| nearest_neighbor(&insert[5..5 + l], 250.0, 50.0).ok();
        assert_eq!(
            Some(p.forward_binding_length),
            choose_binding_length(&tm_vorn)
        );
        let tm_hinten = |l: usize| nearest_neighbor(&rc(&insert[140 - l..140]), 250.0, 50.0).ok();
        assert_eq!(
            Some(p.reverse_binding_length),
            choose_binding_length(&tm_hinten)
        );
    }

    #[test]
    fn kleines_beispiel_aus_der_doku() {
        let vector = format!("{}ACGTTTGG{}", "GATTACA".repeat(2), "CAGT".repeat(5));
        let cut = 14 + 4; // between ACGT and TTGG
        let insert = seq(60, 5);
        let p = design_overlap_primers(&insert, 0, 60, &vector, cut, 4).unwrap();
        assert!(p.forward.starts_with("ACGT"));
        assert!(p.reverse.starts_with("CCAA"));
    }

    #[test]
    fn fehler() {
        let vector = seq(80, 1);
        let insert = seq(120, 2);
        let e = |r: Result<OverlapPrimers>| matches!(r, Err(Error::InvalidInput(_)));
        assert!(e(design_overlap_primers(&insert, 10, 100, &vector, 40, 0)));
        assert!(e(design_overlap_primers(&insert, 10, 100, &vector, 10, 25)));
        assert!(e(design_overlap_primers(&insert, 10, 100, &vector, 70, 25)));
        assert!(e(design_overlap_primers(&insert, 100, 10, &vector, 40, 25)));
        assert!(e(design_overlap_primers(&insert, 10, 200, &vector, 40, 25)));
        assert!(e(design_overlap_primers(&insert, 10, 30, &vector, 40, 25)));
        assert!(e(design_overlap_primers(
            "ACGTNNNNACGTNNNNACGTNNNNACGTNNNNACGT",
            0,
            36,
            &vector,
            40,
            25
        )));
    }
}
