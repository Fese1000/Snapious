//! Restriction cloning: join fragments and form possible constructs from vector and insert.

use std::collections::{BTreeMap, HashSet};

use crate::cloning::enzymes::EnzymeDb;
use crate::cloning::fragment::{digest, Fragment};
use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::sequence::{Sequence, SequenceType};
use crate::util::{new_id, now_secs};

/// A possible cloning product.
#[derive(Clone, Debug, PartialEq)]
pub struct CloningCandidate {
    /// The vector backbone (largest fragment of the vector digest).
    pub vector: Fragment,
    /// The insert fragment in the orientation in which it is inserted.
    pub insert: Fragment,
    /// `true` if the insert was inserted reverse-complemented.
    pub insert_reversed: bool,
    /// `true` if the two vector ends differ (then only one orientation is possible).
    pub directional: bool,
    /// Circular construct.
    pub construct: SequenceDocument,
    pub warnings: Vec<String>,
}

/// Joins fragments in the given order into a top strand: sum of the `top` texts.
/// - Empty list: `Error::InvalidInput`.
/// - For every adjacent pair `fragments[i].can_join(&fragments[i+1])` must hold, otherwise `Error::InvalidInput`
///   (message names both numbers 1-based: `Fragmente {i} und {i+1} passen nicht zusammen`).
/// - If `circular`, additionally the last fragment must fit the first (for a single fragment, itself),
///   otherwise `Error::InvalidInput`.
pub fn join_fragments(fragments: &[Fragment], circular: bool) -> Result<String> {
    if fragments.is_empty() {
        return Err(Error::InvalidInput("Empty fragment list".to_string()));
    }

    let mut result = String::new();
    for (i, frag) in fragments.iter().enumerate() {
        result.push_str(&frag.top);

        if i < fragments.len() - 1 {
            let next = fragments
                .get(i + 1)
                .ok_or_else(|| Error::InvalidInput("Logic error".to_string()))?;
            if !frag.can_join(next) {
                return Err(Error::InvalidInput(format!(
                    "Fragments {} and {} do not fit together",
                    i + 1,
                    i + 2
                )));
            }
        }
    }

    if circular {
        let last = fragments
            .last()
            .ok_or_else(|| Error::InvalidInput("Logic error: empty list".to_string()))?;
        let first = fragments
            .first()
            .ok_or_else(|| Error::InvalidInput("Logic error".to_string()))?;
        if !last.can_join(first) {
            return Err(Error::InvalidInput(format!(
                "Fragments {} and {} do not fit together",
                fragments.len(),
                1
            )));
        }
    }

    Ok(result)
}

