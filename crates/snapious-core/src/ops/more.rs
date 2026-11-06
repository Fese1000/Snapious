//! Weitere eingebaute Operationen (Bearbeiten, Alignment, Bäume, Klonierung, Translation) für Registry und Workflows.

use crate::bio::align_stats::{consensus, strip_columns};
use crate::bio::backends::{align_with, AlignBackend};
use crate::cloning::enzymes::EnzymeDb;
use crate::cloning::fragment::digest;
use crate::cloning::golden_gate;
use crate::cloning::overlap::OverlapAssembly;
use crate::error::{Error, Result};
use crate::model::alignment::AlignmentDocument;
use crate::model::document::{Document, SequenceDocument, Topology};
use crate::model::sequence::{Sequence, SequenceType};
use crate::ops::operation::{
    required, required_usize, Operation, OperationContext, OperationResult, Params,
};
use crate::view::translation::translate_feature;

fn op_err(e: impl std::fmt::Display) -> Error {
    Error::Operation(e.to_string())
}

fn sequences(inputs: &[Document]) -> Result<Vec<&SequenceDocument>> {
    let mut out = Vec::new();
    for d in inputs {
        match d {
            Document::Sequence(s) => out.push(s),
            _ => {
                return Err(Error::Operation(
                    "Alle Eingaben müssen Sequenzen sein".to_string(),
                ))
            }
        }
    }
    if out.is_empty() {
        return Err(Error::Operation(
            "Es wurde keine Sequenz übergeben".to_string(),
        ));
    }
    Ok(out)
}

fn first_alignment(inputs: &[Document]) -> Result<&AlignmentDocument> {
    match inputs.first() {
        Some(Document::Alignment(a)) => Ok(a),
        _ => Err(Error::Operation(
            "Es wurde kein Alignment übergeben".to_string(),
        )),
    }
}

fn result(doc: Document, summary: String) -> Result<OperationResult> {
    Ok(OperationResult {
        outputs: vec![doc],
        summary,
    })
}

fn flag(params: &Params, key: &str, default: bool) -> bool {
    params
        .get(key)
        .map_or(default, |v| matches!(v.trim(), "true" | "1" | "ja"))
}

