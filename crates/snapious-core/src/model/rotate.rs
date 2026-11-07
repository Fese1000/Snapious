//! Ursprung eines zirkulären Dokuments verschieben.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location};
use crate::model::sequence::{Sequence, SequenceType};

/// Dreht ein zirkuläres Dokument so, dass die Position `new_origin` zum neuen Anfang wird.
/// * Fehler `Error::InvalidState`, wenn das Dokument nicht zirkulär ist; `Error::OutOfRange`, wenn `new_origin >= Länge`
///   (auch bei leerer Sequenz).
/// * Neue Basen: `bases[new_origin..] + bases[..new_origin]`; Name, Topologie (zirkulär), Metadaten und Erstellzeit bleiben,
///   die ID ist neu (`SequenceDocument::derive` oder `crate::util::new_id`).
/// * Features: jede Position `p` wird zu `(p − new_origin) mod L`. Für jedes Intervall `[a, b)` eines Features gilt:
///   liegt es ganz hinter dem Ursprung (`new_origin <= a`), wird es `[a − o, b − o)`; ganz davor (`b <= new_origin`):
///   `[a + L − o, b + L − o)`; überspannt es den neuen Ursprung (`a < o < b`), wird es zu den zwei Intervallen
///   `[a + L − o, L)` und `[0, b − o)` (in dieser Reihenfolge). Mehrere Intervalle (Join) werden einzeln abgebildet und der Reihe nach
///   aneinandergehängt. Typ, Name, Strang, Teilmarkierungen und Qualifier bleiben; Ergebnis über `Location::new` und `Feature::new`.
pub fn rotate_document(doc: &SequenceDocument, new_origin: usize) -> Result<SequenceDocument> {
    if doc.topology() != Topology::Circular {
        return Err(Error::InvalidState(
            "Document is not circular".to_string(),
        ));
    }

    let len = doc.len();
    if new_origin >= len {
        return Err(Error::OutOfRange(
            "Origin lies outside the sequence".to_string(),
        ));
    }

    let bases = doc.bases();
    // String slicing is safe because new_origin < len.
    let rotated_bases = format!("{}{}", &bases[new_origin..], &bases[..new_origin]);

    let mut new_features = Vec::with_capacity(doc.features().len());
    for f in doc.features() {
        let loc = f.location();
        let mut new_intervals = Vec::new();

        for iv in loc.intervals() {
            let a = iv.start();
            let b = iv.end();

            if new_origin <= a {
                // Ganz hinter dem Ursprung
                new_intervals.push(Interval::new(a - new_origin, b - new_origin)?);
            } else if b <= new_origin {
                // Ganz davor
                let start = a + len - new_origin;
                let end = b + len - new_origin;
                new_intervals.push(Interval::new(start, end)?);
            } else {
                // Überspannt den neuen Ursprung (a < new_origin < b)
                let start1 = a + len - new_origin;
                let end1 = len;
                let start2 = 0usize;
                let end2 = b - new_origin;

                // Da a < o < b gilt, sind start1 < end1 und start2 < end2 garantiert.
                new_intervals.push(Interval::new(start1, end1)?);
                new_intervals.push(Interval::new(start2, end2)?);
            }
        }

        let new_loc = Location::new(
            new_intervals,
            loc.strand(),
            loc.partial_start(),
            loc.partial_end(),
        )?;
        new_features.push(Feature::new(
            f.feature_type(),
            f.name(),
            new_loc,
            f.qualifiers().to_vec(),
        ));
    }

    // Handle Sequence::new error properly
    let new_seq = Sequence::new(doc.name(), doc.name(), &rotated_bases, SequenceType::Dna)?;

    Ok(SequenceDocument::of(doc.name(), new_seq)
        .with_topology(Topology::Circular)
        .with_features(new_features)
        .with_metadata(doc.metadata().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Topology;
    use crate::model::feature::{Feature, Interval, Location, Strand};
    use crate::model::sequence::{Sequence, SequenceType};

    fn feat(intervals: &[(usize, usize)], strand: Strand) -> Feature {
        let iv = intervals
            .iter()
            .map(|(a, b)| Interval::new(*a, *b).unwrap())
            .collect();
        let loc = Location::new(iv, strand, false, false).unwrap();
        Feature::new(
            "gene",
            "g",
            loc,
            vec![("note".to_string(), vec!["x".to_string()])],
        )
    }

    fn circ(bases: &str, features: Vec<Feature>) -> SequenceDocument {
        let seq = Sequence::new("s", "s", bases, SequenceType::Dna).unwrap();
        SequenceDocument::of("s", seq)
            .with_topology(Topology::Circular)
            .with_features(features)
    }

    fn spans(f: &Feature) -> Vec<(usize, usize)> {
        f.location()
            .intervals()
            .iter()
            .map(|i| (i.start(), i.end()))
            .collect()
    }

    #[test]
    fn basen_und_feature_hinter_dem_ursprung() {
        let d = circ("ACGTAC", vec![feat(&[(2, 4)], Strand::Forward)]);
        let r = rotate_document(&d, 2).unwrap();
        assert_eq!(r.bases(), "GTACAC");
        assert_eq!(r.topology(), Topology::Circular);
        assert_eq!(r.name(), "s");
        assert_ne!(r.id(), d.id());
        assert_eq!(spans(&r.features()[0]), vec![(0, 2)]);
        assert_eq!(r.features()[0].qualifier("note"), Some("x"));
        assert_eq!(r.features()[0].name(), "g");
    }

    #[test]
    fn feature_ueber_den_neuen_ursprung() {
        let d = circ("ACGTAC", vec![feat(&[(1, 4)], Strand::Reverse)]);
        let r = rotate_document(&d, 2).unwrap();
        let f = &r.features()[0];
        assert_eq!(spans(f), vec![(5, 6), (0, 2)]);
        assert_eq!(f.location().length(), 3);
        assert_eq!(f.location().strand(), Strand::Reverse);
    }

    #[test]
    fn feature_vor_dem_ursprung_und_join() {
        let d = circ(
            "ACGTACGTAC",
            vec![
                feat(&[(0, 2)], Strand::Forward),
                feat(&[(1, 3), (6, 8)], Strand::Forward),
            ],
        );
        let r = rotate_document(&d, 4).unwrap();
        assert_eq!(spans(&r.features()[0]), vec![(6, 8)]);
        assert_eq!(spans(&r.features()[1]), vec![(7, 9), (2, 4)]);
        assert_eq!(r.bases(), "ACGTACACGT");
    }

    #[test]
    fn ursprung_null_und_rundlauf() {
        let d = circ("ACGTACGTAC", vec![feat(&[(1, 4)], Strand::Forward)]);
        let same = rotate_document(&d, 0).unwrap();
        assert_eq!(same.bases(), d.bases());
        assert_eq!(spans(&same.features()[0]), vec![(1, 4)]);
        let there = rotate_document(&d, 3).unwrap();
        let back = rotate_document(&there, 7).unwrap();
        assert_eq!(back.bases(), d.bases());
        // Die beim Drehen entstandene Teilung bleibt als zwei aneinanderstoßende Intervalle erhalten.
        let loc = back.features()[0].location();
        assert_eq!(loc.length(), 3);
        assert!((1..4).all(|p| loc.contains(p)));
        assert!(!loc.contains(0) && !loc.contains(4));
    }

    #[test]
    fn fehlerfaelle() {
        let lin = {
            let seq = Sequence::new("s", "s", "ACGT", SequenceType::Dna).unwrap();
            SequenceDocument::of("s", seq)
        };
        assert!(matches!(
            rotate_document(&lin, 1),
            Err(Error::InvalidState(_))
        ));
        let d = circ("ACGT", vec![]);
        assert!(matches!(rotate_document(&d, 4), Err(Error::OutOfRange(_))));
    }
}
