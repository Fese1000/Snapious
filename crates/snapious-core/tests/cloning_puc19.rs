//! Klonierungs-Integrationstest mit dem echten pUC19-Eintrag (NCBI L09137): Verdau, Ligation, Gel, PCR, Primer und Alignment-Abgleich.

use snapious_core::cloning::enzymes::EnzymeDb;
use snapious_core::cloning::fragment::digest;
use snapious_core::cloning::gel::GelSimulator;
use snapious_core::cloning::ligation::restriction_candidates;
use snapious_core::cloning::pcr::{PcrSimulator, Primer};
use snapious_core::io::genbank;
use snapious_core::model::{SequenceDocument, Topology};
use snapious_core::primer::picker::{pick_pairs, PrimerConstraints};

fn puc19() -> SequenceDocument {
    genbank::read(std::path::Path::new("testdata/puc19.gb"))
        .unwrap()
        .remove(0)
}

#[test]
fn einzelschnitt_und_zwei_schnitte_im_mcs() {
    let db = EnzymeDb::standard();
    let d = puc19();
    let eco = digest(&d, &db, &["EcoRI"]).unwrap();
    assert_eq!(eco.len(), 1);
    assert_eq!(eco[0].length(), 2686);
    assert_eq!(eco[0].left, eco[0].right);
    let both = digest(&d, &db, &["EcoRI", "HindIII"]).unwrap();
    assert_eq!(both.len(), 2);
    let mut lengths: Vec<usize> = both.iter().map(|f| f.length()).collect();
    lengths.sort();
    // EcoRI und HindIII liegen im MCS 51 Basen auseinander
    assert_eq!(lengths, vec![51, 2635]);
    assert_eq!(lengths.iter().sum::<usize>(), 2686);
}

#[test]
fn gel_der_fragmente() {
    let db = EnzymeDb::standard();
    let frags = digest(&puc19(), &db, &["EcoRI", "HindIII"]).unwrap();
    let sizes: Vec<usize> = frags.iter().map(|f| f.length()).collect();
    let lane = GelSimulator::new(1.0).lane("pUC19", &sizes, 100.0);
    assert_eq!(lane.bands.len(), 2);
    assert_eq!(lane.bands[0].size_bp, 2635);
    assert!(lane.bands[0].mobility < lane.bands[1].mobility);
    assert!((lane.bands[0].intensity - 1.0).abs() < 1e-9);
}

#[test]
fn umklonieren_des_mcs_in_den_eigenen_vektor() {
    // Das 51-bp-MCS-Fragment (EcoRI-HindIII) in sich selbst gesetzt ergibt wieder ein Plasmid derselben Länge.
    let db = EnzymeDb::standard();
    let d = puc19();
    let c =
        restriction_candidates(&d, &["EcoRI", "HindIII"], &d, &["EcoRI", "HindIII"], &db).unwrap();
    assert!(!c.is_empty());
    assert!(c
        .iter()
        .all(|k| k.construct.topology() == Topology::Circular));
    assert!(c.iter().any(|k| k.construct.len() == 2686 && k.directional));
}

#[test]
fn pcr_ueber_den_ursprung_und_primerwahl() {
    let d = puc19();
    let c = PrimerConstraints::standard();
    let pairs = pick_pairs(d.bases(), 1000, 1300, &c, 3).unwrap();
    assert!(!pairs.is_empty());
    let best = &pairs[0];
    let primers = [
        Primer {
            name: "fwd".to_string(),
            sequence: best.forward.sequence.clone(),
        },
        Primer {
            name: "rev".to_string(),
            sequence: best.reverse.sequence.clone(),
        },
    ];
    let products = PcrSimulator::default().simulate(d.bases(), true, &primers);
    let wanted: Vec<_> = products
        .iter()
        .filter(|p| p.sequence.len() == best.product_size)
        .collect();
    assert_eq!(wanted.len(), 1, "exactly one product of the expected size");
    assert!(wanted[0].sequence.starts_with(&best.forward.sequence));
}
