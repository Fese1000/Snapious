//! Prüft, dass installierte Bioinformatik-Programme über `ToolRegistry` aufrufbar sind.
//! Fehlende Programme werden übersprungen.

use std::time::Duration;

use snapious_core::tools::ToolRegistry;

fn pruefe(tool: &str, args: &[&str]) {
    let reg = ToolRegistry::defaults();
    if !reg.is_available(tool) {
        eprintln!("skipped: {} not installed", tool);
        return;
    }
    let r = reg
        .run(tool, args, None, None, Duration::from_secs(30))
        .unwrap();
    assert!(
        !(r.stdout.trim().is_empty() && r.stderr.trim().is_empty()),
        "{} produced no output",
        tool
    );
}

#[test]
fn mafft_version() {
    pruefe("mafft", &["--version"]);
}

#[test]
fn blastn_version() {
    pruefe("blastn", &["-version"]);
}

#[test]
fn iqtree_version() {
    pruefe("iqtree3", &["--version"]);
}

#[test]
fn primer3_hilfe() {
    pruefe("primer3_core", &["--help"]);
}

#[test]
fn mafft_richtet_sequenzen_aus() {
    let reg = ToolRegistry::defaults();
    if !reg.is_available("mafft") {
        return;
    }
    let fasta = ">a\nATGCGTACGTAGCTAGC\n>b\nATGCGTACGTAGCTAGC\n>c\nATGCGTAGCTAGC\n";
    let r = reg
        .run(
            "mafft",
            &["--quiet", "-"],
            None,
            Some(fasta),
            Duration::from_secs(30),
        )
        .unwrap();
    assert!(r.ok(), "{}", r.stderr);
    let rows: Vec<&str> = r.stdout.lines().filter(|l| !l.starts_with('>')).collect();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|z| z.len() == rows[0].len()));
    assert!(rows[2].contains('-'), "shorter sequence needs a gap");
}
