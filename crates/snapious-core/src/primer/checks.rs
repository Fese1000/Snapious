//! Quality checks for primers: repeats, GC clamp, hairpin, dimer.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::primer::tm;

/// Length of the longest run of identical bases (e.g. `AAAAGC` -> 4).
pub fn max_homopolymer_run(seq: &str) -> usize {
    let mut best = 0;
    let mut run = 0;
    let mut prev = 0u8;
    for b in seq.bytes() {
        let b = b.to_ascii_uppercase();
        if b == prev {
            run += 1;
        } else {
            run = 1;
            prev = b;
        }
        best = best.max(run);
    }
    best
}

/// GC clamp: at least one of the last 2 bases is G/C and at most 3 G/C among the last 5.
pub fn has_gc_clamp(seq: &str) -> bool {
    let b: Vec<u8> = seq.bytes().map(|c| c.to_ascii_uppercase()).collect();
    if b.len() < 2 {
        return false;
    }
    let is_gc = |c: u8| matches!(c, b'G' | b'C');
    let last2 = b[b.len() - 2..].iter().any(|c| is_gc(*c));
    let last5 = b[b.len().saturating_sub(5)..]
        .iter()
        .filter(|c| is_gc(**c))
        .count();
    last2 && last5 <= 3
}

/// Length of the longest stem of a possible hairpin (at least 4 bases, loop at least 3),
/// otherwise 0. Example: `GGATCCAAAAGGATCC` -> 6.
pub fn hairpin_score(seq: &str) -> usize {
    let s = seq.to_ascii_uppercase();
    let b = s.as_bytes();
    let n = b.len();
    let mut best = 0;
    for i in 0..n {
        for k in (4..=(n - i).min(n)).rev() {
            if k <= best {
                break;
            }
            let stem = &s[i..i + k];
            let rc = iupac_reverse_complement(stem);
            let from = i + k + 3;
            if from + k <= n && s[from..].contains(&rc) {
                best = best.max(k);
                break;
            }
        }
    }
    best
}

/// Dimer score of two sequences: longest complementary stretch between `a` and the reverse complement
/// of `b` (at least 3 bases). If the stretch reaches the 3' end of `a`, it counts double because
/// 3' dimers are extended by the enzyme. No stretch >= 3 -> 0.
pub fn dimer_score(a: &str, b: &str) -> usize {
    let a = a.to_ascii_uppercase();
    let rc = iupac_reverse_complement(b);
    let (ab, rb) = (a.as_bytes(), rc.as_bytes());
    let mut best = 0;
    for i in 0..ab.len() {
        for j in 0..rb.len() {
            let mut k = 0;
            while i + k < ab.len() && j + k < rb.len() && ab[i + k] == rb[j + k] {
                k += 1;
            }
            if k >= 3 {
                let at_three_prime = i + k == ab.len();
                best = best.max(if at_three_prime { 2 * k } else { k });
            }
        }
    }
    best
}

/// Overall assessment of a primer.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimerQuality {
    pub length: usize,
    pub tm: f64,
    pub gc: f64,
    pub homopolymer: usize,
    pub gc_clamp: bool,
    pub hairpin: usize,
    pub self_dimer: usize,
    /// Penalty: smaller is better (see [`penalty`]).
    pub penalty: f64,
}

/// Penalty: `|Tm - target| + 5*|GC - 0.5| + 2*hairpin/10 + 2*dimer/10`.
pub fn penalty(tm: f64, opt_tm: f64, gc: f64, hairpin: usize, self_dimer: usize) -> f64 {
    (tm - opt_tm).abs()
        + 5.0 * (gc - 0.5).abs()
        + 2.0 * hairpin as f64 / 10.0
        + 2.0 * self_dimer as f64 / 10.0
}

/// Assesses a primer (Tm by nearest neighbor with 250 nM oligo and 50 mM Na⁺; target temperature 60 °C).
pub fn evaluate_primer(seq: &str) -> crate::error::Result<PrimerQuality> {
    let s = seq.to_ascii_uppercase();
    let t = tm::nearest_neighbor(&s, 250.0, 50.0)?;
    let gc = tm::gc_fraction(&s);
    let hairpin = hairpin_score(&s);
    let self_dimer = dimer_score(&s, &s);
    Ok(PrimerQuality {
        length: s.len(),
        tm: t,
        gc,
        homopolymer: max_homopolymer_run(&s),
        gc_clamp: has_gc_clamp(&s),
        hairpin,
        self_dimer,
        penalty: penalty(t, 60.0, gc, hairpin, self_dimer),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homopolymer() {
        assert_eq!(max_homopolymer_run("AAAAGC"), 4);
        assert_eq!(max_homopolymer_run("ACGT"), 1);
        assert_eq!(max_homopolymer_run(""), 0);
        assert_eq!(max_homopolymer_run("acccct"), 4);
    }

    #[test]
    fn gc_klammer() {
        assert!(has_gc_clamp("ATATATGC"));
        assert!(!has_gc_clamp("ATATATAT"));
        assert!(!has_gc_clamp("ATGCGCGC")); // 4 G/C in the last 5
        assert!(!has_gc_clamp("A"));
        assert!(has_gc_clamp("ATATATAG"));
    }

    #[test]
    fn haarnadel() {
        assert!(hairpin_score("GGATCCAAAAGGATCC") >= 6);
        assert_eq!(hairpin_score("AAAAAAAAAAAA"), 0);
        assert_eq!(hairpin_score("ACGT"), 0);
        // Stem GCGC (4), loop AAA, counterpart GCGC
        assert_eq!(hairpin_score("GCGCAAAGCGC"), 4);
        // Loop too short (2): no stem
        assert_eq!(hairpin_score("GCGCAAGCGC"), 0);
    }

    #[test]
    fn dimer() {
        assert!(dimer_score("GGATCC", "GGATCC") >= 6);
        assert_eq!(dimer_score("AAAAAA", "AAAAAA"), 0);
        // 3' end of a (...CCGG) is complementary to b (CCGG...): stretch at the 3' end counts double
        assert_eq!(dimer_score("AAAACCGG", "CCGGAAAA"), 8);
        // same stretch (CCGG) in the middle of a counts once; TTT at the end is only 3 long and not at the 3' end
        assert_eq!(dimer_score("CCGGTTTA", "CCGGAAAA"), 4);
        // whole sequence complementary up to the 3' end: 2 * 8
        assert_eq!(dimer_score("TTTTCCGG", "CCGGAAAA"), 16);
        assert_eq!(dimer_score("", "ACGT"), 0);
    }

    #[test]
    fn bewertung() {
        let q = evaluate_primer("atgcgtacctaggatcctag").unwrap();
        assert_eq!(q.length, 20);
        assert!((q.tm - 51.7108).abs() < 0.01);
        assert!((q.gc - 0.5).abs() < 1e-9);
        assert!(q.penalty > 0.0);
        assert!(evaluate_primer("A").is_err());
    }

    #[test]
    fn strafe_formel() {
        assert!((penalty(60.0, 60.0, 0.5, 0, 0)).abs() < 1e-12);
        assert!((penalty(58.0, 60.0, 0.4, 5, 10) - (2.0 + 0.5 + 1.0 + 2.0)).abs() < 1e-9);
    }
}
