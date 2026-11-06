//! Paarweise Alignments: Needleman-Wunsch (global), Smith-Waterman (lokal) und semiglobal.
//!
//! Quellen: Needleman & Wunsch, J. Mol. Biol. 48 (1970); Smith & Waterman, J. Mol. Biol. 147 (1981).
//! Lineare Lückenstrafe. Speicher: zwei Score-Zeilen plus ein Byte Rückverfolgung je Zelle.
//! Bei Gleichstand gilt im Traceback die Reihenfolge: Diagonale, Lücke in Sequenz 2 (Schritt in Sequenz 1),
//! Lücke in Sequenz 1.

use crate::error::{Error, Result};

/// Art des Alignments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignType {
    Global,
    Local,
    SemiGlobal,
}

/// Ergebnis eines paarweisen Alignments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignResult {
    pub seq1_aln: String,
    pub seq2_aln: String,
    pub score: i32,
    pub align_type: AlignType,
    /// Start in Sequenz 2 (Referenz) bei lokalem/semiglobalem Alignment, sonst 0.
    pub seq2_start: usize,
    /// Ende (exklusiv) in Sequenz 2 bei lokalem/semiglobalem Alignment, sonst Länge von Sequenz 2.
    pub seq2_end: usize,
    /// Start in Sequenz 1 bei lokalem Alignment, sonst 0.
    pub seq1_start: usize,
    /// Ende (exklusiv) in Sequenz 1 bei lokalem Alignment, sonst Länge von Sequenz 1.
    pub seq1_end: usize,
}

impl AlignResult {
    /// Textdarstellung in Blöcken zu 60 Spalten: Sequenz 1, Markierung (`|` gleich, `:` ungleich), Sequenz 2.
    pub fn format(&self) -> String {
        let a = self.seq1_aln.as_bytes();
        let b = self.seq2_aln.as_bytes();
        let len = a.len().min(b.len());
        let mut out = format!("Score: {:.1}\n\n", self.score as f64);
        let mut start = 0;
        while start < len {
            let end = (start + 60).min(len);
            let marks: String = (start..end)
                .map(|i| {
                    if a[i] == b'-' || b[i] == b'-' {
                        ' '
                    } else if a[i] == b[i] {
                        '|'
                    } else {
                        ':'
                    }
                })
                .collect();
            out.push_str(&String::from_utf8_lossy(&a[start..end]));
            out.push('\n');
            out.push_str(&marks);
            out.push('\n');
            out.push_str(&String::from_utf8_lossy(&b[start..end]));
            out.push('\n');
            if end < len {
                out.push('\n');
            }
            start = end;
        }
        out
    }
}

/// Alignment-Parameter (Standard für DNA: +2 / -1 / -1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aligner {
    pub match_score: i32,
    pub mismatch: i32,
    pub gap: i32,
}

impl Default for Aligner {
    fn default() -> Self {
        Self::new(2, -1, -1)
    }
}

const STOP: u8 = 0;
const DIAG: u8 = 1;
const UP: u8 = 2; // Schritt in Sequenz 1, Lücke in Sequenz 2
const LEFT: u8 = 3; // Schritt in Sequenz 2, Lücke in Sequenz 1

/// Höchstzahl an Matrixzellen (Schutz vor unbeabsichtigtem Speicherverbrauch).
const MAX_CELLS: usize = 4_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Global,
    Local,
    SemiGlobal,
}

impl Aligner {
    pub fn new(match_score: i32, mismatch: i32, gap: i32) -> Self {
        Self {
            match_score,
            mismatch,
            gap,
        }
    }

    #[inline]
    fn sub(&self, a: u8, b: u8) -> i32 {
        if a == b {
            self.match_score
        } else {
            self.mismatch
        }
    }

    /// Globales Alignment (Needleman-Wunsch).
    pub fn needleman_wunsch(&self, seq1: &str, seq2: &str) -> Result<AlignResult> {
        self.run(seq1.as_bytes(), seq2.as_bytes(), Mode::Global)
    }

