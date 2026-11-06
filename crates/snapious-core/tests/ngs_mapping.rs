//! Mapping mit dem echten minimap2 (nur wenn installiert): Reads aus einer Zufallsreferenz müssen an ihrer Herkunft landen.

use snapious_core::io::sam;
use snapious_core::ngs::mapper::{minimap2_args, run_to_file};
use snapious_core::tools::ToolRegistry;
use snapious_core::util::Rng;

#[test]
fn reads_landen_an_der_herkunft() {
    let Some(minimap2) = ToolRegistry::defaults().executable("minimap2") else {
        eprintln!("minimap2 nicht installiert, Test übersprungen");
        return;
    };
    let mut rng = Rng::new(21);
    let reference: String = (0..5000)
        .map(|_| ['A', 'C', 'G', 'T'][rng.below(4)])
        .collect();
    let dir = std::env::temp_dir().join(format!("snapious-mm-{}", snapious_core::util::new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ref.fa"), format!(">ref\n{}\n", reference)).unwrap();
    let mut fq = String::new();
    let mut origins = Vec::new();
    for i in 0..200 {
        let start = rng.below(5000 - 100);
        let mut read: Vec<char> = reference[start..start + 100].chars().collect();
        // etwa 1 % Fehler
        for c in read.iter_mut() {
            if rng.below(100) == 0 {
                *c = if *c == 'A' { 'C' } else { 'A' };
            }
        }
        let read: String = read.into_iter().collect();
        fq.push_str(&format!("@r{}\n{}\n+\n{}\n", i, read, "I".repeat(100)));
        origins.push(start);
    }
    std::fs::write(dir.join("reads.fq"), fq).unwrap();
    let out = dir.join("out.sam");
    let args = minimap2_args(
        "sr",
        2,
        &dir.join("ref.fa").to_string_lossy(),
        &[dir.join("reads.fq").to_string_lossy().to_string()],
    );
    run_to_file(&minimap2.to_string_lossy(), &args, &out).unwrap();
    let parsed = sam::read(&out).unwrap();
    let mut good = 0;
    for r in &parsed.records {
        if let Some(p) = r.position0() {
            let idx: usize = r.read_name[1..].parse().unwrap();
            if p.abs_diff(origins[idx]) <= 5 {
                good += 1;
            }
        }
    }
    assert!(good >= 190, "nur {} von 200 Reads an der Herkunft", good);
    let _ = std::fs::remove_dir_all(dir);
}
