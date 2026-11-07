//! Verzeichnis aller Operationen nach ID.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::ops::builtin::{InsertOperation, ReverseComplementOperation};
use crate::ops::more::all_operations;
use crate::ops::operation::Operation;

/// Alle bekannten Operationen, nach ID sortiert.
#[derive(Default)]
pub struct OperationRegistry {
    ops: BTreeMap<String, Box<dyn Operation>>,
}

impl OperationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry mit allen eingebauten Operationen.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        let mut all: Vec<Box<dyn Operation>> = vec![
            Box::new(ReverseComplementOperation),
            Box::new(InsertOperation),
        ];
        all.extend(all_operations());
        for op in all {
            // IDs der eingebauten Operationen sind eindeutig (durch Test abgesichert)
            let _ = r.register(op);
        }
        r
    }

    /// Fügt eine Operation hinzu; doppelte ID → `Error::InvalidInput`.
    pub fn register(&mut self, op: Box<dyn Operation>) -> Result<()> {
        let id = op.id().to_string();
        if self.ops.contains_key(&id) {
            return Err(Error::InvalidInput(format!(
                "The operation '{}' is already registered",
                id
            )));
        }
        self.ops.insert(id, op);
        Ok(())
    }

    pub fn find(&self, id: &str) -> Option<&dyn Operation> {
        self.ops.get(id).map(|b| b.as_ref())
    }

    /// Alle Operationen nach ID sortiert.
    pub fn all(&self) -> Vec<&dyn Operation> {
        self.ops.values().map(|b| b.as_ref()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebaute() {
        let r = OperationRegistry::with_builtins();
        assert!(r.find("edit.insert").is_some());
        assert!(r.find("edit.reverseComplement").is_some());
        assert!(r.find("clone.goldengate").is_some());
        assert!(r.find("gibt.es.nicht").is_none());
        let ids: Vec<&str> = r.all().iter().map(|o| o.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
        assert_eq!(ids.len(), 26);
    }

    #[test]
    fn doppelte_id() {
        let mut r = OperationRegistry::new();
        r.register(Box::new(InsertOperation)).unwrap();
        assert!(matches!(
            r.register(Box::new(InsertOperation)),
            Err(Error::InvalidInput(_))
        ));
    }
}
