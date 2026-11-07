//! Plugins als externe Programme. Jedes Plugin ist ein Ordner mit einer Datei `plugin.json`:
//! `{"name": "hello", "version": "1.0", "operations": [{"id": "hello.upper", "label": "Uppercase", "command": "run.sh", "args": ["--x"]}]}`.
//! Eine Plugin-Operation startet `command` (Pfad relativ zum Plugin-Ordner), schreibt die Eingabesequenzen als FASTA auf die Standardeingabe,
//! übergibt die Parameter als JSON-Objekt in der Umgebungsvariablen `SNAPIOUS_PARAMS` und liest FASTA von der Standardausgabe.
//! Plugins laufen ohne Schutzraum (Sandbox) mit den Rechten des Programms.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::ops::operation::{Operation, OperationContext, OperationResult, Params};
use crate::ops::registry::OperationRegistry;

/// Beschreibung einer Operation im Manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginOperationSpec {
    pub id: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
}

/// Ein geladenes Plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    /// Ordner des Plugins.
    pub dir: PathBuf,
    pub operations: Vec<PluginOperationSpec>,
}

/// Liest ein Manifest (JSON-Text) für das Plugin im Ordner `dir`. Pflicht: `name` (nicht leerer Text) und `operations` (Liste, darf leer sein);
/// `version` darf fehlen (dann `""`). Jede Operation braucht nicht leere Texte `id`, `command`; `label` fehlt → gleich `id`; `args` fehlt → leer.
/// Fehler `Error::InvalidInput` mit verständlicher Meldung bei ungültigem JSON oder fehlenden Pflichtfeldern.
pub fn parse_manifest(json: &str, dir: &Path) -> Result<Plugin> {
    let bad = |m: &str| Error::InvalidInput(m.to_string());
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| Error::InvalidInput(format!("Invalid JSON in manifest: {}", e)))?;
    let name = v
        .get("name")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| bad("Manifest: 'name' is missing or empty"))?
        .to_string();
    let version = v
        .get("version")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let ops = v
        .get("operations")
        .and_then(|x| x.as_array())
        .ok_or_else(|| bad("Manifest: 'operations' is missing or not a list"))?;
    let mut operations = Vec::new();
    for (i, o) in ops.iter().enumerate() {
        let text = |key: &str| {
            o.get(key)
                .and_then(|x| x.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string())
        };
        let id = text("id")
            .ok_or_else(|| bad(&format!("Manifest: operation {} has no 'id'", i + 1)))?;
        let command = text("command").ok_or_else(|| {
            bad(&format!(
                "Manifest: operation '{}' has no 'command'",
                id
            ))
        })?;
        let label = text("label").unwrap_or_else(|| id.clone());
        let args = o
            .get("args")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        operations.push(PluginOperationSpec {
            id,
            label,
            command,
            args,
        });
    }
    Ok(Plugin {
        name,
        version,
        dir: dir.to_path_buf(),
        operations,
    })
}

/// Sucht in `directory` alle direkten Unterordner mit einer Datei `plugin.json` (alphabetisch nach Ordnername) und liest sie.
/// Fehlerhafte Plugins werden übersprungen und als Warnung `"{ordnername}: {fehlertext}"` gemeldet; ein fehlender Ordner ergibt keine Plugins und keine Warnung.
pub fn discover(directory: &Path) -> (Vec<Plugin>, Vec<String>) {
    let mut plugins = Vec::new();
    let mut warnings = Vec::new();
    let Ok(read) = std::fs::read_dir(directory) else {
        return (plugins, warnings);
    };
    let mut dirs: Vec<PathBuf> = read
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("plugin.json").is_file())
        .collect();
    dirs.sort();
    for d in dirs {
        let folder = d
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let result = std::fs::read_to_string(d.join("plugin.json"))
            .map_err(|e| Error::Io(e.to_string()))
            .and_then(|text| parse_manifest(&text, &d));
        match result {
            Ok(p) => plugins.push(p),
            Err(e) => warnings.push(format!("{}: {}", folder, e)),
        }
    }
    (plugins, warnings)
}

/// Eine Operation, die ein Plugin-Programm aufruft.
pub struct PluginOperation {
    pub spec: PluginOperationSpec,
    pub dir: PathBuf,
}

impl Operation for PluginOperation {
    fn id(&self) -> &str {
        &self.spec.id
    }

    fn label(&self) -> &str {
        &self.spec.label
    }

