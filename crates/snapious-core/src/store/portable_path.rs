//! Store paths of linked files so that they move along with the `<project>.dateien` folder when the project is moved.

use std::path::{Path, PathBuf};

/// Marker for paths in the `<project>.dateien` folder.
pub const PORTABLE_PREFIX: &str = "@dateien/";

/// Folder `<projectname>.dateien` next to the project file (the extension of the project file is replaced by `dateien`, `Path::with_extension`).
pub fn files_dir(project_path: &Path) -> PathBuf {
    project_path.with_extension("dateien")
}

/// Converts a file path into the stored form: if `file` lies in the folder `files_dir(project_path)` (directly or in subfolders, comparing
/// the path components with `strip_prefix`), the result is `"@dateien/"` plus the path relative to that folder (separator `/`); otherwise the
/// path unchanged as text.
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

/// Finds the file for a stored path. `exists` says whether a path exists.
/// * If `stored` starts with `"@dateien/"`: `files_dir(project_path)` plus the rest; if that exists -> `Some`, otherwise `None`.
/// * Otherwise: if `stored` itself exists -> `Some(stored)`; otherwise the folder `files_dir(project_path)` is searched for the file name (last
///   component of `stored`); if found -> `Some`, otherwise `None`.
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

        // Fallback: search the dateien folder for the file name
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
        // old absolute directory is gone, but the file lies in the folder next to the project
        assert_eq!(
            resolve_portable(p, "/alt/ort/read1.ab1", &vorhanden),
            Some(PathBuf::from("/neu/mein.dateien/read1.ab1"))
        );
        assert_eq!(resolve_portable(p, "/alt/ort/nix.ab1", &vorhanden), None);
    }
}
