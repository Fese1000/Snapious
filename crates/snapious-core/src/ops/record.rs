//! Workflow aus dem Verlauf aufzeichnen.

use crate::error::{Error, Result};
use crate::ops::operation::HistoryEntry;
use crate::ops::workflow::{Workflow, WorkflowStep};

/// Baut aus Verlaufseinträgen (in der Reihenfolge ihrer Ausführung) einen Workflow.
/// * Schritt `i` (0-basiert) bekommt die ID `s{i}`, `operation_id` und `parameters` des Eintrags.
/// * Jede Eingabe-ID eines Eintrags wird zum Verweis: wurde sie von einem **früheren** Eintrag als Ausgabe erzeugt (`output_ids`, jeweils der letzte
///   solche Eintrag vor diesem), ist es `$step:S:N` (`S` = Index jenes Eintrags, `N` = Position der ID in dessen `output_ids`); sonst eine
///   externe Eingabe `$in:K`, wobei `K` in der Reihenfolge des ersten Auftretens der verschiedenen externen IDs vergeben wird (0, 1, …).
///   Dieselbe externe ID bekommt überall denselben Verweis.
/// * `input_count` = Zahl der verschiedenen externen Eingaben.
/// * `outputs` = Verweise `$step:L:N` für alle Ausgaben des letzten Eintrags `L` (`N` = 0, 1, …).
/// * `description` = die Zusammenfassungen (`summary`) der Einträge, durch ` → ` getrennt (leere Zusammenfassungen entfallen).
///
/// Fehler `Error::InvalidInput`, wenn `entries` leer ist.
pub fn workflow_from_history(name: &str, entries: &[HistoryEntry]) -> Result<Workflow> {
    if entries.is_empty() {
        return Err(Error::InvalidInput(
            "The history is empty, there is nothing to record".to_string(),
        ));
    }
    let mut externals: Vec<String> = Vec::new();
    let mut steps: Vec<WorkflowStep> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let mut inputs = Vec::new();
        for id in &e.input_ids {
            // letzter früherer Erzeuger
            let producer =
                entries[..i].iter().enumerate().rev().find_map(|(s, prev)| {
                    prev.output_ids.iter().position(|o| o == id).map(|n| (s, n))
                });
            match producer {
                Some((s, n)) => inputs.push(format!("$step:{}:{}", s, n)),
                None => {
                    let k = match externals.iter().position(|x| x == id) {
                        Some(k) => k,
                        None => {
                            externals.push(id.clone());
                            externals.len() - 1
                        }
                    };
                    inputs.push(format!("$in:{}", k));
                }
            }
        }
        steps.push(WorkflowStep {
            id: format!("s{}", i),
            operation_id: e.operation_id.clone(),
            parameters: e.parameters.clone(),
            inputs,
        });
    }
    let last = entries.len() - 1;
    let outputs = (0..entries[last].output_ids.len())
        .map(|n| format!("$step:{}:{}", last, n))
        .collect();
    let description = entries
        .iter()
        .map(|e| e.summary.as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" → ");
    Ok(Workflow {
        name: name.to_string(),
        description,
        input_count: externals.len(),
        steps,
        outputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        op: &str,
        inputs: &[&str],
        outputs: &[&str],
        summary: &str,
        params: &[(&str, &str)],
    ) -> HistoryEntry {
        HistoryEntry {
            id: format!("h-{}", op),
            operation_id: op.to_string(),
            parameters: params
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            input_ids: inputs.iter().map(|s| s.to_string()).collect(),
            output_ids: outputs.iter().map(|s| s.to_string()).collect(),
            time: 0,
            summary: summary.to_string(),
        }
    }

    #[test]
    fn drei_schritte() {
        let h = vec![
            entry(
                "clone.digest",
                &["vec"],
                &["f1", "f2"],
                "Verdau",
                &[("enzymes", "EcoRI")],
            ),
            entry(
                "clone.restriction",
                &["f2", "ins"],
                &["c1"],
                "Klonierung",
                &[],
            ),
            entry("translate.cds", &["c1"], &["p1"], "", &[("feature", "0")]),
        ];
        let w = workflow_from_history("Mein Ablauf", &h).unwrap();
        assert_eq!(w.name, "Mein Ablauf");
        assert_eq!(w.description, "Verdau → Klonierung");
        assert_eq!(w.input_count, 2);
        assert_eq!(w.steps.len(), 3);
        assert_eq!(w.steps[0].id, "s0");
        assert_eq!(w.steps[0].operation_id, "clone.digest");
        assert_eq!(w.steps[0].parameters["enzymes"], "EcoRI");
        assert_eq!(w.steps[0].inputs, vec!["$in:0"]);
        assert_eq!(w.steps[1].inputs, vec!["$step:0:1", "$in:1"]);
        assert_eq!(w.steps[2].inputs, vec!["$step:1:0"]);
        assert_eq!(w.outputs, vec!["$step:2:0"]);
    }

    #[test]
    fn gleiche_eingabe_und_mehrere_ausgaben() {
        let h = vec![
            entry("a.op", &["x", "x"], &["o1", "o2"], "A", &[]),
            entry("b.op", &["x", "o1"], &["o3", "o4", "o5"], "B", &[]),
        ];
        let w = workflow_from_history("W", &h).unwrap();
        assert_eq!(w.input_count, 1);
        assert_eq!(w.steps[0].inputs, vec!["$in:0", "$in:0"]);
        assert_eq!(w.steps[1].inputs, vec!["$in:0", "$step:0:0"]);
        assert_eq!(w.outputs, vec!["$step:1:0", "$step:1:1", "$step:1:2"]);
    }

    #[test]
    fn spaeterer_eintrag_ueberschreibt_und_fehler() {
        // "d" wird von Schritt 0 und Schritt 1 erzeugt; Schritt 2 verweist auf den letzten Erzeuger (Schritt 1)
        let h = vec![
            entry("a.op", &["in"], &["d"], "", &[]),
            entry("b.op", &["in"], &["d"], "", &[]),
            entry("c.op", &["d"], &["e"], "", &[]),
        ];
        let w = workflow_from_history("W", &h).unwrap();
        assert_eq!(w.steps[2].inputs, vec!["$step:1:0"]);
        assert_eq!(w.description, "");
        assert!(matches!(
            workflow_from_history("W", &[]),
            Err(Error::InvalidInput(_))
        ));
        // der Workflow lässt sich in JSON schreiben und lesen
        let again = Workflow::from_json(&w.to_json()).unwrap();
        assert_eq!(again, w);
    }
}
