//! Integrationstest mit dem echten pUC19-Eintrag (NCBI L09137).
//! Erwartungswerte wurden unabhängig (Python) aus der Datei berechnet.

use snapious_core::io::genbank;
use snapious_core::model::{Document, Topology};
use snapious_core::store::ProjectStore;

fn laden() -> snapious_core::model::SequenceDocument {
    genbank::read(std::path::Path::new("testdata/puc19.gb"))
        .unwrap()
        .remove(0)
}

#[test]
fn puc19_grunddaten() {
    let d = laden();
    assert_eq!(d.len(), 2686);
    assert_eq!(d.topology(), Topology::Circular);
    assert_eq!(d.name(), "SYNPUC19CV");
    assert_eq!(
        d.metadata().get("accession").map(String::as_str),
        Some("L09137")
    );
    assert_eq!(
        d.metadata().get("organism").map(String::as_str),
        Some("Cloning vector pUC19c")
    );
    assert_eq!(d.features().len(), 1);
    assert_eq!(d.features()[0].feature_type(), "source");
    assert_eq!(d.features()[0].location().length(), 2686);
}

#[test]
fn puc19_gc_gehalt_und_schnittstellen_im_mcs() {
    let d = laden();
    assert!((d.sequence().gc_content() - 0.506_329_113_9).abs() < 1e-9);
    // 0-basierte Startpositionen der Erkennungssequenzen, jeweils genau eine Stelle
    for (motiv, pos) in [
        ("GAATTC", 395),
        ("GGATCC", 416),
        ("AAGCTT", 446),
        ("GAGCTC", 401),
        ("GGTACC", 407),
        ("CTGCAG", 434),
        ("TCTAGA", 422),
        ("GTCGAC", 428),
    ] {
        assert_eq!(d.sequence().find_motif(motiv), vec![pos], "{}", motiv);
    }
}

#[test]
fn puc19_round_trip_genbank_und_projekt() {
    let d = laden();
    let text = genbank::format(&d);
    let e = genbank::parse(&text).unwrap().remove(0);
    assert_eq!(e.bases(), d.bases());
    assert_eq!(e.features(), d.features());
    assert_eq!(e.topology(), d.topology());

    let mut store = ProjectStore::open_in_memory().unwrap();
    let doc = Document::Sequence(d);
    store.save(&doc).unwrap();
    assert_eq!(store.load(doc.id()).unwrap().unwrap(), doc);
}

#[test]
fn puc19_bearbeiten_ueber_dokument() {
    let d = laden();
    // Einfügen mitten in das source-Feature verlängert es
    let e = d.insert(1000, "GGGG").unwrap();
    assert_eq!(e.len(), 2690);
    assert_eq!(e.features()[0].location().length(), 2690);
    // Reverse-Komplement zweimal ergibt das Original
    let r = e
        .reverse_complement()
        .unwrap()
        .reverse_complement()
        .unwrap();
    assert_eq!(r.bases(), e.bases());
}
