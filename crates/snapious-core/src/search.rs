//! Lokale BLAST-ähnliche Ähnlichkeitssuche nach dem Seed-and-Extend-Prinzip.
//!
//! k-mere (Wörter) der Query werden in den Subjects gesucht (exakte Treffer = Seeds) und ohne Lücken in beide
//! Richtungen verlängert (Treffer +2, Fehlpaarung −1, Abbruch nach 10 Punkten Verlust, X-drop).
//! Nukleotide werden auf beiden Strängen durchsucht. Bewertet wird einfach nach Identität (kein BLOSUM).
//! Quelle: Altschul et al., J. Mol. Biol. 215 (1990), vereinfacht.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rayon::prelude::*;

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::model::sequence::{Sequence, SequenceType};

const MATCH: i32 = 2;
const MISMATCH: i32 = -1;
const X_DROP: i32 = 10;

/// Eine zu durchsuchende Sequenz.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchSubject {
    pub id: String,
    pub sequence: String,
    /// Protein (nur ein Strang, Wörter bis Länge 12) oder Nukleotid (beide Stränge, Wörter bis Länge 32).
    pub protein: bool,
}

impl SearchSubject {
    pub fn from_sequence(s: &Sequence) -> Self {
        Self {
            id: s.id().to_string(),
            sequence: s.bases().to_string(),
            protein: s.seq_type() == SequenceType::Protein,
        }
    }
}

/// High-scoring Segment Pair.
#[derive(Clone, Debug, PartialEq)]
pub struct Hsp {
    pub query_id: String,
    pub subject_id: String,
    /// Bereich auf dem Vorwärtsstrang der Query `[start, end)`.
    pub query_start: usize,
    pub query_end: usize,
    pub subject_start: usize,
    pub subject_end: usize,
    pub score: i32,
    pub matches: usize,
    pub mismatches: usize,
    /// `'+'` oder `'-'` (Query als Reverse-Komplement).
    pub strand: char,
}

impl Hsp {
    pub fn length(&self) -> usize {
        self.subject_end - self.subject_start
    }

    pub fn percent_identity(&self) -> f64 {
        if self.length() == 0 {
            0.0
        } else {
            self.matches as f64 * 100.0 / self.length() as f64
        }
    }

    pub fn format(&self) -> String {
        format!(
            "HSP[{} → {} ({}): {}-{} vs {}-{}, Score={}, ID={:.0}%]",
            self.query_id,
            self.subject_id,
            self.strand,
            self.query_start,
            self.query_end,
            self.subject_start,
            self.subject_end,
            self.score,
            self.percent_identity()
        )
    }
}

fn pack_nucleotide(w: &[u8]) -> Option<u64> {
    let mut v = 0u64;
    for b in w {
        let c = match b {
            b'A' => 0,
            b'C' => 1,
            b'G' => 2,
            b'T' => 3,
            _ => return None,
        };
        v = (v << 2) | c;
    }
    Some(v)
}

fn pack_protein(w: &[u8]) -> Option<u64> {
    let mut v = 0u64;
    for b in w {
        if !b.is_ascii_uppercase() {
            return None;
        }
        v = (v << 5) | (*b - b'A') as u64;
    }
    Some(v)
}

fn pack(w: &[u8], protein: bool) -> Option<u64> {
    if protein {
        pack_protein(w)
    } else {
        pack_nucleotide(w)
    }
}

/// Verlängert einen Seed; liefert (Query-Start, Subject-Start, Länge, Treffer, Score) im Koordinatensystem des Strangs.
fn extend(
    q: &[u8],
    s: &[u8],
    qpos: usize,
    spos: usize,
    k: usize,
) -> (usize, usize, usize, usize, i32) {
    let (mut best_r, mut best_r_score, mut run) = (0usize, 0i32, 0i32);
    let mut i = 0;
    while qpos + k + i < q.len() && spos + k + i < s.len() {
        run += if q[qpos + k + i] == s[spos + k + i] {
            MATCH
        } else {
            MISMATCH
        };
        if run > best_r_score {
            best_r_score = run;
            best_r = i + 1;
        } else if best_r_score - run > X_DROP {
            break;
        }
        i += 1;
    }
    let (mut best_l, mut best_l_score) = (0usize, 0i32);
    run = 0;
    let mut i = 1;
    while qpos >= i && spos >= i {
        run += if q[qpos - i] == s[spos - i] {
            MATCH
        } else {
            MISMATCH
        };
        if run > best_l_score {
            best_l_score = run;
            best_l = i;
        } else if best_l_score - run > X_DROP {
            break;
        }
        i += 1;
    }
    let qs = qpos - best_l;
    let ss = spos - best_l;
    let len = best_l + k + best_r;
    let matches = (0..len).filter(|i| q[qs + i] == s[ss + i]).count();
    (
        qs,
        ss,
        len,
        matches,
        k as i32 * MATCH + best_l_score + best_r_score,
    )
}

