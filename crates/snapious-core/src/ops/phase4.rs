//! Protein- und RNA-Operationen für Registry und Workflows: Eigenschaften berechnen, RNA falten.
//! Beide liefern keine neuen Dokumente, sondern einen Ergebnistext in `summary`.

use crate::bio::protein::{
    aromaticity, extinction_coefficient_280, isoelectric_point, molecular_weight,
};
use crate::bio::rna_fold::fold_rna;
use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::ops::operation::{Operation, OperationContext, OperationResult, Params};

/// `protein.properties`: Eigenschaften einer Proteinsequenz. Eingabe: genau ein Sequenzdokument (sonst `Error::Operation`).
/// `outputs` ist leer. `summary` (eine Zeile, Zahlen mit `format!`):
/// `"{n} aa, {mw:.1} Da, pI {pi:.2}, ε280 {eps:.0} M⁻¹cm⁻¹, Aromatizität {arom:.3}"` mit `n` = Länge der Sequenz,
/// `mw = molecular_weight`, `pi = isoelectric_point`, `eps = extinction_coefficient_280(.., false)`, `arom = aromaticity`
/// (jeweils für `bases()` des Dokuments). Eine leere Sequenz → `Error::Operation("Die Sequenz ist leer")`.
pub struct ProteinPropertiesOperation;

impl Operation for ProteinPropertiesOperation {
    fn id(&self) -> &str {
        "protein.properties"
    }

    fn label(&self) -> &str {
        "Proteineigenschaften"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        if inputs.len() != 1 {
            return Err(Error::Operation(
                "Genau ein Sequenzdokument erwartet".into(),
            ));
        }

        let seq = match &inputs[0] {
            Document::Sequence(doc) => doc.sequence().bases(),
            _ => return Err(Error::Operation("Unerwarteter Dokumenttyp".into())),
        };

        if seq.is_empty() {
            return Err(Error::Operation("Die Sequenz ist leer".into()));
        }

        let n = seq.len();
        let mw = molecular_weight(seq);
        let pi = isoelectric_point(seq);
        let eps = extinction_coefficient_280(seq, false);
        let arom = aromaticity(seq);

        let summary = format!(
            "{} aa, {:.1} Da, pI {:.2}, ε280 {:.0} M⁻¹cm⁻¹, Aromatizität {:.3}",
            n, mw, pi, eps, arom
        );

        Ok(OperationResult {
            outputs: Vec::new(),
            summary,
        })
    }
}

/// `rna.fold`: RNA mit `fold_rna` falten (benötigt das Programm RNAfold). Eingabe: genau ein Sequenzdokument; die Basen werden an
/// `fold_rna` übergeben. `outputs` ist leer. `summary` = `"{structure} ({energy:.2} kcal/mol)"`. Fehler von `fold_rna` werden als
/// `Error::Operation` mit demselben Text weitergegeben.
pub struct RnaFoldOperation;

impl Operation for RnaFoldOperation {
    fn id(&self) -> &str {
        "rna.fold"
    }

    fn label(&self) -> &str {
        "RNA falten"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        if inputs.len() != 1 {
            return Err(Error::Operation(
                "Genau ein Sequenzdokument erwartet".into(),
            ));
        }

        let seq = match &inputs[0] {
            Document::Sequence(doc) => doc.sequence().bases(),
            _ => return Err(Error::Operation("Unerwarteter Dokumenttyp".into())),
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
        assert!(r.summary.contains("Aromatizität 0.400"));
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
        // nur mit installiertem RNAfold
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
