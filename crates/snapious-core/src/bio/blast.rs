//! Local BLAST+ (blastn, blastp): read table format 6 output and run searches against project sequences.

use std::time::Duration;

use crate::error::{Error, Result};
use crate::model::sequence::Sequence;
use crate::tools::ToolRegistry;

/// One hit (one line of `-outfmt 6` with the default fields).
#[derive(Clone, Debug, PartialEq)]
pub struct BlastHit {
    pub query: String,
    pub subject: String,
    pub identity_percent: f64,
    pub alignment_length: usize,
    pub mismatches: usize,
    pub gap_opens: usize,
    /// Start and end in the query (1-based, end inclusive).
    pub query_start: usize,
    pub query_end: usize,
    /// Start and end in the subject (1-based); for a hit on the opposite strand, `subject_start > subject_end`.
    pub subject_start: usize,
    pub subject_end: usize,
    pub evalue: f64,
    pub bit_score: f64,
}

impl BlastHit {
    /// `true` if the hit lies on the opposite strand of the subject (`subject_start > subject_end`).
    pub fn reverse_strand(&self) -> bool {
        self.subject_start > self.subject_end
    }
}

/// Reads BLAST table output (`-outfmt 6`, default fields: qseqid sseqid pident length mismatch gapopen qstart qend sstart send evalue
/// bitscore, tab-separated). Empty lines and lines starting with `#` are skipped. Every other line must have at least
/// 12 columns and contain valid numbers (`evalue` may be written like `1e-20` or `0.0`), otherwise
/// `Error::InvalidInput` with the 1-based line number. Line endings `\n` and `\r\n`. Order as in the input.
pub fn parse_tabular(text: &str) -> Result<Vec<BlastHit>> {
    let mut hits = Vec::new();
    for (line_num, line) in text.lines().enumerate() {
        let line_num = line_num + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = trimmed.split('\t').collect();
        if parts.len() < 12 {
            return Err(Error::InvalidInput(format!(
                "Line {} has too few columns",
                line_num
            )));
        }

        let get = |i: usize, _name: &str| -> Result<&str> {
            parts.get(i).copied().ok_or_else(|| {
                Error::InvalidInput(format!("Column {} is missing in line {}", i + 1, line_num))
            })
        };

        let parse_f = |i: usize, name: &str| -> Result<f64> {
            parts
                .get(i)
                .and_then(|v| v.parse::<f64>().ok())
                .ok_or_else(|| {
                    Error::InvalidInput(format!("Invalid {} in line {}", name, line_num))
                })
        };

        let parse_u = |i: usize, name: &str| -> Result<usize> {
            parts
                .get(i)
                .and_then(|v| v.parse::<usize>().ok())
                .ok_or_else(|| {
                    Error::InvalidInput(format!("Invalid {} in line {}", name, line_num))
                })
        };

        hits.push(BlastHit {
            query: get(0, "qseqid")?.to_string(),
            subject: get(1, "sseqid")?.to_string(),
            identity_percent: parse_f(2, "pident")?,
            alignment_length: parse_u(3, "length")?,
            mismatches: parse_u(4, "mismatch")?,
            gap_opens: parse_u(5, "gapopen")?,
            query_start: parse_u(6, "qstart")?,
            query_end: parse_u(7, "qend")?,
            subject_start: parse_u(8, "sstart")?,
            subject_end: parse_u(9, "send")?,
            evalue: parse_f(10, "evalue")?,
            bit_score: parse_f(11, "bitscore")?,
        });
    }
    Ok(hits)
}

/// Sorted by bit score descending, on ties by E-value ascending, then by subject name (stable).
pub fn sort_hits(hits: &mut [BlastHit]) {
    hits.sort_by(|a, b| {
        b.bit_score
            .partial_cmp(&a.bit_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.evalue
                    .partial_cmp(&b.evalue)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.subject.cmp(&b.subject))
    });
}

