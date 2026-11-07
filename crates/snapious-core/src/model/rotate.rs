//! Move the origin of a circular document.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location};
use crate::model::sequence::{Sequence, SequenceType};

/// Rotates a circular document so that position `new_origin` becomes the new start.
/// * Error `Error::InvalidState` if the document is not circular; `Error::OutOfRange` if `new_origin >= length`
///   (also for an empty sequence).
/// * New bases: `bases[new_origin..] + bases[..new_origin]`; name, topology (circular), metadata and creation time stay,
///   the ID is new (`SequenceDocument::derive` or `crate::util::new_id`).
/// * Features: each position `p` becomes `(p − new_origin) mod L`. For each interval `[a, b)` of a feature:
///   if it lies entirely behind the origin (`new_origin <= a`), it becomes `[a − o, b − o)`; entirely before it (`b <= new_origin`):
///   `[a + L − o, b + L − o)`; if it spans the new origin (`a < o < b`), it becomes the two intervals
///   `[a + L − o, L)` and `[0, b − o)` (in this order). Multiple intervals (join) are mapped individually and appended
///   in order. Type, name, strand, partial markers and qualifiers stay; the result goes through `Location::new` and `Feature::new`.
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
                // Entirely behind the origin
                new_intervals.push(Interval::new(a - new_origin, b - new_origin)?);
            } else if b <= new_origin {
                // Entirely before it
                let start = a + len - new_origin;
                let end = b + len - new_origin;
                new_intervals.push(Interval::new(start, end)?);
            } else {
                // Spans the new origin (a < new_origin < b)
                let start1 = a + len - new_origin;
                let end1 = len;
                let start2 = 0usize;
                let end2 = b - new_origin;

                // Since a < o < b holds, start1 < end1 and start2 < end2 are guaranteed.
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
        // The split created by the rotation is retained as two adjoining intervals.
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
