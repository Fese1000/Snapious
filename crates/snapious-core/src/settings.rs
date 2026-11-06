//! Einstellungen in einer JSON-Datei (z. B. `~/.snapious/settings.json`).

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Programmeinstellungen.
#[derive(Clone, PartialEq)]
pub struct Settings {
    /// Pfade externer Programme (Name → Pfad).
    pub tool_paths: BTreeMap<String, String>,
    pub ncbi_email: String,
    /// NCBI-API-Schlüssel; wird nie in `Debug`-Ausgaben gezeigt.
    pub ncbi_api_key: String,
    pub default_organism: String,
    pub dark_mode: bool,
    /// Höchstzahl der Rückgängig-Schritte.
    pub undo_limit: usize,
}

impl Settings {
    /// Standard: leere Pfade, E-Mail, Schlüssel und Organismus leer, kein Dunkelmodus, `undo_limit = 100`.
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
    /// Wie ein abgeleitetes `Debug` (Feldnamen und Werte), aber `ncbi_api_key` erscheint nur als `"***"` (oder `""`, wenn leer).
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

/// Liest die Einstellungen. Fehlt die Datei, sind es die Standardwerte. Lässt sie sich nicht als JSON lesen (kein Objekt, Syntaxfehler),
/// wird sie nach `<pfad>.bak` umbenannt (eine vorhandene `.bak` wird überschrieben) und es gelten die Standardwerte.
/// Fehlende Felder im JSON bekommen ihren Standardwert; falsch getypte Felder gelten als beschädigte Datei. Schlüssel (JSON, camelCase):
/// `toolPaths` (Objekt Text→Text), `ncbiEmail`, `ncbiApiKey`, `defaultOrganism`, `darkMode`, `undoLimit`.
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

/// Schreibt die Einstellungen als eingerückten JSON-Text (Schlüssel wie bei `load_settings`); fehlende Ordner werden angelegt.
/// Unter Unix wird die Datei nur für den Besitzer lesbar und schreibbar gemacht (Modus 0600). Fehler als `Error::Io`.
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
        // falscher Typ
        std::fs::write(&p, r#"{"darkMode": "ja"}"#).unwrap();
        assert_eq!(load_settings(&p), Settings::defaults());
        assert!(d.join("settings.json.bak").exists());
        // teilweise Angaben
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
