//! Settings in a JSON file (e.g. `~/.snapious/settings.json`).

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Program settings.
#[derive(Clone, PartialEq)]
pub struct Settings {
    /// Paths of external programs (name → path).
    pub tool_paths: BTreeMap<String, String>,
    pub ncbi_email: String,
    /// NCBI API key; never shown in `Debug` output.
    pub ncbi_api_key: String,
    pub default_organism: String,
    pub dark_mode: bool,
    /// Maximum number of undo steps.
    pub undo_limit: usize,
}

impl Settings {
    /// Default: empty paths, e-mail, key and organism empty, no dark mode, `undo_limit = 100`.
    pub fn defaults() -> Settings {
        Settings {
            tool_paths: BTreeMap::new(),
            ncbi_email: String::new(),
            ncbi_api_key: String::new(),
            default_organism: String::new(),
            dark_mode: false,
            undo_limit: 100,
        }
    }
}

impl std::fmt::Debug for Settings {
    /// Like a derived `Debug` (field names and values), but `ncbi_api_key` appears only as `"***"` (or `""` if empty).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settings")
            .field("tool_paths", &self.tool_paths)
            .field("ncbi_email", &self.ncbi_email)
            .field(
                "ncbi_api_key",
                if self.ncbi_api_key.is_empty() {
                    &""
                } else {
                    &"***"
                },
            )
            .field("default_organism", &self.default_organism)
            .field("dark_mode", &self.dark_mode)
            .field("undo_limit", &self.undo_limit)
            .finish()
    }
}

/// Reads the settings. If the file is missing, the defaults apply. If it cannot be read as JSON (not an object, syntax error),
/// it is renamed to `<path>.bak` (an existing `.bak` is overwritten) and the defaults apply.
/// Missing fields in the JSON get their default value; fields of the wrong type count as a corrupt file. Keys (JSON, camelCase):
/// `toolPaths` (object text→text), `ncbiEmail`, `ncbiApiKey`, `defaultOrganism`, `darkMode`, `undoLimit`.
pub fn load_settings(path: &Path) -> Settings {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Settings::defaults(),
    };

    #[derive(Deserialize, Default)]
    struct SettingsJson {
        #[serde(default, rename = "toolPaths")]
        tool_paths: BTreeMap<String, String>,
        #[serde(default, rename = "ncbiEmail")]
        ncbi_email: String,
        #[serde(default, rename = "ncbiApiKey")]
        ncbi_api_key: String,
        #[serde(default, rename = "defaultOrganism")]
        default_organism: String,
        #[serde(default, rename = "darkMode")]
        dark_mode: bool,
        #[serde(default, rename = "undoLimit")]
        undo_limit: usize,
    }

    match serde_json::from_str::<SettingsJson>(&content) {
        Ok(json) => Settings {
            tool_paths: json.tool_paths,
            ncbi_email: json.ncbi_email,
            ncbi_api_key: json.ncbi_api_key,
            default_organism: json.default_organism,
            dark_mode: json.dark_mode,
            undo_limit: json.undo_limit,
        },
        Err(_) => {
            let mut bak = path.to_path_buf();
            if let Some(ext) = path.extension() {
                bak.set_extension(format!("{}.bak", ext.to_str().unwrap_or("")));
            } else {
                bak.set_extension("bak");
            }

            let _ = std::fs::remove_file(&bak);
            if path.exists() {
                let _ = std::fs::rename(path, &bak);
            }
            Settings::defaults()
        }
    }
}

/// Writes the settings as indented JSON text (keys as in `load_settings`); missing folders are created.
/// On Unix the file is made readable and writable by the owner only (mode 0600). Errors as `Error::Io`.
pub fn save_settings(path: &Path, s: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::from)?;
    }

    #[derive(Serialize)]
    struct SettingsJson {
        #[serde(rename = "toolPaths")]
        tool_paths: BTreeMap<String, String>,
        #[serde(rename = "ncbiEmail")]
        ncbi_email: String,
        #[serde(rename = "ncbiApiKey")]
        ncbi_api_key: String,
        #[serde(rename = "defaultOrganism")]
        default_organism: String,
        #[serde(rename = "darkMode")]
        dark_mode: bool,
        #[serde(rename = "undoLimit")]
        undo_limit: usize,
    }

    let json = SettingsJson {
        tool_paths: s.tool_paths.clone(),
        ncbi_email: s.ncbi_email.clone(),
        ncbi_api_key: s.ncbi_api_key.clone(),
        default_organism: s.default_organism.clone(),
        dark_mode: s.dark_mode,
        undo_limit: s.undo_limit,
    };

    let text = serde_json::to_string_pretty(&json).map_err(Error::from)?;
    std::fs::write(path, text).map_err(Error::from)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path).map_err(Error::from)?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(Error::from)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("snapious-set-{}-{}", tag, crate::util::new_id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn rundlauf() {
        let d = dir("a");
        let p = d.join("unter").join("settings.json");
        let mut s = Settings::defaults();
        s.tool_paths
            .insert("mafft".to_string(), "/opt/mafft".to_string());
        s.ncbi_email = "a@b.de".to_string();
        s.ncbi_api_key = "GEHEIM123".to_string();
        s.default_organism = "E. coli".to_string();
        s.dark_mode = true;
        s.undo_limit = 7;
        save_settings(&p, &s).unwrap();
        assert_eq!(load_settings(&p), s);
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("\"ncbiEmail\""));
        assert!(text.contains("\n"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn fehlend_und_beschaedigt() {
        let d = dir("b");
        let p = d.join("settings.json");
        assert_eq!(load_settings(&p), Settings::defaults());
        std::fs::write(&p, "{ das ist kein json").unwrap();
        assert_eq!(load_settings(&p), Settings::defaults());
        assert!(!p.exists());
        assert_eq!(
            std::fs::read_to_string(d.join("settings.json.bak")).unwrap(),
            "{ das ist kein json"
        );
        // wrong type
        std::fs::write(&p, r#"{"darkMode": "ja"}"#).unwrap();
        assert_eq!(load_settings(&p), Settings::defaults());
        assert!(d.join("settings.json.bak").exists());
        // partial values
        std::fs::write(&p, r#"{"undoLimit": 5}"#).unwrap();
        let s = load_settings(&p);
        assert_eq!(s.undo_limit, 5);
        assert!(!s.dark_mode);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn schluessel_wird_maskiert() {
        let mut s = Settings::defaults();
        s.ncbi_api_key = "GEHEIM123".to_string();
        let text = format!("{:?}", s);
        assert!(!text.contains("GEHEIM123"));
        assert!(text.contains("***"));
        assert!(text.contains("undo_limit"));
        assert!(!format!("{:?}", Settings::defaults()).contains("***"));
    }
}
