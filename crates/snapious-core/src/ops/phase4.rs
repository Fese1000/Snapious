//! Protein and RNA operations for registry and workflows: compute properties, fold RNA.
//! Neither produces new documents, but a result text in `summary`.

use crate::bio::protein::{
    aromaticity, extinction_coefficient_280, isoelectric_point, molecular_weight,
};
use crate::bio::rna_fold::fold_rna;
use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::ops::operation::{Operation, OperationContext, OperationResult, Params};

/// `protein.properties`: properties of a protein sequence. Input: exactly one sequence document (otherwise `Error::Operation`).
/// `outputs` is empty. `summary` (one line, numbers via `format!`):
/// `"{n} aa, {mw:.1} Da, pI {pi:.2}, ε280 {eps:.0} M⁻¹cm⁻¹, Aromaticity {arom:.3}"` with `n` = length of the sequence,
/// `mw = molecular_weight`, `pi = isoelectric_point`, `eps = extinction_coefficient_280(.., false)`, `arom = aromaticity`
/// (each for `bases()` of the document). An empty sequence → `Error::Operation("The sequence is empty")`.
pub struct ProteinPropertiesOperation;

impl Operation for ProteinPropertiesOperation {
    fn id(&self) -> &str {
        "protein.properties"
    }

    fn label(&self) -> &str {
        "Protein properties"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        if inputs.len() != 1 {
            return Err(Error::Operation(
                "Exactly one sequence document expected".into(),
            ));
        }

        let seq = match &inputs[0] {
            Document::Sequence(doc) => doc.sequence().bases(),
            _ => return Err(Error::Operation("Unexpected document type".into())),
        };

        if seq.is_empty() {
            return Err(Error::Operation("The sequence is empty".into()));
        }

        let n = seq.len();
        let mw = molecular_weight(seq);
        let pi = isoelectric_point(seq);
        let eps = extinction_coefficient_280(seq, false);
        let arom = aromaticity(seq);

        let summary = format!(
            "{} aa, {:.1} Da, pI {:.2}, ε280 {:.0} M⁻¹cm⁻¹, Aromaticity {:.3}",
            n, mw, pi, eps, arom
        );

        Ok(OperationResult {
            outputs: Vec::new(),
            summary,
        })
    }
}

/// `rna.fold`: folds RNA with `fold_rna` (requires the RNAfold program). Input: exactly one sequence document; the bases are passed to
/// `fold_rna`. `outputs` is empty. `summary` = `"{structure} ({energy:.2} kcal/mol)"`. Errors from `fold_rna` are passed on as
/// `Error::Operation` with the same text.
pub struct RnaFoldOperation;

impl Operation for RnaFoldOperation {
    fn id(&self) -> &str {
        "rna.fold"
    }

    fn label(&self) -> &str {
        "Fold RNA"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        if inputs.len() != 1 {
            return Err(Error::Operation(
                "Exactly one sequence document expected".into(),
            ));
        }

        let seq = match &inputs[0] {
            Document::Sequence(doc) => doc.sequence().bases(),
            _ => return Err(Error::Operation("Unexpected document type".into())),
        };

        match fold_rna(seq) {
            Ok(folding) => {
                let summary = format!("{} ({:.2} kcal/mol)", folding.structure, folding.energy);
                Ok(OperationResult {
                    outputs: Vec::new(),
                    summary,
                })
            }
            Err(e) => Err(Error::Operation(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::SequenceDocument;
    use crate::model::sequence::{Sequence, SequenceType};
    use crate::ops::operation::NullContext;

    fn seq_doc(bases: &str, kind: SequenceType) -> Document {
        let s = Sequence::new("s", "s", bases, kind).unwrap();
        Document::Sequence(SequenceDocument::of("s", s))
    }

    fn run(op: &dyn Operation, inputs: &[Document]) -> Result<OperationResult> {
        op.run(inputs, &Params::new(), &mut NullContext)
    }

    #[test]
    fn eigenschaften() {
        let r = run(
            &ProteinPropertiesOperation,
            &[seq_doc("GGWYK", SequenceType::Protein)],
        )
        .unwrap();
        assert!(r.outputs.is_empty());
        let mw = molecular_weight("GGWYK");
        assert!(r.summary.starts_with(&format!("5 aa, {:.1} Da, pI ", mw)));
        assert!(r.summary.contains("ε280 "));
        assert!(r.summary.contains("Aromaticity 0.400"));
    }

    #[test]
    fn eigenschaften_fehler() {
        assert!(run(&ProteinPropertiesOperation, &[]).is_err());
        let two = [
            seq_doc("GG", SequenceType::Protein),
            seq_doc("AA", SequenceType::Protein),
        ];
        assert!(run(&ProteinPropertiesOperation, &two).is_err());
    }

    #[test]
    fn rna_falten() {
        // only with RNAfold installed
        if fold_rna("GGGGAAAACCCC").is_err() {
            return;
        }
        let r = run(
            &RnaFoldOperation,
            &[seq_doc("GGGGAAAACCCC", SequenceType::Rna)],
        )
        .unwrap();
        assert!(r.outputs.is_empty());
        assert!(r.summary.contains('(') && r.summary.ends_with("kcal/mol)"));
        assert!(run(&RnaFoldOperation, &[]).is_err());
    }
}
