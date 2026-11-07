//! Data model: sequence, features, documents.

pub mod alignment;
pub mod document;
pub mod feature;
pub mod rotate;
pub mod sequence;

pub use alignment::AlignmentDocument;
pub use document::{Document, SequenceDocument, Topology};
pub use feature::{Feature, Interval, Location, Strand};
pub use sequence::{Sequence, SequenceType};