    /// Eingaben müssen Sequenzdokumente sein (sonst `Error::Operation`). Starte `dir.join(command)` mit den `args`, Arbeitsordner = `dir`,
    /// Umgebungsvariable `SNAPIOUS_PARAMS` = JSON-Objekt der Parameter, Standardeingabe = alle Eingaben als FASTA
    /// (`crate::io::fasta::format`, Namen und Basen der Sequenzen), Standardausgabe und -fehler aufgefangen. Exitcode ≠ 0 →
    /// `Error::Operation` mit den ersten 300 Zeichen der Fehlerausgabe; Programm nicht startbar → `Error::Operation`. Die Standardausgabe wird mit
    /// `crate::io::fasta::parse` gelesen; jede Sequenz wird ein Dokument (`SequenceDocument::of`), leere Ausgabe → keine Ausgabedokumente.
    /// `summary` = `"{label}: {n} Sequenz(en)"`.
    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        use std::io::Write;
        let mut seqs = Vec::new();
        for d in inputs {
            match d {
                Document::Sequence(s) => seqs.push(s.sequence().clone()),
                _ => {
                    return Err(Error::Operation(
                        "All inputs must be sequences".to_string(),
                    ))
                }
            }
        }
        let params_json =
            serde_json::to_string(params).map_err(|e| Error::Operation(e.to_string()))?;
        let mut child = std::process::Command::new(self.dir.join(&self.spec.command))
            .args(&self.spec.args)
            .current_dir(&self.dir)
            .env("SNAPIOUS_PARAMS", params_json)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                Error::Operation(format!(
                    "The plugin program '{}' cannot be started: {}",
                    self.spec.command, e
                ))
            })?;
        let fasta = crate::io::fasta::format(&seqs);
        if let Some(mut stdin) = child.stdin.take() {
            // Schreibfehler (Programm liest nichts) werden ignoriert; der Exitcode entscheidet
            let _ = stdin.write_all(fasta.as_bytes());
        }
        let out = child
            .wait_with_output()
            .map_err(|e| Error::Operation(e.to_string()))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let short: String = err.trim().chars().take(300).collect();
            return Err(Error::Operation(format!(
                "Plugin '{}' exited with an error: {}",
                self.spec.id, short
            )));
        }
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let parsed = if text.trim().is_empty() {
            Vec::new()
        } else {
            crate::io::fasta::parse(&text).map_err(|e| Error::Operation(e.to_string()))?
        };
        let outputs: Vec<Document> = parsed
            .into_iter()
            .map(|s| {
                let name = s.name().to_string();
                Document::Sequence(crate::model::document::SequenceDocument::of(&name, s))
            })
            .collect();
        let summary = format!("{}: {} sequence(s)", self.spec.label, outputs.len());
        Ok(OperationResult { outputs, summary })
    }
}

