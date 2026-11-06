//! Codon-Nutzung: Zählung, RSCU, GC3 und Codon Adaptation Index (CAI).

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::sequence::amino_acid_for;

/// Anzahl je Codon (Großbuchstaben, `T` statt `U`), aufsteigend nach Codon sortiert.
pub type CodonCounts = BTreeMap<String, usize>;

/// Zählt die Codons im Leseraster ab Position 0 in Dreierschritten.
/// Schreibweise egal, `U` zählt wie `T`. Codons mit anderen Zeichen als A, C, G, T werden übersprungen
/// (das Raster läuft trotzdem weiter), ein unvollständiges Schluss-Codon entfällt. Stopcodons werden mitgezählt.
pub fn count_codons(bases: &str) -> CodonCounts {
    let mut counts = CodonCounts::new();
    for codon in valid_codons(bases) {
        *counts.entry(codon).or_insert(0) += 1;
    }
    counts
}

/// Alle vollständigen Codons aus A, C, G, T im Raster ab Position 0.
fn valid_codons(bases: &str) -> Vec<String> {
    let text = bases.to_ascii_uppercase().replace('U', "T");
    text.as_bytes()
        .chunks(3)
        .filter(|c| c.len() == 3 && c.iter().all(|b| matches!(b, b'A' | b'C' | b'G' | b'T')))
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect()
}

/// Alle 64 Codons des Standardcodes nach Aminosäure (Stopp = `*`) gruppiert, Codons alphabetisch.
pub fn codon_families() -> BTreeMap<char, Vec<String>> {
    let letters = ['A', 'C', 'G', 'T'];
    let mut map: BTreeMap<char, Vec<String>> = BTreeMap::new();
    for a in letters {
        for b in letters {
            for c in letters {
                let codon: String = [a, b, c].iter().collect();
                let aa = amino_acid_for(codon.as_bytes()) as char;
                map.entry(aa).or_default().push(codon);
            }
        }
    }
    map
}

/// Relative Synonymous Codon Usage: `anzahl / mittlere Anzahl der synonymen Codons`.
/// Synonym heißt: gleiche Aminosäure laut `amino_acid_for` (Stopcodons bilden die Familie `*`).
/// Die Familie hat alle ihre Codons des Standardcodes (nicht nur die in `counts`); fehlende zählen 0.
/// Familien, deren Summe 0 ist, fehlen im Ergebnis ganz. Ein Codon mit Anzahl 0 in einer Familie mit Summe > 0
/// erscheint mit Wert 0.0. Familien mit nur einem Codon (M, W) haben den Wert 1.0.
pub fn rscu(counts: &CodonCounts) -> BTreeMap<String, f64> {
    let mut result = BTreeMap::new();
    for codons in codon_families().values() {
        let total: usize = codons
            .iter()
            .map(|c| counts.get(c).copied().unwrap_or(0))
            .sum();
        if total == 0 {
            continue;
        }
        let mean = total as f64 / codons.len() as f64;
        for c in codons {
            let n = counts.get(c).copied().unwrap_or(0) as f64;
            result.insert(c.clone(), n / mean);
        }
    }
    result
}

/// Anteil von G oder C an der dritten Position der vollständigen, gültigen Codons in Prozent (0 bis 100).
/// Gültig wie bei `count_codons`. Ohne Codons: 0.0.
pub fn gc3_percent(bases: &str) -> f64 {
    let codons = valid_codons(bases);
    if codons.is_empty() {
        return 0.0;
    }
    let gc = codons.iter().filter(|c| c.ends_with(['G', 'C'])).count();
    gc as f64 * 100.0 / codons.len() as f64
}

