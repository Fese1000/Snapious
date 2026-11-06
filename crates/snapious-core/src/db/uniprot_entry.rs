//! Einzelner UniProt-Eintrag (JSON) mit Sequenz und Features; Umwandlung in ein Protein-Dokument.

use crate::db::ncbi::percent_encode;
use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Location, Strand};
use crate::model::sequence::{Sequence, SequenceType};

use serde_json::Value;
use std::collections::BTreeMap;

/// Adresse eines Eintrags: `https://rest.uniprot.org/uniprotkb/<accession>.json` (Akzession mit `percent_encode` aus `db::ncbi`).
pub fn entry_url(accession: &str) -> String {
    format!(
        "https://rest.uniprot.org/uniprotkb/{}.json",
        percent_encode(accession)
    )
}

/// Ein Feature des Eintrags; Positionen wie bei UniProt 1-basiert und eingeschlossen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProteinFeature {
    pub kind: String,
    pub description: String,
    pub start: usize,
    pub end: usize,
}

/// Ein UniProt-Eintrag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniprotEntry {
    pub accession: String,
    /// `uniProtkbId`, sonst leer.
    pub entry_name: String,
    /// `organism.scientificName`, sonst leer.
    pub organism: String,
    pub sequence: String,
    pub features: Vec<ProteinFeature>,
}

