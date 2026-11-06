//! Primer-Auswahl für ein Zielfragment einer Vorlage.
//!
//! Kandidaten sind alle Teilsequenzen passender Länge links (Vorwärtsprimer) bzw. rechts (Rückwärtsprimer)
//! des Ziels. Ein Kandidat ist gültig, wenn Tm, GC-Anteil, GC-Klammer, Wiederholungen, Haarnadel und
//! Selbst-Dimer die Vorgaben erfüllen. Paare werden nach Strafe sortiert.

use rayon::prelude::*;

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::checks::{
    dimer_score, hairpin_score, has_gc_clamp, max_homopolymer_run, penalty,
};
use crate::primer::tm;

/// Vorgaben für die Auswahl.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimerConstraints {
    pub min_length: usize,
    pub max_length: usize,
    pub min_tm: f64,
    pub max_tm: f64,
    pub opt_tm: f64,
    pub min_gc: f64,
    pub max_gc: f64,
    pub max_tm_difference: f64,
    pub min_product: usize,
    pub max_product: usize,
    pub require_gc_clamp: bool,
    pub max_homopolymer: usize,
    pub max_hairpin: usize,
    pub max_dimer: usize,
    pub oligo_nm: f64,
    pub na_mm: f64,
}

impl PrimerConstraints {
    /// 18–27 nt, Tm 57–63 °C (Soll 60), GC 40–60 %, ΔTm ≤ 3 °C, Produkt 100–3000 bp, GC-Klammer.
    pub fn standard() -> Self {
        Self {
            min_length: 18,
            max_length: 27,
            min_tm: 57.0,
            max_tm: 63.0,
            opt_tm: 60.0,
            min_gc: 0.40,
            max_gc: 0.60,
            max_tm_difference: 3.0,
            min_product: 100,
            max_product: 3000,
            require_gc_clamp: true,
            max_homopolymer: 4,
            max_hairpin: 5,
            max_dimer: 6,
            oligo_nm: 250.0,
            na_mm: 50.0,
        }
    }
}

/// Ein ausgewählter Primer.
#[derive(Clone, Debug, PartialEq)]
pub struct PickedPrimer {
    /// Sequenz 5'→3' (beim Rückwärtsprimer das Reverse-Komplement der Vorlage).
    pub sequence: String,
    /// Bereich auf der Vorlage `[start, end)`.
    pub start: usize,
    pub end: usize,
    pub forward: bool,
    pub tm: f64,
    pub gc: f64,
    pub penalty: f64,
}

/// Ein Primerpaar.
#[derive(Clone, Debug, PartialEq)]
pub struct PickedPair {
    pub forward: PickedPrimer,
    pub reverse: PickedPrimer,
    pub product_size: usize,
    pub penalty: f64,
}

fn is_plain_dna(s: &[u8]) -> bool {
    s.iter().all(|b| matches!(b, b'A' | b'C' | b'G' | b'T'))
}

fn make_candidate(
    template: &str,
    start: usize,
    end: usize,
    forward: bool,
    c: &PrimerConstraints,
) -> Option<PickedPrimer> {
    let region = &template[start..end];
    if !is_plain_dna(region.as_bytes()) {
        return None;
    }
    let seq = if forward {
        region.to_string()
    } else {
        iupac_reverse_complement(region)
    };
    let gc = tm::gc_fraction(&seq);
    if gc < c.min_gc || gc > c.max_gc {
        return None;
    }
    let t = tm::nearest_neighbor(&seq, c.oligo_nm, c.na_mm).ok()?;
    if t < c.min_tm || t > c.max_tm {
        return None;
    }
    if c.require_gc_clamp && !has_gc_clamp(&seq) {
        return None;
    }
    if max_homopolymer_run(&seq) > c.max_homopolymer {
        return None;
    }
    let hairpin = hairpin_score(&seq);
    if hairpin > c.max_hairpin {
        return None;
    }
    let dimer = dimer_score(&seq, &seq);
    if dimer > c.max_dimer {
        return None;
    }
    Some(PickedPrimer {
        penalty: penalty(t, c.opt_tm, gc, hairpin, dimer),
        sequence: seq,
        start,
        end,
        forward,
        tm: t,
        gc,
    })
}

fn sort_candidates(v: &mut [PickedPrimer]) {
    v.sort_by(|a, b| {
        a.penalty
            .partial_cmp(&b.penalty)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.start.cmp(&b.start))
            .then(a.end.cmp(&b.end))
    });
}

fn check_template(template: &str) -> String {
    template.to_ascii_uppercase().replace('U', "T")
}

