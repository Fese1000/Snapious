//! Database access (NCBI Entrez) without its own network library: addresses and responses are processed here,
//! the actual request is handled by an `HttpGet` (in the program, `curl`).

pub mod cache;
pub mod ncbi;
pub mod uniprot;
pub mod uniprot_entry;
