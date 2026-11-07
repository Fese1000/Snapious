//! Codon optimization: convert a protein or coding DNA to DNA using a codon frequency table.

use std::collections::BTreeMap;

use crate::bio::codon_usage::{codon_families, CodonCounts};
use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::model::sequence::amino_acid_for;

/// Relative frequency of each codon within its amino acid.
#[derive(Clone, Debug, PartialEq)]
pub struct CodonTable {
    freq: BTreeMap<String, f64>,
}

impl CodonTable {
    /// From pairs `(codon, frequency)`. Error `Error::InvalidInput` if a codon does not consist of exactly three characters
    /// A, C, G, T (uppercase) or a frequency is negative or not a number.
    /// Codons not listed have frequency 0.
    pub fn from_frequencies(pairs: &[(&str, f64)]) -> Result<CodonTable> {
        let mut freq = BTreeMap::new();
        for (codon, f) in pairs {
            let c = codon.to_ascii_uppercase().replace('U', "T");
            if c.len() != 3 || !c.chars().all(|b| matches!(b, 'A' | 'C' | 'G' | 'T')) {
                return Err(Error::InvalidInput(format!("Invalid codon: {}", codon)));
            }
            if f.is_nan() || *f < 0.0 {
                return Err(Error::InvalidInput(format!(
                    "Invalid frequency for {}: {}",
                    codon, f
                )));
            }
            freq.insert(c, *f);
        }
        Ok(CodonTable { freq })
    }

    /// From counts: `count / sum of the counts of the family` (sum 0 → 0.0). Counts of codons
    /// that are not valid codons are ignored.
    pub fn from_counts(counts: &CodonCounts) -> CodonTable {
        let mut freq = BTreeMap::new();
        for codons in codon_families().values() {
            let family_sum: usize = codons
                .iter()
                .map(|c| counts.get(c).copied().unwrap_or(0))
                .sum();
            for c in codons {
                let n = counts.get(c).copied().unwrap_or(0);
                freq.insert(
                    c.clone(),
                    if family_sum == 0 {
                        0.0
                    } else {
                        n as f64 / family_sum as f64
                    },
                );
            }
        }
        CodonTable { freq }
    }

    /// Frequency of the codon (uppercase, `U` as `T`); unknown → 0.0.
    pub fn frequency(&self, codon: &str) -> f64 {
        self.freq
            .get(codon.to_ascii_uppercase().replace('U', "T").as_str())
            .copied()
            .unwrap_or(0.0)
    }

    /// All codons of the amino acid (one-letter code, stop = `*`, uppercase), by descending frequency,
    /// alphabetical on ties. Unknown amino acid → empty list. Uses `codon_families`.
    pub fn codons_for(&self, amino_acid: char) -> Vec<String> {
        let mut codons = codon_families()
            .get(&amino_acid)
            .cloned()
            .unwrap_or_default();
        codons.sort_by(|a, b| {
            self.frequency(b)
                .partial_cmp(&self.frequency(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        codons
    }
}

/// How the codon is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /// Always the most frequent codon (`codons_for(aa)[0]`).
    MostFrequent,
    /// Random according to the frequency distribution (deterministic from `seed`).
    Harmonized,
}

/// Settings.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationOptions {
    pub strategy: Strategy,
    /// Forbidden recognition sites (only A, C, G, T; uppercase); their reverse complement is forbidden too.
    pub avoid_sites: Vec<String>,
    /// Longest allowed run of identical bases.
    pub max_homopolymer: usize,
    /// Allowed GC fraction (0 to 1) of the whole sequence.
    pub min_gc: f64,
    pub max_gc: f64,
    pub seed: u64,
}

impl OptimizationOptions {
    /// `MostFrequent`, no forbidden sites, `max_homopolymer 6`, `min_gc 0.35`, `max_gc 0.65`, `seed 1`.
    pub fn standard() -> OptimizationOptions {
        OptimizationOptions {
            strategy: Strategy::MostFrequent,
            avoid_sites: Vec::new(),
            max_homopolymer: 6,
            min_gc: 0.35,
            max_gc: 0.65,
            seed: 1,
        }
    }
}

/// The random number generator (SplitMix64), so that results are reproducible:
/// `state = state.wrapping_add(0x9E3779B97F4A7C15); z = state; z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
/// z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB); z ^ (z >> 31)`. A random number in `[0, 1)` is
/// `(z >> 11) as f64 / 9007199254740992.0`. The initial state is `seed`.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15u64);
        let mut z = self.state;
        z ^= z >> 30;
        z = z.wrapping_mul(0xBF58476D1CE4E5B9u64);
        z ^= z >> 27;
        z = z.wrapping_mul(0x94D049BB133111EBu64);
        z ^ (z >> 31)
    }