/// Codon Adaptation Index nach Sharp & Li (1987): geometrisches Mittel der Gewichte `w` aller Codons der Sequenz.
/// - `w = max(anzahl, 0.5) / größte Anzahl in der Familie` mit den Anzahlen aus `reference`.
/// - Nicht berücksichtigt werden: ATG, TGG, Stopcodons und Codons, deren Familie in `reference` Summe 0 hat.
/// - Berechnung über die Summe der Logarithmen (`ln`), am Ende `exp(summe / n)`.
///
/// Fehler `Error::InvalidInput`, wenn kein Codon berücksichtigt wurde.
pub fn codon_adaptation_index(bases: &str, reference: &CodonCounts) -> Result<f64> {
    let mut max_per_codon: BTreeMap<String, usize> = BTreeMap::new();
    for codons in codon_families().values() {
        let max = codons
            .iter()
            .map(|c| reference.get(c).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        for c in codons {
            max_per_codon.insert(c.clone(), max);
        }
    }
    let mut log_sum = 0.0;
    let mut used = 0usize;
    for codon in valid_codons(bases) {
        if codon == "ATG" || codon == "TGG" || amino_acid_for(codon.as_bytes()) == b'*' {
            continue;
        }
        let max = max_per_codon.get(&codon).copied().unwrap_or(0);
        if max == 0 {
            continue;
        }
        let count = reference.get(&codon).copied().unwrap_or(0) as f64;
        log_sum += (count.max(0.5) / max as f64).ln();
        used += 1;
    }
    if used == 0 {
        return Err(Error::InvalidInput(
            "Keine Codons für den CAI verwendbar".to_string(),
        ));
    }
    Ok((log_sum / used as f64).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn counts(pairs: &[(&str, usize)]) -> CodonCounts {
        pairs.iter().map(|(c, n)| (c.to_string(), *n)).collect()
    }

    #[test]
    fn codons_zaehlen() {
        let c = count_codons("ATGGCTGCCGCTTAA");
        assert_eq!(c, counts(&[("ATG", 1), ("GCT", 2), ("GCC", 1), ("TAA", 1)]));
    }

    #[test]
    fn zaehlen_sonderfaelle() {
        // klein, U, unvollständiger Rest
        assert_eq!(count_codons("augaaaGC"), counts(&[("ATG", 1), ("AAA", 1)]));
        // ungültiges Codon in der Mitte wird übersprungen, Raster bleibt
        assert_eq!(count_codons("ATGNNNAAA"), counts(&[("ATG", 1), ("AAA", 1)]));
        assert!(count_codons("").is_empty());
        assert!(count_codons("AT").is_empty());
    }

    #[test]
    fn rscu_werte() {
        let r = rscu(&counts(&[("GCT", 3), ("GCC", 1), ("ATG", 5)]));
        assert!(close(r["GCT"], 3.0));
        assert!(close(r["GCC"], 1.0));
        assert!(close(r["GCA"], 0.0));
        assert!(close(r["GCG"], 0.0));
        assert!(close(r["ATG"], 1.0));
        assert!(!r.contains_key("AAA"));
        assert!(!r.contains_key("TGG"));
    }

    #[test]
    fn rscu_zwei_codon_familie_und_stopp() {
        let r = rscu(&counts(&[("AAA", 3), ("AAG", 1), ("TAA", 2)]));
        assert!(close(r["AAA"], 1.5));
        assert!(close(r["AAG"], 0.5));
        // Familie * hat 3 Codons, Summe 2 → Mittel 2/3
        assert!(close(r["TAA"], 3.0));
        assert!(close(r["TAG"], 0.0));
        assert!(close(r["TGA"], 0.0));
        assert!(rscu(&CodonCounts::new()).is_empty());
    }

    #[test]
    fn gc3() {
        assert!(close(gc3_percent("ATGGCC"), 100.0));
        assert!(close(gc3_percent("ATTGCA"), 0.0));
        assert!(close(gc3_percent("ATGAAA"), 50.0));
        assert!(close(gc3_percent(""), 0.0));
        assert!(close(gc3_percent("ATGNNNGCC"), 100.0));
    }

    #[test]
    fn cai_werte() {
        let r = counts(&[("GCT", 3), ("GCC", 1)]);
        assert!(close(codon_adaptation_index("GCTGCT", &r).unwrap(), 1.0));
        assert!(close(
            codon_adaptation_index("GCCGCC", &r).unwrap(),
            1.0 / 3.0
        ));
        assert!(close(
            codon_adaptation_index("GCTGCC", &r).unwrap(),
            (1.0f64 / 3.0).sqrt()
        ));
        // Codon mit Anzahl 0 bekommt 0.5 / 3
        assert!(close(codon_adaptation_index("GCA", &r).unwrap(), 0.5 / 3.0));
    }

    #[test]
    fn cai_ausnahmen() {
        let r = counts(&[("GCT", 3), ("GCC", 1)]);
        // ATG, TGG und Stopp zählen nicht
        assert!(close(
            codon_adaptation_index("ATGTGGGCTTAA", &r).unwrap(),
            1.0
        ));
        // Familie K fehlt in der Referenz → AAA wird übersprungen
        assert!(close(codon_adaptation_index("AAAGCT", &r).unwrap(), 1.0));
        assert!(matches!(
            codon_adaptation_index("ATGTGGTAA", &r),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            codon_adaptation_index("AAA", &r),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            codon_adaptation_index("", &r),
            Err(Error::InvalidInput(_))
        ));
    }
}
