//! Enzymliste für die Kartenansicht: welche Enzyme schneiden wo, gefiltert nach Schnitthäufigkeit.

use crate::cloning::enzymes::{CutSite, EnzymeDb};
use crate::model::document::SequenceDocument;

/// Filter nach Anzahl der Schnittstellen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnzymeFilter {
    /// Alle Enzyme mit mindestens einer Schnittstelle.
    All,
    /// Genau eine Schnittstelle.
    UniqueCutters,
    /// Genau zwei Schnittstellen.
    DoubleCutters,
    /// Enzyme der Datenbank ohne Schnittstelle.
    NonCutters,
}

/// Ein Enzym mit seinen Schnittstellen.
#[derive(Clone, Debug, PartialEq)]
pub struct EnzymeMapEntry {
    pub enzyme: String,
    pub count: usize,
    /// Beginn der Erkennungssequenz je Schnittstelle, aufsteigend sortiert; bei `NonCutters` leer.
    pub positions: Vec<usize>,
}

/// Ergebnis der Enzymsuche für ein Dokument, bereits gefiltert.
#[derive(Clone, Debug, PartialEq)]
pub struct EnzymeMapModel {
    entries: Vec<EnzymeMapEntry>,
    sites: Vec<CutSite>,
}

impl EnzymeMapModel {
    /// Sucht mit `EnzymeDb::find_in_document` und gruppiert nach Enzymname.
    /// - `entries` ist nach Enzymname aufsteigend sortiert.
    /// - `sites` enthält nur die Schnittstellen der Enzyme, die der Filter durchlässt, in der Reihenfolge von
    ///   `find_in_document`; bei `NonCutters` ist sie leer.
    /// - `NonCutters`: ein Eintrag je Enzym der Datenbank ohne Schnittstelle, `count 0`, `positions` leer.
    pub fn new(db: &EnzymeDb, doc: &SequenceDocument, filter: EnzymeFilter) -> Self {
        let all_sites = db.find_in_document(doc);

        // Gruppiere nach Enzymname.
        let mut enzyme_map: std::collections::BTreeMap<String, Vec<CutSite>> =
            std::collections::BTreeMap::new();

        for site in &all_sites {
            enzyme_map
                .entry(site.enzyme.clone())
                .or_default()
                .push(site.clone());
        }

        // Baue entries basierend auf Filter.
        let mut entries: Vec<EnzymeMapEntry> = Vec::new();
        let mut sites: Vec<CutSite> = Vec::new();

        match filter {
            EnzymeFilter::All | EnzymeFilter::UniqueCutters | EnzymeFilter::DoubleCutters => {
                for (enzyme, cutsites) in &enzyme_map {
                    let count = cutsites.len();
                    let passes = match filter {
                        EnzymeFilter::All => count >= 1,
                        EnzymeFilter::UniqueCutters => count == 1,
                        EnzymeFilter::DoubleCutters => count == 2,
                        _ => false,
                    };

                    if passes {
                        entries.push(EnzymeMapEntry {
                            enzyme: enzyme.clone(),
                            count,
                            positions: cutsites.iter().map(|s| s.position).collect(),
                        });
                        sites.extend(cutsites.iter().cloned());
                    }
                }
            }
            EnzymeFilter::NonCutters => {
                // Finde alle Enzyme, die in diesem Dokument schneiden.
                let cutting_sites = db.find_in_document(doc);
                let cutting_names: std::collections::BTreeSet<String> =
                    cutting_sites.iter().map(|s| s.enzyme.clone()).collect();

                // Alle Enzyme aus der Datenbank, die nicht schneiden.
                for enzyme in db.all() {
                    if !cutting_names.contains(enzyme.name()) {
                        entries.push(EnzymeMapEntry {
                            enzyme: enzyme.name().to_string(),
                            count: 0,
                            positions: Vec::new(),
                        });
                    }
                }
            }
        }

        EnzymeMapModel { entries, sites }
    }

    /// Die gefilterten Einträge.
    pub fn entries(&self) -> &[EnzymeMapEntry] {
        &self.entries
    }

    /// Die gefilterten Schnittstellen.
    pub fn sites(&self) -> &[CutSite] {
        &self.sites
    }

