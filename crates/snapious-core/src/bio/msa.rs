//! Progressive multiple sequence alignment (ClustalW-like).
//!
//! Procedure: pairwise distances (1 − identity of the global alignment), UPGMA guide tree,
//! profile-profile alignment along the tree with sum-of-pairs column scores.
//! Sources: Thompson, Higgins & Gibson, Nucleic Acids Res. 22 (1994); Sokal & Michener (1958, UPGMA).

use std::collections::BTreeMap;

use rayon::prelude::*;

use crate::bio::align::{create_consensus, percent_identity, Aligner};
use crate::error::{Error, Result};
use crate::model::sequence::Sequence;

const MATCH: i32 = 2;
const MISMATCH: i32 = -1;
const GAP: i32 = -1;

/// An aligned row.
#[derive(Clone, Debug, PartialEq)]
pub struct AlignedSequence {
    pub name: String,
    /// Original bases without gaps.
    pub bases: String,
    /// Bases with gap character `-`.
    pub aligned: String,
}

impl AlignedSequence {
    pub fn gap_count(&self) -> usize {
        self.aligned.bytes().filter(|b| *b == b'-').count()
    }

    /// Gap fraction in percent.
    pub fn gap_percent(&self) -> f64 {
        if self.aligned.is_empty() {
            0.0
        } else {
            self.gap_count() as f64 * 100.0 / self.aligned.len() as f64
        }
    }
}

/// Result of a multiple alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct MsaResult {
    pub sequences: Vec<AlignedSequence>,
    /// Average sum-of-pairs score over all pairs (sum of column scores per pair).
    pub average_score: f64,
    pub consensus: String,
    /// Weight per sequence: 1 − gap fraction.
    pub weights: BTreeMap<String, f64>,
}

impl MsaResult {
    pub fn alignment_length(&self) -> usize {
        self.sequences.first().map(|s| s.aligned.len()).unwrap_or(0)
    }

    /// FASTA with gaps, 60 characters per line.
    pub fn to_fasta(&self) -> String {
        let mut out = String::new();
        for s in &self.sequences {
            out.push('>');
            out.push_str(&s.name);
            out.push('\n');
            for chunk in s.aligned.as_bytes().chunks(60) {
                out.push_str(&String::from_utf8_lossy(chunk));
                out.push('\n');
            }
        }
        out
    }

    /// Readable block rendering (60 columns) with names.
    pub fn format(&self) -> String {
        let width = self
            .sequences
            .iter()
            .map(|s| s.name.chars().count())
            .max()
            .unwrap_or(0)
            .min(20);
        let len = self.alignment_length();
        let mut out = String::new();
        let mut start = 0;
        while start < len {
            let end = (start + 60).min(len);
            for s in &self.sequences {
                let name: String = s.name.chars().take(20).collect();
                out.push_str(&format!(
                    "{:<w$} {}\n",
                    name,
                    &s.aligned[start..end],
                    w = width
                ));
            }
            out.push('\n');
            start = end;
        }
        out
    }
}

enum Node {
    Leaf(usize),
    Inner {
        left: Box<Node>,
        right: Box<Node>,
        height: f64,
    },
}

impl Node {
    fn size(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Inner { left, right, .. } => left.size() + right.size(),
        }
    }

    fn height(&self) -> f64 {
        match self {
            Node::Leaf(_) => 0.0,
            Node::Inner { height, .. } => *height,
        }
    }
}

struct Profile {
    ids: Vec<usize>,
    rows: Vec<Vec<u8>>,
}

impl Profile {
    fn width(&self) -> usize {
        self.rows.first().map(|r| r.len()).unwrap_or(0)
    }
}

/// Distance = 1 − identity of the global alignment (0 to 1).
fn distance_matrix(seqs: &[Sequence]) -> Result<Vec<Vec<f64>>> {
    let n = seqs.len();
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
        .collect();
    let aligner = Aligner::new(MATCH, MISMATCH, GAP);
    let values: Result<Vec<f64>> = pairs
        .par_iter()
        .map(|(i, j)| {
            let r = aligner.needleman_wunsch(seqs[*i].bases(), seqs[*j].bases())?;
            Ok(1.0 - percent_identity(&r.seq1_aln, &r.seq2_aln) / 100.0)
        })
        .collect();
    let values = values?;
    let mut d = vec![vec![0.0; n]; n];
    for ((i, j), v) in pairs.iter().zip(values) {
        d[*i][*j] = v;
        d[*j][*i] = v;
    }
    Ok(d)
}

