//! Aufruf externer Programme (MAFFT, BLAST+, Primer3, …) mit Zeitlimit.
//!
//! Plattformneutral: Suche im `PATH` und in üblichen Installationsordnern, unter Windows auch mit `.exe`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

/// Ergebnis eines Programmlaufs.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    /// Exitcode (-1, wenn das Programm durch ein Signal beendet wurde).
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

impl ToolResult {
    pub fn ok(&self) -> bool {
        self.exit_code == 0
    }
}

/// Verwaltet die Fundorte externer Programme.
#[derive(Clone, Debug, Default)]
pub struct ToolRegistry {
    overrides: BTreeMap<String, PathBuf>,
    extra_dirs: Vec<PathBuf>,
}

impl ToolRegistry {
    /// Suche im `PATH` und in `/opt/homebrew/bin`, `/usr/local/bin`.
    pub fn defaults() -> Self {
        Self {
            overrides: BTreeMap::new(),
            extra_dirs: vec![
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
            ],
        }
    }

    /// Registry, die nur die angegebenen Ordner durchsucht (für Tests).
    pub fn with_search_dirs(dirs: Vec<PathBuf>) -> Self {
        Self {
            overrides: BTreeMap::new(),
            extra_dirs: dirs,
        }
    }

    /// Kopie mit fest eingetragenem Pfad für ein Programm.
    pub fn with_executable(&self, tool: &str, path: &Path) -> Self {
        let mut copy = self.clone();
        copy.overrides.insert(tool.to_string(), path.to_path_buf());
        copy
    }

    /// Pfad des Programms, falls gefunden.
    pub fn executable(&self, tool: &str) -> Option<PathBuf> {
        if let Some(p) = self.overrides.get(tool) {
            return if is_executable(p) {
                Some(p.clone())
            } else {
                None
            };
        }
        let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        dirs.extend(self.extra_dirs.iter().cloned());
        for dir in dirs {
            for name in [tool.to_string(), format!("{}.exe", tool)] {
                let candidate = dir.join(&name);
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub fn is_available(&self, tool: &str) -> bool {
        self.executable(tool).is_some()
    }

    /// Startet das Programm, wartet höchstens `timeout` und liefert Ausgabe und Exitcode.
    pub fn run(
        &self,
        tool: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        stdin: Option<&str>,
        timeout: Duration,
    ) -> Result<ToolResult> {
        let exe = self
            .executable(tool)
            .ok_or_else(|| Error::Io(format!("Programm '{}' wurde nicht gefunden", tool)))?;
        let mut cmd = Command::new(exe);
        cmd.args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        let start = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Io(format!("'{}' konnte nicht gestartet werden: {}", tool, e)))?;

        let stdin_thread = match (stdin, child.stdin.take()) {
            (Some(text), Some(mut pipe)) => {
                let data = text.as_bytes().to_vec();
                Some(thread::spawn(move || {
                    let _ = pipe.write_all(&data); // Programm darf Eingabe früher schließen
                }))
            }
            _ => None,
        };
        let out_thread = child.stdout.take().map(read_all_in_thread);
        let err_thread = child.stderr.take().map(read_all_in_thread);

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => {
                    return Err(Error::Io(format!(
                        "Warten auf '{}' fehlgeschlagen: {}",
                        tool, e
                    )))
                }
            }
        };

        if let Some(t) = stdin_thread {
            let _ = t.join();
        }
        let stdout = out_thread.map(join_text).unwrap_or_default();
        let stderr = err_thread.map(join_text).unwrap_or_default();
        match status {
            Some(s) => Ok(ToolResult {
                exit_code: s.code().unwrap_or(-1),
                stdout,
                stderr,
                duration: start.elapsed(),
            }),
            None => Err(Error::Io(format!(
                "Zeitüberschreitung: '{}' lief länger als {} s",
                tool,
                timeout.as_secs_f64()
            ))),
        }
    }
}

/// Kurzform: ist das Programm mit den Standardsuchpfaden auffindbar?
pub fn is_available(tool: &str) -> bool {
    ToolRegistry::defaults().is_available(tool)
}

fn read_all_in_thread<R: Read + Send + 'static>(mut r: R) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = r.read_to_end(&mut buf);
        buf
    })
}

fn join_text(h: thread::JoinHandle<Vec<u8>>) -> String {
    String::from_utf8_lossy(&h.join().unwrap_or_default()).to_string()
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.is_file()
        && std::fs::metadata(p)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    const T: Duration = Duration::from_secs(10);

    #[test]
    fn echo_liefert_ausgabe() {
        let r = ToolRegistry::defaults()
            .with_executable("echo", Path::new("/bin/echo"))
            .run("echo", &["hallo"], None, None, T)
            .unwrap();
        assert_eq!(r.stdout, "hallo\n");
        assert!(r.ok());
    }

    #[test]
    fn stdin_wird_durchgereicht() {
        let r = ToolRegistry::defaults()
            .run("cat", &[], None, Some("abc"), T)
            .unwrap();
        assert_eq!(r.stdout, "abc");
    }

    #[test]
    fn exitcode_und_stderr() {
        let r = ToolRegistry::defaults()
            .run("sh", &["-c", "echo fehler >&2; exit 3"], None, None, T)
            .unwrap();
        assert_eq!(r.exit_code, 3);
        assert!(!r.ok());
        assert_eq!(r.stderr, "fehler\n");
    }

    #[test]
    fn zeitueberschreitung_beendet_das_programm() {
        let start = Instant::now();
        let r =
            ToolRegistry::defaults().run("sleep", &["5"], None, None, Duration::from_millis(200));
        assert!(matches!(r, Err(Error::Io(m)) if m.contains("Zeitüberschreitung")));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn unbekanntes_programm() {
        let reg = ToolRegistry::defaults();
        assert!(!reg.is_available("gibtsnicht-xyz"));
        assert!(reg.run("gibtsnicht-xyz", &[], None, None, T).is_err());
    }

    #[test]
    fn grosse_ausgabe_blockiert_nicht() {
        let r = ToolRegistry::defaults()
            .run("sh", &["-c", "yes | head -n 200000"], None, None, T)
            .unwrap();
        assert_eq!(r.stdout.lines().count(), 200_000);
    }

    #[test]
    fn arbeitsverzeichnis() {
        let dir = tempfile::tempdir().unwrap();
        let r = ToolRegistry::defaults()
            .run("pwd", &[], Some(dir.path()), None, T)
            .unwrap();
        let reported = std::fs::canonicalize(r.stdout.trim()).unwrap();
        assert_eq!(reported, std::fs::canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn programm_das_stdin_ignoriert_haengt_nicht() {
        let big = "x".repeat(2_000_000);
        let r = ToolRegistry::defaults()
            .run("sh", &["-c", "exit 0"], None, Some(&big), T)
            .unwrap();
        assert!(r.ok());
    }

    #[test]
    fn eingetragener_pfad_ohne_programm_gilt_als_nicht_verfuegbar() {
        let reg = ToolRegistry::defaults().with_executable("x", Path::new("/nicht/da"));
        assert!(!reg.is_available("x"));
    }

    #[test]
    fn eigene_suchordner() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("meintool");
        std::fs::write(&script, "#!/bin/sh\necho ok\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let reg = ToolRegistry::with_search_dirs(vec![dir.path().to_path_buf()]);
        assert_eq!(
            reg.run("meintool", &[], None, None, T).unwrap().stdout,
            "ok\n"
        );
    }
}
