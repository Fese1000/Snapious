//! Qualitätsbericht für FASTQ-Reads (FastQC-artig), ohne die Reads zu speichern.

use std::collections::BTreeMap;

use crate::io::fastq::FastqRecord;

/// Ergebnis der Analyse. Qualitäten sind Phred+33.
#[derive(Clone, Debug, PartialEq)]
pub struct QcReport {
    pub reads: u64,
    pub min_length: usize,
    pub max_length: usize,
    pub mean_length: f64,
    /// Anteil von G und C an allen Basen in Prozent.
    pub gc_percent: f64,
    /// Je Position (0 bis `max_length − 1`) über alle Reads, die diese Position haben: Mittelwert, unteres Quartil, Median, oberes Quartil.
    pub mean_quality_per_position: Vec<f64>,
    pub q25_per_position: Vec<f64>,
    pub median_per_position: Vec<f64>,
    pub q75_per_position: Vec<f64>,
    /// Länge → Zahl der Reads.
    pub length_distribution: BTreeMap<usize, u64>,
    /// `100 · (1 − eindeutige / gesamt)` über die betrachteten Reads.
    pub duplication_percent: f64,
    /// Anteil der Reads (alle Reads) in Prozent, die die ersten 12 Basen des Adapters irgendwo enthalten.
    pub adapter_percent: f64,
    /// Häufige Sequenzen (erste 50 Basen) mit Anzahl: Anteil an den betrachteten Reads über 0,1 %, höchstens 10, absteigend nach Anzahl,
    /// bei Gleichstand aufsteigend nach Sequenz.
    pub overrepresented: Vec<(String, u64)>,
}

