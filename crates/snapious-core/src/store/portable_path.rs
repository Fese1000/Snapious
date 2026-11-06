//! Pfade verknüpfter Dateien so speichern, dass sie beim Verschieben des Projekts mit dem Ordner `<Projekt>.dateien` mitwandern.

use std::path::{Path, PathBuf};

/// Kennzeichen für Pfade im Ordner `<Projekt>.dateien`.
pub const PORTABLE_PREFIX: &str = "@dateien/";

/// Ordner `<Projektname>.dateien` neben der Projektdatei (die Endung der Projektdatei wird durch `dateien` ersetzt, `Path::with_extension`).
pub fn files_dir(project_path: &Path) -> PathBuf {
    project_path.with_extension("dateien")
}

/// Wandelt einen Dateipfad in die gespeicherte Form: Liegt `file` im Ordner `files_dir(project_path)` (direkt oder in Unterordnern, Vergleich
/// der Pfadbestandteile mit `strip_prefix`), ist das Ergebnis `"@dateien/"` plus der Pfad relativ zu diesem Ordner (Trenner `/`); sonst der
/// Pfad unverändert als Text.
pub fn to_portable(project_path: &Path, file: &Path) -> String {
    let dir = files_dir(project_path);

    if let Ok(relative) = file.strip_prefix(&dir) {
        format!(
            "{}{}",
            PORTABLE_PREFIX,
            relative.to_string_lossy().replace('\\', "/")
        )
    } else {
        file.to_string_lossy().to_string()
    }
}

/// Findet die Datei zu einem gespeicherten Pfad. `exists` sagt, ob ein Pfad vorhanden ist.
/// * Beginnt `stored` mit `"@dateien/"`: `files_dir(project_path)` plus der Rest; ist das vorhanden → `Some`, sonst `None`.
/// * Sonst: ist `stored` selbst vorhanden → `Some(stored)`; sonst wird im Ordner `files_dir(project_path)` nach dem Dateinamen (letzter
///   Bestandteil von `stored`) gesucht; vorhanden → `Some`, sonst `None`.
pub fn resolve_portable(
    project_path: &Path,
    stored: &str,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if let Some(relative) = stored.strip_prefix(PORTABLE_PREFIX) {
        let candidate = files_dir(project_path).join(relative);
        if exists(&candidate) {
            return Some(candidate);
        }
    } else {
        if exists(Path::new(stored)) {
            return Some(PathBuf::from(stored));
        }

        // Fallback: Suche im dateien-Ordner nach dem Dateinamen
        let file_name = Path::new(stored).file_name().and_then(|n| n.to_str());
        if let Some(name) = file_name {
            let candidate = files_dir(project_path).join(name);
            if exists(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordner_und_speichern() {
        let p = Path::new("/arbeit/mein.snapious");
        assert_eq!(files_dir(p), PathBuf::from("/arbeit/mein.dateien"));
        assert_eq!(
            to_portable(p, Path::new("/arbeit/mein.dateien/read1.ab1")),
            "@dateien/read1.ab1"
        );
        assert_eq!(
            to_portable(p, Path::new("/arbeit/mein.dateien/unter/x.ab1")),
            "@dateien/unter/x.ab1"
        );
        assert_eq!(
            to_portable(p, Path::new("/woanders/x.ab1")),
            "/woanders/x.ab1"
        );
        assert_eq!(
            to_portable(p, Path::new("/arbeit/mein.dateienX/x.ab1")),
            "/arbeit/mein.dateienX/x.ab1"
        );
    }

    #[test]
    fn finden() {
        let p = Path::new("/neu/mein.snapious");
        let vorhanden = |x: &Path| {
            x == Path::new("/neu/mein.dateien/read1.ab1") || x == Path::new("/absolut/da.ab1")
        };
        assert_eq!(
            resolve_portable(p, "@dateien/read1.ab1", &vorhanden),
            Some(PathBuf::from("/neu/mein.dateien/read1.ab1"))
        );
        assert_eq!(resolve_portable(p, "@dateien/fehlt.ab1", &vorhanden), None);
        assert_eq!(
            resolve_portable(p, "/absolut/da.ab1", &vorhanden),
            Some(PathBuf::from("/absolut/da.ab1"))
        );
        // altes absolutes Verzeichnis ist weg, die Datei liegt aber im Ordner neben dem Projekt
        assert_eq!(
            resolve_portable(p, "/alt/ort/read1.ab1", &vorhanden),
            Some(PathBuf::from("/neu/mein.dateien/read1.ab1"))
        );
        assert_eq!(resolve_portable(p, "/alt/ort/nix.ab1", &vorhanden), None);
    }
}
