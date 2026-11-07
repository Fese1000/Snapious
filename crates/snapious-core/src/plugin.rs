//! Plugins as external programs. Each plugin is a folder with a file `plugin.json`:
//! `{"name": "hello", "version": "1.0", "operations": [{"id": "hello.upper", "label": "Uppercase", "command": "run.sh", "args": ["--x"]}]}`.
//! A plugin operation starts `command` (path relative to the plugin folder), writes the input sequences as FASTA to standard input,
//! passes the parameters as a JSON object in the environment variable `SNAPIOUS_PARAMS` and reads FASTA from standard output.
//! Plugins run without a sandbox, with the privileges of the program.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::ops::operation::{Operation, OperationContext, OperationResult, Params};
use crate::ops::registry::OperationRegistry;

/// Description of an operation in the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginOperationSpec {
    pub id: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
}

/// A loaded plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    /// Folder of the plugin.
    pub dir: PathBuf,
    pub operations: Vec<PluginOperationSpec>,
}

/// Reads a manifest (JSON text) for the plugin in folder `dir`. Required: `name` (non-empty text) and `operations` (list, may be empty);
/// `version` may be missing (then `""`). Each operation needs non-empty texts `id`, `command`; `label` missing → same as `id`; `args` missing → empty.
/// Error `Error::InvalidInput` with an understandable message for invalid JSON or missing required fields.
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

/// Finds all direct subfolders of `directory` containing a file `plugin.json` (alphabetical by folder name) and reads them.
/// Faulty plugins are skipped and reported as warning `"{folder name}: {error text}"`; a missing folder yields no plugins and no warning.
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

/// An operation that calls a plugin program.
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

    /// Inputs must be sequence documents (otherwise `Error::Operation`). Starts `dir.join(command)` with the `args`, working folder = `dir`,
    /// environment variable `SNAPIOUS_PARAMS` = JSON object of the parameters, standard input = all inputs as FASTA
    /// (`crate::io::fasta::format`, names and bases of the sequences), standard output and error captured. Exit code ≠ 0 →
    /// `Error::Operation` with the first 300 characters of the error output; program cannot be started → `Error::Operation`. The standard output is read with
    /// `crate::io::fasta::parse`; each sequence becomes a document (`SequenceDocument::of`), empty output → no output documents.
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
            // write errors (program reads nothing) are ignored; the exit code decides
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

/// Registers the operations of all plugins in the registry. Duplicate IDs (already present) are skipped and reported as warning
/// `"Operation {id} aus Plugin {name} übersprungen: ID already taken"`.
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
        // prints the parameters as the name of a sequence
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
