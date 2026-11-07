//! Cloning primers: binding part at the fragment end plus clamp and recognition site of a restriction enzyme.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::picker::{
    pick_forward_at, pick_reverse_ending_at, PickedPair, PrimerConstraints,
};

/// Primer pair with tails and warnings.
#[derive(Clone, Debug, PartialEq)]
pub struct CloningPrimerResult {
    /// Binding parts (as supplied by primer picking, without tail); `product_size` and `penalty` belong to it.
    pub pair: PickedPair,
    /// Complete forward primer 5'->3': clamp + recognition site + binding part.
    pub forward_full: String,
    /// Complete reverse primer 5'->3': clamp + recognition site + binding part (reverse complement of the template).
    pub reverse_full: String,
    pub warnings: Vec<String>,
}

/// Designs cloning primers for the fragment `template[start..end]`.
///
/// 1. Template, clamp and recognition sites in uppercase with `T` instead of `U`. Each recognition site must be non-empty and consist only of
///    A, C, G, T, as must the clamp (may be empty), otherwise `Error::InvalidInput`. Invalid range
///    (`start >= end` or `end > length`) -> `Error::InvalidInput`.
/// 2. Binding part forward: `pick_forward_at(template, start, c)`; reverse: `pick_reverse_ending_at(template, end, c)`. If
///    one is not found, `Error::InvalidInput("No suitable primer at the fragment end")`.
/// 3. `forward_full = clamp + forward_site + forward.sequence`, `reverse_full = clamp + reverse_site + reverse.sequence`.
/// 4. `pair.product_size = end - start`; `pair.penalty` = sum of the two individual penalties. Forward and reverse primers come unchanged
///    from primer picking.
/// 5. Warnings (in this order): for `forward_site`, then `reverse_site` (only one warning per distinct site): if the site
///    or its reverse complement occurs in the fragment `template[start..end]`:
///    `Erkennungsstelle {stelle} liegt im Fragment`.
pub fn with_restriction_sites(
    template: &str,
    start: usize,
    end: usize,
    c: &PrimerConstraints,
    forward_site: &str,
    reverse_site: &str,
    clamp: &str,
) -> Result<CloningPrimerResult> {
    // 1. Validate the inputs
    if forward_site.is_empty() || !is_valid_dna(forward_site) {
        return Err(Error::InvalidInput("Invalid recognition site".into()));
    }
    if reverse_site.is_empty() || !is_valid_dna(reverse_site) {
        return Err(Error::InvalidInput("Invalid recognition site".into()));
    }
    if !is_valid_dna(clamp) {
        return Err(Error::InvalidInput("Invalid bracket".into()));
    }

    let fragment = template
        .get(start..end)
        .ok_or_else(|| Error::InvalidInput("Invalid range".into()))?;

    // 2. Primer picking
    let forward = pick_forward_at(template, start, c)
        .ok_or_else(|| Error::InvalidInput("No suitable primer at the fragment end".into()))?;
    let reverse = pick_reverse_ending_at(template, end, c)
        .ok_or_else(|| Error::InvalidInput("No suitable primer at the fragment end".into()))?;

    // 3. Assemble complete primers
    let forward_full = format!("{}{}{}", clamp, forward_site, forward.sequence);
    let reverse_full = format!("{}{}{}", clamp, reverse_site, reverse.sequence);

    // 4. Set pair properties
    let pair_penalty = forward.penalty + reverse.penalty;
    let pair = PickedPair {
        forward,
        reverse,
        product_size: end - start,
        penalty: pair_penalty,
    };

    // 5. Generate warnings
    let mut warnings: Vec<String> = Vec::new();

    if fragment.contains(forward_site) || fragment.contains(&iupac_reverse_complement(forward_site))
    {
        warnings.push(format!(
            "Recognition site {} lies within the fragment",
            forward_site
        ));
    }
    if fragment.contains(reverse_site) || fragment.contains(&iupac_reverse_complement(reverse_site))
    {
        warnings.push(format!(
            "Recognition site {} lies within the fragment",
            reverse_site
        ));
    }

    Ok(CloningPrimerResult {
        pair,
        forward_full,
        reverse_full,
        warnings,
    })
}

