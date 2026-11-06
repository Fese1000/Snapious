//! Qualitätsprüfungen für Primer: Wiederholungen, GC-Klammer, Haarnadel, Dimer.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::primer::tm;

/// Länge des längsten Laufs gleicher Basen (z. B. `AAAAGC` → 4).
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

/// GC-Klammer: mindestens eine der letzten 2 Basen ist G/C und höchstens 3 G/C unter den letzten 5.
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

/// Länge des längsten Stammes einer möglichen Haarnadel (mindestens 4 Basen, Schleife mindestens 3),
/// sonst 0. Beispiel: `GGATCCAAAAGGATCC` → 6.
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

/// Dimer-Wert zweier Sequenzen: längster komplementärer Abschnitt zwischen `a` und dem Reverse-Komplement
/// von `b` (mindestens 3 Basen). Reicht der Abschnitt bis zum 3'-Ende von `a`, zählt er doppelt, weil
/// 3'-Dimere vom Enzym verlängert werden. Kein Abschnitt ≥ 3 → 0.
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

/// Gesamtbewertung eines Primers.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimerQuality {
    pub length: usize,
    pub tm: f64,
    pub gc: f64,
    pub homopolymer: usize,
    pub gc_clamp: bool,
    pub hairpin: usize,
    pub self_dimer: usize,
    /// Strafe: kleiner ist besser (siehe [`penalty`]).
    pub penalty: f64,
}

/// Strafe: `|Tm − Soll| + 5·|GC − 0,5| + 2·Haarnadel/10 + 2·Dimer/10`.
pub fn penalty(tm: f64, opt_tm: f64, gc: f64, hairpin: usize, self_dimer: usize) -> f64 {
    (tm - opt_tm).abs()
        + 5.0 * (gc - 0.5).abs()
        + 2.0 * hairpin as f64 / 10.0
        + 2.0 * self_dimer as f64 / 10.0
}

/// Bewertet einen Primer (Tm nach Nearest-Neighbor mit 250 nM Oligo und 50 mM Na⁺; Solltemperatur 60 °C).
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
        assert!(!has_gc_clamp("ATGCGCGC")); // 4 G/C in den letzten 5
        assert!(!has_gc_clamp("A"));
        assert!(has_gc_clamp("ATATATAG"));
    }

    #[test]
    fn haarnadel() {
        assert!(hairpin_score("GGATCCAAAAGGATCC") >= 6);
        assert_eq!(hairpin_score("AAAAAAAAAAAA"), 0);
        assert_eq!(hairpin_score("ACGT"), 0);
        // Stamm GCGC (4), Schleife AAA, Gegenstück GCGC
        assert_eq!(hairpin_score("GCGCAAAGCGC"), 4);
        // Schleife zu kurz (2): kein Stamm
        assert_eq!(hairpin_score("GCGCAAGCGC"), 0);
    }

    #[test]
    fn dimer() {
        assert!(dimer_score("GGATCC", "GGATCC") >= 6);
        assert_eq!(dimer_score("AAAAAA", "AAAAAA"), 0);
        // 3'-Ende von a (…CCGG) ist komplementär zu b (CCGG…): Abschnitt am 3'-Ende zählt doppelt
        assert_eq!(dimer_score("AAAACCGG", "CCGGAAAA"), 8);
        // gleicher Abschnitt (CCGG) mitten in a zählt einfach; TTT am Ende ist nur 3 lang und nicht am 3'-Ende
        assert_eq!(dimer_score("CCGGTTTA", "CCGGAAAA"), 4);
        // ganze Sequenz komplementär bis zum 3'-Ende: 2 * 8
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
