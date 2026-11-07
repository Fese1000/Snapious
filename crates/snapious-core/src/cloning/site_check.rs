//! Check inserts for additional cut sites of the chosen enzymes.

use crate::cloning::enzymes::EnzymeDb;
use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};

/// Reports enzymes that cut the insert more often than intended.
/// * For each enzyme in `enzymes` (names, order as passed; duplicate names only once) the number of recognition sites in the insert is counted:
///   `RestrictionEnzyme::find_matches(bases)` on the bases in uppercase (both strands are already included; for `Topology::Circular` additionally sites across the
///   origin, by extending the bases at the end by `recognition length − 1` characters from the start and counting duplicate start positions modulo length only once).
/// * Intended is at most 2 sites for exactly one enzyme (both ends of the insert), and at most 1 site per enzyme for several different enzymes.
///   If there are more, the message `Insert contains internal cut site of {name}` is produced.
///
/// Unknown enzyme name (`db.get`) → `Error::InvalidInput` with the name. Empty enzyme list → empty message list.
pub fn internal_site_warnings(
    insert: &SequenceDocument,
    enzymes: &[&str],
    db: &EnzymeDb,
) -> Result<Vec<String>> {
    let mut names: Vec<&str> = Vec::new();
    for n in enzymes {
        if !names.contains(n) {
            names.push(n);
        }
    }
    let bases = insert.bases().to_ascii_uppercase();
    let allowed = if names.len() == 1 { 2 } else { 1 };
    let circular = insert.topology() == Topology::Circular;
    let mut warnings = Vec::new();
    for name in names {
        let enzyme = db
            .get(name)
            .ok_or_else(|| Error::InvalidInput(format!("Unknown enzyme '{}'", name)))?;
        let len = bases.len();
        let count = if circular && len > 0 {
            let ext_len = enzyme.recognition().len().saturating_sub(1);
            let extended: String = bases.chars().chain(bases.chars().take(ext_len)).collect();
            let mut starts: Vec<usize> = enzyme
                .find_matches(extended.as_bytes())
                .iter()
                .map(|m| m.start % len)
                .collect();
            starts.sort_unstable();
            starts.dedup();
            starts.len()
        } else {
            enzyme.find_matches(bases.as_bytes()).len()
        };
        if count > allowed {
            warnings.push(format!("Insert contains internal cut site of {}", name));
        }
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(bases: &str, topology: Topology) -> SequenceDocument {
        let s = Sequence::new("i", "i", bases, SequenceType::Dna).unwrap();
        SequenceDocument::of("i", s).with_topology(topology)
    }

    #[test]
    fn ein_enzym() {
        let db = EnzymeDb::standard();
        // EcoRI = GAATTC; two sites are allowed, three are not
        let two = doc("GAATTCAAAAGAATTC", Topology::Linear);
        assert!(internal_site_warnings(&two, &["EcoRI"], &db)
            .unwrap()
            .is_empty());
        let three = doc("GAATTCAAAAGAATTCAAAAGAATTC", Topology::Linear);
        assert_eq!(
            internal_site_warnings(&three, &["EcoRI"], &db).unwrap(),
            vec!["Insert contains internal cut site of EcoRI".to_string()]
        );
    }

    #[test]
    fn zwei_enzyme() {
        let db = EnzymeDb::standard();
        let ok = doc("GAATTCAAAAGGATCC", Topology::Linear);
        assert!(internal_site_warnings(&ok, &["EcoRI", "BamHI"], &db)
            .unwrap()
            .is_empty());
        let inner = doc("GAATTCAAGAATTCAAAGGATCC", Topology::Linear);
        assert_eq!(
            internal_site_warnings(&inner, &["EcoRI", "BamHI"], &db).unwrap(),
            vec!["Insert contains internal cut site of EcoRI".to_string()]
        );
        // duplicate name counts as one enzyme
        let two = doc("GAATTCAAAAGAATTC", Topology::Linear);
        assert!(internal_site_warnings(&two, &["EcoRI", "EcoRI"], &db)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn zirkulaer_und_fehler() {
        let db = EnzymeDb::standard();
        // one site across the origin plus two normal ones = three
        let circ = doc("TTCAAAAGAATTCAAAAGAATTCAAAAGAA", Topology::Circular);
        assert_eq!(
            internal_site_warnings(&circ, &["EcoRI"], &db)
                .unwrap()
                .len(),
            1
        );
        let lin = doc("TTCAAAAGAATTCAAAAGAATTCAAAAGAA", Topology::Linear);
        assert!(internal_site_warnings(&lin, &["EcoRI"], &db)
            .unwrap()
            .is_empty());
        assert!(internal_site_warnings(&lin, &[], &db).unwrap().is_empty());
        let e = internal_site_warnings(&lin, &["GibtsNicht"], &db).unwrap_err();
        assert!(matches!(e, Error::InvalidInput(_)));
        assert!(e.to_string().contains("GibtsNicht"));
    }
}
