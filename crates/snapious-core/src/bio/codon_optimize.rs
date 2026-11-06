//! Codon-Optimierung: Protein oder codierende DNA mit einer Codon-Häufigkeitstabelle in DNA umsetzen.

use std::collections::BTreeMap;

use crate::bio::codon_usage::{codon_families, CodonCounts};
use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::model::sequence::amino_acid_for;

/// Relative Häufigkeit jedes Codons innerhalb seiner Aminosäure.
#[derive(Clone, Debug, PartialEq)]
pub struct CodonTable {
    freq: BTreeMap<String, f64>,
}

impl CodonTable {
    /// Aus Paaren `(Codon, Häufigkeit)`. Fehler `Error::InvalidInput`, wenn ein Codon nicht aus genau drei Zeichen
    /// A, C, G, T (Großbuchstaben) besteht oder eine Häufigkeit negativ oder keine Zahl ist.
    /// Nicht genannte Codons haben die Häufigkeit 0.
    pub fn from_frequencies(pairs: &[(&str, f64)]) -> Result<CodonTable> {
        let mut freq = BTreeMap::new();
        for (codon, f) in pairs {
            let c = codon.to_ascii_uppercase().replace('U', "T");
            if c.len() != 3 || !c.chars().all(|b| matches!(b, 'A' | 'C' | 'G' | 'T')) {
                return Err(Error::InvalidInput(format!("Ungültiges Codon: {}", codon)));
            }
            if f.is_nan() || *f < 0.0 {
                return Err(Error::InvalidInput(format!(
                    "Ungültige Häufigkeit für {}: {}",
                    codon, f
                )));
            }
            freq.insert(c, *f);
        }
        Ok(CodonTable { freq })
    }

    /// Aus Zählungen: `anzahl / Summe der Anzahlen der Familie` (Summe 0 → 0.0). Zählungen von Codons,
    /// die keine gültigen Codons sind, werden ignoriert.
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

    /// Häufigkeit des Codons (Großbuchstaben, `U` wie `T`); unbekannt → 0.0.
    pub fn frequency(&self, codon: &str) -> f64 {
        self.freq
            .get(codon.to_ascii_uppercase().replace('U', "T").as_str())
            .copied()
            .unwrap_or(0.0)
    }

    /// Alle Codons der Aminosäure (Einbuchstabencode, Stopp = `*`, Großbuchstaben), nach Häufigkeit absteigend,
    /// bei Gleichstand alphabetisch. Unbekannte Aminosäure → leere Liste. Verwende `codon_families`.
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

/// Wie das Codon gewählt wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /// Immer das häufigste Codon (`codons_for(aa)[0]`).
    MostFrequent,
    /// Zufällig nach der Häufigkeitsverteilung (deterministisch aus `seed`).
    Harmonized,
}

/// Einstellungen.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationOptions {
    pub strategy: Strategy,
    /// Verbotene Erkennungsstellen (nur A, C, G, T; Großbuchstaben); auch ihr Reverse-Komplement ist verboten.
    pub avoid_sites: Vec<String>,
    /// Längste erlaubte Folge gleicher Basen.
    pub max_homopolymer: usize,
    /// Erlaubter GC-Anteil (0 bis 1) der ganzen Sequenz.
    pub min_gc: f64,
    pub max_gc: f64,
    pub seed: u64,
}

