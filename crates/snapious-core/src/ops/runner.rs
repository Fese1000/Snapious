//! Führt Operationen aus und schreibt den Verlauf.

use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::ops::operation::{HistoryEntry, Operation, OperationContext, OperationResult, Params};
use crate::util::{new_id, now_secs};

type Listener = Box<dyn FnMut(&HistoryEntry)>;

/// Führt Operationen aus. Jeder erfolgreiche Lauf erzeugt einen [`HistoryEntry`].
#[derive(Default)]
pub struct OperationRunner {
    history: Vec<HistoryEntry>,
    listener: Option<Listener>,
}

impl OperationRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Runner, der jeden neuen Verlaufseintrag zusätzlich an `listener` meldet.
    pub fn with_listener(listener: Listener) -> Self {
        Self {
            history: Vec::new(),
            listener: Some(listener),
        }
    }

    /// Bisheriger Verlauf in Ausführungsreihenfolge.
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Führt `op` aus. Bei Fehler oder Abbruch wird nichts protokolliert.
    pub fn execute(
        &mut self,
        op: &dyn Operation,
        inputs: &[Document],
        params: &Params,
        ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let result = op.run(inputs, params, ctx)?;
        if ctx.is_cancelled() {
            return Err(Error::Operation("abgebrochen".to_string()));
        }
        let entry = HistoryEntry {
            id: new_id(),
            operation_id: op.id().to_string(),
            parameters: params.clone(),
            input_ids: inputs.iter().map(|d| d.id().to_string()).collect(),
            output_ids: result.outputs.iter().map(|d| d.id().to_string()).collect(),
            time: now_secs(),
            summary: result.summary.clone(),
        };
        if let Some(l) = self.listener.as_mut() {
            l(&entry);
        }
        self.history.push(entry);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;
    use crate::model::document::SequenceDocument;
    use crate::model::sequence::{Sequence, SequenceType};
    use crate::ops::builtin::{InsertOperation, ReverseComplementOperation};
    use crate::ops::operation::NullContext;

    fn doc(bases: &str) -> Document {
        let s = Sequence::new("s", "S", bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of("Test", s))
    }

    fn bases_of(d: &Document) -> String {
        match d {
            Document::Sequence(s) => s.bases().to_string(),
            _ => String::new(),
        }
    }

    #[test]
    fn reverse_komplement_laeuft_und_protokolliert() {
        let mut runner = OperationRunner::new();
        let input = doc("ATGC");
        let r = runner
            .execute(
                &ReverseComplementOperation,
                std::slice::from_ref(&input),
                &Params::new(),
                &mut NullContext,
            )
            .unwrap();
        assert_eq!(bases_of(&r.outputs[0]), "GCAT");
        assert_ne!(r.outputs[0].id(), input.id());
        assert_eq!(runner.history().len(), 1);
        let h = &runner.history()[0];
        assert_eq!(h.operation_id, "edit.reverseComplement");
        assert_eq!(h.input_ids, vec![input.id().to_string()]);
        assert_eq!(h.output_ids, vec![r.outputs[0].id().to_string()]);
    }

    #[test]
    fn palindrom_bleibt_gleich() {
        let mut runner = OperationRunner::new();
        let r = runner
            .execute(
                &ReverseComplementOperation,
                &[doc("AACCGGTT")],
                &Params::new(),
                &mut NullContext,
            )
            .unwrap();
        assert_eq!(bases_of(&r.outputs[0]), "AACCGGTT");
    }

    #[test]
    fn einfuegen_mit_parametern() {
        let mut runner = OperationRunner::new();
        let mut p = Params::new();
        p.insert("position".into(), "2".into());
        p.insert("bases".into(), "GG".into());
        let r = runner
            .execute(&InsertOperation, &[doc("AATT")], &p, &mut NullContext)
            .unwrap();
        assert_eq!(bases_of(&r.outputs[0]), "AAGGTT");
    }

    #[test]
    fn fehlerhafter_parameter_wird_nicht_protokolliert() {
        let mut runner = OperationRunner::new();
        let mut p = Params::new();
        p.insert("position".into(), "abc".into());
        p.insert("bases".into(), "GG".into());
        let r = runner.execute(&InsertOperation, &[doc("AATT")], &p, &mut NullContext);
        assert!(matches!(r, Err(Error::Operation(_))));
        assert!(runner.history().is_empty());
        let r = runner.execute(
            &InsertOperation,
            &[doc("AATT")],
            &Params::new(),
            &mut NullContext,
        );
        assert!(r.is_err());
    }

    #[test]
    fn einfuegen_ausserhalb_ist_fehler() {
        let mut runner = OperationRunner::new();
        let mut p = Params::new();
        p.insert("position".into(), "99".into());
        p.insert("bases".into(), "GG".into());
        assert!(runner
            .execute(&InsertOperation, &[doc("AATT")], &p, &mut NullContext)
            .is_err());
        assert!(runner.history().is_empty());
    }

    #[test]
    fn ohne_eingabe_ist_fehler() {
        let mut runner = OperationRunner::new();
        assert!(runner
            .execute(
                &ReverseComplementOperation,
                &[],
                &Params::new(),
                &mut NullContext
            )
            .is_err());
    }

    struct CancellingContext {
        cancelled: Rc<Cell<bool>>,
    }

    impl OperationContext for CancellingContext {
        fn progress(&mut self, _f: f64, _m: &str) {
            self.cancelled.set(true);
        }

        fn is_cancelled(&self) -> bool {
            self.cancelled.get()
        }
    }

    struct MeldetFortschritt;

    impl Operation for MeldetFortschritt {
        fn id(&self) -> &str {
            "test.progress"
        }

        fn label(&self) -> &str {
            "Test"
        }

        fn run(
            &self,
            _i: &[Document],
            _p: &Params,
            ctx: &mut dyn OperationContext,
        ) -> Result<OperationResult> {
            ctx.progress(0.5, "halb");
            Ok(OperationResult {
                outputs: vec![],
                summary: String::new(),
            })
        }
    }

    #[test]
    fn abbruch_verwirft_ergebnis_und_protokolliert_nicht() {
        let mut runner = OperationRunner::new();
        let mut ctx = CancellingContext {
            cancelled: Rc::new(Cell::new(false)),
        };
        let r = runner.execute(&MeldetFortschritt, &[], &Params::new(), &mut ctx);
        assert_eq!(r, Err(Error::Operation("abgebrochen".to_string())));
        assert!(runner.history().is_empty());
    }

    #[test]
    fn listener_bekommt_jeden_eintrag() {
        let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let seen2 = seen.clone();
        let mut runner = OperationRunner::with_listener(Box::new(move |e| {
            seen2.borrow_mut().push(e.operation_id.clone());
        }));
        runner
            .execute(
                &ReverseComplementOperation,
                &[doc("ATGC")],
                &Params::new(),
                &mut NullContext,
            )
            .unwrap();
        runner
            .execute(
                &ReverseComplementOperation,
                &[doc("ATGC")],
                &Params::new(),
                &mut NullContext,
            )
            .unwrap();
        assert_eq!(seen.borrow().len(), 2);
    }
}
