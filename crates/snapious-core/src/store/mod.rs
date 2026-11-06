//! Persistenz: Projektdatei (SQLite).

pub mod external;
pub mod folders;
pub mod portable_path;
pub mod project_store;
pub mod versions;

pub use external::ExternalFile;
pub use folders::Folder;
pub use project_store::{DocumentInfo, ProjectStore};
pub use versions::VersionInfo;
