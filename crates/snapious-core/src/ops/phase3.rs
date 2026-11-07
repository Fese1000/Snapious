//! Alignment- und Baum-Operationen für Registry und Workflows: Profil verbinden, Translations-Alignment, Bereich neu ausrichten,
//! Baum wurzeln, Baum einklappen.

use crate::bio::align_tools::{align_profiles, realign_region, translation_align};
use crate::bio::backends::{align_with, AlignBackend};
use crate::error::{Error, Result};
use crate::model::alignment::AlignmentDocument;
use crate::model::document::Document;
use crate::model::sequence::{Sequence, SequenceType};
use crate::ops::operation::{required_usize, Operation, OperationContext, OperationResult, Params};

/// Fehlertext der Kernfunktionen als `Error::Operation`.
fn op_err(e: impl std::fmt::Display) -> Error {
    Error::Operation(e.to_string())
}

/// Zerlegt eine Liste wie `0, 2;3` (Trenner Komma, Semikolon oder Leerzeichen) in nicht leere Teile.
fn split_list(text: &str) -> Vec<String> {
    text.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

/// Verwendet das eingebaute Alignment (`align_with(AlignBackend::Builtin, ...)`) für Zeilenlisten.
/// Die Zeilen werden als DNA-Sequenzen mit den Namen `s1`, `s2`, … übergeben (Uppercase, nur A, C, G, T, sonst Fehler
/// `Error::Operation`); Rückgabe: die ausgerichteten Zeilen in Eingabereihenfolge. Weniger als zwei Zeilen: unverändert
/// (ungapped) zurückgeben.
fn builtin_rows(rows: &[String]) -> Result<Vec<String>> {
    if rows.len() < 2 {
        return Ok(rows.to_vec());
    }
    let seqs = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let n = format!("s{}", i + 1);
            Sequence::new(&n, &n, &r.to_ascii_uppercase(), SequenceType::Dna)
        })
        .collect::<Result<Vec<_>>>()
        .map_err(op_err)?;
    let aligned = align_with(AlignBackend::Builtin, &seqs).map_err(op_err)?;
    Ok(aligned.into_iter().map(|(_, r)| r).collect())
}

/// `align.profile`: zwei Alignments verbinden. Eingaben: genau zwei Alignment-Dokumente `a`, `b` (gleicher Sequenztyp), sonst
/// `Error::Operation`. Ergebnis: ein Alignment `{a} + {b} (Profil)` mit den Zeilennamen von `a`, dann `b`, Zeilen aus `align_profiles`;
/// `summary` = `"Profile verbunden: {n} Zeilen, {m} Spalten"`.
pub struct ProfileOperation;

impl Operation for ProfileOperation {
    fn id(&self) -> &str {
        "align.profile"
    }

    fn label(&self) -> &str {
        "Join profiles"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let [Document::Alignment(a), Document::Alignment(b)] = inputs else {
            return Err(Error::Operation(
                "Exactly two alignments expected".to_string(),
            ));
        };
        if a.seq_type() != b.seq_type() {
            return Err(Error::Operation(
                "The alignments have different sequence types".to_string(),
            ));
        }
        let rows = align_profiles(a.rows(), b.rows()).map_err(op_err)?;
        let mut names = a.row_names().to_vec();
        names.extend(b.row_names().iter().cloned());
        let out = AlignmentDocument::of(
            &format!("{} + {} (profile)", a.name(), b.name()),
            a.seq_type(),
            names,
            rows,
        )
        .map_err(op_err)?;
        let summary = format!(
            "Profiles joined: {} rows, {} columns",
            out.row_count(),
            out.column_count()
        );
        Ok(OperationResult {
            outputs: vec![Document::Alignment(out)],
            summary,
        })
    }
}

/// `align.translate`: Translations-Alignment. Eingaben: mindestens zwei Sequenzdokumente (DNA). Verwendet `translation_align`
/// mit einem Aligner, der die Proteine mit `align_with(AlignBackend::Builtin, ...)` ausrichtet (Proteinsequenzen als
/// `SequenceType::Protein`). Ergebnis: ein DNA-Alignment `Translations-Alignment` mit den Dokumentnamen als Zeilennamen;
/// `summary` = `"Translations-Alignment: {n} Zeilen"`. Fehler der Kernfunktionen als `Error::Operation`.
pub struct TranslateAlignOperation;

impl Operation for TranslateAlignOperation {
    fn id(&self) -> &str {
        "align.translate"
    }

    fn label(&self) -> &str {
        "Translation alignment"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        if inputs.len() < 2 {
            return Err(Error::Operation(
                "At least two sequences required".to_string(),
            ));
        }
        let mut rows = Vec::new();
        for d in inputs {
            let Document::Sequence(s) = d else {
                return Err(Error::Operation(
                    "All inputs must be sequences".to_string(),
                ));
            };
            rows.push((s.name().to_string(), s.bases().to_ascii_uppercase()));
        }
        let aligner = |proteins: &[(String, String)]| -> Result<Vec<(String, String)>> {
            let seqs = proteins
                .iter()
                .map(|(n, p)| Sequence::new(n, n, p, SequenceType::Protein))
                .collect::<Result<Vec<_>>>()?;
            align_with(AlignBackend::Builtin, &seqs)
        };
        let aligned = translation_align(&rows, &aligner).map_err(op_err)?;
        let (names, lines): (Vec<String>, Vec<String>) = aligned.into_iter().unzip();
        let n = names.len();
        let out = AlignmentDocument::of("Translation alignment", SequenceType::Dna, names, lines)
            .map_err(op_err)?;
        Ok(OperationResult {
            outputs: vec![Document::Alignment(out)],
            summary: format!("Translation alignment: {} rows", n),
        })
    }
}