/// UPGMA: merge the two clusters with the smallest distance each time (smallest index first).
fn upgma_tree(d: &[Vec<f64>]) -> Result<Node> {
    let n = d.len();
    if n == 0 {
        return Err(Error::InvalidInput("No sequences".to_string()));
    }
    let mut dist: Vec<Vec<f64>> = d.to_vec();
    let mut nodes: Vec<Option<Node>> = (0..n).map(|i| Some(Node::Leaf(i))).collect();
    for _ in 0..n - 1 {
        let mut best = (usize::MAX, usize::MAX);
        let mut min = f64::MAX;
        for i in 0..n {
            if nodes[i].is_none() {
                continue;
            }
            for j in (i + 1)..n {
                if nodes[j].is_some() && dist[i][j] < min {
                    min = dist[i][j];
                    best = (i, j);
                }
            }
        }
        let (i, j) = best;
        let (Some(ni), Some(nj)) = (
            nodes.get_mut(i).and_then(Option::take),
            nodes.get_mut(j).and_then(Option::take),
        ) else {
            return Err(Error::InvalidState("Guide tree inconsistent".to_string()));
        };
        let (si, sj) = (ni.size() as f64, nj.size() as f64);
        for k in 0..n {
            if k != i && k != j && nodes[k].is_some() {
                let v = (si * dist[i][k] + sj * dist[j][k]) / (si + sj);
                dist[i][k] = v;
                dist[k][i] = v;
            }
        }
        nodes[i] = Some(Node::Inner {
            left: Box::new(ni),
            right: Box::new(nj),
            height: min / 2.0,
        });
    }
    nodes
        .into_iter()
        .flatten()
        .next()
        .ok_or_else(|| Error::InvalidState("Guide tree is empty".to_string()))
}

fn pair_score(x: u8, y: u8) -> i32 {
    if x == b'-' && y == b'-' {
        0
    } else if x == b'-' || y == b'-' {
        GAP
    } else if x == y {
        MATCH
    } else {
        MISMATCH
    }
}

fn gap_column_score(rows: &[Vec<u8>], col: usize) -> f64 {
    let residues = rows.iter().filter(|r| r[col] != b'-').count();
    (residues as i32 * GAP) as f64 / rows.len() as f64
}

fn column_score(a: &[Vec<u8>], i: usize, b: &[Vec<u8>], j: usize) -> f64 {
    let mut sum = 0i32;
    for ra in a {
        let x = ra[i];
        for rb in b {
            sum += pair_score(x, rb[j]);
        }
    }
    sum as f64 / (a.len() * b.len()) as f64
}

const D_DIAG: u8 = 1;
const D_UP: u8 = 2;
const D_LEFT: u8 = 3;

