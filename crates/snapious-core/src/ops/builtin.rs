//! Einfache eingebaute Operationen (Reverse-Komplement, Einfügen).

use crate::error::{Error, Result};
use crate::model::document::{Document, SequenceDocument};
use crate::ops::operation::{
    required, required_usize, Operation, OperationContext, OperationResult, Params,
};

fn first_sequence(inputs: &[Document]) -> Result<&SequenceDocument> {
    match inputs.first() {
        Some(Document::Sequence(d)) => Ok(d),
        Some(_) => Err(Error::Operation(
            "Das Dokument ist keine Sequenz".to_string(),
        )),
        None => Err(Error::Operation(
            "Es wurde keine Sequenz übergeben".to_string(),
        )),
    }
}

/// Reverse-Komplement eines Sequenzdokuments (ID `edit.reverseComplement`).
pub struct ReverseComplementOperation;

impl Operation for ReverseComplementOperation {
    fn id(&self) -> &str {
        "edit.reverseComplement"
    }

    fn label(&self) -> &str {
        "Reverse-Komplement"
    }

    fn run(
        &self,
        inputs: &[Document],
        _params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let doc = first_sequence(inputs)?;
        let rc = doc
            .reverse_complement()
            .map_err(|e| Error::Operation(e.to_string()))?
            .derive(&format!("{} (rev-kom)", doc.name()));
        Ok(OperationResult {
            outputs: vec![Document::Sequence(rc)],
            summary: format!("Reverse-Komplement von {} ({} bp)", doc.name(), doc.len()),
        })
    }
}

/// Einfügen von Basen (ID `edit.insert`, Parameter `position` und `bases`).
pub struct InsertOperation;

impl Operation for InsertOperation {
    fn id(&self) -> &str {
        "edit.insert"
    }

    fn label(&self) -> &str {
        "Basen einfügen"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let doc = first_sequence(inputs)?;
        let position = required_usize(params, "position")?;
        let bases = required(params, "bases")?;
        let edited = doc
            .insert(position, bases)
            .map_err(|e| Error::Operation(e.to_string()))?
            .derive(&format!("{} (bearbeitet)", doc.name()));
        Ok(OperationResult {
            outputs: vec![Document::Sequence(edited)],
            summary: format!("{} Basen an Position {} eingefügt", bases.len(), position),
        })
    }
}
