//! Klonierungsprimer: bindender Teil am Fragmentende plus Klammer und Erkennungsstelle eines Restriktionsenzyms.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::picker::{
    pick_forward_at, pick_reverse_ending_at, PickedPair, PrimerConstraints,
};

/// Primerpaar mit Schwänzen und Hinweisen.
#[derive(Clone, Debug, PartialEq)]
pub struct CloningPrimerResult {
    /// Bindende Teile (wie von der Primerwahl geliefert, ohne Schwanz); `product_size` und `penalty` gehören dazu.
    pub pair: PickedPair,
    /// Vollständiger Vorwärtsprimer 5'→3': Klammer + Erkennungsstelle + bindender Teil.
    pub forward_full: String,
    /// Vollständiger Rückwärtsprimer 5'→3': Klammer + Erkennungsstelle + bindender Teil (Reverse-Komplement der Vorlage).
    pub reverse_full: String,
    pub warnings: Vec<String>,
}

/// Entwirft Klonierungsprimer für das Fragment `template[start..end]`.
///
/// 1. Vorlage, Klammer und Erkennungsstellen in Uppercase mit `T` statt `U`. Jede Erkennungsstelle muss nicht leer sein und nur aus
///    A, C, G, T bestehen, ebenso die Klammer (darf leer sein), sonst `Error::InvalidInput`. Ungültiger Bereich
///    (`start >= end` oder `end > Länge`) → `Error::InvalidInput`.
/// 2. Bindender Teil vorwärts: `pick_forward_at(template, start, c)`; rückwärts: `pick_reverse_ending_at(template, end, c)`. Findet sich
///    einer nicht, `Error::InvalidInput("No suitable primer at the fragment end")`.
/// 3. `forward_full = klammer + forward_site + forward.sequence`, `reverse_full = klammer + reverse_site + reverse.sequence`.
/// 4. `pair.product_size = end − start`; `pair.penalty` = Summe der beiden Einzelstrafen. Vorwärts- und Rückwärtsprimer stammen unverändert
///    aus der Primerwahl.
/// 5. Warnungen (in dieser Reihenfolge): für `forward_site`, dann `reverse_site` (nur eine Warnung je verschiedener Stelle): wenn die Stelle
///    oder ihr Reverse-Komplement im Fragment `template[start..end]` vorkommt:
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
    // 1. Validierung der Eingaben
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

    // 2. Primerwahl
    let forward = pick_forward_at(template, start, c)
        .ok_or_else(|| Error::InvalidInput("No suitable primer at the fragment end".into()))?;
    let reverse = pick_reverse_ending_at(template, end, c)
        .ok_or_else(|| Error::InvalidInput("No suitable primer at the fragment end".into()))?;

    // 3. Vollständige Primer zusammenstellen
    let forward_full = format!("{}{}{}", clamp, forward_site, forward.sequence);
    let reverse_full = format!("{}{}{}", clamp, reverse_site, reverse.sequence);

    // 4. Paar-Eigenschaften setzen
    let pair_penalty = forward.penalty + reverse.penalty;
    let pair = PickedPair {
        forward,
        reverse,
        product_size: end - start,
        penalty: pair_penalty,
    };

    // 5. Warnungen generieren
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

/// Prüft, ob eine Sequenz nur aus den Buchstaben A, C, G und T besteht.
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

    /// 300 Basen ohne EcoRI (GAATTC) und BamHI (GGATCC), bei denen am Fragmentanfang 50 und am Fragmentende 250 zulässige Primer
    /// existieren (der kleinste Startwert, für den das gilt).
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
        // Stelle außerhalb des Fragments zählt nicht
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
        // Fragmentende ohne zulässigen Primer (nur A)
        let poly = "A".repeat(300);
        assert!(e(with_restriction_sites(
            &poly, 50, 250, &c, "GAATTC", "GGATCC", "GC"
        )));
    }
}
