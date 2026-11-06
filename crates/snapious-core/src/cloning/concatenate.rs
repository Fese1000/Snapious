//! Sequenzdokumente hintereinander zu einer neuen Sequenz zusammenfügen (Features wandern mit).

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::Feature;
use crate::model::sequence::Sequence;
use crate::util::{new_id, now_secs};
use std::collections::BTreeMap;

/// Fügt die Dokumente in der gegebenen Reihenfolge zusammen. Die Basen werden direkt aneinandergehängt. Die Features jedes
/// Dokuments übernimmt das Ergebnis mit dem Versatz der bisher angehängten Basen (`Location::shifted`, ein Feature behält Typ,
/// Name und Qualifier; `with_location`). Das Ergebnis hat eine neue ID, `name` als Namen (leer: `"Zusammengefügt"`), den
/// Sequenztyp der Eingaben (`Sequence::new(&new_id(), name, &bases, typ)`), `Topology::Circular`, wenn `circular`, sonst
/// `Linear`, keine Metadaten (`BTreeMap::new()`) und `now_secs()` als Erstellzeit (`SequenceDocument::new`).
/// Fehler `Error::InvalidInput`: keine Dokumente, oder die Sequenztypen (`doc.sequence().seq_type()`) sind nicht alle gleich.
pub fn concatenate(
    docs: &[SequenceDocument],
    name: &str,
    circular: bool,
) -> Result<SequenceDocument> {
    let first = docs
        .first()
        .ok_or_else(|| Error::InvalidInput("Keine Sequenzen zum Zusammenfügen".to_string()))?;
    let seq_type = first.sequence().seq_type();
    if docs.iter().any(|d| d.sequence().seq_type() != seq_type) {
        return Err(Error::InvalidInput(
            "Die Sequenzen haben verschiedene Typen (DNA, RNA, Protein)".to_string(),
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
        "Zusammengefügt"
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
        assert_eq!(r.name(), "Zusammengefügt");
        assert!(concatenate(&[], "x", false).is_err());
        let p = SequenceDocument::of(
            "p",
            Sequence::new("y", "p", "MKV", SequenceType::Protein).unwrap(),
        );
        assert!(concatenate(&[a, p], "x", false).is_err());
    }
}
