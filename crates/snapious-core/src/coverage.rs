//! Coverage-Analyse: Lesetiefe (Depth) je Position, Histogramm, abgedeckte Regionen und Lücken.
//!
//! Positionen sind 0-basiert, Enden exklusiv.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::io::quality::{self, EncodingType};
use crate::io::sam::SamRecord;
use crate::util::Rng;

/// Standard-Schwelle für „abgedeckt“ (mindestens 10×).
pub const DEFAULT_THRESHOLD: u32 = 10;

/// Ein Read mit Position auf der Referenz.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub name: String,
    pub bases: String,
    /// Startposition auf der Referenz (0-basiert).
    pub position: usize,
    pub quality: Option<String>,
    pub encoding: EncodingType,
}

impl Read {
    pub fn new(name: &str, bases: &str, position: usize) -> Self {
        Self {
            name: name.to_string(),
            bases: bases.to_ascii_uppercase(),
            position,
            quality: None,
            encoding: EncodingType::Sanger,
        }
    }

    pub fn with_quality(mut self, quality: &str, encoding: EncodingType) -> Self {
        self.quality = Some(quality.to_string());
        self.encoding = encoding;
        self
    }

    pub fn length(&self) -> usize {
        self.bases.len()
    }

    /// Ende auf der Referenz (exklusiv).
    pub fn end_position(&self) -> usize {
        self.position + self.bases.len()
    }

    /// Alle Basen mindestens `min_q`? Ohne Qualitätsangabe gilt der Read als gültig.
    pub fn has_valid_quality(&self, min_q: i32) -> bool {
        match &self.quality {
            Some(q) if !q.is_empty() => quality::to_phred_scores(q, self.encoding)
                .iter()
                .all(|s| *s >= min_q),
            _ => true,
        }
    }

    /// Mittlere Qualität; 30 ohne Angabe.
    pub fn avg_quality(&self) -> f64 {
        match &self.quality {
            Some(q) if !q.is_empty() => {
                let s = quality::to_phred_scores(q, self.encoding);
                s.iter().sum::<i32>() as f64 / s.len() as f64
            }
            _ => 30.0,
        }
    }
}

/// Referenz (Chromosom, Länge).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub id: String,
    pub length: usize,
}

impl Reference {
    pub fn new(id: &str, length: usize) -> Self {
        Self {
            id: id.to_string(),
            length,
        }
    }
}

/// Zusammenhängender Bereich mit Tiefe über der Schwelle.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageRegion {
    pub seq_id: String,
    pub start: usize,
    pub end: usize,
    pub mean_depth: f64,
    pub percent_covered: f64,
}

impl CoverageRegion {
    pub fn length(&self) -> usize {
        self.end - self.start
    }

    /// BED-Zeile: `Chromosom, Start, Ende, Name, Score (0–1000), Strang`.
    pub fn to_bed(&self) -> String {
        let score = self.mean_depth.round().min(1000.0) as i64;
        format!(
            "{}\t{}\t{}\tCoverage_{:.1}x\t{}\t.",
            self.seq_id, self.start, self.end, self.mean_depth, score
        )
    }

    pub fn format(&self) -> String {
        format!(
            "{}:{}-{} (depth {:.1}x, {:.0}% covered)",
            self.seq_id, self.start, self.end, self.mean_depth, self.percent_covered
        )
    }
}

/// Bereich ohne jede Abdeckung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GapRegion {
    pub seq_id: String,
    pub start: usize,
    pub end: usize,
}

impl GapRegion {
    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

impl std::fmt::Display for GapRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gap {}:{}-{} ({} bp)",
            self.seq_id,
            self.start,
            self.end,
            self.length()
        )
    }
}

/// Ergebnis der Analyse.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageResult {
    pub seq_id: String,
    pub region_length: usize,
    /// Tiefe je Position.
    pub depth: Vec<u32>,
    /// Tiefe → Anzahl Positionen.
    pub histogram: BTreeMap<u32, usize>,
    pub covered_regions: Vec<CoverageRegion>,
    pub gap_regions: Vec<GapRegion>,
    /// Anteil der Positionen mit Tiefe ≥ Schwelle (Prozent).
    pub percent_covered: f64,
    /// Anteil der Positionen mit mindestens einem Read (Prozent).
    pub percent_with_reads: f64,
    pub mean_depth: f64,
    /// Unterer Median.
    pub median_depth: u32,
    pub threshold: u32,
}