fn names(text: &str) -> Vec<String> {
    text.split([',', ';', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

macro_rules! simple_op {
    ($name:ident, $id:expr, $label:expr, |$inputs:ident, $params:ident, $ctx:ident| $body:block) => {
        pub struct $name;

        impl Operation for $name {
            fn id(&self) -> &str {
                $id
            }

            fn label(&self) -> &str {
                $label
            }

            fn run(&self, $inputs: &[Document], $params: &Params, $ctx: &mut dyn OperationContext) -> Result<OperationResult> $body
        }
    };
}

simple_op!(
    DeleteOperation,
    "edit.delete",
    "Basen löschen",
    |inputs, params, _ctx| {
        let d = sequences(inputs)?[0];
        let (s, e) = (
            required_usize(params, "start")?,
            required_usize(params, "end")?,
        );
        let out = d
            .delete(s, e)
            .map_err(op_err)?
            .derive(&format!("{} (bearbeitet)", d.name()));
        result(
            Document::Sequence(out),
            format!("{} Basen gelöscht", e.saturating_sub(s)),
        )
    }
);

simple_op!(
    ReplaceOperation,
    "edit.replace",
    "Basen ersetzen",
    |inputs, params, _ctx| {
        let d = sequences(inputs)?[0];
        let (s, e) = (
            required_usize(params, "start")?,
            required_usize(params, "end")?,
        );
        let out = d
            .replace(s, e, required(params, "bases")?)
            .map_err(op_err)?
            .derive(&format!("{} (bearbeitet)", d.name()));
        result(
            Document::Sequence(out),
            format!("Bereich {}-{} ersetzt", s, e),
        )
    }
);

simple_op!(
    ExtractOperation,
    "edit.extract",
    "Bereich ausschneiden",
    |inputs, params, _ctx| {
        let d = sequences(inputs)?[0];
        let (s, e) = (
            required_usize(params, "start")?,
            required_usize(params, "end")?,
        );
        let out = d.sub_document(s, e).map_err(op_err)?.derive(&format!(
            "{} [{}-{}]",
            d.name(),
            s + 1,
            e
        ));
        result(
            Document::Sequence(out),
            format!("Bereich {}-{} extrahiert", s + 1, e),
        )
    }
);

simple_op!(
    TopologyOperation,
    "edit.topology",
    "Topologie ändern",
    |inputs, params, _ctx| {
        let d = sequences(inputs)?[0];
        let t = match required(params, "value")?.trim() {
            "circular" => Topology::Circular,
            "linear" => Topology::Linear,
            other => {
                return Err(Error::Operation(format!(
                    "Unbekannte Topologie '{}' (circular oder linear)",
                    other
                )))
            }
        };
        let out = d.with_topology(t).derive(&format!(
            "{} ({})",
            d.name(),
            if t == Topology::Circular {
                "zirkulär"
            } else {
                "linear"
            }
        ));
        result(Document::Sequence(out), "Topologie geändert".to_string())
    }
);

simple_op!(
    AlignOperation,
    "align.multiple",
    "Mehrfachalignment",
    |inputs, params, _ctx| {
        let docs = sequences(inputs)?;
        let backend_name = params.get("backend").map_or("builtin", |s| s.as_str());
        let backend = AlignBackend::from_name(backend_name).ok_or_else(|| {
            Error::Operation(format!(
                "Unbekanntes Alignment-Verfahren '{}'",
                backend_name
            ))
        })?;
        let seqs: Vec<Sequence> = docs.iter().map(|d| d.sequence().clone()).collect();
        let aligned = align_with(backend, &seqs).map_err(op_err)?;
        let seq_type = seqs.first().map_or(SequenceType::Dna, |s| s.seq_type());
        let doc = AlignmentDocument::of(
            "Alignment",
            seq_type,
            aligned.iter().map(|r| r.0.clone()).collect(),
            aligned.iter().map(|r| r.1.clone()).collect(),
        )
        .map_err(op_err)?;
        result(
            Document::Alignment(doc),
            format!("{} Sequenzen ausgerichtet ({})", seqs.len(), backend_name),
        )
    }
);

simple_op!(
    ConsensusOperation,
    "align.consensus",
    "Konsens",
    |inputs, params, _ctx| {
        let a = first_alignment(inputs)?;
        let threshold = params
            .get("threshold")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.5);
        let text: String = consensus(a.rows(), threshold, true)
            .map_err(op_err)?
            .chars()
            .filter(|c| *c != '-')
            .collect();
        let name = format!("{} Konsens", a.name());
        let seq = Sequence::new(&name, &name, &text, a.seq_type()).map_err(op_err)?;
        result(
            Document::Sequence(SequenceDocument::of(&name, seq)),
            "Konsens berechnet".to_string(),
        )
    }
);

simple_op!(
    StripOperation,
    "align.strip",
    "Lückenspalten entfernen",
    |inputs, params, _ctx| {
        let a = first_alignment(inputs)?;
        let threshold = params
            .get("threshold")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(1.0);
        let rows = strip_columns(a.rows(), threshold).map_err(op_err)?;
        let doc = AlignmentDocument::of(
            &format!("{} (bereinigt)", a.name()),
            a.seq_type(),
            a.row_names().to_vec(),
            rows,
        )
        .map_err(op_err)?;
        result(Document::Alignment(doc), "Spalten entfernt".to_string())
    }
);

simple_op!(
    DigestOperation,
    "clone.digest",
    "Restriktionsverdau",
    |inputs, params, _ctx| {
        let d = sequences(inputs)?[0];
        let enzymes = names(required(params, "enzymes")?);
        let refs: Vec<&str> = enzymes.iter().map(String::as_str).collect();
        let fragments = digest(d, &EnzymeDb::standard(), &refs).map_err(op_err)?;
        let mut outputs = Vec::new();
        for (i, f) in fragments.iter().enumerate() {
            let name = format!("{} Fragment {}", d.name(), i + 1);
            let seq = Sequence::new(&name, &name, &f.top, SequenceType::Dna).map_err(op_err)?;
            let doc = SequenceDocument::of(&name, seq).with_features(f.features.clone());
            outputs.push(Document::Sequence(doc));
        }
        let n = outputs.len();
        Ok(OperationResult {
            outputs,
            summary: format!("{} Fragmente", n),
        })
    }
);

simple_op!(
    OverlapOperation,
    "clone.overlap",
    "Gibson-Zusammenbau",
    |inputs, params, _ctx| {
        let docs: Vec<SequenceDocument> = sequences(inputs)?.into_iter().cloned().collect();
        let assembly = match params.get("method").map_or("gibson", |s| s.as_str()) {
            "gibson" => OverlapAssembly::gibson(),
            "infusion" => OverlapAssembly::in_fusion(),
            "slic" => OverlapAssembly::slic(),
            other => return Err(Error::Operation(format!("Unbekannte Methode '{}'", other))),
        };
        let r = assembly
            .assemble(&docs, flag(params, "circular", true))
            .map_err(op_err)?;
        result(
            Document::Sequence(r.construct),
            format!("Zusammenbau mit {} Überlappen", r.overlaps.len()),
        )
    }
);

simple_op!(
    GoldenGateOperation,
    "clone.goldengate",
    "Golden-Gate-Zusammenbau",
    |inputs, params, _ctx| {
        let docs: Vec<SequenceDocument> = sequences(inputs)?.into_iter().cloned().collect();
        let enzyme = params.get("enzyme").map_or("BsaI", |s| s.as_str());
        let r = golden_gate::assemble(
            &EnzymeDb::standard(),
            enzyme,
            &docs,
            flag(params, "circular", true),
        )
        .map_err(op_err)?;
        result(
            Document::Sequence(r.construct),
            format!("Golden Gate mit {}", enzyme),
        )
    }
);

simple_op!(
    TranslateOperation,
    "translate.cds",
    "CDS übersetzen",
    |inputs, params, _ctx| {
        let d = sequences(inputs)?[0];
        let index = match params.get("feature") {
            Some(v) => v
                .trim()
                .parse::<usize>()
                .map_err(|_| Error::Operation("Parameter 'feature' ist keine Zahl".to_string()))?,
            None => d
                .features()
                .iter()
                .position(|f| f.feature_type().eq_ignore_ascii_case("CDS"))
                .ok_or_else(|| Error::Operation("Das Dokument hat kein CDS-Feature".to_string()))?,
        };
        let feature = d
            .features()
            .get(index)
            .ok_or_else(|| Error::Operation(format!("Feature {} gibt es nicht", index)))?;
        let cells = translate_feature(d.bases(), feature).map_err(op_err)?;
        let protein: String = cells
            .iter()
            .map(|c| c.amino_acid)
            .filter(|c| *c != '*')
            .collect();
        if protein.is_empty() {
            return Err(Error::Operation("Die Übersetzung ist leer".to_string()));
        }
        let name = format!("{} Protein", feature.name());
        let seq = Sequence::new(&name, &name, &protein, SequenceType::Protein).map_err(op_err)?;
        result(
            Document::Sequence(SequenceDocument::of(&name, seq)),
            format!("{} Aminosäuren", protein.len()),
        )
    }
);

/// Alle Operationen dieses Moduls.
pub fn all_operations() -> Vec<Box<dyn Operation>> {
    vec![
        Box::new(DeleteOperation),
        Box::new(ReplaceOperation),
        Box::new(ExtractOperation),
        Box::new(TopologyOperation),
        Box::new(AlignOperation),
        Box::new(ConsensusOperation),
        Box::new(StripOperation),
        Box::new(DigestOperation),
        Box::new(OverlapOperation),
        Box::new(GoldenGateOperation),
        Box::new(TranslateOperation),
        Box::new(crate::ops::phase2::RestrictionCloneOperation),
        Box::new(crate::ops::phase2::PcrOperation),
        Box::new(crate::ops::phase2::CodonOptimizeOperation),
        Box::new(crate::ops::phase3::ProfileOperation),
        Box::new(crate::ops::phase3::TranslateAlignOperation),
        Box::new(crate::ops::phase3::RealignOperation),
        Box::new(crate::ops::phase4::ProteinPropertiesOperation),
        Box::new(crate::ops::phase4::RnaFoldOperation),
        Box::new(crate::ops::analysis::PrimerPickOperation),
        Box::new(crate::ops::analysis::CrisprFindOperation),
        Box::new(crate::ops::analysis::SeqCheckOperation),
        Box::new(crate::ops::network::NcbiFetchOperation {
            http: crate::db::ncbi::CurlClient,
        }),
        Box::new(crate::ops::network::UniprotFetchOperation {
            http: crate::db::ncbi::CurlClient,
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::operation::NullContext;

    fn doc(bases: &str) -> Document {
        let s = Sequence::new("s", "S", bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of("Test", s))
    }

    fn params(p: &[(&str, &str)]) -> Params {
        p.iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn bases(d: &Document) -> String {
        match d {
            Document::Sequence(s) => s.bases().to_string(),
            _ => String::new(),
        }
    }

    fn run(op: &dyn Operation, inputs: &[Document], p: &[(&str, &str)]) -> Result<OperationResult> {
        op.run(inputs, &params(p), &mut NullContext)
    }

    #[test]
    fn bearbeiten() {
        let r = run(
            &DeleteOperation,
            &[doc("AACCGGTT")],
            &[("start", "2"), ("end", "4")],
        )
        .unwrap();
        assert_eq!(bases(&r.outputs[0]), "AAGGTT");
        let r = run(
            &ReplaceOperation,
            &[doc("AACCGGTT")],
            &[("start", "0"), ("end", "2"), ("bases", "TTT")],
        )
        .unwrap();
        assert_eq!(bases(&r.outputs[0]), "TTTCCGGTT");
        let r = run(
            &ExtractOperation,
            &[doc("AACCGGTT")],
            &[("start", "2"), ("end", "6")],
        )
        .unwrap();
        assert_eq!(bases(&r.outputs[0]), "CCGG");
        let r = run(&TopologyOperation, &[doc("AACC")], &[("value", "circular")]).unwrap();
        match &r.outputs[0] {
            Document::Sequence(s) => assert_eq!(s.topology(), Topology::Circular),
            _ => panic!("Sequenz erwartet"),
        }
        assert!(run(&TopologyOperation, &[doc("AACC")], &[("value", "rund")]).is_err());
        assert!(run(&DeleteOperation, &[doc("AACC")], &[("start", "2")]).is_err());
        assert!(run(&DeleteOperation, &[], &[("start", "0"), ("end", "1")]).is_err());
    }

    #[test]
    fn alignment_und_konsens() {
        let ins = [
            doc("ACGTACGTACGTAAAA"),
            doc("ACGTACGTACGTAAAT"),
            doc("ACGAACGTACCTAAAA"),
        ];
        let a = run(&AlignOperation, &ins, &[]).unwrap();
        assert!(matches!(a.outputs[0], Document::Alignment(_)));
        let c = run(&ConsensusOperation, &a.outputs, &[]).unwrap();
        assert_eq!(bases(&c.outputs[0]).len(), 16);
        let s = run(&StripOperation, &a.outputs, &[]).unwrap();
        assert!(matches!(s.outputs[0], Document::Alignment(_)));
        assert!(run(&AlignOperation, &ins[..1], &[]).is_err());
        assert!(run(&AlignOperation, &ins, &[("backend", "x")]).is_err());
    }

    #[test]
    fn verdau_und_zusammenbau() {
        let d = run(
            &DigestOperation,
            &[doc("AAGGAATTCGATTAGCTAGC")],
            &[("enzymes", "EcoRI")],
        )
        .unwrap();
        assert_eq!(d.outputs.len(), 2);
        assert_eq!(bases(&d.outputs[0]), "AAGG");
        assert!(run(&DigestOperation, &[doc("AAAA")], &[("enzymes", "Nix")]).is_err());
        let o1 = "ACGTACGTACGTACGTACGT";
        let o2 = "TTGGCCAATTGGCCAATTGG";
        let parts = [
            doc(&format!("{}{}", "AT".repeat(15), o1)),
            doc(&format!("{}{}{}", o1, "CCGG".repeat(5), o2)),
            doc(&format!("{}{}", o2, "TA".repeat(15))),
        ];
        let r = run(&OverlapOperation, &parts, &[("circular", "false")]).unwrap();
        assert_eq!(bases(&r.outputs[0]).len(), 30 + 20 + 20 + 20 + 30);
        assert!(run(&OverlapOperation, &parts, &[("method", "x")]).is_err());
    }

    #[test]
    fn translation() {
        use crate::model::feature::{Feature, Interval, Location, Strand};
        let loc = Location::new(
            vec![Interval::new(0, 9).unwrap()],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        let s = Sequence::new("s", "S", "ATGAAATAG", SequenceType::Dna).unwrap();
        let d = Document::Sequence(
            SequenceDocument::of("T", s).with_features(vec![Feature::new(
                "CDS",
                "gen",
                loc,
                Vec::new(),
            )]),
        );
        let r = run(&TranslateOperation, std::slice::from_ref(&d), &[]).unwrap();
        assert_eq!(bases(&r.outputs[0]), "MK");
        assert!(run(&TranslateOperation, &[doc("ATGAAATAG")], &[]).is_err());
        assert!(run(
            &TranslateOperation,
            std::slice::from_ref(&d),
            &[("feature", "5")]
        )
        .is_err());
    }

    #[test]
    fn liste() {
        let ops = all_operations();
        let mut ids: Vec<&str> = ops.iter().map(|o| o.id()).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
        assert!(ops.iter().all(|o| !o.label().is_empty()));
    }
}
