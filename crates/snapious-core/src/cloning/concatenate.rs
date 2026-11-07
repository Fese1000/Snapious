//! Join sequence documents one after another into a new sequence (features move along).

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::Feature;
use crate::model::sequence::Sequence;
use crate::util::{new_id, now_secs};
use std::collections::BTreeMap;

/// Joins the documents in the given order. The bases are directly concatenated. The result takes over the features of each
/// document with the offset of the bases appended so far (`Location::shifted`; a feature keeps type,
/// name and qualifiers; `with_location`). The result has a new ID, `name` as its name (empty: `"Joined"`), the
/// sequence type of the inputs (`Sequence::new(&new_id(), name, &bases, type)`), `Topology::Circular` if `circular`, otherwise
/// `Linear`, no metadata (`BTreeMap::new()`) and `now_secs()` as creation time (`SequenceDocument::new`).
/// Error `Error::InvalidInput`: no documents, or the sequence types (`doc.sequence().seq_type()`) are not all equal.
pub fn concatenate(
    docs: &[SequenceDocument],
    name: &str,
    circular: bool,
) -> Result<SequenceDocument> {
    let first = docs
        .first()
        .ok_or_else(|| Error::InvalidInput("No sequences to join".to_string()))?;
    let seq_type = first.sequence().seq_type();
    if docs.iter().any(|d| d.sequence().seq_type() != seq_type) {
        return Err(Error::InvalidInput(
            "The sequences have different types (DNA, RNA, protein)".to_string(),
        ));
    }
    let mut bases = String::new();
    let mut features: Vec<Feature> = Vec::new();
    for d in docs {
        let offset = bases.len() as isize;
        for f in d.features() {
            features.push(f.with_location(f.location().shifted(offset)?));
        }
        bases.push_str(d.bases());
    }
    let name = if name.trim().is_empty() {
        "Joined"
    } else {
        name.trim()
    };
    let sequence = Sequence::new(&new_id(), name, &bases, seq_type)?;
    let topology = if circular {
        Topology::Circular
    } else {
        Topology::Linear
    };
    Ok(SequenceDocument::new(
        &new_id(),
        name,
        sequence,
        topology,
        features,
        BTreeMap::new(),
        now_secs(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::Strand;
    use crate::model::sequence::SequenceType;

    fn doc(bases: &str, features: Vec<Feature>) -> SequenceDocument {
        let seq = Sequence::new("x", "d", bases, SequenceType::Dna).unwrap();
        SequenceDocument::of("d", seq).with_features(features)
    }

    fn feat(name: &str, start: usize, end: usize) -> Feature {
        Feature::new(
            "gene",
            name,
            crate::model::feature::Location::of(start, end, Strand::Forward).unwrap(),
            vec![],
        )
    }

    #[test]
    fn zwei_dokumente() {
        let a = doc("ACGT", vec![feat("a", 1, 3)]);
        let b = doc("TTGCA", vec![feat("b", 0, 2), feat("c", 3, 5)]);
        let r = concatenate(&[a, b], "neu", false).unwrap();
        assert_eq!(r.bases(), "ACGTTTGCA");
        assert_eq!(r.name(), "neu");
        assert_eq!(r.topology(), Topology::Linear);
        let f = r.features();
        assert_eq!(f.len(), 3);
        assert_eq!((f[0].location().start(), f[0].location().end()), (1, 3));
        assert_eq!((f[1].location().start(), f[1].location().end()), (4, 6));
        assert_eq!((f[2].location().start(), f[2].location().end()), (7, 9));
        assert_eq!(f[2].name(), "c");
    }

    #[test]
    fn zirkulaer_und_fehler() {
        let a = doc("AC", vec![]);
        let r = concatenate(std::slice::from_ref(&a), "", true).unwrap();
        assert_eq!(r.topology(), Topology::Circular);
        assert_eq!(r.name(), "Joined");
        assert!(concatenate(&[], "x", false).is_err());
        let p = SequenceDocument::of(
            "p",
            Sequence::new("y", "p", "MKV", SequenceType::Protein).unwrap(),
        );
        assert!(concatenate(&[a, p], "x", false).is_err());
    }
}