    /// Lokales Alignment (Smith-Waterman).
    pub fn smith_waterman(&self, seq1: &str, seq2: &str) -> Result<AlignResult> {
        self.run(seq1.as_bytes(), seq2.as_bytes(), Mode::Local)
    }

    /// Semiglobales Alignment: `read` wird vollständig ausgerichtet, Lücken am Anfang und Ende der
    /// `reference` kosten nichts. Ergebnis: `seq1` = Read, `seq2` = Referenz.
    pub fn semi_global(&self, read: &str, reference: &str) -> Result<AlignResult> {
        self.run(read.as_bytes(), reference.as_bytes(), Mode::SemiGlobal)
    }

    fn run(&self, a: &[u8], b: &[u8], mode: Mode) -> Result<AlignResult> {
        let (n, m) = (a.len(), b.len());
        let cells = (n + 1).saturating_mul(m + 1);
        if cells > MAX_CELLS {
            return Err(Error::InvalidInput(format!(
                "Sequenzen zu lang für ein Alignment ({} x {})",
                n, m
            )));
        }
        let w = m + 1;
        let mut dirs = vec![STOP; (n + 1) * w];
        let mut prev = vec![0i32; w];
        let mut cur = vec![0i32; w];

        // Erste Zeile (i = 0)
        for j in 1..=m {
            match mode {
                Mode::Global => {
                    prev[j] = prev[j - 1] + self.gap;
                    dirs[j] = LEFT;
                }
                Mode::Local | Mode::SemiGlobal => {
                    prev[j] = 0;
                    dirs[j] = STOP;
                }
            }
        }

        let (mut best, mut best_i, mut best_j) = (0i32, 0usize, 0usize);
        for i in 1..=n {
            // Erste Spalte (j = 0)
            match mode {
                Mode::Global | Mode::SemiGlobal => {
                    cur[0] = prev[0] + self.gap;
                    dirs[i * w] = UP;
                }
                Mode::Local => {
                    cur[0] = 0;
                    dirs[i * w] = STOP;
                }
            }
            let ai = a[i - 1];
            for j in 1..=m {
                let d = prev[j - 1] + self.sub(ai, b[j - 1]);
                let u = prev[j] + self.gap;
                let l = cur[j - 1] + self.gap;
                // Global/semiglobal starten mit der Diagonale; lokal mit 0 (Abbruch).
                let (mut score, mut dir) = if mode == Mode::Local {
                    (0, STOP)
                } else {
                    (d, DIAG)
                };
                if mode == Mode::Local && d > score {
                    score = d;
                    dir = DIAG;
                }
                if u > score {
                    score = u;
                    dir = UP;
                }
                if l > score {
                    score = l;
                    dir = LEFT;
                }
                cur[j] = score;
                dirs[i * w + j] = dir;
                if mode == Mode::Local && score > best {
                    best = score;
                    best_i = i;
                    best_j = j;
                }
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        // Nach dem Tausch steht die letzte Zeile in `prev`.

        let (end_i, end_j, score) = match mode {
            Mode::Global => (n, m, prev[m]),
            Mode::Local => (best_i, best_j, best),
            Mode::SemiGlobal => {
                if n == 0 {
                    (0, 0, 0)
                } else {
                    let mut bj = 0;
                    for j in 1..=m {
                        if prev[j] > prev[bj] {
                            bj = j;
                        }
                    }
                    (n, bj, prev[bj])
                }
            }
        };

        // Rückverfolgung
        let (mut i, mut j) = (end_i, end_j);
        let mut ra: Vec<u8> = Vec::new();
        let mut rb: Vec<u8> = Vec::new();
        loop {
            let stop = match mode {
                Mode::Global => i == 0 && j == 0,
                Mode::Local => dirs[i * w + j] == STOP,
                Mode::SemiGlobal => i == 0,
            };
            if stop {
                break;
            }
            let dir = if mode == Mode::Global && i == 0 {
                LEFT
            } else if mode == Mode::Global && j == 0 {
                UP
            } else {
                dirs[i * w + j]
            };
            match dir {
                DIAG => {
                    ra.push(a[i - 1]);
                    rb.push(b[j - 1]);
                    i -= 1;
                    j -= 1;
                }
                UP => {
                    ra.push(a[i - 1]);
                    rb.push(b'-');
                    i -= 1;
                }
                LEFT => {
                    ra.push(b'-');
                    rb.push(b[j - 1]);
                    j -= 1;
                }
                _ => break,
            }
        }
        ra.reverse();
        rb.reverse();
        let (align_type, s1, e1, s2, e2) = match mode {
            Mode::Global => (AlignType::Global, 0, n, 0, m),
            Mode::Local => (AlignType::Local, i, end_i, j, end_j),
            Mode::SemiGlobal => (AlignType::SemiGlobal, 0, n, j, end_j),
        };
        Ok(AlignResult {
            seq1_aln: String::from_utf8_lossy(&ra).to_string(),
            seq2_aln: String::from_utf8_lossy(&rb).to_string(),
            score,
            align_type,
            seq1_start: s1,
            seq1_end: e1,
            seq2_start: s2,
            seq2_end: e2,
        })
    }
}

/// Konsensus aus alignierten Zeilen: häufigstes Zeichen ohne Lücke je Spalte; bei Gleichstand das
/// alphabetisch kleinste; `N`, wenn die Spalte nur Lücken enthält. Leere Eingabe ergibt `""`.
pub fn create_consensus(rows: &[String]) -> String {
    let Some(first) = rows.first() else {
        return String::new();
    };
    let len = first.len();
    let mut out = String::with_capacity(len);
    for col in 0..len {
        let mut counts = [0usize; 256];
        for r in rows {
            if let Some(c) = r.as_bytes().get(col) {
                counts[*c as usize] += 1;
            }
        }
        let mut best = b'N';
        let mut best_n = 0;
        for (c, n) in counts.iter().enumerate() {
            if c != b'-' as usize && *n > best_n {
                best_n = *n;
                best = c as u8;
            }
        }
        out.push(best as char);
    }
    out
}

/// Levenshtein-Distanz (Einfügen, Löschen, Ersetzen kosten je 1).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Prozent-Identität über die gemeinsame Länge. Zwei leere Sequenzen: 100, eine leere: 0.
pub fn percent_identity(a: &str, b: &str) -> f64 {
    let len = a.len().min(b.len());
    if len == 0 {
        return if a.len() == b.len() { 100.0 } else { 0.0 };
    }
    let same = a.bytes().zip(b.bytes()).filter(|(x, y)| x == y).count();
    same as f64 / len as f64 * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dna() -> Aligner {
        Aligner::new(2, -1, -1)
    }

    #[test]
    fn global_gleiche_sequenzen_score_8() {
        let r = dna().needleman_wunsch("ATCG", "ATCG").unwrap();
        assert_eq!(r.score, 8);
        assert_eq!(r.seq1_aln, "ATCG");
        assert_eq!(r.seq2_aln, "ATCG");
        assert_eq!(r.align_type, AlignType::Global);
    }

    #[test]
    fn global_mit_luecke() {
        let r = dna().needleman_wunsch("ACGT", "AGT").unwrap();
        assert_eq!(r.score, 5);
        assert_eq!(r.seq1_aln, "ACGT");
        assert_eq!(r.seq2_aln, "A-GT");
    }

    #[test]
    fn global_leere_sequenz() {
        let r = dna().needleman_wunsch("", "ATCG").unwrap();
        assert_eq!(r.score, -4);
        assert_eq!(r.seq1_aln, "----");
        assert_eq!(r.seq2_aln, "ATCG");
        let r = dna().needleman_wunsch("ATCG", "").unwrap();
        assert_eq!(r.seq2_aln, "----");
        let r = dna().needleman_wunsch("", "").unwrap();
        assert_eq!(r.score, 0);
    }

    #[test]
    fn global_gleichstand_bevorzugt_diagonale() {
        let r = dna().needleman_wunsch("AAT", "AT").unwrap();
        assert_eq!(r.score, 3);
        assert_eq!(r.seq1_aln, "AAT");
        assert_eq!(r.seq2_aln, "-AT");
    }

    #[test]
    fn global_mismatch_senkt_score() {
        let r = dna().needleman_wunsch("ATCG", "AACG").unwrap();
        assert_eq!(r.score, 5); // 3 Treffer * 2 - 1
    }

    #[test]
    fn eigene_bewertung() {
        let r = Aligner::new(5, -2, -3)
            .needleman_wunsch("AAAA", "AAAA")
            .unwrap();
        assert_eq!(r.score, 20);
        assert_eq!(Aligner::default(), dna());
    }

    #[test]
    fn einzelne_basen() {
        assert_eq!(dna().needleman_wunsch("A", "A").unwrap().score, 2);
        assert!(dna().needleman_wunsch("A", "T").unwrap().score <= 0);
        assert_eq!(dna().smith_waterman("AT", "AT").unwrap().score, 4);
    }

    #[test]
    fn lokal_findet_teilstueck() {
        let r = dna().smith_waterman("TTTACGTTTT", "ACGT").unwrap();
        assert_eq!(r.score, 8);
        assert_eq!(r.seq1_aln, "ACGT");
        assert_eq!(r.seq2_aln, "ACGT");
        assert_eq!((r.seq1_start, r.seq1_end), (3, 7));
        assert_eq!((r.seq2_start, r.seq2_end), (0, 4));
        assert_eq!(r.align_type, AlignType::Local);
    }

    #[test]
    fn lokal_ohne_treffer_und_leer() {
        let r = dna().smith_waterman("ZZZZ", "XXXX").unwrap();
        assert_eq!(r.score, 0);
        assert!(r.seq1_aln.is_empty() && r.seq2_aln.is_empty());
        assert_eq!(dna().smith_waterman("", "ATCG").unwrap().score, 0);
    }

    #[test]
    fn lokal_teiltreffer_in_xxatcgyy() {
        let r = dna().smith_waterman("XXATCGYY", "AATCG").unwrap();
        assert_eq!(r.score, 8);
        assert_eq!(r.seq1_aln, "ATCG");
    }

    #[test]
    fn semiglobal_read_in_referenz() {
        let r = dna().semi_global("ACGT", "TTTACGTTTT").unwrap();
        assert_eq!(r.score, 8);
        assert_eq!(r.seq1_aln, "ACGT");
        assert_eq!(r.seq2_aln, "ACGT");
        assert_eq!((r.seq2_start, r.seq2_end), (3, 7));
        assert_eq!(r.align_type, AlignType::SemiGlobal);
    }

    #[test]
    fn semiglobal_mit_fehler_und_luecke() {
        // Read mit einer Substitution (C -> T an Position 2)
        let r = dna().semi_global("ACTT", "GGACGTGG").unwrap();
        assert_eq!(r.score, 5);
        assert_eq!((r.seq2_start, r.seq2_end), (2, 6));
        // Read mit fehlender Base
        let r = dna().semi_global("ACT", "GGACGTGG").unwrap();
        assert_eq!(r.seq1_aln.len(), r.seq2_aln.len());
        assert_eq!(r.seq1_aln.replace('-', ""), "ACT");
    }

    #[test]
    fn semiglobal_leerer_read() {
        let r = dna().semi_global("", "ACGT").unwrap();
        assert_eq!(r.score, 0);
        assert!(r.seq1_aln.is_empty());
    }

    #[test]
    fn invarianz_ohne_luecken_ergibt_eingabe() {
        let a = "ATGCGTACGTTAGCATCG";
        let b = "ATGGTACCGTAGCTCG";
        for r in [
            dna().needleman_wunsch(a, b).unwrap(),
            dna().smith_waterman(a, b).unwrap(),
        ] {
            assert_eq!(r.seq1_aln.len(), r.seq2_aln.len());
        }
        let g = dna().needleman_wunsch(a, b).unwrap();
        assert_eq!(g.seq1_aln.replace('-', ""), a);
        assert_eq!(g.seq2_aln.replace('-', ""), b);
    }

    #[test]
    fn global_score_stimmt_mit_neu_berechnetem_alignment_ueberein() {
        let a = "ATGCGTACGTTAGCATCG";
        let b = "ATGGTACCGTAGCTCG";
        let al = dna();
        let r = al.needleman_wunsch(a, b).unwrap();
        let mut score = 0;
        for (x, y) in r.seq1_aln.bytes().zip(r.seq2_aln.bytes()) {
            score += if x == b'-' || y == b'-' {
                al.gap
            } else if x == y {
                al.match_score
            } else {
                al.mismatch
            };
        }
        assert_eq!(score, r.score);
    }

    #[test]
    fn grosses_alignment_ist_schnell() {
        let s = "ATCG".repeat(500);
        let t = std::time::Instant::now();
        let r = dna().needleman_wunsch(&s, &s).unwrap();
        assert_eq!(r.score, 4000);
        assert!(t.elapsed().as_secs() < 5);
    }

    /// Leistungsmessung: `cargo test --release -p snapious-core -- --ignored leistung`
    #[test]
    #[ignore = "Leistungsmessung, nur im Release-Build sinnvoll"]
    fn leistung_5000_mal_5000() {
        let a = crate::util::Rng::new(1);
        let mut r = a;
        let s1: String = (0..5000)
            .map(|_| ['A', 'C', 'G', 'T'][r.below(4)])
            .collect();
        let s2: String = (0..5000)
            .map(|_| ['A', 'C', 'G', 'T'][r.below(4)])
            .collect();
        let t = std::time::Instant::now();
        let res = dna().needleman_wunsch(&s1, &s2).unwrap();
        eprintln!("NW 5000x5000: {:?}, Score {}", t.elapsed(), res.score);
        assert!(t.elapsed().as_secs_f64() < 3.0);
    }

    #[test]
    fn format_zeigt_score_und_markierungen() {
        let r = dna().needleman_wunsch("ACGT", "AGGT").unwrap();
        let f = r.format();
        assert!(f.starts_with("Score: 5.0"));
        assert!(f.contains("|:||"));
    }

    #[test]
    fn format_bricht_nach_60_spalten_um() {
        let s = "A".repeat(130);
        let r = dna().needleman_wunsch(&s, &s).unwrap();
        let blocks = r.format().split("\n\n").count();
        assert_eq!(blocks, 1 + 3); // Kopf + 3 Blöcke
    }

    #[test]
    fn konsens() {
        let rows = vec!["ATCG".to_string(), "A-TG".to_string(), "ATCG".to_string()];
        assert_eq!(create_consensus(&rows), "ATCG");
        assert_eq!(create_consensus(&[]), "");
        let gaps = vec!["--".to_string(), "--".to_string()];
        assert_eq!(create_consensus(&gaps), "NN");
        let tie = vec!["A".to_string(), "C".to_string()];
        assert_eq!(create_consensus(&tie), "A");
    }

    #[test]
    fn editierdistanz() {
        assert_eq!(edit_distance("ATCG", "ATCG"), 0);
        assert_eq!(edit_distance("", "ATCG"), 4);
        assert_eq!(edit_distance("ATCG", "ATGG"), 1);
        assert_eq!(edit_distance("ATCG", "AT"), 2);
        let (a, b, c) = ("AAA", "AAB", "BBB");
        assert!(edit_distance(a, c) <= edit_distance(a, b) + edit_distance(b, c));
    }

    #[test]
    fn prozent_identitaet() {
        assert_eq!(percent_identity("ATCG", "ATCG"), 100.0);
        assert_eq!(percent_identity("ATCG", "AATG"), 50.0);
        assert_eq!(percent_identity("", ""), 100.0);
        assert_eq!(percent_identity("A", ""), 0.0);
        assert_eq!(
            percent_identity("ATCG", "AAGG"),
            percent_identity("AAGG", "ATCG")
        );
    }
}
