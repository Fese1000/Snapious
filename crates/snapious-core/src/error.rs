//! Gemeinsamer Fehlertyp des Kerns.

use thiserror::Error;

/// Fehler des Kerns. Meldungen sind für Nutzer gedacht (deutsch).
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    /// Ungültige Eingabe (falscher Wert, leere Sequenz, unbekannter Name).
    #[error("Ungültige Eingabe: {0}")]
    InvalidInput(String),
    /// Aufruf im falschen Zustand.
    #[error("Ungültiger Zustand: {0}")]
    InvalidState(String),
    /// Position oder Bereich außerhalb der Sequenz.
    #[error("Außerhalb des gültigen Bereichs: {0}")]
    OutOfRange(String),
    /// Fehler beim Lesen oder Schreiben von Dateien.
    #[error("Dateifehler: {0}")]
    Io(String),
    /// Eine Operation ist fehlgeschlagen (Meldung für die Oberfläche).
    #[error("{0}")]
    Operation(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// Ergebnis mit dem Kern-Fehlertyp.
pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Io(format!("Datenbank: {}", e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Io(format!("JSON: {}", e))
    }
}