/// `align.realign`: Bereich neu ausrichten. Eingabe: ein Alignment. Parameter `from` und `to` (Pflicht, Spalten 0-basiert,
/// `to` exklusiv), optional `rows` (Liste von Zeilenindizes, Standard alle). Verwendet `realign_region` mit `builtin_rows`.
/// Ergebnis: Alignment `{Name} (neu ausgerichtet)`, gleiche Zeilennamen und Typ; `summary` = `"Bereich {from}–{to} neu ausgerichtet"`.
/// Ungültige Zahlen oder Bereiche → `Error::Operation`.
pub struct RealignOperation;

impl Operation for RealignOperation {
    fn id(&self) -> &str {
        "align.realign"
    }

    fn label(&self) -> &str {
        "Realign range"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let [Document::Alignment(a)] = inputs else {
            return Err(Error::Operation(
                "Exactly one alignment expected".to_string(),
            ));
        };
        let from = required_usize(params, "from")?;
        let to = required_usize(params, "to")?;
        let selected: Vec<usize> = match params.get("rows") {
            Some(text) => split_list(text)
                .iter()
                .map(|p| {
                    p.parse::<usize>()
                        .map_err(|_| Error::Operation(format!("Invalid row index '{}'", p)))
                })
                .collect::<Result<Vec<_>>>()?,
            None => (0..a.row_count()).collect(),
        };
        let rows = realign_region(a.rows(), from, to, &selected, &builtin_rows).map_err(op_err)?;
        let out = AlignmentDocument::of(
            &format!("{} (realigned)", a.name()),
            a.seq_type(),
            a.row_names().to_vec(),
            rows,
        )
        .map_err(op_err)?;
        Ok(OperationResult {
            outputs: vec![Document::Alignment(out)],
            summary: format!("Range {}–{} realigned", from, to),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{SequenceDocument, Topology};
    use crate::ops::operation::NullContext;

    fn run(op: &dyn Operation, inputs: &[Document], p: &Params) -> Result<OperationResult> {
        op.run(inputs, p, &mut NullContext)
    }

    fn params(items: &[(&str, &str)]) -> Params {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn aln(name: &str, names: &[&str], rows: &[&str]) -> Document {
        Document::Alignment(
            AlignmentDocument::of(
                name,
                SequenceType::Dna,
                names.iter().map(|s| s.to_string()).collect(),
                rows.iter().map(|s| s.to_string()).collect(),
            )
            .unwrap(),
        )
    }

    fn dna(name: &str, bases: &str) -> Document {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of(name, seq).with_topology(Topology::Linear))
    }

    fn alignment_of(r: &OperationResult) -> &AlignmentDocument {
        match &r.outputs[0] {
            Document::Alignment(a) => a,
            _ => panic!("kein Alignment"),
        }
    }

    #[test]
    fn liste_zerlegen() {
        assert_eq!(split_list("0, 2;3"), vec!["0", "2", "3"]);
        assert!(split_list(" ").is_empty());
    }

    #[test]
    fn profil_verbinden() {
        let a = aln("A", &["n1", "n2"], &["ACGT", "ACGT"]);
        let b = aln("B", &["m1"], &["AGT"]);
        let r = run(&ProfileOperation, &[a, b], &Params::new()).unwrap();
        let out = alignment_of(&r);
        assert_eq!(out.name(), "A + B (profile)");
        assert_eq!(out.row_names(), ["n1", "n2", "m1"]);
        assert_eq!(out.rows(), ["ACGT", "ACGT", "A-GT"]);
        assert_eq!(r.summary, "Profiles joined: 3 rows, 4 columns");
        assert!(run(
            &ProfileOperation,
            &[aln("A", &["n"], &["AC"])],
            &Params::new()
        )
        .is_err());
    }

    #[test]
    fn translations_alignment() {
        let inputs = [dna("a", "ATGAAATTT"), dna("b", "ATGAAGTTT")];
        let r = run(&TranslateAlignOperation, &inputs, &Params::new()).unwrap();
        let out = alignment_of(&r);
        assert_eq!(out.row_names(), ["a", "b"]);
        assert_eq!(out.rows(), ["ATGAAATTT", "ATGAAGTTT"]);
        assert_eq!(r.summary, "Translation alignment: 2 rows");
        let bad = [dna("a", "ATGAAATTT"), dna("kurz", "ATGA")];
        let e = run(&TranslateAlignOperation, &bad, &Params::new()).unwrap_err();
        assert!(e.to_string().contains("kurz"));
        assert!(run(&TranslateAlignOperation, &inputs[..1], &Params::new()).is_err());
    }

    #[test]
    fn bereich_neu() {
        let a = aln("A", &["x", "y"], &["AAAATTTT", "AAAATTTT"]);
        let p = params(&[("from", "2"), ("to", "6")]);
        let r = run(&RealignOperation, std::slice::from_ref(&a), &p).unwrap();
        let out = alignment_of(&r);
        assert_eq!(out.name(), "A (realigned)");
        assert_eq!(out.rows()[0].len(), out.rows()[1].len());
        for i in 0..2 {
            assert_eq!(out.ungapped(i).unwrap(), "AAAATTTT");
        }
        assert_eq!(r.summary, "Range 2–6 realigned");
        let bad = params(&[("from", "6"), ("to", "2")]);
        assert!(run(&RealignOperation, std::slice::from_ref(&a), &bad).is_err());
        assert!(run(&RealignOperation, &[a], &Params::new()).is_err());
    }
}