/// Bis zu `max_results` beste Primerpaare, deren Produkt das Ziel `[target_start, target_end)` einschließt.
/// Ergebnis nach Strafe aufsteigend (bei Gleichstand nach Position). Leere Liste, wenn nichts passt.
pub fn pick_pairs(
    template: &str,
    target_start: usize,
    target_end: usize,
    c: &PrimerConstraints,
    max_results: usize,
) -> Result<Vec<PickedPair>> {
    let t = check_template(template);
    if target_start >= target_end || target_end > t.len() {
        return Err(Error::InvalidInput(format!(
            "Ungültiger Zielbereich {}-{} bei Länge {}",
            target_start,
            target_end,
            t.len()
        )));
    }
    if c.min_length == 0 || c.min_length > c.max_length {
        return Err(Error::InvalidInput("Ungültige Primerlängen".to_string()));
    }
    let target_len = target_end - target_start;
    let slack = c.max_product.saturating_sub(target_len);
    let f_lo = target_start.saturating_sub(slack);
    let r_hi = (target_end + slack).min(t.len());

    let tlen = t.len();
    let fwd_starts: Vec<usize> = (f_lo..=target_start).collect();
    let mut fwd: Vec<PickedPrimer> = fwd_starts
        .par_iter()
        .flat_map_iter(|&s| {
            (c.min_length..=c.max_length)
                .filter(move |l| s + l <= tlen)
                .filter_map(|l| make_candidate(&t, s, s + l, true, c))
                .collect::<Vec<_>>()
        })
        .collect();
    let rev_ends: Vec<usize> = (target_end..=r_hi).collect();
    let mut rev: Vec<PickedPrimer> = rev_ends
        .par_iter()
        .flat_map_iter(|&e| {
            (c.min_length..=c.max_length)
                .filter(move |l| *l <= e)
                .filter_map(|l| make_candidate(&t, e - l, e, false, c))
                .collect::<Vec<_>>()
        })
        .collect();
    sort_candidates(&mut fwd);
    sort_candidates(&mut rev);
    fwd.truncate(300);
    rev.truncate(300);

    let mut pairs = Vec::new();
    for f in &fwd {
        for r in &rev {
            if r.end <= f.start {
                continue;
            }
            let product = r.end - f.start;
            if product < c.min_product || product > c.max_product {
                continue;
            }
            let diff = (f.tm - r.tm).abs();
            if diff > c.max_tm_difference {
                continue;
            }
            if dimer_score(&f.sequence, &r.sequence) > c.max_dimer {
                continue;
            }
            pairs.push(PickedPair {
                penalty: f.penalty + r.penalty + diff,
                product_size: product,
                forward: f.clone(),
                reverse: r.clone(),
            });
        }
    }
    pairs.sort_by(|a, b| {
        a.penalty
            .partial_cmp(&b.penalty)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.forward.start.cmp(&b.forward.start))
            .then(a.reverse.end.cmp(&b.reverse.end))
    });
    pairs.truncate(max_results);
    Ok(pairs)
}

/// Bester Vorwärtsprimer, der bei `start` beginnt (beliebige zulässige Länge).
pub fn pick_forward_at(
    template: &str,
    start: usize,
    c: &PrimerConstraints,
) -> Option<PickedPrimer> {
    let t = check_template(template);
    let mut v: Vec<PickedPrimer> = (c.min_length..=c.max_length)
        .filter(|l| start + l <= t.len())
        .filter_map(|l| make_candidate(&t, start, start + l, true, c))
        .collect();
    sort_candidates(&mut v);
    v.into_iter().next()
}

