//! Aufruf von SPAdes (De-novo-Assembly): Kommandozeile bauen und das Ergebnis `contigs.fasta` in Dokumente umsetzen.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::io::fasta;
use crate::model::document::SequenceDocument;

/// Argumente für `spades.py`: mit zwei Dateien `-1 <reads1> -2 <reads2> -o <work_dir> -t <threads>`, mit einer Datei
/// `-s <reads1> -o <work_dir> -t <threads>` (ungepaart). `threads` wird mindestens 1. Programmname nicht enthalten.
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

/// Liest Länge und Abdeckung aus einem SPAdes-Contignamen `NODE_<n>_length_<länge>_cov_<abdeckung>` (Abdeckung als Dezimalzahl, ein
/// Rest wie `_1` hinter der Zahl wird nicht erwartet). `None`, wenn das Muster nicht passt.
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

/// Setzt den Text von `contigs.fasta` in Dokumente um: je Eintrag ein DNA-Dokument mit dem Namen des FASTA-Eintrags (erstes Wort der Kopfzeile)
/// und Metadaten `length` (Zahl der Basen als Text), und, wenn `parse_node_name` passt, `coverage` (Abdeckung als Text mit `to_string` der Zahl, z. B. `5.5`)
/// sowie `source = "spades"`. Ein Text ohne Einträge ergibt `Error::InvalidInput("SPAdes returned no contigs")`; FASTA-Fehler werden weitergegeben.
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