fn search_strand(
    query_id: &str,
    q: &[u8],
    subject: &SearchSubject,
    s: &[u8],
    k: usize,
    strand: char,
    out: &mut Vec<Hsp>,
) {
    if q.len() < k || s.len() < k {
        return;
    }
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    for i in 0..=(q.len() - k) {
        if let Some(key) = pack(&q[i..i + k], subject.protein) {
            index.entry(key).or_default().push(i);
        }
    }
    // Diagonale (spos - qpos) → bereits abgedecktes Ende in der Query
    let mut covered: HashMap<i64, usize> = HashMap::new();
    for spos in 0..=(s.len() - k) {
        let Some(key) = pack(&s[spos..spos + k], subject.protein) else {
            continue;
        };
        let Some(qs) = index.get(&key) else { continue };
        for &qpos in qs {
            let diag = spos as i64 - qpos as i64;
            if covered.get(&diag).map(|end| qpos < *end).unwrap_or(false) {
                continue;
            }
            let (qstart, sstart, len, matches, score) = extend(q, s, qpos, spos, k);
            covered.insert(diag, qstart + len);
            let (qa, qb) = if strand == '+' {
                (qstart, qstart + len)
            } else {
                (q.len() - (qstart + len), q.len() - qstart)
            };
            out.push(Hsp {
                query_id: query_id.to_string(),
                subject_id: subject.id.clone(),
                query_start: qa,
                query_end: qb,
                subject_start: sstart,
                subject_end: sstart + len,
                score,
                matches,
                mismatches: len - matches,
                strand,
            });
        }
    }
}

/// Sucht `query` in allen `subjects`. Ergebnis: überlappungsfreie HSPs nach Score absteigend.
/// `word_size` ist die Seed-Länge (Nukleotide 1–32, Protein 1–12).
pub fn search(
    query_id: &str,
    query: &str,
    subjects: &[SearchSubject],
    word_size: usize,
) -> Result<Vec<Hsp>> {
    if word_size == 0 {
        return Err(Error::InvalidInput(
            "Wortlänge muss mindestens 1 sein".to_string(),
        ));
    }
    for s in subjects {
        let max = if s.protein { 12 } else { 32 };
        if word_size > max {
            return Err(Error::InvalidInput(format!(
                "Wortlänge {} ist für {} zu groß (höchstens {})",
                word_size,
                if s.protein { "Proteine" } else { "Nukleotide" },
                max
            )));
        }
    }
    let q_fwd = query.to_ascii_uppercase().replace('U', "T");
    let per_subject: Vec<Vec<Hsp>> = subjects
        .par_iter()
        .map(|subject| {
            let mut out = Vec::new();
            let s = if subject.protein {
                subject.sequence.to_ascii_uppercase()
            } else {
                subject.sequence.to_ascii_uppercase().replace('U', "T")
            };
            let q = if subject.protein {
                query.to_ascii_uppercase()
            } else {
                q_fwd.clone()
            };
            search_strand(
                query_id,
                q.as_bytes(),
                subject,
                s.as_bytes(),
                word_size,
                '+',
                &mut out,
            );
            if !subject.protein {
                let rc = iupac_reverse_complement(&q);
                search_strand(
                    query_id,
                    rc.as_bytes(),
                    subject,
                    s.as_bytes(),
                    word_size,
                    '-',
                    &mut out,
                );
            }
            out
        })
        .collect();
    let mut all: Vec<Hsp> = per_subject.into_iter().flatten().collect();
    Ok(filter_and_sort(&mut all))
}