    pub fn next_f64(&mut self) -> f64 {
        let z = self.next_u64();
        (z >> 11) as f64 / 9007199254740992.0
    }
}

/// Converts a protein sequence to DNA (stop `*` is allowed, lowercase too).
///
/// 1. Codon per amino acid according to `options.strategy` (`Harmonized`: random number `r = next_f64() · sum of the frequencies` of the
///    family; the first codon in the order of `codons_for` whose running sum is `> r` is chosen;
///    if the sum is 0, the first codon). One random number per amino acid, in sequence order.
/// 2. Corrections in this fixed order, each with at most 10000 passes:
///    a) Forbidden sites (also reverse complement): while one occurs (the first match in the DNA, smallest position),
///    the codons touching the match are considered, left to right; for each of them the alternatives
///    (the remaining codons of the amino acid in the order of `codons_for`). The first replacement that reduces the
///    total number of forbidden occurrences is accepted. If there is none, the loop ends.
///    b) Homopolymers: while a run of identical bases longer than `max_homopolymer` exists (the first, smallest start),
///    the codons touching the run are considered, starting with the middle one (index `(first + last) / 2`),
///    then alternating left and right. The first replacement is accepted that reduces the metric
///    `sum over all runs of (length − max_homopolymer), only runs longer than max_homopolymer` and
///    does not increase the number of forbidden occurrences. If there is none, the loop ends. `max_homopolymer == 0` is treated as 1.
///    c) GC fraction (fraction of G and C among all bases) below `min_gc` or above `max_gc`: among all replacements (codon index from
///    left to right, alternatives in the order of `codons_for`) that change the number of G/C in the desired direction
///    (more G/C if GC is too low, fewer if too high), the one with the smallest frequency loss is chosen,
///    i.e. the smallest `frequency(current) − frequency(alt)`, on ties the first in the stated order. It is taken only if it increases neither the number of forbidden occurrences nor the
///    homopolymer metric. If there is none, the loop ends.
/// 3. The result always encodes the same protein (uppercase, T instead of U).
///
/// Error `Error::InvalidInput` for characters that are not amino acids (20 standard amino acids or `*`). Empty input → "".
pub fn optimize(
    table: &CodonTable,
    protein: &str,
    options: &OptimizationOptions,
) -> Result<String> {
    let protein = protein.to_ascii_uppercase().replace('U', "");
    if protein.is_empty() {
        return Ok(String::new());
    }

    let valid_aa = b"ACDEFGHIKLMNPQRSTVWY*";
    for c in protein.bytes() {
        if !valid_aa.contains(&c) {
            return Err(Error::InvalidInput(format!(
                "Invalid character in protein: {}",
                c as char
            )));
        }
    }

    let mut dna = String::with_capacity(protein.len() * 3);
    let max_hp = if options.max_homopolymer == 0 {
        1
    } else {
        options.max_homopolymer
    };
    let mut rng = SplitMix64::new(options.seed);

    // Step 1: initial translation
    for aa_byte in protein.bytes() {
        let codons = table.codons_for(aa_byte as char);
        if options.strategy == Strategy::MostFrequent {
            dna.push_str(codons.first().unwrap_or(&String::from("???")));
        } else {
            let freqs: Vec<f64> = codons.iter().map(|c| table.frequency(c)).collect();
            let total: f64 = freqs.iter().sum();
            if total == 0.0 {
                dna.push_str(codons.first().unwrap_or(&String::from("???")));
            } else {
                let r = rng.next_f64() * total;
                let mut acc = 0.0;
                for (j, c) in codons.iter().enumerate() {
                    acc += freqs[j];
                    if acc > r {
                        dna.push_str(c);
                        break;
                    }
                }
            }
        }
    }

    // Step 2a: remove forbidden sites
    for _ in 0..10000 {
        match find_first_forbidden(&dna, &options.avoid_sites) {
            None => break,
            Some((pos, site_len)) => {
                let start_codon = (pos.saturating_sub(2)) / 3;
                let end_codon = (pos + site_len).saturating_sub(1) / 3;
                let mut replaced = false;

                for ci in start_codon..=end_codon {
                    if ci * 3 >= dna.len() {
                        break;
                    }
                    let current_codon = &dna[ci * 3..ci * 3 + 3];
                    let aa = protein.as_bytes()[ci] as char;
                    let alternatives = table.codons_for(aa);

                    for alt in &alternatives {
                        if alt == current_codon {
                            continue;
                        }
                        let mut new_dna = dna.clone();
                        new_dna.replace_range(ci * 3..ci * 3 + 3, alt);
                        if count_forbidden(&new_dna, &options.avoid_sites)
                            < count_forbidden(&dna, &options.avoid_sites)
                        {
                            dna = new_dna;
                            replaced = true;
                            break;
                        }
                    }
                    if replaced {
                        break;
                    }
                }
                if !replaced {
                    break;
                }
            }
        }
    }

    // Step 2b: shorten homopolymers
    for _ in 0..10000 {
        match find_first_long_run(&dna, max_hp) {
            None => break,
            Some((start, len)) => {
                let start_codon = (start.saturating_sub(2)) / 3;
                let end_codon = (start + len).saturating_sub(1) / 3;
                let mut codon_indices: Vec<usize> = (start_codon..=end_codon).collect();
                let mid = (start_codon + end_codon) / 2;
                codon_indices.sort_by_key(|&i| (i as i64 - mid as i64).abs());

                let current_metric = homopolymer_metric(&dna, max_hp);
                let current_forbidden = count_forbidden(&dna, &options.avoid_sites);
                let mut replaced = false;

                for &ci in &codon_indices {
                    if ci * 3 >= dna.len() {
                        break;
                    }
                    let current_codon = &dna[ci * 3..ci * 3 + 3];
                    let aa = protein.as_bytes()[ci] as char;
                    let alternatives = table.codons_for(aa);

                    for alt in &alternatives {
                        if alt == current_codon {
                            continue;
                        }
                        let mut new_dna = dna.clone();
                        new_dna.replace_range(ci * 3..ci * 3 + 3, alt);
                        if homopolymer_metric(&new_dna, max_hp) < current_metric
                            && count_forbidden(&new_dna, &options.avoid_sites) <= current_forbidden
                        {
                            dna = new_dna;
                            replaced = true;
                            break;
                        }
                    }
                    if replaced {
                        break;
                    }
                }
                if !replaced {
                    break;
                }
            }
        }
    }

    // Step 2c: adjust GC fraction
    for _ in 0..10000 {
        let current_gc = gc(&dna);
        if current_gc >= options.min_gc && current_gc <= options.max_gc {
            break;
        }

        let want_more_gc = current_gc < options.min_gc;
        let mut best_ci: Option<usize> = None;
        let mut best_alt: Option<String> = None;
        let mut best_loss = f64::MAX;

        let current_forbidden = count_forbidden(&dna, &options.avoid_sites);
        let current_metric = homopolymer_metric(&dna, max_hp);

        for ci in 0..(dna.len() / 3) {
            let current_codon = &dna[ci * 3..ci * 3 + 3];
            let aa = protein.as_bytes()[ci] as char;
            let alternatives = table.codons_for(aa);

            for alt in &alternatives {
                if alt == current_codon {
                    continue;
                }
                let gc_diff = count_gcs(alt) as isize - count_gcs(current_codon) as isize;
                if (want_more_gc && gc_diff <= 0) || (!want_more_gc && gc_diff >= 0) {
                    continue;
                }
                let loss = table.frequency(current_codon) - table.frequency(alt);
                if loss < best_loss {
                    best_loss = loss;
                    best_ci = Some(ci);
                    best_alt = Some(alt.clone());
                }
            }
        }

        match (best_ci, best_alt) {
            (Some(ci), Some(alt)) => {
                let mut new_dna = dna.clone();
                new_dna.replace_range(ci * 3..ci * 3 + 3, &alt);
                if count_forbidden(&new_dna, &options.avoid_sites) <= current_forbidden
                    && homopolymer_metric(&new_dna, max_hp) <= current_metric
                {
                    dna = new_dna;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }

    Ok(dna)
}

/// Translates `coding_dna` (length divisible by 3, only A, C, G, T/U in any case) with `amino_acid_for`
/// into a protein (stop = `*`) and calls `optimize`. Otherwise `Error::InvalidInput`.
pub fn optimize_dna(
    table: &CodonTable,
    coding_dna: &str,
    options: &OptimizationOptions,
) -> Result<String> {
    let dna = coding_dna.to_ascii_uppercase().replace('U', "T");
    if !dna.chars().all(|c| matches!(c, 'A' | 'C' | 'G' | 'T')) {
        return Err(Error::InvalidInput("Invalid characters in DNA".to_string()));
    }
    if !dna.len().is_multiple_of(3) {
        return Err(Error::InvalidInput(
            "DNA length not divisible by 3".to_string(),
        ));
    }
    let protein_bytes: Vec<u8> = dna.as_bytes().chunks(3).map(amino_acid_for).collect();
    if protein_bytes.contains(&b'X') {
        return Err(Error::InvalidInput(
            "Unknown amino acid in translation".to_string(),
        ));
    }
    let protein: String = protein_bytes.iter().map(|&b| b as char).collect();
    optimize(table, &protein, options)
}

fn count_forbidden(dna: &str, avoid_sites: &[String]) -> usize {
    let mut count = 0;
    for site in avoid_sites {
        let rc = iupac_reverse_complement(site);
        for _ in dna.match_indices(site).map(|(i, _)| i) {
            count += 1;
        }
        for _ in dna.match_indices(&rc).map(|(i, _)| i) {
            count += 1;
        }
    }
    count
}

fn find_first_forbidden(dna: &str, avoid_sites: &[String]) -> Option<(usize, usize)> {
    let mut min_pos = dna.len();
    let mut min_len = 0;
    for site in avoid_sites {
        let rc = iupac_reverse_complement(site);
        for pos in dna.match_indices(site).map(|(i, _)| i) {
            if pos < min_pos {
                min_pos = pos;
                min_len = site.len();
            }
        }
        for pos in dna.match_indices(&rc).map(|(i, _)| i) {
            if pos < min_pos {
                min_pos = pos;
                min_len = rc.len();
            }
        }
    }
    if min_pos < dna.len() {
        Some((min_pos, min_len))
    } else {
        None
    }
}

fn find_first_long_run(dna: &str, max_hp: usize) -> Option<(usize, usize)> {
    let b = dna.as_bytes();
    let mut best_start = dna.len();
    let mut best_len = 0;
    let mut run_start = 0;
    for i in 1..b.len() {
        if b[i] != b[i - 1] {
            let len = i - run_start;
            if len > max_hp && run_start < best_start {
                best_start = run_start;
                best_len = len;
            }
            run_start = i;
        }
    }
    let len = b.len() - run_start;
    if len > max_hp && run_start < best_start {
        Some((run_start, len))
    } else if best_len > 0 {
        Some((best_start, best_len))
    } else {
        None
    }
}

fn homopolymer_metric(dna: &str, max_hp: usize) -> usize {
    let b = dna.as_bytes();
    let mut metric = 0;
    let mut run_start = 0;
    for i in 1..b.len() {
        if b[i] != b[i - 1] {
            let len = i - run_start;
            if len > max_hp {
                metric += len - max_hp;
            }
            run_start = i;
        }
    }
    let len = b.len() - run_start;
    if len > max_hp {
        metric += len - max_hp;
    }
    metric
}

fn count_gcs(s: &str) -> usize {
    s.chars().filter(|c| *c == 'G' || *c == 'C').count()
}

fn gc(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    count_gcs(s) as f64 / s.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic table: per family the alphabetically first codon 0.6, the second 0.4, the rest 0 (single codons 1.0).
    fn table() -> CodonTable {
        let mut pairs: Vec<(String, f64)> = Vec::new();
        for codons in codon_families().values() {
            for (i, c) in codons.iter().enumerate() {
                let f = if codons.len() == 1 {
                    1.0
                } else {
                    match i {
                        0 => 0.6,
                        1 => 0.4,
                        _ => 0.0,
                    }
                };
                pairs.push((c.clone(), f));
            }
        }
        let refs: Vec<(&str, f64)> = pairs.iter().map(|(c, f)| (c.as_str(), *f)).collect();
        CodonTable::from_frequencies(&refs).unwrap()
    }

    /// Default values without GC window, so that the codon choice itself is tested.
    fn no_gc() -> OptimizationOptions {
        let mut o = OptimizationOptions::standard();
        o.min_gc = 0.0;
        o.max_gc = 1.0;
        o
    }

    fn translate(dna: &str) -> String {
        dna.as_bytes()
            .chunks(3)
            .map(|c| amino_acid_for(c) as char)
            .collect()
    }

    fn longest_run(s: &str) -> usize {
        let b = s.as_bytes();
        let mut best = 0;
        let mut run = 0;
        for i in 0..b.len() {
            if i > 0 && b[i] == b[i - 1] {
                run += 1;
            } else {
                run = 1;
            }
            best = best.max(run);
        }
        best
    }

    fn gc(s: &str) -> f64 {
        s.chars().filter(|c| *c == 'G' || *c == 'C').count() as f64 / s.len() as f64
    }

    #[test]
    fn tabelle() {
        let t = table();
        assert!((t.frequency("GCA") - 0.6).abs() < 1e-12);
        assert!((t.frequency("GCC") - 0.4).abs() < 1e-12);
        assert_eq!(t.frequency("GCG"), 0.0);
        assert_eq!(t.frequency("ATG"), 1.0);
        assert_eq!(t.frequency("gca"), 0.6);
        assert_eq!(t.frequency("XYZ"), 0.0);
        assert_eq!(t.codons_for('A'), vec!["GCA", "GCC", "GCG", "GCT"]);
        assert_eq!(t.codons_for('M'), vec!["ATG"]);
        assert_eq!(t.codons_for('*')[0], "TAA");
        assert!(t.codons_for('B').is_empty());
        assert!(CodonTable::from_frequencies(&[("AT", 1.0)]).is_err());
        assert!(CodonTable::from_frequencies(&[("ATG", -1.0)]).is_err());
        assert!(CodonTable::from_frequencies(&[("ATG", f64::NAN)]).is_err());
    }

    #[test]
    fn tabelle_aus_zaehlungen() {
        let mut counts = CodonCounts::new();
        counts.insert("GCT".to_string(), 3);
        counts.insert("GCC".to_string(), 1);
        counts.insert("NNN".to_string(), 9);
        let t = CodonTable::from_counts(&counts);
        assert!((t.frequency("GCT") - 0.75).abs() < 1e-12);
        assert!((t.frequency("GCC") - 0.25).abs() < 1e-12);
        assert_eq!(t.frequency("AAA"), 0.0);
        assert_eq!(t.codons_for('A')[0], "GCT");
    }

    #[test]
    fn zufallsgenerator() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(1);
        assert_eq!(a.next_u64(), b.next_u64());
        assert_eq!(SplitMix64::new(0).next_u64(), 0xE220A8397B1DCDAF);
        let x = a.next_f64();
        assert!((0.0..1.0).contains(&x));
    }

    #[test]
    fn haeufigste_codons() {
        let d = optimize(&table(), "MKL*", &no_gc()).unwrap();
        assert_eq!(d, "ATGAAACTATAA");
        assert_eq!(translate(&d), "MKL*");
        assert_eq!(optimize(&table(), "", &no_gc()).unwrap(), "");
        assert_eq!(optimize(&table(), "mkl", &no_gc()).unwrap(), "ATGAAACTA");
        assert!(matches!(
            optimize(&table(), "MKB", &no_gc()),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            optimize(&table(), "M K", &no_gc()),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn verbotene_stelle() {
        let mut o = no_gc();
        // E (GAA) followed by F (TTC) would give GAATTC
        assert_eq!(optimize(&table(), "EF", &o).unwrap(), "GAATTC");
        o.avoid_sites = vec!["GAATTC".to_string()];
        let d = optimize(&table(), "EF", &o).unwrap();
        assert!(!d.contains("GAATTC"));
        assert_eq!(translate(&d), "EF");
        // non-palindromic site: its reverse complement is forbidden too (A = GCA, S = AGC gives GCAAGC)
        o.avoid_sites = Vec::new();
        assert_eq!(optimize(&table(), "AS", &o).unwrap(), "GCAAGC");
        o.avoid_sites = vec!["GCAAGC".to_string()];
        let d = optimize(&table(), "AS", &o).unwrap();
        assert_eq!(translate(&d), "AS");
        assert!(!d.contains("GCAAGC") && !d.contains(&iupac_reverse_complement("GCAAGC")));
        o.avoid_sites = vec!["GCTTGC".to_string()];
        let d = optimize(&table(), "AS", &o).unwrap();
        assert_eq!(translate(&d), "AS");
        assert!(!d.contains("GCAAGC"));
    }

    #[test]
    fn homopolymer() {
        let mut o = OptimizationOptions::standard();
        o.max_homopolymer = 6;
        let d = optimize(&table(), "KKKKKK", &o).unwrap();
        assert_eq!(translate(&d), "KKKKKK");
        assert!(longest_run(&d) <= 6, "{}", d);
        assert!(!d.contains("AAAAAAA"));
    }

    #[test]
    fn gc_fenster() {
        let o = OptimizationOptions::standard();
        let d = optimize(&table(), "LLLLLL", &o).unwrap();
        assert_eq!(translate(&d), "LLLLLL");
        assert!(gc(&d) >= 0.35, "{}", d);
        assert!(gc(&d) <= 0.65);
    }

    #[test]
    fn harmonisiert_und_deterministisch() {
        let mut o = OptimizationOptions::standard();
        o.strategy = Strategy::Harmonized;
        o.seed = 7;
        let protein = "L".repeat(1000);
        let a = optimize(&table(), &protein, &o).unwrap();
        let b = optimize(&table(), &protein, &o).unwrap();
        assert_eq!(a, b);
        let cta = a.as_bytes().chunks(3).filter(|c| *c == b"CTA").count();
        assert!((520..=680).contains(&cta), "{}", cta);
        assert_eq!(translate(&a), protein);
        o.seed = 8;
        assert_ne!(optimize(&table(), &protein, &o).unwrap(), a);
    }

    #[test]
    fn dna_eingabe() {
        let o = no_gc();
        // GCG GCG (alanine) → GCA GCA
        assert_eq!(optimize_dna(&table(), "gcggcg", &o).unwrap(), "GCAGCA");
        assert_eq!(optimize_dna(&table(), "AUGUAA", &o).unwrap(), "ATGTAA");
        assert!(optimize_dna(&table(), "ATGA", &o).is_err());
        assert!(optimize_dna(&table(), "ATN", &o).is_err());
        assert_eq!(optimize_dna(&table(), "", &o).unwrap(), "");
    }
}