/// Checks whether a sequence consists only of the letters A, C, G and T.
fn is_valid_dna(s: &str) -> bool {
    s.chars().all(|c| matches!(c, 'A' | 'C' | 'G' | 'T'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rnd(n: usize, seed: u64) -> String {
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

    /// 300 bases without EcoRI (GAATTC) and BamHI (GGATCC), in which 50 at the fragment start and 250 at the fragment end allow valid primers
    /// (the smallest start value for which this holds).
    fn template() -> String {
        let c = PrimerConstraints::standard();
        (1u64..2000)
            .map(|seed| rnd(300, seed))
            .find(|t| {
                !t.contains("GAATTC")
                    && !t.contains("GGATCC")
                    && pick_forward_at(t, 50, &c).is_some()
                    && pick_reverse_ending_at(t, 250, &c).is_some()
            })
            .unwrap()
    }

    #[test]
    fn primer_mit_schwaenzen() {
        let t = template();
        let c = PrimerConstraints::standard();
        let r = with_restriction_sites(&t, 50, 250, &c, "GAATTC", "GGATCC", "GCGC");
        let r = match r {
            Ok(r) => r,
            Err(e) => panic!("{:?}", e),
        };
        assert!(r.forward_full.starts_with("GCGCGAATTC"));
        assert!(r.reverse_full.starts_with("GCGCGGATCC"));
        assert!(r.forward_full.ends_with(&r.pair.forward.sequence));
        assert!(r.reverse_full.ends_with(&r.pair.reverse.sequence));
        assert_eq!(r.forward_full.len(), 10 + r.pair.forward.sequence.len());
        assert_eq!(r.pair.forward.start, 50);
        assert_eq!(r.pair.reverse.end, 250);
        assert_eq!(r.pair.product_size, 200);
        assert_eq!(
            &t[50..50 + r.pair.forward.sequence.len()],
            r.pair.forward.sequence
        );
        assert_eq!(
            r.pair.reverse.sequence,
            iupac_reverse_complement(&t[250 - r.pair.reverse.sequence.len()..250])
        );
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert!((r.pair.penalty - (r.pair.forward.penalty + r.pair.reverse.penalty)).abs() < 1e-9);
    }

    #[test]
    fn warnung_bei_innerer_stelle() {
        let mut t = template();
        t.replace_range(120..126, "GAATTC");
        let c = PrimerConstraints::standard();
        let r = with_restriction_sites(&t, 50, 250, &c, "GAATTC", "GGATCC", "GCGC").unwrap();
        assert_eq!(
            r.warnings,
            vec!["Recognition site GAATTC lies within the fragment".to_string()]
        );
        // Site outside the fragment does not count
        let mut u = template();
        u.replace_range(10..16, "GGATCC");
        let r = with_restriction_sites(&u, 50, 250, &c, "GAATTC", "GGATCC", "").unwrap();
        assert!(r.warnings.is_empty());
        assert!(r.forward_full.starts_with("GAATTC"));
    }

    #[test]
    fn fehler() {
        let t = template();
        let c = PrimerConstraints::standard();
        let e = |r: Result<CloningPrimerResult>| matches!(r, Err(Error::InvalidInput(_)));
        assert!(e(with_restriction_sites(
            &t, 50, 250, &c, "GAANTC", "GGATCC", "GC"
        )));
        assert!(e(with_restriction_sites(
            &t, 50, 250, &c, "", "GGATCC", "GC"
        )));
        assert!(e(with_restriction_sites(
            &t, 50, 250, &c, "GAATTC", "GGATCC", "GN"
        )));
        assert!(e(with_restriction_sites(
            &t, 250, 50, &c, "GAATTC", "GGATCC", "GC"
        )));
        assert!(e(with_restriction_sites(
            &t, 50, 999, &c, "GAATTC", "GGATCC", "GC"
        )));
        // Fragment end without a valid primer (only A)
        let poly = "A".repeat(300);
        assert!(e(with_restriction_sites(
            &poly, 50, 250, &c, "GAATTC", "GGATCC", "GC"
        )));
    }
}
