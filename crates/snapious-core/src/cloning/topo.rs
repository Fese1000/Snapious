//! TA and TOPO cloning: insert directly into a linear vector.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::sequence::{Sequence, SequenceType};
use std::collections::BTreeMap;

/// TA cloning: the PCR product (with 3' A overhangs from the PCR) is inserted into the linear T vector; both orientations are possible.
///
/// - Error `Error::InvalidInput` if `t_vector` or `pcr_product` is not linear.
/// - Returns: exactly two circular documents, first orientation `+`, then `−`.
/// - Top strand `+`: `t_vector.bases() + pcr_product.bases()`; `−`: `t_vector.bases() + reverse complement of the PCR product`
///   (use `pcr_product.reverse_complement()` for this so that the features of the insert are mirrored).
/// - Names: `"{vector} + {product} (orientation +)"` or `"… (orientation −)"`.
/// - Features: those of the vector, then those of the (possibly mirrored) insert, shifted right by the vector length
///   (`Location::shifted`, `Feature::with_location`).
/// - Metadata `source = "clone.ta"`. New ID (`new_id()`), sequence type DNA (id and name of the sequence equal to the document name), creation time `now_secs()`.
pub fn ta_cloning(
    t_vector: &SequenceDocument,
    pcr_product: &SequenceDocument,
) -> Result<Vec<SequenceDocument>> {
    if t_vector.topology() != Topology::Linear || pcr_product.topology() != Topology::Linear {
        return Err(Error::InvalidInput(
            "Both input documents must be linear.".to_string(),
        ));
    }

    let shift = t_vector.len();
    let t_bases = t_vector.bases();
    let p_bases = pcr_product.bases();

    // Orientation +
    let bases_plus = format!("{}{}", t_bases, p_bases);
    let seq_plus = Sequence::new(
        t_vector.name(),
        pcr_product.name(),
        &bases_plus,
        SequenceType::Dna,
    )?;
    let mut doc_plus =
        SequenceDocument::of(t_vector.name(), seq_plus).with_topology(Topology::Circular);

    let mut feats_plus = Vec::new();
    for f in t_vector.features() {
        feats_plus.push(f.clone());
    }
    for f in pcr_product.features() {
        feats_plus.push(f.with_location(f.location().shifted(shift as isize)?));
    }

    doc_plus = doc_plus.with_name(&format!(
        "{} + {} (orientation +)",
        t_vector.name(),
        pcr_product.name()
    ));
    doc_plus = doc_plus
        .with_features(feats_plus)
        .with_metadata(BTreeMap::from([(
            "source".to_string(),
            "clone.ta".to_string(),
        )]));

    // Orientation −
    let rc_product = pcr_product.reverse_complement()?;
    let bases_minus = format!("{}{}", t_bases, rc_product.bases());
    let seq_minus = Sequence::new(
        t_vector.name(),
        pcr_product.name(),
        &bases_minus,
        SequenceType::Dna,
    )?;
    let mut doc_minus =
        SequenceDocument::of(t_vector.name(), seq_minus).with_topology(Topology::Circular);

    let mut feats_minus = Vec::new();
    for f in t_vector.features() {
        feats_minus.push(f.clone());
    }
    for f in rc_product.features() {
        feats_minus.push(f.with_location(f.location().shifted(shift as isize)?));
    }

    doc_minus = doc_minus.with_name(&format!(
        "{} + {} (orientation −)",
        t_vector.name(),
        pcr_product.name()
    ));
    doc_minus = doc_minus
        .with_features(feats_minus)
        .with_metadata(BTreeMap::from([(
            "source".to_string(),
            "clone.ta".to_string(),
        )]));

    Ok(vec![doc_plus, doc_minus])
}