/// Bester Rückwärtsprimer, dessen Bereich auf der Vorlage bei `end_exclusive` endet.
pub fn pick_reverse_ending_at(
    template: &str,
    end_exclusive: usize,
    c: &PrimerConstraints,
) -> Option<PickedPrimer> {
    let t = check_template(template);
    if end_exclusive > t.len() {
        return None;
    }
    let mut v: Vec<PickedPrimer> = (c.min_length..=c.max_length)
        .filter(|l| *l <= end_exclusive)
        .filter_map(|l| make_candidate(&t, end_exclusive - l, end_exclusive, false, c))
        .collect();
    sort_candidates(&mut v);
    v.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Rng;

    fn zufall(n: usize, seed: u64) -> String {
        let mut r = Rng::new(seed);
        (0..n).map(|_| ['A', 'C', 'G', 'T'][r.below(4)]).collect()
    }

    #[test]
    fn paare_erfuellen_alle_vorgaben() {
        let t = zufall(400, 42);
        let c = PrimerConstraints::standard();
        let pairs = pick_pairs(&t, 150, 250, &c, 5).unwrap();
        assert!(!pairs.is_empty());
        for p in &pairs {
            for pr in [&p.forward, &p.reverse] {
                assert!(pr.sequence.len() >= c.min_length && pr.sequence.len() <= c.max_length);
                assert!(pr.tm >= c.min_tm && pr.tm <= c.max_tm);
                assert!(pr.gc >= c.min_gc && pr.gc <= c.max_gc);
                assert!(has_gc_clamp(&pr.sequence));
            }
            assert!((p.forward.tm - p.reverse.tm).abs() <= c.max_tm_difference);
            assert!(p.product_size >= c.min_product && p.product_size <= c.max_product);
            // Ziel liegt im Produkt
            assert!(p.forward.start <= 150 && p.reverse.end >= 250);
            // Sequenzen stimmen mit der Vorlage überein
            assert_eq!(p.forward.sequence, t[p.forward.start..p.forward.end]);
            assert_eq!(
                p.reverse.sequence,
                iupac_reverse_complement(&t[p.reverse.start..p.reverse.end])
            );
            assert_eq!(p.product_size, p.reverse.end - p.forward.start);
        }
        // sortiert nach Strafe
        assert!(pairs.windows(2).all(|w| w[0].penalty <= w[1].penalty));
    }

    #[test]
    fn ergebnis_ist_deterministisch() {
        let t = zufall(400, 42);
        let c = PrimerConstraints::standard();
        assert_eq!(
            pick_pairs(&t, 150, 250, &c, 3).unwrap(),
            pick_pairs(&t, 150, 250, &c, 3).unwrap()
        );
    }

    #[test]
    fn primer_an_fester_position() {
        let t = zufall(400, 42);
        let c = PrimerConstraints::standard();
        // erste Startposition ab 100, an der ein gültiger Primer beginnt
        let (start, p) = (100..300)
            .find_map(|s| pick_forward_at(&t, s, &c).map(|p| (s, p)))
            .expect("irgendein Primer");
        assert_eq!(p.start, start);
        assert!(p.forward);
        assert_eq!(p.sequence, t[start..p.end]);
        let (end, r) = (150..390)
            .find_map(|e| pick_reverse_ending_at(&t, e, &c).map(|p| (e, p)))
            .expect("irgendein Primer");
        assert_eq!(r.end, end);
        assert!(!r.forward);
        assert_eq!(r.sequence, iupac_reverse_complement(&t[r.start..end]));
    }

    #[test]
    fn gc_reiche_vorlage_hat_kein_paar() {
        let t = "GC".repeat(200);
        let pairs = pick_pairs(&t, 150, 250, &PrimerConstraints::standard(), 5).unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn zu_kurze_vorlage_und_fehler() {
        let c = PrimerConstraints::standard();
        assert!(pick_pairs("ACGT", 1, 3, &c, 5).unwrap().is_empty());
        assert!(pick_pairs("ACGT", 3, 1, &c, 5).is_err());
        assert!(pick_pairs("ACGT", 0, 99, &c, 5).is_err());
        let bad = PrimerConstraints {
            min_length: 30,
            max_length: 20,
            ..c
        };
        assert!(pick_pairs("ACGTACGTACGT", 2, 8, &bad, 5).is_err());
        assert!(pick_forward_at("ACGT", 0, &PrimerConstraints::standard()).is_none());
        assert!(pick_reverse_ending_at("ACGT", 99, &PrimerConstraints::standard()).is_none());
    }

    #[test]
    fn vorlage_mit_unbekannten_basen_wird_uebersprungen() {
        let mut t = zufall(400, 42);
        t.replace_range(100..110, "NNNNNNNNNN");
        let c = PrimerConstraints::standard();
        let pairs = pick_pairs(&t, 150, 250, &c, 5).unwrap();
        for p in pairs {
            assert!(!p.forward.sequence.contains('N') && !p.reverse.sequence.contains('N'));
        }
    }

    #[test]
    fn kleinbuchstaben_und_u_werden_akzeptiert() {
        let t = zufall(400, 42);
        let c = PrimerConstraints::standard();
        let a = pick_pairs(&t, 150, 250, &c, 1).unwrap();
        let b = pick_pairs(&t.to_lowercase(), 150, 250, &c, 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn suche_ist_schnell() {
        let t = zufall(3000, 7);
        let start = std::time::Instant::now();
        let _ = pick_pairs(&t, 1000, 1500, &PrimerConstraints::standard(), 5).unwrap();
        assert!(start.elapsed().as_secs() < 20, "{:?}", start.elapsed());
    }
}
