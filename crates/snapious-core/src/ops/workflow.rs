//! Workflows: Folgen von Operationen, deren Ergebnisse weitergereicht werden.
//!
//! JSON-Format:
//! `{"name": "…", "description": "…", "inputCount": 1,
//!   "steps": [{"id": "rc", "operation": "edit.reverseComplement", "parameters": {"k": "v"}, "inputs": ["$in:0"]}, …],
//!   "outputs": ["$step:1:0"]}`
//! Verweise: `$in:N` ist das Eingabedokument N (0-basiert), `$step:S:N` die Ausgabe N des Schritts S (0-basiert; nur frühere Schritte).

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::ops::operation::{HistoryEntry, OperationContext, Params};
use crate::ops::registry::OperationRegistry;
use crate::ops::runner::OperationRunner;

/// Ein Schritt eines Workflows.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowStep {
    pub id: String,
    pub operation_id: String,
    pub parameters: Params,
    /// Verweise wie `$in:0` oder `$step:2:1`.
    pub inputs: Vec<String>,
}

/// Ein Workflow.
#[derive(Clone, Debug, PartialEq)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub input_count: usize,
    pub steps: Vec<WorkflowStep>,
    /// Verweise auf die Ergebnisse (wie bei den Eingaben der Schritte).
    pub outputs: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reference {
    Input(usize),
    Step(usize, usize),
}

fn parse_reference(text: &str) -> Result<Reference> {
    let bad = || {
        Error::InvalidInput(format!(
            "Ungültiger Verweis '{}' (erlaubt: $in:N oder $step:S:N)",
            text
        ))
    };
    let parts: Vec<&str> = text.trim().split(':').collect();
    match parts.as_slice() {
        ["$in", n] => Ok(Reference::Input(n.parse().map_err(|_| bad())?)),
        ["$step", s, n] => Ok(Reference::Step(
            s.parse().map_err(|_| bad())?,
            n.parse().map_err(|_| bad())?,
        )),
        _ => Err(bad()),
    }
}