/// All circular products of restriction cloning.
///
/// 1. `digest(vector, db, vector_enzymes)` and `digest(insert, db, insert_enzymes)` (errors are passed on).
/// 2. If the vector was not cut (exactly one fragment with `source_start == 0` and `source_end == vector length`),
///    there are no candidates (empty list). Backbone `V` is the longest vector fragment (on a tie the first).
/// 3. For each insert fragment `I` and both orientations (`I`, then `I.reverse_complement()`) a candidate arises if
///    `V.can_join(&I')` and `I'.can_join(&V)` hold.
/// 4. `directional = V.left != V.right`.
/// 5. Construct: top strand `V.top + I'.top`; name `"{vector name} + {insert name}"`; new ID (`new_id()`), `Topology::Circular`;
///    sequence of type DNA (id and name of the sequence equal to the document name); features: those of `V` and those of `I'`,
///    the latter shifted right by `V.length()` (`Location::shifted`, `Feature::with_location`);
///    metadata `source = "clone.restriction"`; creation time `now_secs()`.
/// 6. Warnings (in this order):
///    - `Insert orientation not directed (both orientations possible)` if both orientations
///      gave a candidate for the same insert fragment (appears on both candidates);
///    - `Vector re-ligation possible` if `V.right.pairs_with(&V.left)`.
/// 7. Sorting: directional first, then fewer warnings, then shorter construct, then non-reversed before reversed,
///    then smaller `source_start` of the insert.
pub fn restriction_candidates(
    vector: &SequenceDocument,
    vector_enzymes: &[&str],
    insert: &SequenceDocument,
    insert_enzymes: &[&str],
    db: &EnzymeDb,
) -> Result<Vec<CloningCandidate>> {
    let vector_frags = digest(vector, db, vector_enzymes)?;
    let insert_frags = digest(insert, db, insert_enzymes)?;

    // Step 2: vector check
    let vec_len = vector.len();
    if vector_frags.len() == 1 {
        let first_frag = vector_frags
            .first()
            .ok_or_else(|| Error::InvalidInput("Logic error".to_string()))?;
        if first_frag.source_start == 0 && first_frag.source_end == vec_len {
            return Ok(Vec::new());
        }
    }

    // Find the longest vector fragment (on a tie the first)
    let mut v_idx = 0;
    let mut v_len = 0;
    for (i, frag) in vector_frags.iter().enumerate() {
        let l = frag.length();
        if l > v_len {
            v_len = l;
            v_idx = i;
        }
    }
    let v_frag = vector_frags
        .get(v_idx)
        .ok_or_else(|| Error::InvalidInput("Logic error: no vector fragment".to_string()))?;

    // Step 3: generate candidates
    let mut raw_candidates: Vec<(usize, Fragment, bool)> = Vec::new();

    for (i_frag, base_i) in insert_frags.iter().enumerate() {
        // Orientation 1: normal
        let i_prime = base_i.clone();
        if v_frag.can_join(&i_prime) && i_prime.can_join(v_frag) {
            raw_candidates.push((i_frag, i_prime.clone(), false));
        }

        // Orientation 2: reverse-complemented
        let i_rev = base_i.reverse_complement()?;
        if v_frag.can_join(&i_rev) && i_rev.can_join(v_frag) {
            raw_candidates.push((i_frag, i_rev.clone(), true));
        }
    }

    if raw_candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Step 6: determine warnings
    let mut insert_indices_with_both = HashSet::new();
    for (idx, _, reversed) in &raw_candidates {
        let has_other = raw_candidates
            .iter()
            .any(|(i, _, r)| *i == *idx && *r != *reversed);
        if has_other {
            insert_indices_with_both.insert(*idx);
        }
    }

    let mut candidates = Vec::new();

    for (idx, frag, reversed) in &raw_candidates {
        let mut warnings_list = Vec::new();
        if insert_indices_with_both.contains(idx) {
            warnings_list.push(
                "Insert orientation not directed (both orientations possible)".to_string(),
            );
        }
        if v_frag.right.pairs_with(&v_frag.left) {
            warnings_list.push("Vector re-ligation possible".to_string());
        }

        let construct_bases = join_fragments(&[v_frag.clone(), frag.clone()], true)?;
        let construct_name = format!("{} + {}", vector.name(), insert.name());

        let construct_seq = Sequence::new(
            &construct_name,
            &construct_name,
            &construct_bases,
            SequenceType::Dna,
        )?;

        let mut construct_features = v_frag.features.clone();
        for f in &frag.features {
            let shifted_loc = f.location().shifted(v_frag.length() as isize)?;
            construct_features.push(f.with_location(shifted_loc));
        }

        let mut metadata = BTreeMap::new();
        metadata.insert("source".to_string(), "clone.restriction".to_string());

        let construct_doc = SequenceDocument::new(
            &new_id(),
            &construct_name,
            construct_seq,
            Topology::Circular,
            construct_features,
            metadata,
            now_secs(),
        );

        candidates.push(CloningCandidate {
            vector: v_frag.clone(),
            insert: frag.clone(),
            insert_reversed: *reversed,
            directional: v_frag.left != v_frag.right,
            construct: construct_doc,
            warnings: warnings_list,
        });
    }

    // Step 7: sorting
    candidates.sort_by(|a, b| {
        a.directional
            .cmp(&b.directional)
            .reverse() // directional first (true > false)
            .then_with(|| a.warnings.len().cmp(&b.warnings.len())) // fewer warnings first
            .then_with(|| a.construct.len().cmp(&b.construct.len())) // shorter construct first
            .then_with(|| a.insert_reversed.cmp(&b.insert_reversed)) // non-reversed first (false < true)
            .then_with(|| a.insert.source_start.cmp(&b.insert.source_start)) // smaller source_start first
    });

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Feature, Interval, Location, Strand};

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

    fn circular_contains(c: &SequenceDocument, site: &str) -> bool {
        let b = c.bases();
        format!("{}{}", b, &b[..site.len() - 1]).contains(site)
    }

    fn mcs_vector() -> SequenceDocument {
        let b = format!(
            "{}GGATCC{}GAATTC{}",
            "A".repeat(10),
            "T".repeat(8),
            "C".repeat(10)
        );
        doc("vec", &b, Topology::Circular).with_features(vec![feat(31, 35)])
    }

    fn directional_insert() -> SequenceDocument {
        doc("ins", "CCGGATCCACGTACGTGAATTCGG", Topology::Linear).with_features(vec![feat(5, 9)])
    }

    #[test]
    fn fragmente_verbinden() {
        let d = doc("s", "AAGGAATTCGATTAGCTAGC", Topology::Linear);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(join_fragments(&f, false).unwrap(), "AAGGAATTCGATTAGCTAGC");
        // f0 onto f0: right end AATT does not fit the blunt left end
        let doppelt = vec![f[0].clone(), f[0].clone()];
        let msg = match join_fragments(&doppelt, false) {
            Err(Error::InvalidInput(m)) => m,
            other => panic!("unerwartet: {:?}", other),
        };
        assert!(msg.contains('1') && msg.contains('2'));
        assert!(matches!(
            join_fragments(&[], false),
            Err(Error::InvalidInput(_))
        ));
        // circular: a single fragment with ends AATT/blunt does not fit itself
        assert!(matches!(
            join_fragments(&f[..1], true),
            Err(Error::InvalidInput(_))
        ));
        // both fragments circular: end (blunt) fits start (blunt)
        assert_eq!(join_fragments(&f, true).unwrap(), "AAGGAATTCGATTAGCTAGC");
    }

    #[test]
    fn gerichtete_klonierung() {
        let db = EnzymeDb::standard();
        let c = restriction_candidates(
            &mcs_vector(),
            &["BamHI", "EcoRI"],
            &directional_insert(),
            &["BamHI", "EcoRI"],
            &db,
        )
        .unwrap();
        assert_eq!(c.len(), 1);
        let k = &c[0];
        assert!(k.directional);
        assert!(!k.insert_reversed);
        assert!(k.warnings.is_empty());
        assert_eq!(k.vector.length(), 26);
        assert_eq!(k.insert.length(), 14);
        assert_eq!(k.construct.len(), 40);
        assert_eq!(k.construct.topology(), Topology::Circular);
        assert_eq!(k.construct.name(), "vec + ins");
        assert_eq!(
            k.construct.metadata().get("source").map(String::as_str),
            Some("clone.restriction")
        );
        assert!(circular_contains(&k.construct, "GAATTC"));
        assert!(circular_contains(&k.construct, "GGATCC"));
        // Features: vector 31..35 lies in the backbone (from 25) → 6..10; insert 5..9 lies in the fragment (from 3) → 2..6, plus 26
        let mut starts: Vec<usize> = k
            .construct
            .features()
            .iter()
            .map(|f| f.location().start())
            .collect();
        starts.sort();
        assert_eq!(starts, vec![6, 28]);
        assert_eq!(
            k.construct.bases(),
            format!("{}{}", k.vector.top, k.insert.top)
        );
    }

    #[test]
    fn ein_enzym_zwei_orientierungen() {
        let db = EnzymeDb::standard();
        let vec = doc(
            "vec",
            &format!("{}GAATTC{}", "A".repeat(20), "C".repeat(14)),
            Topology::Circular,
        );
        let ins = doc("ins", "CCGAATTCACGTACGTGAATTCGG", Topology::Linear);
        let c = restriction_candidates(&vec, &["EcoRI"], &ins, &["EcoRI"], &db).unwrap();
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|k| !k.directional));
        assert!(c.iter().all(|k| k.construct.len() == 54));
        assert!(c.iter().all(|k| k.warnings
            == vec![
                "Insert orientation not directed (both orientations possible)".to_string(),
                "Vector re-ligation possible".to_string()
            ]));
        assert!(!c[0].insert_reversed && c[1].insert_reversed);
    }

    #[test]
    fn hybridstellen_bamhi_bglii() {
        let db = EnzymeDb::standard();
        let vec = doc(
            "vec",
            &format!("{}GGATCC{}", "A".repeat(10), "C".repeat(14)),
            Topology::Circular,
        );
        let ins = doc("ins", "TTAGATCTACGTACGTAGATCTTT", Topology::Linear);
        let c = restriction_candidates(&vec, &["BamHI"], &ins, &["BglII"], &db).unwrap();
        assert_eq!(c.len(), 2);
        let vorwaerts = c.iter().find(|k| !k.insert_reversed).unwrap();
        assert!(circular_contains(&vorwaerts.construct, "GGATCT"));
        assert!(circular_contains(&vorwaerts.construct, "AGATCC"));
        assert!(!circular_contains(&vorwaerts.construct, "GGATCC"));
    }

    #[test]
    fn unvertraegliche_enden_und_ungeschnittener_vektor() {
        let db = EnzymeDb::standard();
        let vec = doc(
            "vec",
            &format!("{}GAATTC{}", "A".repeat(20), "C".repeat(14)),
            Topology::Circular,
        );
        let ins = doc("ins", "CCGGATCCACGTACGTGGATCCGG", Topology::Linear);
        assert!(
            restriction_candidates(&vec, &["EcoRI"], &ins, &["BamHI"], &db)
                .unwrap()
                .is_empty()
        );
        // Vector without a cut site of the chosen enzyme
        assert!(
            restriction_candidates(&vec, &["BamHI"], &ins, &["BamHI"], &db)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            restriction_candidates(&vec, &["Nix"], &ins, &["BamHI"], &db),
            Err(Error::InvalidInput(_))
        ));
    }
}