fn filter_and_sort(hsps: &mut Vec<Hsp>) -> Vec<Hsp> {
    hsps.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.subject_id.cmp(&b.subject_id))
            .then_with(|| a.query_start.cmp(&b.query_start))
            .then_with(|| a.subject_start.cmp(&b.subject_start))
            .then_with(|| a.strand.cmp(&b.strand))
    });
    let mut unique: Vec<Hsp> = Vec::new();
    for h in hsps.drain(..) {
        let redundant = unique.iter().any(|u| {
            u.subject_id == h.subject_id
                && u.strand == h.strand
                && h.query_end.min(u.query_end) > h.query_start.max(u.query_start)
                && h.subject_end.min(u.subject_end) > h.subject_start.max(u.subject_start)
        });
        if !redundant {
            unique.push(h);
        }
    }
    unique
}

/// Alle verschiedenen k-mere einer Sequenz (Großbuchstaben).
pub fn unique_kmers(seq: &str, k: usize) -> BTreeSet<String> {
    kmer_frequencies(seq, k).into_keys().collect()
}

/// Häufigkeit jedes k-mers (Großbuchstaben).
pub fn kmer_frequencies(seq: &str, k: usize) -> BTreeMap<String, usize> {
    let s = seq.to_ascii_uppercase();
    let mut m = BTreeMap::new();
    if k == 0 || s.len() < k {
        return m;
    }
    for i in 0..=(s.len() - k) {
        *m.entry(s[i..i + k].to_string()).or_insert(0) += 1;
    }
    m
}

