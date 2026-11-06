//! Operationen: Eingabedokumente + Parameter → Ergebnisdokumente, mit Verlauf.

pub mod analysis;
pub mod builtin;
pub mod more;
pub mod network;
pub mod operation;
pub mod phase2;
pub mod phase3;
pub mod phase4;
pub mod record;
pub mod registry;
pub mod runner;
pub mod workflow;

pub use operation::{
    HistoryEntry, NullContext, Operation, OperationContext, OperationResult, Params,
};
pub use registry::OperationRegistry;
pub use runner::OperationRunner;
pub use workflow::{Workflow, WorkflowResult, WorkflowRunner, WorkflowStep};