/// Profile-profile alignment (Needleman-Wunsch on columns). Tie: diagonal before gap.
fn merge_profiles(a: Profile, b: Profile) -> Profile {
    let (la, lb) = (a.width(), b.width());
    let w = lb + 1;
    let gap_a: Vec<f64> = (0..la).map(|i| gap_column_score(&a.rows, i)).collect();
    let gap_b: Vec<f64> = (0..lb).map(|j| gap_column_score(&b.rows, j)).collect();
    let mut dirs = vec![0u8; (la + 1) * w];
    let mut prev = vec![0.0f64; w];
    let mut cur = vec![0.0f64; w];
    for j in 1..=lb {
        prev[j] = prev[j - 1] + gap_b[j - 1];
        dirs[j] = D_LEFT;
    }
    for i in 1..=la {
        cur[0] = prev[0] + gap_a[i - 1];
        dirs[i * w] = D_UP;
        for j in 1..=lb {
            let d = prev[j - 1] + column_score(&a.rows, i - 1, &b.rows, j - 1);
            let u = prev[j] + gap_a[i - 1];
            let l = cur[j - 1] + gap_b[j - 1];
            let (mut s, mut dir) = (d, D_DIAG);
            if u > s + 1e-9 {
                s = u;
                dir = D_UP;
            }
            if l > s + 1e-9 {
                s = l;
                dir = D_LEFT;
            }
            cur[j] = s;
            dirs[i * w + j] = dir;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let mut out_a: Vec<Vec<u8>> = vec![Vec::new(); a.rows.len()];
    let mut out_b: Vec<Vec<u8>> = vec![Vec::new(); b.rows.len()];
    let (mut i, mut j) = (la, lb);
    while i > 0 || j > 0 {
        let dir = if i == 0 {
            D_LEFT
        } else if j == 0 {
            D_UP
        } else {
            dirs[i * w + j]
        };
        match dir {
            D_DIAG => {
                for (r, row) in a.rows.iter().enumerate() {
                    out_a[r].push(row[i - 1]);
                }
                for (r, row) in b.rows.iter().enumerate() {
                    out_b[r].push(row[j - 1]);
                }
                i -= 1;
                j -= 1;
            }
            D_UP => {
                for (r, row) in a.rows.iter().enumerate() {
                    out_a[r].push(row[i - 1]);
                }
                for o in out_b.iter_mut() {
                    o.push(b'-');
                }
                i -= 1;
            }
            _ => {
                for o in out_a.iter_mut() {
                    o.push(b'-');
                }
                for (r, row) in b.rows.iter().enumerate() {
                    out_b[r].push(row[j - 1]);
                }
                j -= 1;
            }
        }
    }
    let mut ids = a.ids;
    ids.extend(b.ids);
    let mut rows = Vec::new();
    for mut r in out_a.into_iter().chain(out_b) {
        r.reverse();
        rows.push(r);
    }
    Profile { ids, rows }
}

fn align_node(node: &Node, seqs: &[Sequence]) -> Profile {
    match node {
        Node::Leaf(i) => Profile {
            ids: vec![*i],
            rows: vec![seqs[*i].as_bytes().to_vec()],
        },
        Node::Inner { left, right, .. } => {
            merge_profiles(align_node(left, seqs), align_node(right, seqs))
        }
    }
}

/// Average sum-of-pairs score over all row pairs.
fn average_pair_score(rows: &[AlignedSequence]) -> f64 {
    let n = rows.len();
    if n < 2 {
        return 0.0;
    }
    let mut total = 0i64;
    let mut pairs = 0i64;
    for i in 0..n {
        for j in (i + 1)..n {
            for (x, y) in rows[i].aligned.bytes().zip(rows[j].aligned.bytes()) {
                total += pair_score(x, y) as i64;
            }
            pairs += 1;
        }
    }
    total as f64 / pairs as f64
}

/// Progressive alignment. The input order is preserved.
pub fn progressive_alignment(seqs: &[Sequence]) -> Result<MsaResult> {
    if seqs.is_empty() {
        return Err(Error::InvalidInput(
            "At least one sequence is required".to_string(),
        ));
    }
    if seqs.len() == 1 {
        let s = &seqs[0];
        let one = AlignedSequence {
            name: s.name().to_string(),
            bases: s.bases().to_string(),
            aligned: s.bases().to_string(),
        };
        return Ok(MsaResult {
            consensus: s.bases().to_string(),
            weights: BTreeMap::from([(s.name().to_string(), 1.0)]),
            sequences: vec![one],
            average_score: 0.0,
        });
    }
    let tree = upgma_tree(&distance_matrix(seqs)?)?;
    let profile = align_node(&tree, seqs);
    let mut ordered: Vec<Option<AlignedSequence>> = vec![None; seqs.len()];
    for (row, id) in profile.rows.iter().zip(&profile.ids) {
        ordered[*id] = Some(AlignedSequence {
            name: seqs[*id].name().to_string(),
            bases: seqs[*id].bases().to_string(),
            aligned: String::from_utf8_lossy(row).to_string(),
        });
    }
    let aligned: Vec<AlignedSequence> = ordered.into_iter().flatten().collect();
    let rows: Vec<String> = aligned.iter().map(|a| a.aligned.clone()).collect();
    let weights = aligned
        .iter()
        .map(|a| (a.name.clone(), 1.0 - a.gap_percent() / 100.0))
        .collect();
    Ok(MsaResult {
        average_score: average_pair_score(&aligned),
        consensus: create_consensus(&rows),
        weights,
        sequences: aligned,
    })
}

fn newick_label(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_whitespace() || "(),:;[]'".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}

fn to_newick(node: &Node, parent_height: f64, seqs: &[Sequence]) -> String {
    let len = format!(":{:.4}", (parent_height - node.height()).max(0.0));
    match node {
        Node::Leaf(i) => format!("{}{}", newick_label(seqs[*i].name()), len),
        Node::Inner {
            left,
            right,
            height,
        } => format!(
            "({},{}){}",
            to_newick(left, *height, seqs),
            to_newick(right, *height, seqs),
            len
        ),
    }
}

/// Guide tree of the sequences as Newick text (branch lengths from the UPGMA heights).
pub fn guide_tree_newick(seqs: &[Sequence]) -> Result<String> {
    if seqs.is_empty() {
        return Err(Error::InvalidInput(
            "At least one sequence is required".to_string(),
        ));
    }
    if seqs.len() == 1 {
        return Ok(format!("{};", newick_label(seqs[0].name())));
    }
    let tree = upgma_tree(&distance_matrix(seqs)?)?;
    let h = tree.height();
    Ok(format!("{};", to_newick(&tree, h, seqs)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::SequenceType;

    fn seqs(list: &[(&str, &str)]) -> Vec<Sequence> {
        list.iter()
            .map(|(n, b)| Sequence::new(n, n, b, SequenceType::Dna).unwrap())
            .collect()
    }

    #[test]
    fn drei_aehnliche_sequenzen() {
        let s = seqs(&[("s1", "ATGCGTAC"), ("s2", "ATGAGTAC"), ("s3", "ATGCGTGC")]);
        let r = progressive_alignment(&s).unwrap();
        assert_eq!(r.sequences.len(), 3);
        assert!(r.sequences.iter().all(|a| a.aligned.len() == 8));
        for (a, orig) in r.sequences.iter().zip(&s) {
            assert_eq!(a.aligned.replace('-', ""), orig.bases());
            assert_eq!(a.name, orig.name());
        }
        assert_eq!(r.alignment_length(), 8);
        assert_eq!(r.consensus, "ATGCGTAC");
    }

    #[test]
    fn identische_sequenzen_ohne_luecken() {
        let s = seqs(&[("a", "ACGTACGT"), ("b", "ACGTACGT")]);
        let r = progressive_alignment(&s).unwrap();
        assert_eq!(r.sequences[0].aligned, "ACGTACGT");
        assert_eq!(r.average_score, 16.0);
        assert_eq!(r.weights["a"], 1.0);
    }

    #[test]
    fn fehlende_base_ergibt_eine_luecke() {
        let s = seqs(&[("a", "ACGT"), ("b", "ACGT"), ("c", "AGT")]);
        let r = progressive_alignment(&s).unwrap();
        assert_eq!(r.sequences[2].aligned, "A-GT");
        assert_eq!(r.sequences[0].aligned, "ACGT");
        assert!((r.sequences[2].gap_percent() - 25.0).abs() < 1e-9);
        assert!((r.weights["c"] - 0.75).abs() < 1e-9);
        assert_eq!(r.consensus, "ACGT");
    }

    #[test]
    fn eingabereihenfolge_bleibt_erhalten() {
        let s = seqs(&[
            ("z", "AAAAAAAAAA"),
            ("a", "CCCCCCCCCC"),
            ("m", "AAAAAAAAAC"),
        ]);
        let r = progressive_alignment(&s).unwrap();
        let names: Vec<&str> = r.sequences.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["z", "a", "m"]);
    }

    #[test]
    fn einzelne_und_leere_eingabe() {
        let one = progressive_alignment(&seqs(&[("a", "ACGT")])).unwrap();
        assert_eq!(one.sequences[0].aligned, "ACGT");
        assert_eq!(one.average_score, 0.0);
        assert!(progressive_alignment(&[]).is_err());
    }

    #[test]
    fn fasta_ausgabe_und_format() {
        let r = progressive_alignment(&seqs(&[("a", "ACGT"), ("b", "AGT")])).unwrap();
        let f = r.to_fasta();
        assert!(f.starts_with(">a\n"));
        assert_eq!(f.lines().filter(|l| l.starts_with('>')).count(), 2);
        assert!(r.format().contains("a "));
    }

    #[test]
    fn viele_sequenzen_invarianz() {
        let base = "ATGCGTACGTAGCTAGCTAGGATCGATCGA";
        let mut list = vec![("s0".to_string(), base.to_string())];
        for k in 1..8usize {
            let mut b: Vec<u8> = base.bytes().collect();
            b[k * 3] = if b[k * 3] == b'A' { b'C' } else { b'A' };
            if k % 3 == 0 {
                b.remove(k + 2);
            }
            list.push((format!("s{}", k), String::from_utf8(b).unwrap()));
        }
        let refs: Vec<(&str, &str)> = list.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let s = seqs(&refs);
        let r = progressive_alignment(&s).unwrap();
        let len = r.alignment_length();
        for (a, orig) in r.sequences.iter().zip(&s) {
            assert_eq!(a.aligned.len(), len);
            assert_eq!(a.aligned.replace('-', ""), orig.bases());
        }
    }

    #[test]
    fn guide_tree_gruppiert_aehnliche() {
        let s = seqs(&[
            ("A1", "AAAAAAAAAAAAAAAAAAAA"),
            ("A2", "AAAAAAAAAAAAAAAAAAAT"),
            ("B1", "CCCCCCCCCCCCCCCCCCCC"),
            ("B2", "CCCCCCCCCCCCCCCCCCCG"),
        ]);
        let n = guide_tree_newick(&s).unwrap();
        assert!(n.starts_with("((A1:"), "{}", n);
        assert!(n.contains("(B1:"), "{}", n);
        assert!(n.ends_with(';'));
        assert_eq!(guide_tree_newick(&seqs(&[("x", "ACGT")])).unwrap(), "x;");
        assert!(guide_tree_newick(&[]).is_err());
    }

    #[test]
    fn leere_sequenz_unter_den_eingaben() {
        let s = seqs(&[("a", "ACGT"), ("b", "")]);
        let r = progressive_alignment(&s).unwrap();
        assert_eq!(r.sequences[1].aligned, "----");
    }
}