impl OptimizationOptions {
    /// `MostFrequent`, keine verbotenen Stellen, `max_homopolymer 6`, `min_gc 0.35`, `max_gc 0.65`, `seed 1`.
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

/// Der Zufallsgenerator (SplitMix64), damit Ergebnisse reproduzierbar sind:
/// `state = state.wrapping_add(0x9E3779B97F4A7C15); z = state; z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
/// z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB); z ^ (z >> 31)`. Eine Zufallszahl in `[0, 1)` ist
/// `(z >> 11) as f64 / 9007199254740992.0`. Der Anfangszustand ist `seed`.
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

/// Setzt eine Proteinsequenz in DNA um (Stopp `*` ist erlaubt, Kleinbuchstaben auch).
///
/// 1. Codon je Aminosäure laut `options.strategy` (`Harmonized`: Zufallszahl `r = next_f64() · Summe der Häufigkeiten` der
///    Familie, gewählt wird das erste Codon in der Reihenfolge von `codons_for`, bei dem die laufende Summe `> r` ist;
///    ist die Summe 0, das erste Codon). Eine Zufallszahl pro Aminosäure, in Reihenfolge der Sequenz.
/// 2. Korrekturen in dieser festen Reihenfolge, jeweils mit höchstens 10000 Durchgängen:
///    a) Verbotene Stellen (auch Reverse-Komplement): Solange eine vorkommt (die erste Fundstelle in der DNA, kleinste Position),
///    werden die Codons betrachtet, die die Fundstelle berühren, von links nach rechts; für jedes davon die Alternativen
///    (die übrigen Codons der Aminosäure in der Reihenfolge von `codons_for`). Angenommen wird die erste Ersetzung, die die
///    Gesamtzahl der verbotenen Vorkommen verringert. Gibt es keine, wird die Schleife beendet.
///    b) Homopolymere: Solange ein Lauf gleicher Basen länger als `max_homopolymer` existiert (der erste, kleinste Beginn), werden
///    die Codons betrachtet, die den Lauf berühren, beginnend mit dem mittleren (Index `(erstes + letztes) / 2`),
///    danach abwechselnd nach links und rechts. Angenommen wird die erste Ersetzung, die die Kennzahl
///    `Summe über alle Läufe von (Länge − max_homopolymer), nur Läufe länger als max_homopolymer` verringert und
///    die Zahl verbotener Vorkommen nicht erhöht. Gibt es keine, wird die Schleife beendet. `max_homopolymer == 0` wird wie 1 behandelt.
///    c) GC-Anteil (Anteil von G und C an allen Basen) unter `min_gc` oder über `max_gc`: Unter allen Ersetzungen (Codon-Index von
///    links nach rechts, Alternativen in der Reihenfolge von `codons_for`), die die Zahl der G/C im gewünschten Sinn ändern
///    (bei zu wenig GC mehr G/C, bei zu viel weniger), wird die mit dem kleinsten Häufigkeitsverlust gewählt,
///    also dem kleinsten `frequency(aktuell) − frequency(alt)`, bei Gleichstand die erste in der genannten Reihenfolge. Sie wird nur genommen, wenn sie weder die Zahl verbotener Vorkommen noch die
///    Homopolymer-Kennzahl erhöht. Gibt es keine, wird die Schleife beendet.
/// 3. Das Ergebnis codiert immer dasselbe Protein (Großbuchstaben, T statt U).
///
/// Fehler `Error::InvalidInput` bei Zeichen, die keine Aminosäure (20 Standard-Aminosäuren oder `*`) sind. Leere Eingabe → "".
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
                "Ungültiges Zeichen in Protein: {}",
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

    // Schritt 1: Initiale Translation
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

    // Schritt 2a: Verbotene Stellen entfernen
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

    // Schritt 2b: Homopolymere kürzen
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

    // Schritt 2c: GC-Anteil anpassen
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

/// Übersetzt `coding_dna` (Länge durch 3 teilbar, nur A, C, G, T/U in beliebiger Schreibweise) mit `amino_acid_for`
/// in ein Protein (Stopp = `*`) und ruft `optimize` auf. Sonst `Error::InvalidInput`.
pub fn optimize_dna(
    table: &CodonTable,
    coding_dna: &str,
    options: &OptimizationOptions,
) -> Result<String> {
    let dna = coding_dna.to_ascii_uppercase().replace('U', "T");
    if !dna.chars().all(|c| matches!(c, 'A' | 'C' | 'G' | 'T')) {
        return Err(Error::InvalidInput("Ungültige Zeichen in DNA".to_string()));
    }
    if !dna.len().is_multiple_of(3) {
        return Err(Error::InvalidInput(
            "DNA-Länge nicht durch 3 teilbar".to_string(),
        ));
    }
    let protein_bytes: Vec<u8> = dna.as_bytes().chunks(3).map(amino_acid_for).collect();
    if protein_bytes.contains(&b'X') {
        return Err(Error::InvalidInput(
            "Unbekannte Aminosäure in Übersetzung".to_string(),
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

    /// Synthetische Tabelle: je Familie das alphabetisch erste Codon 0.6, das zweite 0.4, der Rest 0 (Einzelcodons 1.0).
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

    /// Standardwerte ohne GC-Fenster, damit die Codonwahl selbst geprüft wird.
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
        // E (GAA) gefolgt von F (TTC) ergäbe GAATTC
        assert_eq!(optimize(&table(), "EF", &o).unwrap(), "GAATTC");
        o.avoid_sites = vec!["GAATTC".to_string()];
        let d = optimize(&table(), "EF", &o).unwrap();
        assert!(!d.contains("GAATTC"));
        assert_eq!(translate(&d), "EF");
        // nicht palindromische Stelle: auch das Reverse-Komplement ist verboten (A = GCA, S = AGC ergibt GCAAGC)
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
        // GCG GCG (Alanin) → GCA GCA
        assert_eq!(optimize_dna(&table(), "gcggcg", &o).unwrap(), "GCAGCA");
        assert_eq!(optimize_dna(&table(), "AUGUAA", &o).unwrap(), "ATGTAA");
        assert!(optimize_dna(&table(), "ATGA", &o).is_err());
        assert!(optimize_dna(&table(), "ATN", &o).is_err());
        assert_eq!(optimize_dna(&table(), "", &o).unwrap(), "");
    }
}