/// Wertet die Reads aus.
/// * Quartilsregel („unterer Wert“): für die sortierten Qualitäten `v` einer Position (n Werte) ist das Quantil `q` (0.25, 0.5, 0.75) der
///   Wert mit Index `(n − 1) · q` abgerundet; mit Ganzzahlen `(n − 1) · 25 / 100`, `/ 2`, `(n − 1) · 75 / 100`. Der Median von `[0, 0, 40, 40]` ist also 0.
///   Es sollen nicht alle Werte gespeichert werden: je Position ein Zählfeld für die Phred-Werte 0 bis 93 genügt.
/// * Qualitätszeichen `c` ergibt Phred `c − 33` (auf 0 bis 93 begrenzt).
/// * Duplikation und überrepräsentierte Sequenzen: Schlüssel = die ersten 50 Basen (oder der ganze Read, wenn kürzer) der ersten
///   `max_dup_reads` Reads; `unique` = Zahl der verschiedenen Schlüssel, `total` = Zahl dieser Reads; ohne Reads 0.
/// * Adapter: die ersten 12 Zeichen von `adapter` (oder alle, wenn kürzer); ein Read zählt, wenn seine Basen (Uppercase) dieses Wort
///   enthalten. Ist `adapter` leer, ist der Anteil 0.
/// * Ohne Reads: alles 0, leere Listen und Verteilungen, `min_length = max_length = 0`; keine Division durch 0.
pub fn analyze_reads<I: IntoIterator<Item = FastqRecord>>(
    reads: I,
    adapter: &str,
    max_dup_reads: usize,
) -> QcReport {
    let mut count = 0u64;
    let mut total_len = 0u64;
    let (mut min_len, mut max_len) = (usize::MAX, 0usize);
    let mut gc = 0u64;
    let mut bases = 0u64;
    let mut hist: Vec<[u64; 94]> = Vec::new();
    let mut lengths: BTreeMap<usize, u64> = BTreeMap::new();
    let mut dup_total = 0u64;
    let mut keys: BTreeMap<String, u64> = BTreeMap::new();
    let adapter_word: String = adapter.to_ascii_uppercase().chars().take(12).collect();
    let mut adapter_hits = 0u64;
    for r in reads {
        count += 1;
        let len = r.sequence.len();
        total_len += len as u64;
        min_len = min_len.min(len);
        max_len = max_len.max(len);
        *lengths.entry(len).or_insert(0) += 1;
        let upper = r.sequence.to_ascii_uppercase();
        gc += upper.bytes().filter(|b| matches!(b, b'G' | b'C')).count() as u64;
        bases += len as u64;
        if hist.len() < r.quality.len() {
            hist.resize(r.quality.len(), [0u64; 94]);
        }
        for (i, b) in r.quality.bytes().enumerate() {
            let q = (b as i32 - 33).clamp(0, 93) as usize;
            hist[i][q] += 1;
        }
        if (count as usize) <= max_dup_reads {
            dup_total += 1;
            let key: String = upper.chars().take(50).collect();
            *keys.entry(key).or_insert(0) += 1;
        }
        if !adapter_word.is_empty() && upper.contains(&adapter_word) {
            adapter_hits += 1;
        }
    }
    if count == 0 {
        return QcReport {
            reads: 0,
            min_length: 0,
            max_length: 0,
            mean_length: 0.0,
            gc_percent: 0.0,
            mean_quality_per_position: Vec::new(),
            q25_per_position: Vec::new(),
            median_per_position: Vec::new(),
            q75_per_position: Vec::new(),
            length_distribution: BTreeMap::new(),
            duplication_percent: 0.0,
            adapter_percent: 0.0,
            overrepresented: Vec::new(),
        };
    }
    let mut mean = Vec::new();
    let (mut q25, mut q50, mut q75) = (Vec::new(), Vec::new(), Vec::new());
    for h in &hist {
        let n: u64 = h.iter().sum();
        let sum: u64 = h.iter().enumerate().map(|(q, c)| q as u64 * c).sum();
        mean.push(if n == 0 { 0.0 } else { sum as f64 / n as f64 });
        let pick = |num: u64, den: u64| -> f64 {
            if n == 0 {
                return 0.0;
            }
            let idx = (n - 1) * num / den;
            let mut seen = 0u64;
            for (q, c) in h.iter().enumerate() {
                seen += c;
                if seen > idx {
                    return q as f64;
                }
            }
            0.0
        };
        q25.push(pick(25, 100));
        q50.push(pick(1, 2));
        q75.push(pick(75, 100));
    }
    let unique = keys.len() as f64;
    let duplication = if dup_total == 0 {
        0.0
    } else {
        100.0 * (1.0 - unique / dup_total as f64)
    };
    let mut over: Vec<(String, u64)> = keys
        .into_iter()
        .filter(|(_, c)| *c >= 2 && *c as f64 / dup_total as f64 > 0.001)
        .collect();
    over.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    over.truncate(10);
    QcReport {
        reads: count,
        min_length: min_len,
        max_length: max_len,
        mean_length: total_len as f64 / count as f64,
        gc_percent: if bases == 0 {
            0.0
        } else {
            100.0 * gc as f64 / bases as f64
        },
        mean_quality_per_position: mean,
        q25_per_position: q25,
        median_per_position: q50,
        q75_per_position: q75,
        length_distribution: lengths,
        duplication_percent: duplication,
        adapter_percent: 100.0 * adapter_hits as f64 / count as f64,
        overrepresented: over,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(seq: &str, qual: &str) -> FastqRecord {
        FastqRecord {
            id: "r".to_string(),
            comment: String::new(),
            sequence: seq.to_string(),
            quality: qual.to_string(),
        }
    }

    const AD: &str = "AGATCGGAAGAGC";

    #[test]
    fn einfache_kennzahlen() {
        let reads = vec![rec("ACGT", "IIII"); 4];
        let r = analyze_reads(reads, AD, 100_000);
        assert_eq!(r.reads, 4);
        assert_eq!((r.min_length, r.max_length), (4, 4));
        assert_eq!(r.mean_length, 4.0);
        assert_eq!(r.gc_percent, 50.0);
        assert_eq!(r.mean_quality_per_position, vec![40.0; 4]);
        assert_eq!(r.median_per_position, vec![40.0; 4]);
        assert_eq!(r.length_distribution.get(&4), Some(&4));
    }

    #[test]
    fn quartile_unterer_wert() {
        let reads = vec![rec("A", "!"), rec("A", "!"), rec("A", "I"), rec("A", "I")];
        let r = analyze_reads(reads, AD, 10);
        assert_eq!(r.mean_quality_per_position, vec![20.0]);
        assert_eq!(r.q25_per_position, vec![0.0]);
        assert_eq!(r.median_per_position, vec![0.0]);
        assert_eq!(r.q75_per_position, vec![40.0]);
    }

    #[test]
    fn laengen_und_positionen() {
        let reads = vec![
            rec("ACGT", "IIII"),
            rec("ACGT", "IIII"),
            rec("ACGT", "IIII"),
            rec("ACGTA", "IIII5"),
        ];
        let r = analyze_reads(reads, AD, 10);
        assert_eq!((r.min_length, r.max_length), (4, 5));
        assert_eq!(r.mean_length, 4.25);
        assert_eq!(r.length_distribution.get(&4), Some(&3));
        assert_eq!(r.length_distribution.get(&5), Some(&1));
        assert_eq!(r.mean_quality_per_position.len(), 5);
        // Position 4 hat nur einen Wert: '5' = 20
        assert_eq!(r.mean_quality_per_position[4], 20.0);
        assert_eq!(r.q25_per_position[4], 20.0);
    }

    #[test]
    fn duplikation_und_ueberrepraesentiert() {
        let reads = vec![rec("ACGTACGT", "IIIIIIII"); 10];
        let r = analyze_reads(reads, AD, 100);
        assert!((r.duplication_percent - 90.0).abs() < 1e-9);
        assert_eq!(r.overrepresented, vec![("ACGTACGT".to_string(), 10)]);
        // nur die ersten 3 Reads zählen
        let mut mixed = vec![rec("AAAA", "IIII"); 2];
        mixed.push(rec("CCCC", "IIII"));
        mixed.push(rec("GGGG", "IIII"));
        let r2 = analyze_reads(mixed, AD, 3);
        assert!((r2.duplication_percent - 100.0 * (1.0 - 2.0 / 3.0)).abs() < 1e-9);
        // bei vielen verschiedenen Reads gibt es keine überrepräsentierte Sequenz
        let many: Vec<FastqRecord> = (0..2000)
            .map(|i| {
                let s: String = (0..12)
                    .map(|k| ['A', 'C', 'G', 'T'][(i / 4usize.pow(k % 6) + k as usize) % 4])
                    .collect();
                rec(&s, "IIIIIIIIIIII")
            })
            .collect();
        let r3 = analyze_reads(many, AD, 100_000);
        assert!(r3
            .overrepresented
            .iter()
            .all(|(_, n)| *n as f64 / 2000.0 > 0.001));
    }

    #[test]
    fn adapteranteil() {
        let mut reads = vec![rec("ACGTACGTACGT", "IIIIIIIIIIII"); 8];
        reads.push(rec("ACGTAGATCGGAAGAGCC", "IIIIIIIIIIIIIIIIII"));
        reads.push(rec("AGATCGGAAGAG", "IIIIIIIIIIII"));
        let r = analyze_reads(reads, AD, 100);
        assert!((r.adapter_percent - 20.0).abs() < 1e-9);
        let none = analyze_reads(vec![rec("ACGT", "IIII")], "", 10);
        assert_eq!(none.adapter_percent, 0.0);
    }

    #[test]
    fn leere_eingabe() {
        let r = analyze_reads(Vec::<FastqRecord>::new(), AD, 10);
        assert_eq!(r.reads, 0);
        assert_eq!((r.min_length, r.max_length), (0, 0));
        assert_eq!(r.mean_length, 0.0);
        assert_eq!(r.gc_percent, 0.0);
        assert_eq!(r.duplication_percent, 0.0);
        assert_eq!(r.adapter_percent, 0.0);
        assert!(r.mean_quality_per_position.is_empty());
        assert!(r.length_distribution.is_empty());
        assert!(r.overrepresented.is_empty());
    }
}