/// Trägt die Operationen aller Plugins in die Registry ein. Doppelte IDs (schon vorhanden) werden übersprungen und als Warnung
/// `"Operation {id} aus Plugin {name} übersprungen: ID already taken"` gemeldet.
pub fn register_plugins(registry: &mut OperationRegistry, plugins: &[Plugin]) -> Vec<String> {
    let mut warnings = Vec::new();
    for p in plugins {
        for spec in &p.operations {
            let op = PluginOperation {
                spec: spec.clone(),
                dir: p.dir.clone(),
            };
            if registry.register(Box::new(op)).is_err() {
                warnings.push(format!(
                    "Operation {} from plugin {} skipped: ID already taken",
                    spec.id, p.name
                ));
            }
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::SequenceDocument;
    use crate::model::sequence::{Sequence, SequenceType};
    use crate::ops::operation::NullContext;

    fn dir(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("snapious-plug-{}-{}", tag, crate::util::new_id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const MANIFEST: &str = r#"{"name":"hello","version":"1.2","operations":[
        {"id":"hello.upper","label":"Uppercase","command":"run.sh","args":["-x"]},
        {"id":"hello.other","command":"run.sh"}]}"#;

    #[test]
    fn manifest_lesen() {
        let p = parse_manifest(MANIFEST, Path::new("/p/hello")).unwrap();
        assert_eq!(p.name, "hello");
        assert_eq!(p.version, "1.2");
        assert_eq!(p.dir, PathBuf::from("/p/hello"));
        assert_eq!(p.operations.len(), 2);
        assert_eq!(p.operations[0].label, "Uppercase");
        assert_eq!(p.operations[0].args, vec!["-x"]);
        assert_eq!(p.operations[1].label, "hello.other");
        assert!(p.operations[1].args.is_empty());
        let min = parse_manifest(r#"{"name":"x","operations":[]}"#, Path::new("/p")).unwrap();
        assert_eq!(min.version, "");
        for bad in [
            "kein json",
            r#"{"operations":[]}"#,
            r#"{"name":"","operations":[]}"#,
            r#"{"name":"x"}"#,
            r#"{"name":"x","operations":[{"id":"a"}]}"#,
            r#"{"name":"x","operations":[{"command":"a"}]}"#,
        ] {
            assert!(
                matches!(
                    parse_manifest(bad, Path::new("/p")),
                    Err(Error::InvalidInput(_))
                ),
                "{}",
                bad
            );
        }
    }

    #[test]
    fn plugins_finden() {
        let d = dir("find");
        std::fs::create_dir_all(d.join("b_plugin")).unwrap();
        std::fs::write(d.join("b_plugin/plugin.json"), MANIFEST).unwrap();
        std::fs::create_dir_all(d.join("a_kaputt")).unwrap();
        std::fs::write(d.join("a_kaputt/plugin.json"), "{").unwrap();
        std::fs::create_dir_all(d.join("c_ohne")).unwrap();
        std::fs::write(d.join("datei.txt"), "x").unwrap();
        let (plugins, warnings) = discover(&d);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "hello");
        assert_eq!(plugins[0].dir, d.join("b_plugin"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("a_kaputt: "));
        let (none, w) = discover(&d.join("gibtsnicht"));
        assert!(none.is_empty() && w.is_empty());
        let _ = std::fs::remove_dir_all(d);
    }

    #[cfg(unix)]
    fn script(dir: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join("run.sh");
        std::fs::write(&p, format!("#!/bin/sh\n{}\n", body)).unwrap();
        let mut perm = std::fs::metadata(&p).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&p, perm).unwrap();
    }

    fn seq_doc(name: &str, bases: &str) -> Document {
        let s = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of(name, s))
    }

    #[cfg(unix)]
    #[test]
    fn operation_laeuft() {
        let d = dir("run");
        script(&d, "tr a-z A-Z");
        let op = PluginOperation {
            spec: PluginOperationSpec {
                id: "hello.upper".to_string(),
                label: "Uppercase".to_string(),
                command: "run.sh".to_string(),
                args: vec![],
            },
            dir: d.clone(),
        };
        let r = op
            .run(&[seq_doc("s", "acgt")], &Params::new(), &mut NullContext)
            .unwrap();
        assert_eq!(r.outputs.len(), 1);
        match &r.outputs[0] {
            Document::Sequence(s) => {
                assert_eq!(s.bases(), "ACGT");
                assert_eq!(s.name(), "S");
            }
            _ => panic!("keine Sequenz"),
        }
        assert_eq!(r.summary, "Uppercase: 1 sequence(s)");
        assert_eq!(op.id(), "hello.upper");
        let _ = std::fs::remove_dir_all(d);
    }

    #[cfg(unix)]
    #[test]
    fn parameter_und_fehler() {
        let d = dir("par");
        // gibt die Parameter als Name einer Sequenz aus
        script(&d, "cat > /dev/null; echo \">$SNAPIOUS_PARAMS\"; echo ACGT");
        let mk = |d: &PathBuf| PluginOperation {
            spec: PluginOperationSpec {
                id: "x.y".to_string(),
                label: "X".to_string(),
                command: "run.sh".to_string(),
                args: vec![],
            },
            dir: d.clone(),
        };
        let params: Params = [("k".to_string(), "v".to_string())].into_iter().collect();
        let r = mk(&d)
            .run(&[seq_doc("s", "AC")], &params, &mut NullContext)
            .unwrap();
        match &r.outputs[0] {
            Document::Sequence(s) => assert!(s.name().contains("\"k\":\"v\"")),
            _ => panic!("keine Sequenz"),
        }
        script(&d, "echo kaputt 1>&2; exit 4");
        let e = mk(&d)
            .run(&[seq_doc("s", "AC")], &Params::new(), &mut NullContext)
            .unwrap_err();
        assert!(matches!(e, Error::Operation(_)));
        assert!(e.to_string().contains("kaputt"));
        script(&d, "cat > /dev/null");
        let leer = mk(&d)
            .run(&[seq_doc("s", "AC")], &Params::new(), &mut NullContext)
            .unwrap();
        assert!(leer.outputs.is_empty());
        let fehlt = PluginOperation {
            spec: PluginOperationSpec {
                id: "x.z".to_string(),
                label: "Z".to_string(),
                command: "gibtsnicht.sh".to_string(),
                args: vec![],
            },
            dir: d.clone(),
        };
        assert!(matches!(
            fehlt.run(&[seq_doc("s", "AC")], &Params::new(), &mut NullContext),
            Err(Error::Operation(_))
        ));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn registrieren() {
        let p = parse_manifest(MANIFEST, Path::new("/p/hello")).unwrap();
        let mut reg = OperationRegistry::with_builtins();
        let before = reg.all().len();
        let w = register_plugins(&mut reg, std::slice::from_ref(&p));
        assert!(w.is_empty());
        assert_eq!(reg.all().len(), before + 2);
        assert!(reg.find("hello.upper").is_some());
        let w2 = register_plugins(&mut reg, &[p]);
        assert_eq!(w2.len(), 2);
        assert!(w2[0].contains("hello.upper") && w2[0].contains("already taken"));
        assert_eq!(reg.all().len(), before + 2);
    }
}
