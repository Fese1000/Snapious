//! Grundtypen für Operationen.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::document::Document;

/// Parameter einer Operation (Name → Textwert).
pub type Params = BTreeMap<String, String>;

/// Fortschritt und Abbruch für laufende Operationen.
pub trait OperationContext {
    /// Fortschritt melden (`fraction` von 0.0 bis 1.0).
    fn progress(&mut self, fraction: f64, message: &str);
    /// Wurde ein Abbruch angefordert?
    fn is_cancelled(&self) -> bool;
}

/// Kontext ohne Anzeige und ohne Abbruch.
#[derive(Default)]
pub struct NullContext;

impl OperationContext for NullContext {
    fn progress(&mut self, _fraction: f64, _message: &str) {}

    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Ergebnis einer Operation.
#[derive(Clone, Debug, PartialEq)]
pub struct OperationResult {
    pub outputs: Vec<Document>,
    pub summary: String,
}

/// Eine ausführbare Operation.
pub trait Operation {
    /// Stabile ID, z. B. `edit.insert`.
    fn id(&self) -> &str;
    /// Anzeigename (deutsch).
    fn label(&self) -> &str;
    /// Ausführen. Fehler sollten verständliche deutsche Meldungen enthalten.
    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult>;
}

/// Eintrag im Verlauf einer erfolgreich ausgeführten Operation.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub id: String,
    pub operation_id: String,
    pub parameters: Params,
    pub input_ids: Vec<String>,
    pub output_ids: Vec<String>,
    pub time: i64,
    pub summary: String,
}

/// Pflichtparameter lesen.
pub fn required<'a>(params: &'a Params, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| Error::Operation(format!("Parameter '{}' is missing", key)))
}

/// Pflichtparameter als nicht-negative ganze Zahl lesen.
pub fn required_usize(params: &Params, key: &str) -> Result<usize> {
    let v = required(params, key)?;
    v.trim().parse::<usize>().map_err(|_| {
        Error::Operation(format!(
            "Parameter '{}' is not a non-negative integer: {}",
            key, v
        ))
    })
}
