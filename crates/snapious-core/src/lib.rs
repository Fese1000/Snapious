//! Snapious core: all logic without UI and without operating-system dependencies.

pub mod bio;
pub mod cloning;
pub mod coverage;
pub mod crispr;
pub mod db;
pub mod error;
pub mod export;
pub mod io;
pub mod model;
pub mod ngs;
pub mod ops;
pub mod plugin;
pub mod primer;
pub mod project;
pub mod search;
pub mod seqcheck;
pub mod settings;
pub mod store;
pub mod tools;
pub mod util;
pub mod view;

pub use error::{Error, Result};