impl CoverageResult {
    pub fn coverage_label(&self) -> &'static str {
        if self.mean_depth >= 30.0 {
            "High coverage"
        } else if self.mean_depth >= 10.0 {
            "Medium coverage"
        } else if self.mean_depth >= 5.0 {
            "Low coverage"
        } else {
            "Insufficient coverage"
        }
    }

    /// Textbericht.
    pub fn report(&self) -> String {
        let mut s = format!(
            "=== Coverage analysis: {} ===\n\nReference length: {} bp\nAverage depth: {:.1}x\nMedian depth: {}x\nCoverage rating: {}\nCovered region (≥{}x): {:.0}%\n",
            self.seq_id,
            self.region_length,
            self.mean_depth,
            self.median_depth,
            self.coverage_label(),
            self.threshold,
            self.percent_covered
        );
        s.push_str("\nCoverage-Histogramm:\n");
        for (d, n) in &self.histogram {
            let width = (*n * 50 / self.region_length.max(1)).min(50);
            s.push_str(&format!(
                "  {:>3}x: {} ({} bp, {:.0}%)\n",
                d,
                "█".repeat(width),
                n,
                *n as f64 * 100.0 / self.region_length.max(1) as f64
            ));
        }
        if !self.gap_regions.is_empty() {
            s.push_str(&format!(
                "\nGap regions (0x coverage): {}\n",
                self.gap_regions.len()
            ));
            for g in self.gap_regions.iter().take(10) {
                s.push_str(&format!("  {}\n", g));
            }
            if self.gap_regions.len() > 10 {
                s.push_str(&format!(
                    "  ... and {} more gap regions\n",
                    self.gap_regions.len() - 10
                ));
            }
            let total: usize = self.gap_regions.iter().map(|g| g.length()).sum();
            s.push_str(&format!(
                "\nTotal gap length: {} bp ({:.0}%)\n",
                total,
                total as f64 * 100.0 / self.region_length.max(1) as f64
            ));
        }
        s
    }

    /// BED-Datei der abgedeckten Regionen (mit `track`-Kopfzeile).
    pub fn to_bed(&self) -> String {
        let mut s = format!(
            "track type=coverage name=\"{}\" visibility=all\n",
            self.seq_id
        );
        for r in &self.covered_regions {
            s.push_str(&r.to_bed());
            s.push('\n');
        }
        s
    }

    /// Mittlere Tiefe je Bin (`Chromosom, Start, Ende, Mittelwert`, tabgetrennt).
    pub fn binned_coverage(&self, bin_size: usize) -> Result<String> {
        if bin_size == 0 {
            return Err(Error::InvalidInput(
                "Bin size must be at least 1".to_string(),
            ));
        }
        let mut s = String::new();
        for (i, chunk) in self.depth.chunks(bin_size).enumerate() {
            let mean = chunk.iter().map(|d| *d as f64).sum::<f64>() / chunk.len() as f64;
            s.push_str(&format!(
                "{}\t{}\t{}\t{:.1}\n",
                self.seq_id,
                i * bin_size,
                i * bin_size + chunk.len(),
                mean
            ));
        }
        Ok(s)
    }
}

