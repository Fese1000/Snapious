//! Externe Programme für Alignment (MAFFT, MUSCLE, Clustal Omega) mit einheitlicher Schnittstelle.
//!
//! Die Sequenznamen werden vor dem Aufruf durch `s0`, `s1`, … ersetzt und danach wieder zurückgesetzt, damit Leer- und Sonderzeichen
//! in Namen keine Probleme machen.

use std::time::Duration;

use crate::bio::msa::progressive_alignment;
use crate::error::{Error, Result};
use crate::model::sequence::Sequence;
use crate::tools::ToolRegistry;

/// Wahl des Alignment-Verfahrens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignBackend {
    /// Eingebautes progressives Alignment.
    Builtin,
    Mafft,
    Muscle,
    ClustalOmega,
}

impl AlignBackend {
    /// Name aus der Oberfläche: `builtin`, `mafft`, `muscle`, `clustalo`.
    pub fn from_name(name: &str) -> Option<AlignBackend> {
        match name {
            "builtin" => Some(AlignBackend::Builtin),
            "mafft" => Some(AlignBackend::Mafft),
            "muscle" => Some(AlignBackend::Muscle),
            "clustalo" => Some(AlignBackend::ClustalOmega),
            _ => None,
        }
    }
}

/// Liest FASTA-Text mit mehrzeiligen Sequenzen in `(Kopfzeile bis zum ersten Leerzeichen, Sequenz in Großbuchstaben)`.
pub fn parse_fasta_rows(text: &str) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('>') {
            let name = header.split_whitespace().next().unwrap_or("").to_string();
            rows.push((name, String::new()));
        } else if let Some((_, seq)) = rows.last_mut() {
            seq.push_str(&line.to_ascii_uppercase());
        }
    }
    rows
}

fn fasta_for(seqs: &[Sequence]) -> String {
    seqs.iter()
        .enumerate()
        .map(|(i, s)| format!(">s{}\n{}\n", i, s.bases()))
        .collect()
}

/// Ordnet die Ausgabezeilen (`s0`, `s1`, …) den Eingabesequenzen zu und gibt `(Originalname, Zeile)` in Eingabereihenfolge zurück.
fn map_back(seqs: &[Sequence], rows: Vec<(String, String)>) -> Result<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(seqs.len());
    for (i, s) in seqs.iter().enumerate() {
        let key = format!("s{}", i);
        let row = rows.iter().find(|(n, _)| *n == key).ok_or_else(|| {
            Error::Io(format!(
                "Das Programm lieferte keine Zeile für '{}'",
                s.name()
            ))
        })?;
        out.push((s.name().to_string(), row.1.clone()));
    }
    let width = out.first().map_or(0, |r| r.1.chars().count());
    if out.iter().any(|r| r.1.chars().count() != width) {
        return Err(Error::Io(
            "Das Programm lieferte Zeilen unterschiedlicher Länge".to_string(),
        ));
    }
    Ok(out)
}

fn run_checked(
    tool: &str,
    args: &[&str],
    stdin: Option<&str>,
    dir: Option<&std::path::Path>,
    secs: u64,
) -> Result<String> {
    let reg = ToolRegistry::defaults();
    if !reg.is_available(tool) {
        return Err(Error::Io(format!(
            "Programm '{}' ist nicht installiert",
            tool
        )));
    }
    let r = reg.run(tool, args, dir, stdin, Duration::from_secs(secs))?;
    if !r.ok() {
        return Err(Error::Io(format!(
            "'{}' meldete einen Fehler: {}",
            tool,
            r.stderr.trim()
        )));
    }
    Ok(r.stdout)
}