/// Textzusammenfassung der besten Treffer.
pub fn search_summary(hsps: &[Hsp], query_name: &str) -> String {
    let mut out = format!(
        "=== BLAST-like Suchergebnisse für: {} ===\n\nFundstellen (HSPs): {}\n",
        query_name,
        hsps.len()
    );
    if hsps.is_empty() {
        out.push_str("Keine signifikanten Übereinstimmungen gefunden.\n");
        return out;
    }
    out.push_str("\nTop 10 Ergebnisse:\n");
    for (i, h) in hsps.iter().take(10).enumerate() {
        out.push_str(&format!(
            "{}. {}\n   Länge: {} bp, Identität: {:.0}%\n\n",
            i + 1,
            h.format(),
            h.length(),
            h.percent_identity()
        ));
    }
    let n = hsps.len() as f64;
    out.push_str(&format!(
        "Durchschnittlicher Score: {:.1}\nDurchschnittliche Identität: {:.0}%\n",
        hsps.iter().map(|h| h.score as f64).sum::<f64>() / n,
        hsps.iter().map(|h| h.percent_identity()).sum::<f64>() / n
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Rng;

    fn nt(id: &str, s: &str) -> SearchSubject {
        SearchSubject {
            id: id.to_string(),
            sequence: s.to_string(),
            protein: false,
        }
    }

    fn zufall(n: usize, seed: u64) -> String {
        let mut r = Rng::new(seed);
        (0..n).map(|_| ['A', 'C', 'G', 'T'][r.below(4)]).collect()
    }

    #[test]
    fn exakter_treffer() {
        let hits = search(
            "q",
            "ATGCGTACGTAGCTAG",
            &[nt("s", "TTTTATGCGTACGTAGCTAGCCCC")],
            8,
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
        let h = &hits[0];
        assert_eq!(h.subject_id, "s");
        assert_eq!((h.query_start, h.query_end), (0, 16));
        assert_eq!((h.subject_start, h.subject_end), (4, 20));
        assert_eq!(h.score, 32);
        assert_eq!(h.percent_identity(), 100.0);
        assert_eq!(h.strand, '+');
    }

    #[test]
    fn reverse_komplement_auf_minusstrang() {
        let hits = search(
            "q",
            "CTAGCTACGTACGCAT",
            &[nt("s", "CCCCATGCGTACGTAGCTAGCCCC")],
            8,
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
        let h = &hits[0];
        assert_eq!(h.strand, '-');
        assert_eq!(h.length(), 16);
        assert_eq!((h.subject_start, h.subject_end), (4, 20));
        assert_eq!((h.query_start, h.query_end), (0, 16));
    }

    #[test]
    fn kein_treffer() {
        // A…A gegen C…C: auch das Reverse-Komplement (G…G) passt nicht
        assert!(search("q", &"A".repeat(16), &[nt("s", &"C".repeat(16))], 8)
            .unwrap()
            .is_empty());
        // A…A gegen T…T trifft dagegen auf dem Minusstrang
        let hits = search("q", &"A".repeat(16), &[nt("s", &"T".repeat(16))], 8).unwrap();
        assert_eq!(hits[0].strand, '-');
    }

    #[test]
    fn treffer_ueberbrueckt_fehlpaarung() {
        let subject = zufall(300, 5);
        let mut query: Vec<u8> = subject[100..160].bytes().collect();
        query[30] = if query[30] == b'A' { b'C' } else { b'A' };
        let query = String::from_utf8(query).unwrap();
        let hits = search("q", &query, &[nt("s", &subject)], 11).unwrap();
        assert_eq!(hits.len(), 1, "{:?}", hits);
        let h = &hits[0];
        assert_eq!((h.subject_start, h.subject_end), (100, 160));
        assert_eq!(h.matches, 59);
        assert_eq!(h.mismatches, 1);
        assert_eq!(h.score, 59 * 2 - 1);
    }

    #[test]
    fn mehrere_subjects_nach_score_sortiert() {
        let q = "ATGCGTACGTAGCTAGGCTA";
        let halb = "ATGCGTACGT";
        let hits = search(
            "q",
            q,
            &[
                nt("kurz", &format!("TTTT{}TTTT", halb)),
                nt("voll", &format!("GG{}GG", q)),
            ],
            6,
        )
        .unwrap();
        assert_eq!(hits[0].subject_id, "voll");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn protein_suche() {
        let subject = SearchSubject {
            id: "p".to_string(),
            sequence: "AAAMKVLAAGIVAAA".to_string(),
            protein: true,
        };
        let hits = search("q", "MKVLAAGIV", &[subject], 3).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].strand, '+');
        assert_eq!((hits[0].subject_start, hits[0].subject_end), (3, 12));
    }

    #[test]
    fn fehlerfaelle_und_leere_eingaben() {
        let s = [nt("s", "ACGTACGTACGT")];
        assert!(search("q", "ACGT", &s, 0).is_err());
        assert!(search("q", "ACGT", &s, 33).is_err());
        assert!(search("q", "AC", &s, 4).unwrap().is_empty());
        assert!(search("q", "ACGTACGT", &[], 4).unwrap().is_empty());
        assert!(search("q", "", &s, 4).unwrap().is_empty());
        let p = SearchSubject {
            id: "p".into(),
            sequence: "MKV".into(),
            protein: true,
        };
        assert!(search("q", "MKV", &[p], 13).is_err());
    }

    #[test]
    fn kleinbuchstaben_und_rna() {
        let a = search("q", "acgtacgtacgt", &[nt("s", "TTACGTACGTACGTTT")], 6).unwrap();
        let b = search("q", "ACGUACGUACGU", &[nt("s", "TTACGTACGTACGTTT")], 6).unwrap();
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn kmer_hilfen() {
        let k = unique_kmers("ATGCGT", 3);
        assert_eq!(k.len(), 4);
        assert!(k.contains("ATG"));
        let f = kmer_frequencies("AAAA", 2);
        assert_eq!(f["AA"], 3);
        assert!(kmer_frequencies("AC", 3).is_empty());
        assert!(kmer_frequencies("ACGT", 0).is_empty());
    }

    #[test]
    fn zusammenfassung() {
        let hits = search(
            "q",
            "ATGCGTACGTAGCTAG",
            &[nt("s", "TTTTATGCGTACGTAGCTAGCCCC")],
            8,
        )
        .unwrap();
        let s = search_summary(&hits, "q");
        assert!(s.contains("Fundstellen (HSPs): 1"));
        assert!(s.contains("Identität: 100%"));
        assert!(search_summary(&[], "q").contains("Keine signifikanten"));
        assert!(hits[0].format().starts_with("HSP[q → s (+)"));
    }

    #[test]
    fn grosse_datenbank_ist_schnell() {
        let db = zufall(1_000_000, 1);
        let query = db[500_000..501_000].to_string();
        let t = std::time::Instant::now();
        let hits = search("q", &query, &[nt("db", &db)], 11).unwrap();
        assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
        assert_eq!(hits[0].length(), 1000);
        assert_eq!(hits[0].subject_start, 500_000);
    }

    #[test]
    fn aus_sequenz_objekt() {
        let s = Sequence::new("id", "n", "ACGTACGT", SequenceType::Dna).unwrap();
        let subj = SearchSubject::from_sequence(&s);
        assert_eq!(subj.id, "id");
        assert!(!subj.protein);
    }
}
