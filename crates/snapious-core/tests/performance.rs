//! Leistungstests mit großzügigen Zeitgrenzen (Fehlerfall: die Rechnung wird merklich zu langsam).

use std::time::Instant;

use snapious_core::bio::msa::progressive_alignment;
use snapious_core::cloning::enzymes::EnzymeDb;
use snapious_core::cloning::fragment::digest;
use snapious_core::cloning::gel::GelSimulator;
use snapious_core::model::{Sequence, SequenceDocument, SequenceType};
use snapious_core::util::Rng;

fn random(len: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    (0..len)
        .map(|_| ['A', 'C', 'G', 'T'][rng.below(4)])
        .collect()
}

fn limit(what: &str, start: Instant, seconds: u64) {
    let e = start.elapsed();
    assert!(e.as_secs() < seconds, "{} dauerte {:?}", what, e);
}

#[test]
fn eine_megabase() {
    let bases = random(1_000_000, 1);
    let seq = Sequence::new("m", "m", &bases, SequenceType::Dna).unwrap();
    let doc = SequenceDocument::of("m", seq);
    let t = Instant::now();
    let doc2 = doc.insert(500_000, "ACGT").unwrap();
    assert_eq!(doc2.len(), 1_000_004);
    limit("Einfügen in 1 Mb", t, 5);
    let t = Instant::now();
    let f = digest(&doc, &EnzymeDb::standard(), &["EcoRI", "BamHI", "HindIII"]).unwrap();
    assert!(f.len() > 100);
    limit("Verdau von 1 Mb", t, 20);
    let t = Instant::now();
    let _ = doc.reverse_complement().unwrap();
    limit("Reverskomplement 1 Mb", t, 5);
}

#[test]
fn alignment_von_dreissig_sequenzen() {
    let ancestor = random(300, 3);
    let mut rng = Rng::new(9);
    let seqs: Vec<Sequence> = (0..30)
        .map(|i| {
            let bases: String = ancestor
                .chars()
                .map(|c| {
                    if rng.below(20) == 0 {
                        ['A', 'C', 'G', 'T'][rng.below(4)]
                    } else {
                        c
                    }
                })
                .collect();
            Sequence::new(
                &format!("s{i}"),
                &format!("s{i}"),
                &bases,
                SequenceType::Dna,
            )
            .unwrap()
        })
        .collect();
    let t = Instant::now();
    let r = progressive_alignment(&seqs).unwrap();
    assert_eq!(r.sequences.len(), 30);
    limit("Alignment von 30 Sequenzen", t, 30);
}

#[test]
fn gel_mit_tausend_fragmenten() {
    let sizes: Vec<usize> = (1..=1000).map(|i| i * 10).collect();
    let t = Instant::now();
    let lane = GelSimulator::new(1.0).lane("x", &sizes, 100.0);
    assert!(!lane.bands.is_empty());
    limit("Gel mit 1000 Fragmenten", t, 5);
}
