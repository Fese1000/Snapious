//! Aufruf von Mapping-Programmen (minimap2, BWA): Kommandozeilen bauen und mit Umleitung der Ausgabe in eine Datei ausführen.

use std::path::Path;

use crate::error::{Error, Result};

/// Argumente für `minimap2`: `-ax <preset> -t <threads> <reference> <reads…>` (Voreinstellungen `sr`, `map-ont`, `map-pb`).
/// `threads` wird mindestens 1. Die Programmnamen selbst gehören nicht in die Liste.
pub fn minimap2_args(
    preset: &str,
    threads: usize,
    reference: &str,
    reads: &[String],
) -> Vec<String> {
    let mut v = vec![
        "-ax".to_string(),
        preset.to_string(),
        "-t".to_string(),
        threads.max(1).to_string(),
        reference.to_string(),
    ];
    v.extend(reads.iter().cloned());
    v
}

/// Argumente für `bwa index <reference>`.
pub fn bwa_index_args(reference: &str) -> Vec<String> {
    vec!["index".to_string(), reference.to_string()]
}

/// Argumente für `bwa mem -t <threads> <reference> <reads…>` (`threads` mindestens 1).
pub fn bwa_mem_args(threads: usize, reference: &str, reads: &[String]) -> Vec<String> {
    let mut v = vec![
        "mem".to_string(),
        "-t".to_string(),
        threads.max(1).to_string(),
        reference.to_string(),
    ];
    v.extend(reads.iter().cloned());
    v
}

/// Ob die BWA-Indexdateien vorliegen: es muss für `reference` die Datei `<reference>.bwt` existieren.
pub fn bwa_index_exists(reference: &str) -> bool {
    Path::new(&format!("{}.bwt", reference)).exists()
}

/// Führt `program args…` aus und schreibt die Standardausgabe in die Datei `out` (angelegt oder überschrieben).
/// Fehler `Error::Io`: Programm startet nicht (Text mit Programmname), Datei nicht anlegbar, Exitcode ≠ 0 (Meldung mit den letzten höchstens
/// 20 Zeilen der Fehlerausgabe).
pub fn run_to_file(program: &str, args: &[String], out: &Path) -> Result<()> {
    let file =
        std::fs::File::create(out).map_err(|e| Error::Io(format!("{}: {}", out.display(), e)))?;
    let result = std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::from(file))
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| Error::Io(format!("{} could not be started: {}", program, e)))?;
    if !result.status.success() {
        let err = String::from_utf8_lossy(&result.stderr);
        let lines: Vec<&str> = err.lines().collect();
        let tail = lines[lines.len().saturating_sub(20)..].join("\n");
        return Err(Error::Io(format!(
            "{} exited with an error: {}",
            program, tail
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn minimap2() {
        assert_eq!(
            minimap2_args("sr", 4, "ref.fa", &s(&["r1.fq", "r2.fq"])),
            s(&["-ax", "sr", "-t", "4", "ref.fa", "r1.fq", "r2.fq"])
        );
        assert_eq!(
            minimap2_args("map-ont", 0, "ref.fa", &s(&["r.fq"])),
            s(&["-ax", "map-ont", "-t", "1", "ref.fa", "r.fq"])
        );
    }

    #[test]
    fn bwa() {
        assert_eq!(bwa_index_args("ref.fa"), s(&["index", "ref.fa"]));
        assert_eq!(
            bwa_mem_args(2, "ref.fa", &s(&["a.fq"])),
            s(&["mem", "-t", "2", "ref.fa", "a.fq"])
        );
        assert_eq!(
            bwa_mem_args(0, "ref.fa", &s(&["a.fq", "b.fq"])),
            s(&["mem", "-t", "1", "ref.fa", "a.fq", "b.fq"])
        );
        let dir = std::env::temp_dir().join(format!("snapious-bwa-{}", crate::util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let r = dir.join("ref.fa");
        let r = r.to_string_lossy().to_string();
        assert!(!bwa_index_exists(&r));
        std::fs::write(format!("{}.bwt", r), "x").unwrap();
        assert!(bwa_index_exists(&r));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ausfuehren_mit_umleitung() {
        let dir = std::env::temp_dir().join(format!("snapious-map-{}", crate::util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out.sam");
        run_to_file(
            "/bin/sh",
            &s(&["-c", "echo '@HD\tVN:1.6'; echo zeile"]),
            &out,
        )
        .unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("@HD"));
        assert!(text.contains("zeile"));
        // Exitcode und Fehlertext
        let e = run_to_file("/bin/sh", &s(&["-c", "echo kaputt 1>&2; exit 3"]), &out).unwrap_err();
        assert!(matches!(e, Error::Io(_)));
        assert!(e.to_string().contains("kaputt"));
        // Programm fehlt
        let e = run_to_file("/nicht/vorhanden/programm", &[], &out).unwrap_err();
        assert!(e.to_string().contains("programm"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