/// Directional TOPO cloning: the insert must begin with `CACC` (case-insensitive), otherwise `Error::InvalidInput`.
/// There is only one orientation: top strand `topo_vector.bases() + insert.bases()` (nothing is removed).
/// Both documents must be linear (otherwise `Error::InvalidInput`). Name `"{vector} + {insert}"`, metadata `source = "clone.topo"`;
/// features as in `ta_cloning`; circular result.
pub fn directional_topo(
    topo_vector: &SequenceDocument,
    insert: &SequenceDocument,
) -> Result<SequenceDocument> {
    if topo_vector.topology() != Topology::Linear || insert.topology() != Topology::Linear {
        return Err(Error::InvalidInput(
            "Both input documents must be linear.".to_string(),
        ));
    }

    let insert_bases = insert.bases();
    if !insert_bases.to_uppercase().starts_with("CACC") {
        return Err(Error::InvalidInput(
            "Insert must start with CACC.".to_string(),
        ));
    }

    let shift = topo_vector.len();
    let bases = format!("{}{}", topo_vector.bases(), insert_bases);

    let seq = Sequence::new(topo_vector.name(), insert.name(), &bases, SequenceType::Dna)?;
    let mut doc = SequenceDocument::of(topo_vector.name(), seq).with_topology(Topology::Circular);

    let mut feats = Vec::new();
    for f in topo_vector.features() {
        feats.push(f.clone());
    }
    for f in insert.features() {
        feats.push(f.with_location(f.location().shifted(shift as isize)?));
    }

    doc = doc.with_name(&format!("{} + {}", topo_vector.name(), insert.name()));
    doc = doc.with_features(feats).with_metadata(BTreeMap::from([(
        "source".to_string(),
        "clone.topo".to_string(),
    )]));

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Feature, Interval, Location, Strand};
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(name: &str, bases: &str, topology: Topology) -> SequenceDocument {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        SequenceDocument::of(name, seq).with_topology(topology)
    }

    fn feat(start: usize, end: usize) -> Feature {
        let loc = Location::new(
            vec![Interval::new(start, end).unwrap()],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        Feature::new("gene", "g", loc, Vec::new())
    }

    #[test]
    fn ta_beide_orientierungen() {
        let v = doc("tvec", "GGGGTTTT", Topology::Linear).with_features(vec![feat(0, 3)]);
        let p = doc("prod", "AACCGGTA", Topology::Linear).with_features(vec![feat(0, 2)]);
        let r = ta_cloning(&v, &p).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].bases(), "GGGGTTTTAACCGGTA");
        assert_eq!(r[1].bases(), "GGGGTTTTTACCGGTT");
        assert_eq!(r[0].name(), "tvec + prod (orientation +)");
        assert_eq!(r[1].name(), "tvec + prod (orientation −)");
        assert!(r.iter().all(|d| d.topology() == Topology::Circular));
        assert_eq!(
            r[0].metadata().get("source").map(String::as_str),
            Some("clone.ta")
        );
        // Features: vector 0..3; insert 0..2 forward at 8..10; mirrored at 8 + (8 − 2)..8 + 8 = 14..16
        let mut a: Vec<(usize, usize)> = r[0]
            .features()
            .iter()
            .map(|f| (f.location().start(), f.location().end()))
            .collect();
        a.sort();
        assert_eq!(a, vec![(0, 3), (8, 10)]);
        let mut b: Vec<(usize, usize)> = r[1]
            .features()
            .iter()
            .map(|f| (f.location().start(), f.location().end()))
            .collect();
        b.sort();
        assert_eq!(b, vec![(0, 3), (14, 16)]);
        assert_eq!(
            r[1].features()
                .iter()
                .find(|f| f.location().start() == 14)
                .map(|f| f.location().strand()),
            Some(Strand::Reverse)
        );
    }

    #[test]
    fn ta_fehler() {
        let lin = doc("v", "GGGG", Topology::Linear);
        let circ = doc("c", "GGGG", Topology::Circular);
        assert!(matches!(
            ta_cloning(&circ, &lin),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            ta_cloning(&lin, &circ),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn gerichtetes_topo() {
        let v = doc("topo", "TTTTGGGG", Topology::Linear);
        let i = doc("ins", "CACCATGAAA", Topology::Linear).with_features(vec![feat(4, 7)]);
        let r = directional_topo(&v, &i).unwrap();
        assert_eq!(r.bases(), "TTTTGGGGCACCATGAAA");
        assert_eq!(r.name(), "topo + ins");
        assert_eq!(r.topology(), Topology::Circular);
        assert_eq!(
            r.metadata().get("source").map(String::as_str),
            Some("clone.topo")
        );
        assert_eq!(r.features().len(), 1);
        assert_eq!(r.features()[0].location().start(), 12);
        // Lowercase letters
        let k = doc("ins2", "caccatg", Topology::Linear);
        assert!(directional_topo(&v, &k).is_ok());
    }

    #[test]
    fn topo_fehler() {
        let v = doc("topo", "TTTTGGGG", Topology::Linear);
        let ohne = doc("ins", "ATGAAACACC", Topology::Linear);
        assert!(matches!(
            directional_topo(&v, &ohne),
            Err(Error::InvalidInput(_))
        ));
        let circ = doc("c", "CACCATG", Topology::Circular);
        assert!(matches!(
            directional_topo(&v, &circ),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            directional_topo(&circ, &doc("i", "CACCATG", Topology::Linear)),
            Err(Error::InvalidInput(_))
        ));
    }
}