/// Liest ein Eintrag-JSON: Pflicht `primaryAccession` (Text) und `sequence.value` (Text), sonst `Error::InvalidInput`; kein JSON ebenfalls.
/// `features` (Liste, darf fehlen): je Eintrag `type` (Text, Pflicht), `description` (Text, sonst leer), `location.start.value` und
/// `location.end.value` (ganze Zahlen). Features ohne `type` oder ohne beide Positionen werden übersprungen; Reihenfolge wie im JSON.
pub fn parse_entry(json: &str) -> Result<UniprotEntry> {
    let root = serde_json::from_str::<Value>(json)
        .map_err(|e| Error::InvalidInput(format!("Ungültiges JSON: {}", e)))?;

    let accession = root
        .get("primaryAccession")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InvalidInput("Fehlendes 'primaryAccession'".to_string()))?
        .to_string();

    let sequence = root
        .get("sequence")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InvalidInput("Fehlendes 'sequence.value'".to_string()))?
        .to_string();

    let entry_name = root
        .get("uniProtkbId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let organism = root
        .get("organism")
        .and_then(|v| v.get("scientificName"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let features: Vec<ProteinFeature> = root
        .get("features")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let kind = item
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(String::from)?;
                    let description = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let start = item
                        .get("location")
                        .and_then(|l| l.get("start"))
                        .and_then(|s| s.get("value"))
                        .and_then(|v| v.as_u64())?;
                    let end = item
                        .get("location")
                        .and_then(|l| l.get("end"))
                        .and_then(|e| e.get("value"))
                        .and_then(|v| v.as_u64())?;
                    Some(ProteinFeature {
                        kind,
                        description,
                        start: start as usize,
                        end: end as usize,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(UniprotEntry {
        accession,
        entry_name: entry_name.to_string(),
        organism: organism.to_string(),
        sequence,
        features,
    })
}

impl UniprotEntry {
    /// Protein-Dokument: Name = `entry_name`, falls nicht leer, sonst `accession`; Sequenz = `sequence` (Großbuchstaben, `SequenceType::Protein`);
    /// Topologie linear; Metadaten `accession`, `organism` (nur wenn nicht leer), `source = "uniprot"`.
    /// Jedes Feature wird ein `Feature` des Typs `kind` mit Name = `description`, falls nicht leer, sonst `kind`, Strang `Strand::None`, Intervall
    /// `[start − 1, end)` (UniProt 1-basiert, eingeschlossen). Features mit `start == 0`, `end < start` oder `end` hinter der Sequenzlänge
    /// werden übersprungen.
    pub fn to_document(&self) -> Result<SequenceDocument> {
        let name = if !self.entry_name.is_empty() {
            self.entry_name.clone()
        } else {
            self.accession.clone()
        };

        let mut metadata = BTreeMap::new();
        metadata.insert("accession".to_string(), self.accession.clone());
        if !self.organism.is_empty() {
            metadata.insert("organism".to_string(), self.organism.clone());
        }
        metadata.insert("source".to_string(), "uniprot".to_string());

        let features: Vec<Feature> = self
            .features
            .iter()
            .filter_map(|pf| {
                if pf.start == 0 || pf.end < pf.start || pf.end > self.sequence.len() {
                    return None;
                }
                let loc = Location::of(pf.start.saturating_sub(1), pf.end, Strand::None).ok()?;
                let fname = if !pf.description.is_empty() {
                    pf.description.clone()
                } else {
                    pf.kind.clone()
                };
                Some(Feature::new(&pf.kind, &fname, loc, Vec::new()))
            })
            .collect();

        let seq = Sequence::new(
            &self.accession,
            &name,
            &self.sequence,
            SequenceType::Protein,
        )
        .map_err(|e| Error::InvalidInput(e.to_string()))?;
        Ok(SequenceDocument::of(&name, seq)
            .with_topology(Topology::Linear)
            .with_features(features)
            .with_metadata(metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::Strand;
    use crate::model::sequence::SequenceType;

    const JSON: &str = r#"{
      "primaryAccession": "P12345",
      "uniProtkbId": "TEST_HUMAN",
      "organism": {"scientificName": "Homo sapiens"},
      "sequence": {"value": "MKVLAAGIVGLLLAQPSTHEWK", "length": 22},
      "features": [
        {"type": "Domain", "description": "Kinase", "location": {"start": {"value": 10}, "end": {"value": 20}}},
        {"type": "Active site", "location": {"start": {"value": 5}, "end": {"value": 5}}},
        {"description": "ohne Typ", "location": {"start": {"value": 1}, "end": {"value": 2}}},
        {"type": "Region", "description": "kaputt", "location": {"start": {"value": 3}}},
        {"type": "Region", "description": "zu lang", "location": {"start": {"value": 20}, "end": {"value": 99}}}
      ]
    }"#;

    #[test]
    fn adresse() {
        assert_eq!(
            entry_url("P12345"),
            "https://rest.uniprot.org/uniprotkb/P12345.json"
        );
    }

    #[test]
    fn eintrag_lesen() {
        let e = parse_entry(JSON).unwrap();
        assert_eq!(e.accession, "P12345");
        assert_eq!(e.entry_name, "TEST_HUMAN");
        assert_eq!(e.organism, "Homo sapiens");
        assert_eq!(e.sequence.len(), 22);
        assert_eq!(e.features.len(), 3);
        assert_eq!(
            e.features[0],
            ProteinFeature {
                kind: "Domain".to_string(),
                description: "Kinase".to_string(),
                start: 10,
                end: 20
            }
        );
        assert_eq!(e.features[1].description, "");
        assert_eq!(e.features[2].end, 99);
    }

    #[test]
    fn dokument() {
        let d = parse_entry(JSON).unwrap().to_document().unwrap();
        assert_eq!(d.name(), "TEST_HUMAN");
        assert_eq!(d.sequence().seq_type(), SequenceType::Protein);
        assert_eq!(
            d.metadata().get("accession").map(String::as_str),
            Some("P12345")
        );
        assert_eq!(
            d.metadata().get("source").map(String::as_str),
            Some("uniprot")
        );
        assert_eq!(d.features().len(), 2);
        let f = &d.features()[0];
        assert_eq!(f.feature_type(), "Domain");
        assert_eq!(f.name(), "Kinase");
        assert_eq!((f.location().start(), f.location().end()), (9, 20));
        assert_eq!(f.location().strand(), Strand::None);
        let a = &d.features()[1];
        assert_eq!(a.name(), "Active site");
        assert_eq!((a.location().start(), a.location().end()), (4, 5));
    }

    #[test]
    fn ohne_features_und_fehler() {
        let e = parse_entry(r#"{"primaryAccession":"Q1","sequence":{"value":"MK"}}"#).unwrap();
        assert!(e.features.is_empty());
        assert_eq!(e.entry_name, "");
        assert_eq!(e.to_document().unwrap().name(), "Q1");
        assert!(matches!(
            parse_entry("kein json"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_entry(r#"{"sequence":{"value":"MK"}}"#),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_entry(r#"{"primaryAccession":"Q1"}"#),
            Err(Error::InvalidInput(_))
        ));
    }
}
