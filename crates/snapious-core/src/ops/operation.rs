//! Basic types for operations.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::document::Document;

/// Parameters of an operation (name → text value).
pub type Params = BTreeMap<String, String>;

/// Progress and cancellation for running operations.
pub trait OperationContext {
    /// Report progress (`fraction` from 0.0 to 1.0).
    fn progress(&mut self, fraction: f64, message: &str);
    /// Was cancellation requested?
    fn is_cancelled(&self) -> bool;
}

/// Context without display and without cancellation.
#[derive(Default)]
pub struct NullContext;

impl OperationContext for NullContext {
    fn progress(&mut self, _fraction: f64, _message: &str) {}

    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Result of an operation.
#[derive(Clone, Debug, PartialEq)]
pub struct OperationResult {
    pub outputs: Vec<Document>,
    pub summary: String,
}

/// An executable operation.
pub trait Operation {
    /// Stable ID, e.g. `edit.insert`.
    fn id(&self) -> &str;
    /// Display name (German).
    fn label(&self) -> &str;
    /// Execute. Errors should contain understandable German messages.
    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult>;
}

/// Entry in the history of a successfully executed operation.
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

/// Read a required parameter.
pub fn required<'a>(params: &'a Params, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| Error::Operation(format!("Parameter '{}' is missing", key)))
}

/// Read a required parameter as a non-negative integer.
pub fn required_usize(params: &Params, key: &str) -> Result<usize> {
    let v = required(params, key)?;
    v.trim().parse::<usize>().map_err(|_| {
        Error::Operation(format!(
            "Parameter '{}' is not a non-negative integer: {}",
            key, v
        ))
    })
}
