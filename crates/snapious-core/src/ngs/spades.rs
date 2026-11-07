//! Calling SPAdes (de novo assembly): build the command line and convert the result `contigs.fasta` into documents.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::io::fasta;
use crate::model::document::SequenceDocument;

/// Arguments for `spades.py`: with two files `-1 <reads1> -2 <reads2> -o <work_dir> -t <threads>`, with one file
/// `-s <reads1> -o <work_dir> -t <threads>` (unpaired). `threads` becomes at least 1. Program name not included.
pub fn spades_args(
    reads1: &str,
    reads2: Option<&str>,
    work_dir: &str,
    threads: usize,
) -> Vec<String> {
    let t = std::cmp::max(threads, 1);
    match reads2 {
        Some(r2) => vec![
            "-1".to_string(),
            reads1.to_string(),
            "-2".to_string(),
            r2.to_string(),
            "-o".to_string(),
            work_dir.to_string(),
            "-t".to_string(),
            t.to_string(),
        ],
        None => vec![
            "-s".to_string(),
            reads1.to_string(),
            "-o".to_string(),
            work_dir.to_string(),
            "-t".to_string(),
            t.to_string(),
        ],
    }
}

/// Reads length and coverage from a SPAdes contig name `NODE_<n>_length_<length>_cov_<coverage>` (coverage as a decimal number, a
/// remainder such as `_1` after the number is not expected). `None` if the pattern does not match.
pub fn parse_node_name(name: &str) -> Option<(u64, f64)> {
    let parts: Vec<&str> = name.split('_').collect();
    if parts.len() != 6 {
        return None;
    }
    if parts[0] != "NODE" || parts[2] != "length" || parts[4] != "cov" {
        return None;
    }
    let length: u64 = parts[3].parse().ok()?;
    let coverage: f64 = parts[5].parse().ok()?;
    Some((length, coverage))
}

/// Converts the text of `contigs.fasta` into documents: one DNA document per entry with the name of the FASTA entry (first word of the header line)
/// and metadata `length` (number of bases as text) and, if `parse_node_name` matches, `coverage` (coverage as text via `to_string` of the number, e.g. `5.5`)
/// as well as `source = "spades"`. A text without entries yields `Error::InvalidInput("SPAdes returned no contigs")`; FASTA errors are passed on.
pub fn contigs_to_documents(fasta_text: &str) -> Result<Vec<SequenceDocument>> {
    let sequences = fasta::parse(fasta_text)?;
    if sequences.is_empty() {
        return Err(Error::InvalidInput(
            "SPAdes returned no contigs".to_string(),
        ));
    }

    let mut docs = Vec::with_capacity(sequences.len());
    for seq in sequences {
        let name = seq.id().to_string();
        let mut meta = BTreeMap::new();
        meta.insert("length".to_string(), seq.bases().len().to_string());

        if let Some((_, cov)) = parse_node_name(&name) {
            meta.insert("coverage".to_string(), cov.to_string());
        }
        meta.insert("source".to_string(), "spades".to_string());

        let doc = SequenceDocument::of(&name, seq).with_metadata(meta);
        docs.push(doc);
    }
    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn kommandozeile() {
        assert_eq!(
            spades_args("a.fq", Some("b.fq"), "work", 4),
            s(&["-1", "a.fq", "-2", "b.fq", "-o", "work", "-t", "4"])
        );
        assert_eq!(
            spades_args("a.fq", None, "work", 0),
            s(&["-s", "a.fq", "-o", "work", "-t", "1"])
        );
    }

    #[test]
    fn contigname() {
        assert_eq!(
            parse_node_name("NODE_1_length_100_cov_5.5"),
            Some((100, 5.5))
        );
        assert_eq!(
            parse_node_name("NODE_12_length_1234_cov_56.789"),
            Some((1234, 56.789))
        );
        assert_eq!(parse_node_name("NODE_2_length_50_cov_2"), Some((50, 2.0)));
        assert_eq!(parse_node_name("contig1"), None);
        assert_eq!(parse_node_name("NODE_1_length_x_cov_5"), None);
        assert_eq!(parse_node_name("NODE_1_length_10"), None);
    }

    #[test]
    fn dokumente() {
        let fa = ">NODE_1_length_8_cov_5.5\nACGTACGT\n>NODE_2_length_4_cov_2.0 Kommentar\nGGCC\n>frei\nTTTT\n";
        let d = contigs_to_documents(fa).unwrap();
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].name(), "NODE_1_length_8_cov_5.5");
        assert_eq!(d[0].len(), 8);
        assert_eq!(d[0].metadata().get("length").map(String::as_str), Some("8"));
        assert_eq!(
            d[0].metadata().get("coverage").map(String::as_str),
            Some("5.5")
        );
        assert_eq!(
            d[0].metadata().get("source").map(String::as_str),
            Some("spades")
        );
        assert_eq!(d[1].name(), "NODE_2_length_4_cov_2.0");
        assert_eq!(
            d[1].metadata().get("coverage").map(String::as_str),
            Some("2")
        );
        assert_eq!(d[2].metadata().get("coverage"), None);
        assert_eq!(d[2].metadata().get("length").map(String::as_str), Some("4"));
        assert!(matches!(
            contigs_to_documents(""),
            Err(Error::InvalidInput(_))
        ));
    }
}