/// Baut das Ergebnis aus einer fertigen Tiefenliste (z. B. aus SAM/BAM).
pub fn from_depth(seq_id: &str, depth: Vec<u32>, threshold: u32) -> CoverageResult {
    let len = depth.len();
    let mut histogram = BTreeMap::new();
    for d in &depth {
        *histogram.entry(*d).or_insert(0usize) += 1;
    }
    let mut sorted = depth.clone();
    sorted.sort_unstable();
    let median_depth = if sorted.is_empty() {
        0
    } else {
        sorted[(len - 1) / 2]
    };
    let mean_depth = if len == 0 {
        0.0
    } else {
        depth.iter().map(|d| *d as f64).sum::<f64>() / len as f64
    };
    let percent = |n: usize| {
        if len == 0 {
            0.0
        } else {
            n as f64 * 100.0 / len as f64
        }
    };
    let at_threshold = depth.iter().filter(|d| **d >= threshold).count();
    let with_reads = depth.iter().filter(|d| **d > 0).count();

    let mut covered = Vec::new();
    let mut gaps = Vec::new();
    let mut run_start: Option<usize> = None;
    let mut gap_start: Option<usize> = None;
    for (i, d) in depth.iter().enumerate() {
        if *d >= threshold {
            run_start.get_or_insert(i);
        } else if let Some(s) = run_start.take() {
            covered.push(region(seq_id, s, i, &depth[s..i]));
        }
        if *d == 0 {
            gap_start.get_or_insert(i);
        } else if let Some(s) = gap_start.take() {
            gaps.push(GapRegion {
                seq_id: seq_id.to_string(),
                start: s,
                end: i,
            });
        }
    }
    if let Some(s) = run_start {
        covered.push(region(seq_id, s, len, &depth[s..]));
    }
    if let Some(s) = gap_start {
        gaps.push(GapRegion {
            seq_id: seq_id.to_string(),
            start: s,
            end: len,
        });
    }
    CoverageResult {
        seq_id: seq_id.to_string(),
        region_length: len,
        depth,
        histogram,
        covered_regions: covered,
        gap_regions: gaps,
        percent_covered: percent(at_threshold),
        percent_with_reads: percent(with_reads),
        mean_depth,
        median_depth,
        threshold,
    }
}

fn region(seq_id: &str, start: usize, end: usize, depths: &[u32]) -> CoverageRegion {
    CoverageRegion {
        seq_id: seq_id.to_string(),
        start,
        end,
        mean_depth: depths.iter().map(|d| *d as f64).sum::<f64>() / depths.len() as f64,
        percent_covered: 100.0,
    }
}

/// Tiefe aus Reads (jede Base des Reads zählt; Teile hinter der Referenz werden abgeschnitten).
pub fn depth_from_reads(reference: &Reference, reads: &[Read]) -> Vec<u32> {
    // Differenzfeld: O(Reads + Länge) statt O(Reads · Readlänge)
    let mut diff = vec![0i64; reference.length + 1];
    for r in reads {
        if r.position >= reference.length || r.bases.is_empty() {
            continue;
        }
        diff[r.position] += 1;
        diff[r.end_position().min(reference.length)] -= 1;
    }
    let mut depth = Vec::with_capacity(reference.length);
    let mut cur = 0i64;
    for d in diff.iter().take(reference.length) {
        cur += d;
        depth.push(cur as u32);
    }
    depth
}

/// Analysiert die Abdeckung (Schwelle 10×).
pub fn analyze(reference: &Reference, reads: &[Read]) -> CoverageResult {
    analyze_with_threshold(reference, reads, DEFAULT_THRESHOLD)
}

pub fn analyze_with_threshold(
    reference: &Reference,
    reads: &[Read],
    threshold: u32,
) -> CoverageResult {
    from_depth(&reference.id, depth_from_reads(reference, reads), threshold)
}

/// Nur Reads mit durchgehend mindestens Q20 zählen.
pub fn analyze_with_quality_filter(reference: &Reference, reads: &[Read]) -> CoverageResult {
    let valid: Vec<Read> = reads
        .iter()
        .filter(|r| r.has_valid_quality(20))
        .cloned()
        .collect();
    analyze(reference, &valid)
}

/// Zahl der Reads mit durchgehend mindestens `min_q` (nur Reads mit Qualitätsangabe).
pub fn count_reads_above_quality(reads: &[Read], min_q: i32) -> usize {
    reads
        .iter()
        .filter(|r| r.quality.as_ref().map(|q| !q.is_empty()).unwrap_or(false))
        .filter(|r| r.has_valid_quality(min_q))
        .count()
}

