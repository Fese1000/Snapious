//! Directory of all operations by ID.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::ops::builtin::{InsertOperation, ReverseComplementOperation};
use crate::ops::more::all_operations;
use crate::ops::operation::Operation;

/// All known operations, sorted by ID.
#[derive(Default)]
pub struct OperationRegistry {
    ops: BTreeMap<String, Box<dyn Operation>>,
}

impl OperationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry with all built-in operations.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        let mut all: Vec<Box<dyn Operation>> = vec![
            Box::new(ReverseComplementOperation),
            Box::new(InsertOperation),
        ];
        all.extend(all_operations());
        for op in all {
            // IDs of the built-in operations are unique (guaranteed by a test)
            let _ = r.register(op);
        }
        r
    }

    /// Adds an operation; duplicate ID → `Error::InvalidInput`.
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

    /// All operations sorted by ID.
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