/// Richtet Sequenzen aus; Rückgabe `(Name, ausgerichtete Zeile)` in Eingabereihenfolge. Mindestens zwei Sequenzen nötig
/// (`Error::InvalidInput`). Nicht installierte Programme ergeben `Error::Io`.
pub fn align_with(backend: AlignBackend, seqs: &[Sequence]) -> Result<Vec<(String, String)>> {
    if seqs.len() < 2 {
        return Err(Error::InvalidInput(
            "Für ein Alignment sind mindestens zwei Sequenzen nötig".to_string(),
        ));
    }
    if backend == AlignBackend::Builtin {
        let r = progressive_alignment(seqs)?;
        return Ok(r
            .sequences
            .into_iter()
            .map(|s| (s.name, s.aligned))
            .collect());
    }
    let input = fasta_for(seqs);
    let output = match backend {
        AlignBackend::Mafft => run_checked(
            "mafft",
            &["--quiet", "--auto", "-"],
            Some(&input),
            None,
            600,
        )?,
        AlignBackend::ClustalOmega => run_checked(
            "clustalo",
            &["-i", "-", "--outfmt=fasta", "--force"],
            Some(&input),
            None,
            600,
        )?,
        AlignBackend::Muscle => {
            let dir =
                std::env::temp_dir().join(format!("snapious-muscle-{}", crate::util::new_id()));
            std::fs::create_dir_all(&dir)?;
            let inp = dir.join("in.fa");
            let outp = dir.join("out.fa");
            std::fs::write(&inp, &input)?;
            let res = run_checked(
                "muscle",
                &[
                    "-align",
                    &inp.to_string_lossy(),
                    "-output",
                    &outp.to_string_lossy(),
                ],
                None,
                None,
                600,
            );
            let text = std::fs::read_to_string(&outp);
            let _ = std::fs::remove_dir_all(&dir);
            res?;
            text?
        }
        AlignBackend::Builtin => String::new(),
    };
    map_back(seqs, parse_fasta_rows(&output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::SequenceType;
    use crate::tools::is_available;

    fn seqs() -> Vec<Sequence> {
        [
            ("a", "ATGCGTACGTAGCTAGCTAGGATCC"),
            ("b mit Leerzeichen", "ATGCGTACGTAGCTAGCTAGGATCC"),
            ("c", "ATGCGTAGCTAGCTAGGATCC"),
            ("d", "ATGCGTACGAAGCTAGCTAGGTTCC"),
        ]
        .iter()
        .map(|(n, b)| Sequence::new(n, n, b, SequenceType::Dna).unwrap())
        .collect()
    }

    #[test]
    fn namen_und_fasta() {
        assert_eq!(AlignBackend::from_name("mafft"), Some(AlignBackend::Mafft));
        assert_eq!(AlignBackend::from_name("x"), None);
        let rows = parse_fasta_rows(">s0 beschreibung\nacg\nTT\n>s1\nGG\n");
        assert_eq!(
            rows,
            vec![
                ("s0".to_string(), "ACGTT".to_string()),
                ("s1".to_string(), "GG".to_string())
            ]
        );
        assert!(parse_fasta_rows("").is_empty());
    }

    #[test]
    fn eingebautes_alignment_und_fehler() {
        let r = align_with(AlignBackend::Builtin, &seqs()).unwrap();
        assert_eq!(r.len(), 4);
        assert_eq!(r[1].0, "b mit Leerzeichen");
        assert!(r.iter().all(|x| x.1.len() == r[0].1.len()));
        assert!(matches!(
            align_with(AlignBackend::Mafft, &seqs()[..1]),
            Err(Error::InvalidInput(_))
        ));
    }

    fn pruefe_aligner(backend: AlignBackend, tool: &str) {
        if !is_available(tool) {
            eprintln!("übersprungen: {} nicht installiert", tool);
            return;
        }
        let input = seqs();
        let r = align_with(backend, &input).unwrap();
        assert_eq!(r.len(), 4);
        assert_eq!(r[1].0, "b mit Leerzeichen");
        assert!(r.iter().all(|x| x.1.len() == r[0].1.len()));
        for (s, (_, row)) in input.iter().zip(&r) {
            assert_eq!(row.replace('-', ""), s.bases().to_ascii_uppercase());
        }
    }

    #[test]
    fn mafft() {
        pruefe_aligner(AlignBackend::Mafft, "mafft");
    }

    #[test]
    fn muscle() {
        pruefe_aligner(AlignBackend::Muscle, "muscle");
    }

    #[test]
    fn clustal_omega() {
        pruefe_aligner(AlignBackend::ClustalOmega, "clustalo");
    }
}