/// Searches `query` against `subjects` with local BLAST+. `program` is `blastn` or `blastp`. In a throwaway directory creates
/// `subjects.fa`, `query.fa` and, with `makeblastdb`, the database, and reads the hits; sequence names are replaced by `q`/`s0`, `s1`, …
/// and restored afterwards. Error `Error::Io` if a program is missing or fails; `Error::InvalidInput` without subjects.
pub fn blast_local(
    program: &str,
    query: &Sequence,
    subjects: &[Sequence],
    max_hits: usize,
) -> Result<Vec<BlastHit>> {
    if subjects.is_empty() {
        return Err(Error::InvalidInput(
            "Cannot search without target sequences".to_string(),
        ));
    }
    let (dbtype, make) = match program {
        "blastn" => ("nucl", "makeblastdb"),
        "blastp" => ("prot", "makeblastdb"),
        other => {
            return Err(Error::InvalidInput(format!(
                "Unknown program '{}'",
                other
            )))
        }
    };
    let reg = ToolRegistry::defaults();
    for tool in [program, make] {
        if !reg.is_available(tool) {
            return Err(Error::Io(format!(
                "Program '{}' is not installed",
                tool
            )));
        }
    }
    let dir = std::env::temp_dir().join(format!("snapious-blast-{}", crate::util::new_id()));
    std::fs::create_dir_all(&dir)?;
    let result = (|| -> Result<Vec<BlastHit>> {
        let subject_fasta: String = subjects
            .iter()
            .enumerate()
            .map(|(i, s)| format!(">s{}\n{}\n", i, s.bases()))
            .collect();
        std::fs::write(dir.join("subjects.fa"), subject_fasta)?;
        std::fs::write(dir.join("query.fa"), format!(">q\n{}\n", query.bases()))?;
        let run = |tool: &str, args: &[&str]| -> Result<String> {
            let r = reg.run(tool, args, Some(&dir), None, Duration::from_secs(600))?;
            if !r.ok() {
                return Err(Error::Io(format!(
                    "'{}' reported an error: {}",
                    tool,
                    r.stderr.trim()
                )));
            }
            Ok(r.stdout)
        };
        run(
            make,
            &["-in", "subjects.fa", "-dbtype", dbtype, "-out", "db"],
        )?;
        let max = max_hits.max(1).to_string();
        // blastn: more sensitive than the default method (megablast); very short queries need blastn-short
        let task = if query.len() < 30 {
            "blastn-short"
        } else {
            "blastn"
        };
        let mut args = vec![
            "-query",
            "query.fa",
            "-db",
            "db",
            "-outfmt",
            "6",
            "-max_target_seqs",
            &max,
        ];
        if program == "blastn" {
            args.extend(["-task", task]);
        }
        let out = run(program, &args)?;
        let mut hits = parse_tabular(&out)?;
        for h in &mut hits {
            h.query = query.name().to_string();
            if let Some(i) = h
                .subject
                .strip_prefix('s')
                .and_then(|n| n.parse::<usize>().ok())
            {
                if let Some(s) = subjects.get(i) {
                    h.subject = s.name().to_string();
                }
            }
        }
        sort_hits(&mut hits);
        hits.truncate(max_hits.max(1));
        Ok(hits)
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::SequenceType;
    use crate::tools::is_available;

    const TABELLE: &str = "q1\ts1\t100.000\t50\t0\t0\t1\t50\t101\t150\t1e-20\t92.5\n\
# Kommentar\n\
\n\
q1\ts2\t90.5\t40\t4\t0\t5\t44\t300\t261\t0.0\t80\n";

    #[test]
    fn tabelle_lesen() {
        let h = parse_tabular(TABELLE).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].query, "q1");
        assert_eq!(h[0].subject, "s1");
        assert!((h[0].identity_percent - 100.0).abs() < 1e-9);
        assert_eq!(
            (h[0].alignment_length, h[0].mismatches, h[0].gap_opens),
            (50, 0, 0)
        );
        assert_eq!(
            (
                h[0].query_start,
                h[0].query_end,
                h[0].subject_start,
                h[0].subject_end
            ),
            (1, 50, 101, 150)
        );
        assert!((h[0].evalue - 1e-20).abs() < 1e-30);
        assert!((h[0].bit_score - 92.5).abs() < 1e-9);
        assert!(!h[0].reverse_strand());
        assert!(h[1].reverse_strand());
        assert_eq!(h[1].evalue, 0.0);
        let w = parse_tabular(&TABELLE.replace('\n', "\r\n")).unwrap();
        assert_eq!(w, h);
        assert!(parse_tabular("").unwrap().is_empty());
    }

    #[test]
    fn tabelle_fehler() {
        match parse_tabular("# x\nq\ts\t100\t5\t0\t0\t1\t5\t1\t5\n") {
            Err(Error::InvalidInput(m)) => assert!(m.contains('2'), "{}", m),
            other => panic!("unerwartet: {:?}", other),
        }
        assert!(matches!(
            parse_tabular("q\ts\tx\t5\t0\t0\t1\t5\t1\t5\t0\t10\n"),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn sortierung() {
        let mut h = parse_tabular(
            "q\tb\t90\t10\t0\t0\t1\t10\t1\t10\t1e-5\t50\nq\ta\t90\t10\t0\t0\t1\t10\t1\t10\t1e-5\t50\nq\tc\t90\t10\t0\t0\t1\t10\t1\t10\t1e-9\t50\nq\td\t90\t10\t0\t0\t1\t10\t1\t10\t1e-1\t70\n",
        )
        .unwrap();
        sort_hits(&mut h);
        let names: Vec<&str> = h.iter().map(|x| x.subject.as_str()).collect();
        assert_eq!(names, vec!["d", "c", "a", "b"]);
    }

    #[test]
    fn lokale_suche() {
        if !is_available("blastn") || !is_available("makeblastdb") {
            eprintln!("skipped: BLAST+ not installed");
            return;
        }
        let gen = "ATGGCGTACGTTAGCTAGCTAGGATCCGATCGATCGTACGATCGTAGCTAGCTAGCTAGCATCGATCGGCTAAGCTT";
        let subjects = vec![
            Sequence::new(
                "a",
                "a",
                "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
                SequenceType::Dna,
            )
            .unwrap(),
            Sequence::new(
                "b gen",
                "b gen",
                &format!("CCCCCCCCCC{}CCCCCCCCCC", gen),
                SequenceType::Dna,
            )
            .unwrap(),
        ];
        let query = Sequence::new("frage", "frage", &gen[10..60], SequenceType::Dna).unwrap();
        let hits = blast_local("blastn", &query, &subjects, 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].query, "frage");
        assert_eq!(hits[0].subject, "b gen");
        assert!(hits[0].identity_percent > 99.0);
        assert_eq!((hits[0].subject_start, hits[0].subject_end), (21, 70));
        // opposite strand
        let rev = Sequence::new(
            "rev",
            "rev",
            &crate::cloning::enzymes::iupac_reverse_complement(&gen[10..60]),
            SequenceType::Dna,
        )
        .unwrap();
        let h2 = blast_local("blastn", &rev, &subjects, 5).unwrap();
        assert!(h2[0].reverse_strand());
        assert!(blast_local("blastn", &query, &[], 5).is_err());
        assert!(blast_local("tblastx", &query, &subjects, 5).is_err());
    }
}