    /// Eintrag zu einem Enzymnamen (exakter Vergleich).
    pub fn entry(&self, enzyme: &str) -> Option<&EnzymeMapEntry> {
        self.entries.iter().find(|e| e.enzyme == enzyme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Topology;
    use crate::model::sequence::{Sequence, SequenceType};

    // EcoRI (GAATTC) zweimal bei 0 und 10, BamHI (GGATCC) einmal bei 16.
    const BASES: &str = "GAATTCAAAAGAATTCGGATCC";

    fn doc(topology: Topology) -> SequenceDocument {
        let seq = Sequence::new("s", "s", BASES, SequenceType::Dna).unwrap();
        SequenceDocument::of("s", seq).with_topology(topology)
    }

    #[test]
    fn alle_enzyme() {
        let db = EnzymeDb::standard();
        let m = EnzymeMapModel::new(&db, &doc(Topology::Linear), EnzymeFilter::All);
        let eco = m.entry("EcoRI").unwrap();
        assert_eq!(eco.count, 2);
        assert_eq!(eco.positions, vec![0, 10]);
        let bam = m.entry("BamHI").unwrap();
        assert_eq!(bam.count, 1);
        assert_eq!(bam.positions, vec![16]);
        assert!(m.entry("HindIII").is_none());
        assert!(m.entries().iter().all(|e| e.count >= 1));
        let names: Vec<&str> = m.entries().iter().map(|e| e.enzyme.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn einzelschneider() {
        let db = EnzymeDb::standard();
        let m = EnzymeMapModel::new(&db, &doc(Topology::Linear), EnzymeFilter::UniqueCutters);
        assert!(m.entry("BamHI").is_some());
        assert!(m.entry("EcoRI").is_none());
        assert!(m.entries().iter().all(|e| e.count == 1));
        assert!(m.sites().iter().all(|s| s.enzyme != "EcoRI"));
        assert!(m.sites().iter().any(|s| s.enzyme == "BamHI"));
    }

    #[test]
    fn doppelschneider() {
        let db = EnzymeDb::standard();
        let m = EnzymeMapModel::new(&db, &doc(Topology::Linear), EnzymeFilter::DoubleCutters);
        assert!(m.entry("EcoRI").is_some());
        assert!(m.entry("BamHI").is_none());
        assert!(m.entries().iter().all(|e| e.count == 2));
        assert_eq!(m.sites().iter().filter(|s| s.enzyme == "EcoRI").count(), 2);
    }

    #[test]
    fn nichtschneider() {
        let db = EnzymeDb::standard();
        let m = EnzymeMapModel::new(&db, &doc(Topology::Linear), EnzymeFilter::NonCutters);
        assert!(m.entry("HindIII").is_some());
        assert!(m.entry("EcoRI").is_none());
        assert!(m.entry("BamHI").is_none());
        assert!(m
            .entries()
            .iter()
            .all(|e| e.count == 0 && e.positions.is_empty()));
        assert!(m.sites().is_empty());
        let all = EnzymeMapModel::new(&db, &doc(Topology::Linear), EnzymeFilter::All);
        assert_eq!(m.entries().len() + all.entries().len(), db.len());
    }

    #[test]
    fn leere_datenbank_und_leeres_dokument() {
        let m = EnzymeMapModel::new(
            &EnzymeDb::empty(),
            &doc(Topology::Linear),
            EnzymeFilter::All,
        );
        assert!(m.entries().is_empty() && m.sites().is_empty());
        let m = EnzymeMapModel::new(
            &EnzymeDb::empty(),
            &doc(Topology::Linear),
            EnzymeFilter::NonCutters,
        );
        assert!(m.entries().is_empty());
    }

    #[test]
    fn zirkulaer_findet_ueber_den_nullpunkt() {
        // Zirkulär umfasst GAATTC auch über das Ende hinweg: "…GGATCC" + "GAATTC…" enthält CCGAAT, kein neuer Treffer;
        // die Zahl der EcoRI-Treffer bleibt 2.
        let db = EnzymeDb::standard();
        let m = EnzymeMapModel::new(&db, &doc(Topology::Circular), EnzymeFilter::All);
        assert_eq!(m.entry("EcoRI").unwrap().count, 2);
    }
}
