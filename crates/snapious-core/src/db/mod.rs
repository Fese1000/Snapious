//! Datenbankzugriff (NCBI Entrez) ohne eigene Netzwerkbibliothek: Adressen und Antworten werden hier verarbeitet,
//! die eigentliche Abfrage übernimmt ein `HttpGet` (im Programm `curl`).

pub mod cache;
pub mod ncbi;
pub mod uniprot;
pub mod uniprot_entry;
