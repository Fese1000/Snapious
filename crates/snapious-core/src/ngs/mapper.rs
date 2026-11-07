//! Calling mapping programs (minimap2, BWA): build command lines and run them with the output redirected to a file.

use std::path::Path;

use crate::error::{Error, Result};

/// Arguments for `minimap2`: `-ax <preset> -t <threads> <reference> <reads…>` (presets `sr`, `map-ont`, `map-pb`).
/// `threads` becomes at least 1. The program names themselves do not belong in the list.
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

/// Arguments for `bwa index <reference>`.
pub fn bwa_index_args(reference: &str) -> Vec<String> {
    vec!["index".to_string(), reference.to_string()]
}

/// Arguments for `bwa mem -t <threads> <reference> <reads…>` (`threads` at least 1).
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

/// Whether the BWA index files exist: the file `<reference>.bwt` must exist for `reference`.
pub fn bwa_index_exists(reference: &str) -> bool {
    Path::new(&format!("{}.bwt", reference)).exists()
}

/// Runs `program args…` and writes the standard output to the file `out` (created or overwritten).
/// Error `Error::Io`: program does not start (text with program name), file cannot be created, exit code ≠ 0 (message with the last at most
/// 20 lines of the error output).
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
        // exit code and error text
        let e = run_to_file("/bin/sh", &s(&["-c", "echo kaputt 1>&2; exit 3"]), &out).unwrap_err();
        assert!(matches!(e, Error::Io(_)));
        assert!(e.to_string().contains("kaputt"));
        // program missing
        let e = run_to_file("/nicht/vorhanden/programm", &[], &out).unwrap_err();
        assert!(e.to_string().contains("programm"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