/// Tiefe aus SAM-Einträgen (CIGAR-bewusst). `M`, `=` und `X` zählen, `D` nur mit `count_deletions`;
/// `I`, `S`, `H`, `P`, `N` verbrauchen keine Tiefe. Nicht ausgerichtete und sekundäre Einträge,
/// (je nach Parameter) Duplikate und Einträge mit zu geringer MAPQ werden übersprungen.
pub fn depth_from_sam(
    records: &[SamRecord],
    reference_name: &str,
    reference_length: usize,
    skip_duplicates: bool,
    min_mapq: u8,
    count_deletions: bool,
) -> Result<Vec<u32>> {
    let mut diff = vec![0i64; reference_length + 1];
    for r in records {
        if r.reference != reference_name
            || r.is_unmapped()
            || r.is_secondary()
            || (skip_duplicates && r.is_duplicate())
            || r.mapq < min_mapq
        {
            continue;
        }
        let Some(mut pos) = r.position0() else {
            continue;
        };
        for (n, op) in r.cigar_ops()? {
            match op {
                'M' | '=' | 'X' => {
                    add_range(&mut diff, pos, pos + n, reference_length);
                    pos += n;
                }
                'D' => {
                    if count_deletions {
                        add_range(&mut diff, pos, pos + n, reference_length);
                    }
                    pos += n;
                }
                'N' => pos += n,
                _ => {}
            }
        }
    }
    let mut depth = Vec::with_capacity(reference_length);
    let mut cur = 0i64;
    for d in diff.iter().take(reference_length) {
        cur += d;
        depth.push(cur as u32);
    }
    Ok(depth)
}

fn add_range(diff: &mut [i64], start: usize, end: usize, len: usize) {
    if start >= len {
        return;
    }
    diff[start] += 1;
    diff[end.min(len)] -= 1;
}

/// Vergleich zweier Proben als Text.
pub fn compare(reference: &Reference, reads1: &[Read], reads2: &[Read]) -> String {
    let (a, b) = (analyze(reference, reads1), analyze(reference, reads2));
    let diff = (a.mean_depth - b.mean_depth).abs();
    let mut s = format!("=== Coverage comparison ===\n\n{}:\n", reference.id);
    for (i, r) in [&a, &b].iter().enumerate() {
        s.push_str(&format!(
            "  Sample {}: mean {:.1}x, coverage {:.0}%, gaps {}\n",
            i + 1,
            r.mean_depth,
            r.percent_covered,
            r.gap_regions.len()
        ));
    }
    s.push_str(&format!("\nDepth difference: {:.1}x\n", diff));
    s.push_str(if diff < 5.0 {
        "Both samples have similar coverage\n"
    } else {
        "The samples differ significantly\n"
    });
    s
}