impl Workflow {
    /// Prüft den Workflow vor der Ausführung: jede Operation ist in `registry` bekannt, jeder Verweis ist gültig
    /// (`$in:N` mit `N < input_count`, `$step:S:…` nur mit `S` kleiner als die Nummer des Schritts, der den Verweis benutzt;
    /// für `outputs` muss `S` ein vorhandener Schritt sein), Schritte haben mindestens eine Eingabe, `outputs` ist nicht leer.
    /// Fehler `Error::InvalidInput` mit Schrittnummer (1-basiert) wo es einen Schritt betrifft.
    pub fn validate(&self, registry: &OperationRegistry) -> Result<()> {
        if self.outputs.is_empty() {
            return Err(Error::InvalidInput(
                "Der Workflow hat keine Ausgabe".to_string(),
            ));
        }
        for (i, step) in self.steps.iter().enumerate() {
            let no = i + 1;
            if registry.find(&step.operation_id).is_none() {
                return Err(Error::InvalidInput(format!(
                    "Schritt {}: unbekannte Operation '{}'",
                    no, step.operation_id
                )));
            }
            if step.inputs.is_empty() {
                return Err(Error::InvalidInput(format!(
                    "Schritt {}: keine Eingabe",
                    no
                )));
            }
            for r in &step.inputs {
                match parse_reference(r)? {
                    Reference::Input(n) if n >= self.input_count => {
                        return Err(Error::InvalidInput(format!(
                            "Schritt {}: Eingabe {} gibt es nicht",
                            no, n
                        )));
                    }
                    Reference::Step(s, _) if s >= i => {
                        return Err(Error::InvalidInput(format!(
                            "Schritt {}: Verweis '{}' zeigt nicht auf einen früheren Schritt",
                            no, r
                        )));
                    }
                    _ => {}
                }
            }
        }
        for r in &self.outputs {
            match parse_reference(r)? {
                Reference::Input(n) if n >= self.input_count => {
                    return Err(Error::InvalidInput(format!(
                        "Ausgabe-Verweis '{}': Eingabe {} gibt es nicht",
                        r, n
                    )));
                }
                Reference::Step(s, _) if s >= self.steps.len() => {
                    return Err(Error::InvalidInput(format!(
                        "Ausgabe-Verweis '{}': Schritt {} gibt es nicht",
                        r, s
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Als JSON-Text (eingerückt).
    pub fn to_json(&self) -> String {
        let steps: Vec<Value> = self
            .steps
            .iter()
            .map(|s| json!({"id": s.id, "operation": s.operation_id, "parameters": s.parameters, "inputs": s.inputs}))
            .collect();
        let v = json!({
            "name": self.name,
            "description": self.description,
            "inputCount": self.input_count,
            "steps": steps,
            "outputs": self.outputs,
        });
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }

    /// Liest JSON im Format oben. Fehler `Error::InvalidInput` bei ungültigem JSON, fehlendem `name`, `steps` oder `outputs` oder falschen Typen.
    /// `description` und `parameters` dürfen fehlen, `inputCount` fehlt → 1, `id` fehlt → `schritt{N}` (1-basiert). Parameterwerte
    /// dürfen Zahlen, Wahrheitswerte oder Texte sein (alles wird zu Text).
    pub fn from_json(text: &str) -> Result<Workflow> {
        let bad = |what: &str| Error::InvalidInput(format!("Workflow-JSON: {}", what));
        let v: Value = serde_json::from_str(text).map_err(|e| bad(&e.to_string()))?;
        let name = v
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("'name' fehlt"))?
            .to_string();
        let description = v
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let input_count = match v.get("inputCount") {
            Some(n) => n
                .as_u64()
                .ok_or_else(|| bad("'inputCount' ist keine Zahl"))? as usize,
            None => 1,
        };
        let strings = |v: &Value, what: &str| -> Result<Vec<String>> {
            v.as_array()
                .ok_or_else(|| bad(&format!("'{}' ist keine Liste", what)))?
                .iter()
                .map(|x| {
                    x.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| bad(&format!("'{}' enthält Nicht-Text", what)))
                })
                .collect()
        };
        let outputs = strings(
            v.get("outputs").ok_or_else(|| bad("'outputs' fehlt"))?,
            "outputs",
        )?;
        let mut steps = Vec::new();
        for (i, s) in v
            .get("steps")
            .and_then(Value::as_array)
            .ok_or_else(|| bad("'steps' fehlt"))?
            .iter()
            .enumerate()
        {
            let operation_id = s
                .get("operation")
                .and_then(Value::as_str)
                .ok_or_else(|| bad(&format!("Schritt {}: 'operation' fehlt", i + 1)))?
                .to_string();
            let id = s
                .get("id")
                .and_then(Value::as_str)
                .map_or_else(|| format!("schritt{}", i + 1), str::to_string);
            let inputs = strings(
                s.get("inputs")
                    .ok_or_else(|| bad(&format!("Schritt {}: 'inputs' fehlt", i + 1)))?,
                "inputs",
            )?;
            let mut parameters = Params::new();
            if let Some(obj) = s.get("parameters") {
                for (k, val) in obj.as_object().ok_or_else(|| {
                    bad(&format!("Schritt {}: 'parameters' ist kein Objekt", i + 1))
                })? {
                    let text = match val {
                        Value::String(t) => t.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => {
                            return Err(bad(&format!(
                                "Schritt {}: Parameter '{}' hat einen ungültigen Wert",
                                i + 1,
                                k
                            )))
                        }
                    };
                    parameters.insert(k.clone(), text);
                }
            }
            steps.push(WorkflowStep {
                id,
                operation_id,
                parameters,
                inputs,
            });
        }
        Ok(Workflow {
            name,
            description,
            input_count,
            steps,
            outputs,
        })
    }
}

/// Ergebnis eines Laufs.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowResult {
    pub outputs: Vec<Document>,
    /// Die bei diesem Lauf neu entstandenen Verlaufseinträge.
    pub history: Vec<HistoryEntry>,
    /// Fehlermeldungen (leer bei Erfolg).
    pub errors: Vec<String>,
}

/// Führt Workflows aus.
pub struct WorkflowRunner<'a> {
    pub registry: &'a OperationRegistry,
}

impl<'a> WorkflowRunner<'a> {
    pub fn new(registry: &'a OperationRegistry) -> Self {
        Self { registry }
    }

    /// Führt `wf` auf `inputs` aus. Zuerst `validate` und die Prüfung `inputs.len() == input_count` (beides vor jeder Ausführung, ohne
    /// Verlaufseinträge). Jeder Schritt läuft über `runner.execute`; ein Fehler wird zu `Error::Operation("Schritt N: …")`
    /// (1-basiert). Verweise auf nicht vorhandene Ausgaben eines Schritts ebenfalls (`Schritt N: Verweis … liefert keine Ausgabe`).
    pub fn run(
        &self,
        wf: &Workflow,
        inputs: &[Document],
        runner: &mut OperationRunner,
        ctx: &mut dyn OperationContext,
    ) -> Result<WorkflowResult> {
        wf.validate(self.registry)?;
        if inputs.len() != wf.input_count {
            return Err(Error::InvalidInput(format!(
                "Der Workflow braucht {} Eingabedokument(e), übergeben wurden {}",
                wf.input_count,
                inputs.len()
            )));
        }
        let history_start = runner.history().len();
        let mut step_outputs: Vec<Vec<Document>> = Vec::new();
        let lookup =
            |r: &str, step_outputs: &[Vec<Document>], no: Option<usize>| -> Result<Document> {
                let fail = |m: String| match no {
                    Some(n) => Error::Operation(format!("Schritt {}: {}", n, m)),
                    None => Error::Operation(m),
                };
                match parse_reference(r)? {
                    Reference::Input(n) => inputs
                        .get(n)
                        .cloned()
                        .ok_or_else(|| fail(format!("Eingabe {} fehlt", n))),
                    Reference::Step(s, n) => step_outputs
                        .get(s)
                        .and_then(|o| o.get(n))
                        .cloned()
                        .ok_or_else(|| fail(format!("Verweis {} liefert keine Ausgabe", r))),
                }
            };
        for (i, step) in wf.steps.iter().enumerate() {
            let no = i + 1;
            let op = self
                .registry
                .find(&step.operation_id)
                .ok_or_else(|| Error::Operation(format!("Schritt {}: unbekannte Operation", no)))?;
            let docs: Vec<Document> = step
                .inputs
                .iter()
                .map(|r| lookup(r, &step_outputs, Some(no)))
                .collect::<Result<_>>()?;
            let result = runner
                .execute(op, &docs, &step.parameters, ctx)
                .map_err(|e| Error::Operation(format!("Schritt {}: {}", no, e)))?;
            step_outputs.push(result.outputs);
        }
        let outputs = wf
            .outputs
            .iter()
            .map(|r| lookup(r, &step_outputs, None))
            .collect::<Result<Vec<_>>>()?;
        Ok(WorkflowResult {
            outputs,
            history: runner
                .history()
                .iter()
                .skip(history_start)
                .cloned()
                .collect(),
            errors: Vec::new(),
        })
    }

    /// Führt `wf` für jedes Dokument einzeln aus (der Workflow braucht dafür genau eine Eingabe). Ein Fehler stoppt die anderen Läufe nicht:
    /// er steht in `errors` des betreffenden Ergebnisses (`outputs` und `history` leer). Ist der Workflow selbst ungültig, hat jedes
    /// Ergebnis diese Fehlermeldung.
    pub fn run_batch(
        &self,
        wf: &Workflow,
        inputs: &[Document],
        runner: &mut OperationRunner,
        ctx: &mut dyn OperationContext,
    ) -> Vec<WorkflowResult> {
        inputs
            .iter()
            .map(
                |d| match self.run(wf, std::slice::from_ref(d), runner, ctx) {
                    Ok(r) => r,
                    Err(e) => WorkflowResult {
                        outputs: Vec::new(),
                        history: Vec::new(),
                        errors: vec![e.to_string()],
                    },
                },
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::SequenceDocument;
    use crate::model::sequence::{Sequence, SequenceType};
    use crate::ops::operation::NullContext;

    fn doc(bases: &str) -> Document {
        let s = Sequence::new("s", "S", bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of("Test", s))
    }

    fn bases(d: &Document) -> String {
        match d {
            Document::Sequence(s) => s.bases().to_string(),
            _ => String::new(),
        }
    }

    fn step(id: &str, op: &str, p: &[(&str, &str)], inputs: &[&str]) -> WorkflowStep {
        WorkflowStep {
            id: id.to_string(),
            operation_id: op.to_string(),
            parameters: p
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn rc_dann_einfuegen() -> Workflow {
        Workflow {
            name: "rc und einfügen".to_string(),
            description: "Test".to_string(),
            input_count: 1,
            steps: vec![
                step("rc", "edit.reverseComplement", &[], &["$in:0"]),
                step(
                    "ins",
                    "edit.insert",
                    &[("position", "1"), ("bases", "GG")],
                    &["$step:0:0"],
                ),
            ],
            outputs: vec!["$step:1:0".to_string()],
        }
    }

    #[test]
    fn zwei_schritte() {
        let reg = OperationRegistry::with_builtins();
        let mut runner = OperationRunner::new();
        let r = WorkflowRunner::new(&reg)
            .run(
                &rc_dann_einfuegen(),
                &[doc("AATT")],
                &mut runner,
                &mut NullContext,
            )
            .unwrap();
        assert_eq!(bases(&r.outputs[0]), "AGGATT");
        assert_eq!(r.history.len(), 2);
        assert_eq!(r.history[0].operation_id, "edit.reverseComplement");
        assert!(r.errors.is_empty());
        assert_eq!(runner.history().len(), 2);
    }

    #[test]
    fn stapel() {
        let reg = OperationRegistry::with_builtins();
        let mut runner = OperationRunner::new();
        let r = WorkflowRunner::new(&reg).run_batch(
            &rc_dann_einfuegen(),
            &[doc("ATGC"), doc("GGGG")],
            &mut runner,
            &mut NullContext,
        );
        assert_eq!(r.len(), 2);
        // rc(ATGC)=GCAT → G GG CAT
        assert_eq!(bases(&r[0].outputs[0]), "GGGCAT");
        assert_eq!(bases(&r[1].outputs[0]), "CGGCCC");
    }

    #[test]
    fn pruefung_vor_der_ausfuehrung() {
        let reg = OperationRegistry::with_builtins();
        let mut wf = rc_dann_einfuegen();
        wf.steps[1].operation_id = "gibt.es.nicht".to_string();
        let mut runner = OperationRunner::new();
        let e = WorkflowRunner::new(&reg).run(&wf, &[doc("AATT")], &mut runner, &mut NullContext);
        assert!(matches!(e, Err(Error::InvalidInput(m)) if m.contains("Schritt 2")));
        assert!(runner.history().is_empty());
        let mut v = rc_dann_einfuegen();
        v.steps[0].inputs = vec!["$step:1:0".to_string()];
        assert!(v.validate(&reg).is_err());
        let mut w = rc_dann_einfuegen();
        w.steps[0].inputs = vec!["$in:3".to_string()];
        assert!(w.validate(&reg).is_err());
        let mut x = rc_dann_einfuegen();
        x.outputs = vec!["$step:9:0".to_string()];
        assert!(x.validate(&reg).is_err());
        x.outputs = vec![];
        assert!(x.validate(&reg).is_err());
        let mut y = rc_dann_einfuegen();
        y.steps[0].inputs = vec!["$kaputt".to_string()];
        assert!(y.validate(&reg).is_err());
        assert!(rc_dann_einfuegen().validate(&reg).is_ok());
        // falsche Zahl Eingaben
        let falsch =
            WorkflowRunner::new(&reg).run(&rc_dann_einfuegen(), &[], &mut runner, &mut NullContext);
        assert!(matches!(falsch, Err(Error::InvalidInput(_))));
    }

    #[test]
    fn fehler_im_zweiten_schritt() {
        let reg = OperationRegistry::with_builtins();
        let mut wf = rc_dann_einfuegen();
        wf.steps[1]
            .parameters
            .insert("position".to_string(), "99".to_string());
        let mut runner = OperationRunner::new();
        let e = WorkflowRunner::new(&reg)
            .run(&wf, &[doc("AATT")], &mut runner, &mut NullContext)
            .unwrap_err();
        assert!(e.to_string().contains("Schritt 2"), "{}", e);
        // Stapel: ein gültiger, ein ungültiger Lauf (Position 3 passt nur zur längeren Sequenz)
        let mut wf2 = rc_dann_einfuegen();
        wf2.steps[1]
            .parameters
            .insert("position".to_string(), "3".to_string());
        let r = WorkflowRunner::new(&reg).run_batch(
            &wf2,
            &[doc("AATT"), doc("AA")],
            &mut runner,
            &mut NullContext,
        );
        assert!(r[0].errors.is_empty());
        assert_eq!(r[1].errors.len(), 1);
        assert!(r[1].errors[0].contains("Schritt 2"));
        assert!(r[1].outputs.is_empty());
    }

    #[test]
    fn json_rundreise() {
        let wf = rc_dann_einfuegen();
        let text = wf.to_json();
        assert!(text.contains("\"operation\": \"edit.insert\""));
        assert_eq!(Workflow::from_json(&text).unwrap(), wf);
        let knapp = Workflow::from_json(
            r#"{"name":"k","steps":[{"operation":"edit.insert","parameters":{"position":1,"bases":"A"},"inputs":["$in:0"]}],"outputs":["$step:0:0"]}"#,
        )
        .unwrap();
        assert_eq!(knapp.input_count, 1);
        assert_eq!(knapp.steps[0].id, "schritt1");
        assert_eq!(
            knapp.steps[0]
                .parameters
                .get("position")
                .map(String::as_str),
            Some("1")
        );
        assert!(Workflow::from_json("x").is_err());
        assert!(Workflow::from_json(r#"{"steps":[],"outputs":[]}"#).is_err());
        assert!(
            Workflow::from_json(r#"{"name":"a","steps":[{"inputs":[]}],"outputs":[]}"#).is_err()
        );
        assert!(Workflow::from_json(r#"{"name":"a","steps":[],"outputs":[1]}"#).is_err());
    }
}
