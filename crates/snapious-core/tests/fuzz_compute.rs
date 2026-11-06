//! Zufallstest: Rechenfunktionen mit zufälligen und extremen Eingaben dürfen Fehler melden oder leere Ergebnisse liefern, aber nie abstürzen.

use std::panic::catch_unwind;

use snapious_core::bio::align_tools::align_profiles;
use snapious_core::bio::assembly::assemble_contigs;
use snapious_core::bio::dotplot::{diagonals, dotplot_points};
use snapious_core::bio::motif::find_motif;
use snapious_core::bio::protein::{
    charge_at_ph, isoelectric_point, kyte_doolittle, molecular_weight,
};
use snapious_core::bio::protein_digest::{digest_protein, Protease};
use snapious_core::bio::read_map::{map_read, map_reads, pileup_consensus};
use snapious_core::bio::repeats::{find_inverted_repeats, find_tandem_repeats};
use snapious_core::bio::variants::find_variants;
use snapious_core::cloning::enzymes::EnzymeDb;
use snapious_core::cloning::fragment::digest;
use snapious_core::cloning::pcr::{PcrSimulator, Primer};
use snapious_core::crispr::{find_guides, PamSpec};
use snapious_core::model::Sequence;
use snapious_core::model::{SequenceDocument, SequenceType, Topology};
use snapious_core::primer::mutagenesis::design_mutagenesis;
use snapious_core::primer::picker::{pick_pairs, PrimerConstraints};
use snapious_core::util::Rng;
use snapious_core::view::translation::translate_frame;

fn dna(rng: &mut Rng, max: usize) -> String {
    let n = rng.below(max + 1);
    let alphabet: &[u8] = if rng.below(4) == 0 {
        b"ACGTNRYacgtn-"
    } else {
        b"ACGT"
    };
    (0..n)
        .map(|_| alphabet[rng.below(alphabet.len())] as char)
        .collect()
}

fn num(rng: &mut Rng) -> usize {
    match rng.below(6) {
        0 => 0,
        1 => 1,
        2 => rng.below(10),
        3 => rng.below(300),
        4 => usize::MAX,
        _ => usize::MAX - rng.below(5),
    }
}

#[test]
fn zufaellige_eingaben_stuerzen_nicht_ab() {
    let mut rng = Rng::new(77);
    let db = EnzymeDb::standard();
    let enzyme_names: Vec<String> = db.all().iter().map(|e| e.name().to_string()).collect();
    let mut failures: Vec<String> = Vec::new();
    macro_rules! chk {
        ($name:expr, $e:expr) => {
            if catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = $e;
            }))
            .is_err()
            {
                failures.push($name.to_string());
            }
        };
    }
    for _ in 0..400 {
        let a = dna(&mut rng, 80);
        let b = dna(&mut rng, 80);
        let (n1, n2, n3, n4) = (num(&mut rng), num(&mut rng), num(&mut rng), num(&mut rng));
        let small = rng.below(30);
        chk!("tandem", find_tandem_repeats(&a, n1, n2, n3));
        chk!("inverted", find_inverted_repeats(&a, n1, n2));
        chk!(
            "dotplot",
            dotplot_points(&a, &b, n1 % 50, n2 % 50, rng.below(2) == 0)
        );
        chk!("diagonals", diagonals(&[(n1 % 100, n2 % 100), (0, 0)]));
        chk!(
            "assemble",
            assemble_contigs(&[a.clone(), b.clone()], n1 % 60)
        );
        chk!("map_read", map_read(&a, &b, n1 % 20));
        chk!("map_reads", map_reads(&a, &[b.clone(), a.clone()], n1 % 5));
        chk!(
            "pileup",
            pileup_consensus(&a, std::slice::from_ref(&b), &[None])
        );
        chk!("variants", find_variants(&[a.clone(), b.clone()], n1 % 3));
        chk!(
            "profiles",
            align_profiles(std::slice::from_ref(&a), std::slice::from_ref(&b))
        );
        chk!(
            "mutagenesis",
            design_mutagenesis(&a, n1, n2, &b, n3 % 40, n4 % 80, 60.0)
        );
        chk!(
            "pick_pairs",
            pick_pairs(
                &a.repeat(8),
                n1 % 400,
                n2 % 700,
                &PrimerConstraints::standard(),
                3
            )
        );
        chk!(
            "guides",
            find_guides(
                &PamSpec::sp_cas9(),
                &a,
                n1 % 100,
                n2 % 200,
                rng.below(2) == 0
            )
        );
        chk!("motif", find_motif(&a, &b, true, rng.below(2) == 0));
        chk!(
            "translate_frame",
            translate_frame(&a, small % 4, n1 % 100, n2 % 100)
        );
        chk!(
            "protein",
            (
                molecular_weight(&a),
                isoelectric_point(&a),
                charge_at_ph(&a, 7.0),
                kyte_doolittle(&a, small)
            )
        );
        chk!(
            "protein_digest",
            digest_protein(
                &a.to_ascii_uppercase(),
                Protease::Trypsin,
                small % 4,
                small % 10
            )
        );
        chk!(
            "pcr",
            PcrSimulator::new(n1 % 30, n2 % 4, n3 % 5000).simulate(
                &a.repeat(3),
                rng.below(2) == 0,
                &[
                    Primer {
                        name: "f".to_string(),
                        sequence: b.clone()
                    },
                    Primer {
                        name: "r".to_string(),
                        sequence: a.clone()
                    },
                ],
            )
        );
        // Dokumente: Verdau mit zufälligen Enzymen
        if !a.is_empty() && a.chars().all(|c| "ACGT".contains(c)) {
            if let Ok(seq) = Sequence::new("s", "s", &a, SequenceType::Dna) {
                let topology = if rng.below(2) == 0 {
                    Topology::Circular
                } else {
                    Topology::Linear
                };
                let doc = SequenceDocument::of("s", seq).with_topology(topology);
                let names: Vec<&str> = (0..1 + rng.below(3))
                    .map(|_| enzyme_names[rng.below(enzyme_names.len())].as_str())
                    .collect();
                chk!("digest", digest(&doc, &db, &names));
                chk!("insert", doc.insert(n1 % 100, &b));
                chk!("delete", doc.delete(n1 % 100, n2 % 100));
                chk!("sub_document", doc.sub_document(n1 % 100, n2 % 100));
            }
        }
    }
    failures.sort();
    failures.dedup();
    assert!(failures.is_empty(), "Abstürze in: {:?}", failures);
}