/// Zufällige Test-Reads (Länge höchstens 100) mit etwa der gewünschten mittleren Tiefe; mit festem Startwert reproduzierbar.
pub fn generate_synthetic_reads(reference: &Reference, avg_depth: u32, seed: u64) -> Vec<Read> {
    let read_len = reference.length.min(100);
    if read_len == 0 {
        return Vec::new();
    }
    let count = (reference.length as u64 * avg_depth as u64 / read_len as u64) as usize;
    let mut rng = Rng::new(seed);
    (0..count)
        .map(|i| {
            let pos = rng.below(reference.length - read_len + 1);
            let bases: String = (0..read_len)
                .map(|_| ['A', 'C', 'G', 'T'][rng.below(4)])
                .collect();
            Read::new(&format!("read_{:08}", i), &bases, pos)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::sam;

    fn reads(spec: &[(usize, usize)]) -> Vec<Read> {
        spec.iter()
            .enumerate()
            .map(|(i, (pos, len))| Read::new(&format!("r{}", i), &"A".repeat(*len), *pos))
            .collect()
    }

    #[test]
    fn tiefe_und_statistik() {
        let r = Reference::new("chr1", 20);
        // zwei Reads: [0,10) und [5,15)
        let res = analyze_with_threshold(&r, &reads(&[(0, 10), (5, 10)]), 1);
        assert_eq!(
            res.depth,
            vec![1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]
        );
        assert_eq!(res.mean_depth, 1.0);
        assert_eq!(res.median_depth, 1);
        assert_eq!(res.histogram[&0], 5);
        assert_eq!(res.histogram[&1], 10);
        assert_eq!(res.histogram[&2], 5);
        assert_eq!(res.percent_covered, 75.0);
        assert_eq!(res.percent_with_reads, 75.0);
    }

    #[test]
    fn luecken_und_regionen() {
        let r = Reference::new("chr1", 100);
        let res = analyze_with_threshold(&r, &reads(&[(0, 48)]), 1);
        assert_eq!(res.gap_regions.len(), 1);
        assert_eq!(
            (res.gap_regions[0].start, res.gap_regions[0].end),
            (48, 100)
        );
        assert_eq!(res.gap_regions[0].length(), 52);
        assert_eq!(res.covered_regions.len(), 1);
        assert_eq!(
            (res.covered_regions[0].start, res.covered_regions[0].end),
            (0, 48)
        );
        assert_eq!(res.covered_regions[0].mean_depth, 1.0);
        // volle Abdeckung → keine Lücke
        assert!(analyze_with_threshold(&r, &reads(&[(0, 100)]), 1)
            .gap_regions
            .is_empty());
    }

    #[test]
    fn schwelle_trennt_regionen() {
        let r = Reference::new("c", 30);
        // Tiefe 2 auf [0,10), 1 auf [10,20), 2 auf [20,30)
        let res = analyze_with_threshold(
            &r,
            &reads(&[(0, 10), (0, 10), (10, 10), (20, 10), (20, 10)]),
            2,
        );
        assert_eq!(res.covered_regions.len(), 2);
        assert_eq!(res.covered_regions[1].start, 20);
        assert_eq!(res.percent_covered, 100.0 * 20.0 / 30.0);
        assert!(res.gap_regions.is_empty());
    }

    #[test]
    fn reads_ueber_das_ende_werden_gekuerzt() {
        let r = Reference::new("c", 10);
        let res = analyze_with_threshold(&r, &reads(&[(8, 50), (20, 5), (0, 0)]), 1);
        assert_eq!(res.depth, vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    }

    #[test]
    fn leere_referenz_und_keine_reads() {
        let res = analyze(&Reference::new("e", 0), &[]);
        assert_eq!(res.region_length, 0);
        assert_eq!(res.mean_depth, 0.0);
        assert_eq!(res.median_depth, 0);
        assert!(res.covered_regions.is_empty() && res.gap_regions.is_empty());
        let none = analyze(&Reference::new("c", 10), &[]);
        assert_eq!(none.gap_regions.len(), 1);
        assert_eq!(none.coverage_label(), "Insufficient coverage");
    }

    #[test]
    fn bed_und_bins() {
        let r = Reference::new("chr1", 100);
        let res = analyze_with_threshold(&r, &reads(&[(0, 100); 30]), 10);
        assert_eq!(
            res.covered_regions[0].to_bed(),
            "chr1\t0\t100\tCoverage_30.0x\t30\t."
        );
        let bed = res.to_bed();
        assert!(bed.starts_with("track type=coverage name=\"chr1\""));
        assert_eq!(bed.lines().count(), 2);
        let bins = res.binned_coverage(40).unwrap();
        assert_eq!(
            bins,
            "chr1\t0\t40\t30.0\nchr1\t40\t80\t30.0\nchr1\t80\t100\t30.0\n"
        );
        assert!(res.binned_coverage(0).is_err());
    }

    #[test]
    fn qualitaetsfilter() {
        let r = Reference::new("c", 10);
        let gut = Read::new("g", "AAAAA", 0).with_quality("IIIII", EncodingType::Sanger);
        let schlecht = Read::new("s", "AAAAA", 5).with_quality("IIII+", EncodingType::Sanger);
        let ohne = Read::new("o", "AA", 8);
        let all = vec![gut.clone(), schlecht.clone(), ohne.clone()];
        assert!(
            gut.has_valid_quality(20)
                && !schlecht.has_valid_quality(20)
                && ohne.has_valid_quality(20)
        );
        assert_eq!(gut.avg_quality(), 40.0);
        assert_eq!(ohne.avg_quality(), 30.0);
        let res = analyze_with_quality_filter(&r, &all);
        assert_eq!(res.depth, vec![1, 1, 1, 1, 1, 0, 0, 0, 1, 1]);
        assert_eq!(count_reads_above_quality(&all, 20), 1);
        assert_eq!(count_reads_above_quality(&all, 5), 2);
    }

    #[test]
    fn tiefe_aus_sam_cigar_bewusst() {
        let text = "@SQ\tSN:ref\tLN:20\n\
A\t0\tref\t1\t60\t10M\t*\t0\t0\tAAAAAAAAAA\tIIIIIIIIII\n\
B\t0\tref\t6\t60\t5M2D5M\t*\t0\t0\tAAAAAAAAAA\tIIIIIIIIII\n";
        let f = sam::parse(text).unwrap();
        let d = depth_from_sam(&f.records, "ref", 20, false, 0, false).unwrap();
        assert_eq!(
            d,
            vec![1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0]
        );
        let with_del = depth_from_sam(&f.records, "ref", 20, false, 0, true).unwrap();
        assert_eq!(&with_del[10..12], &[1, 1]);
    }

    #[test]
    fn sam_filter() {
        let text = "A\t0\tref\t1\t60\t4M\t*\t0\t0\tAAAA\tIIII\n\
B\t1024\tref\t1\t60\t4M\t*\t0\t0\tAAAA\tIIII\n\
C\t0\tref\t1\t10\t4M\t*\t0\t0\tAAAA\tIIII\n\
D\t4\tref\t1\t60\t4M\t*\t0\t0\tAAAA\tIIII\n\
E\t256\tref\t1\t60\t4M\t*\t0\t0\tAAAA\tIIII\n\
F\t0\tother\t1\t60\t4M\t*\t0\t0\tAAAA\tIIII\n\
G\t0\tref\t1\t60\t2S2M2I\t*\t0\t0\tAAAAAA\tIIIIII\n";
        let f = sam::parse(text).unwrap();
        let alle = depth_from_sam(&f.records, "ref", 6, false, 0, false).unwrap();
        // A, B, C zählen je 4; G zählt 2 (Soft-Clip und Insertion nicht)
        assert_eq!(alle, vec![4, 4, 3, 3, 0, 0]);
        let ohne_dup = depth_from_sam(&f.records, "ref", 6, true, 0, false).unwrap();
        assert_eq!(ohne_dup[0], 3);
        let hohe_mapq = depth_from_sam(&f.records, "ref", 6, false, 20, false).unwrap();
        assert_eq!(hohe_mapq[0], 3);
        assert!(depth_from_sam(
            &[sam::parse("A\t0\tref\t1\t60\t4Q\t*\t0\t0\tAAAA\tIIII\n")
                .unwrap()
                .records[0]
                .clone()],
            "ref",
            4,
            false,
            0,
            false
        )
        .is_err());
    }

    #[test]
    fn synthetische_reads_sind_reproduzierbar_und_haben_passende_tiefe() {
        let r = Reference::new("chr1", 1000);
        let a = generate_synthetic_reads(&r, 20, 42);
        let b = generate_synthetic_reads(&r, 20, 42);
        assert_eq!(a, b);
        assert_eq!(a.len(), 200);
        assert!(a
            .iter()
            .all(|x| x.length() == 100 && x.end_position() <= 1000));
        let res = analyze(&r, &a);
        assert!((res.mean_depth - 20.0).abs() < 1e-9);
        assert!(generate_synthetic_reads(&Reference::new("x", 0), 10, 1).is_empty());
    }

    #[test]
    fn bericht_und_vergleich() {
        let r = Reference::new("chr1", 100);
        let res = analyze_with_threshold(&r, &reads(&[(0, 60)]), 1);
        let rep = res.report();
        assert!(rep.contains("Coverage analysis: chr1"));
        assert!(rep.contains("Gap regions (0x coverage): 1"));
        assert!(rep.contains("Total gap length: 40 bp (40%)"));
        let cmp = compare(&r, &reads(&[(0, 100)]), &reads(&[(0, 100); 10]));
        assert!(cmp.contains("Coverage comparison"));
        assert!(cmp.contains("differ significantly"));
    }

    #[test]
    fn grosse_referenz_ist_schnell() {
        let r = Reference::new("g", 1_000_000);
        let rs = generate_synthetic_reads(&r, 30, 1);
        let t = std::time::Instant::now();
        let res = analyze(&r, &rs);
        assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
        assert!((res.mean_depth - 30.0).abs() < 0.1);
    }
}
