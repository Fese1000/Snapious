//! Project store: documents and history in a SQLite file.
//!
//! Schema (version 1): tables `documents` and `history`. Documents are stored as JSON text.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::alignment::AlignmentDocument;
use crate::model::document::{Document, SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location, Strand};
use crate::model::sequence::{Sequence, SequenceType};
use crate::ops::operation::{HistoryEntry, Params};

const SCHEMA_VERSION: i64 = 1;

/// Short info about a stored document.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentInfo {
    pub id: String,
    pub name: String,
    pub doc_type: String,
    pub created: i64,
}

/// A project file.
pub struct ProjectStore {
    pub(crate) conn: Connection,
}

impl ProjectStore {
    /// Opens the file or creates it together with the schema.
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    /// Project in memory only (for tests and temporary work).
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 0 && version != SCHEMA_VERSION {
            return Err(Error::Io(format!(
                "Project file has schema version {} (expected {})",
                version, SCHEMA_VERSION
            )));
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents(
                 id TEXT PRIMARY KEY, type TEXT NOT NULL, name TEXT NOT NULL,
                 created INTEGER NOT NULL, payload TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS history(
                 id TEXT PRIMARY KEY, time INTEGER NOT NULL, operation TEXT NOT NULL,
                 parameters TEXT NOT NULL, inputs TEXT NOT NULL, outputs TEXT NOT NULL,
                 summary TEXT);
             CREATE TABLE IF NOT EXISTS workflows(
                 name TEXT PRIMARY KEY, json TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS folders(
                 id TEXT PRIMARY KEY, name TEXT NOT NULL, parent_id TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS doc_folder(
                 doc_id TEXT PRIMARY KEY, folder_id TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS notes(
                 doc_id TEXT PRIMARY KEY, text TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS favorites(
                 doc_id TEXT PRIMARY KEY);
             CREATE TABLE IF NOT EXISTS versions(
                 doc_id TEXT NOT NULL, version INTEGER NOT NULL, time INTEGER NOT NULL,
                 type TEXT NOT NULL, name TEXT NOT NULL, created INTEGER NOT NULL,
                 payload TEXT NOT NULL, summary TEXT NOT NULL,
                 PRIMARY KEY(doc_id, version));
             CREATE TABLE IF NOT EXISTS external_files(
                 id TEXT PRIMARY KEY, path TEXT NOT NULL, kind TEXT NOT NULL,
                 doc_id TEXT NOT NULL, label TEXT NOT NULL, added INTEGER NOT NULL);",
        )?;
        conn.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION))?;
        Ok(Self { conn })
    }

    /// Saves a document (insert or overwrite by ID).
    pub fn save(&mut self, doc: &Document) -> Result<()> {
        let payload = match doc {
            Document::Sequence(d) => serde_json::to_string(&to_payload(d))?,
            Document::Alignment(d) => serde_json::to_string(&AlignmentPayload {
                seq_type: seq_type_text(d.seq_type()).to_string(),
                row_names: d.row_names().to_vec(),
                rows: d.rows().to_vec(),
                metadata: d.metadata().clone(),
            })?,
        };
        self.conn.execute(
            "INSERT INTO documents(id, type, name, created, payload) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET type = ?2, name = ?3, created = ?4, payload = ?5",
            params![doc.id(), doc.doc_type(), doc.name(), doc.created(), payload],
        )?;
        Ok(())
    }

    /// Loads a document; `None` if the ID does not exist.
    pub fn load(&self, id: &str) -> Result<Option<Document>> {
        let row: Option<(String, String, String, i64, String)> = self
            .conn
            .query_row(
                "SELECT id, type, name, created, payload FROM documents WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some((id, doc_type, name, created, payload)) => match doc_type.as_str() {
                "sequence" => {
                    let p: SequencePayload = serde_json::from_str(&payload)?;
                    Ok(Some(Document::Sequence(from_payload(
                        &id, &name, created, p,
                    )?)))
                }
                "alignment" => {
                    let p: AlignmentPayload = serde_json::from_str(&payload)?;
                    Ok(Some(Document::Alignment(AlignmentDocument::new(
                        &id,
                        &name,
                        seq_type_from_text(&p.seq_type),
                        p.row_names,
                        p.rows,
                        p.metadata,
                        created,
                    )?)))
                }
                // Phylogenetic trees no longer exist; old projects keep the rows, but the documents are not shown
                "tree" => Ok(None),
                other => Err(Error::Io(format!("Unknown document type: {}", other))),
            },
        }
    }

    /// All documents, sorted by creation time and name.
    pub fn list(&self) -> Result<Vec<DocumentInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, type, created FROM documents WHERE type <> 'tree' ORDER BY created, name, id")?;
        let rows = stmt.query_map([], |r| {
            Ok(DocumentInfo {
                id: r.get(0)?,
                name: r.get(1)?,
                doc_type: r.get(2)?,
                created: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Deletes a document (no error if it is missing).
    pub fn delete(&mut self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM documents WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Saves a workflow as JSON text (overwrites the same name).
    pub fn save_workflow(&mut self, name: &str, json: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO workflows(name, json) VALUES(?1, ?2) ON CONFLICT(name) DO UPDATE SET json = ?2",
            params![name, json],
        )?;
        Ok(())
    }

    /// All saved workflows `(name, JSON)`, sorted by name.
    pub fn workflows(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, json FROM workflows ORDER BY name")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Deletes a workflow (no error if it does not exist).
    pub fn delete_workflow(&mut self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM workflows WHERE name = ?1", params![name])?;
        Ok(())
    }

    /// Saves a history entry.
    pub fn add_history(&mut self, e: &HistoryEntry) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO history(id, time, operation, parameters, inputs, outputs, summary)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                e.id,
                e.time,
                e.operation_id,
                serde_json::to_string(&e.parameters)?,
                serde_json::to_string(&e.input_ids)?,
                serde_json::to_string(&e.output_ids)?,
                e.summary
            ],
        )?;
        Ok(())
    }

    /// Entire history, ascending by time (for equal times in insertion order).
    pub fn history(&self) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, time, operation, parameters, inputs, outputs, summary
             FROM history ORDER BY time, rowid",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, time, operation, parameters, inputs, outputs, summary) = row?;
            out.push(HistoryEntry {
                id,
                operation_id: operation,
                parameters: serde_json::from_str::<Params>(&parameters)?,
                input_ids: serde_json::from_str(&inputs)?,
                output_ids: serde_json::from_str(&outputs)?,
                time,
                summary: summary.unwrap_or_default(),
            });
        }
        Ok(out)
    }

    /// History entries in which the document ID appears as input or output.
    pub fn history_for(&self, document_id: &str) -> Result<Vec<HistoryEntry>> {
        Ok(self
            .history()?
            .into_iter()
            .filter(|e| {
                e.input_ids.iter().any(|i| i == document_id)
                    || e.output_ids.iter().any(|i| i == document_id)
            })
            .collect())
    }
}

// ---------------------------------------------------------------- Payload (JSON)

#[derive(Serialize, Deserialize)]
struct FeaturePayload {
    feature_type: String,
    name: String,
    strand: String,
    partial_start: bool,
    partial_end: bool,
    intervals: Vec<[usize; 2]>,
    qualifiers: Vec<(String, Vec<String>)>,
}

#[derive(Serialize, Deserialize)]
struct AlignmentPayload {
    seq_type: String,
    row_names: Vec<String>,
    rows: Vec<String>,
    metadata: BTreeMap<String, String>,
}

fn seq_type_text(t: SequenceType) -> &'static str {
    match t {
        SequenceType::Dna => "DNA",
        SequenceType::Rna => "RNA",
        SequenceType::Protein => "PROTEIN",
    }
}

fn seq_type_from_text(s: &str) -> SequenceType {
    match s {
        "RNA" => SequenceType::Rna,
        "PROTEIN" => SequenceType::Protein,
        _ => SequenceType::Dna,
    }
}

#[derive(Serialize, Deserialize)]
struct SequencePayload {
    sequence_id: String,
    sequence_name: String,
    description: String,
    seq_type: String,
    bases: String,
    topology: String,
    metadata: BTreeMap<String, String>,
    features: Vec<FeaturePayload>,
}

fn to_payload(d: &SequenceDocument) -> SequencePayload {
    let s = d.sequence();
    SequencePayload {
        sequence_id: s.id().to_string(),
        sequence_name: s.name().to_string(),
        description: s.description().to_string(),
        seq_type: match s.seq_type() {
            SequenceType::Dna => "DNA",
            SequenceType::Rna => "RNA",
            SequenceType::Protein => "PROTEIN",
        }
        .to_string(),
        bases: s.bases().to_string(),
        topology: match d.topology() {
            Topology::Linear => "LINEAR",
            Topology::Circular => "CIRCULAR",
        }
        .to_string(),
        metadata: d.metadata().clone(),
        features: d
            .features()
            .iter()
            .map(|f| FeaturePayload {
                feature_type: f.feature_type().to_string(),
                name: f.name().to_string(),
                strand: match f.location().strand() {
                    Strand::Forward => "FORWARD",
                    Strand::Reverse => "REVERSE",
                    Strand::None => "NONE",
                }
                .to_string(),
                partial_start: f.location().partial_start(),
                partial_end: f.location().partial_end(),
                intervals: f
                    .location()
                    .intervals()
                    .iter()
                    .map(|i| [i.start(), i.end()])
                    .collect(),
                qualifiers: f.qualifiers().to_vec(),
            })
            .collect(),
    }
}

fn from_payload(
    id: &str,
    name: &str,
    created: i64,
    p: SequencePayload,
) -> Result<SequenceDocument> {
    let seq_type = match p.seq_type.as_str() {
        "DNA" => SequenceType::Dna,
        "RNA" => SequenceType::Rna,
        "PROTEIN" => SequenceType::Protein,
        other => return Err(Error::Io(format!("Unknown sequence type: {}", other))),
    };
    let topology = match p.topology.as_str() {
        "LINEAR" => Topology::Linear,
        "CIRCULAR" => Topology::Circular,
        other => return Err(Error::Io(format!("Unknown topology: {}", other))),
    };
    let mut sequence = Sequence::new(&p.sequence_id, &p.sequence_name, &p.bases, seq_type)?;
    sequence.set_description(&p.description);
    let mut features = Vec::with_capacity(p.features.len());
    for f in p.features {
        let strand = match f.strand.as_str() {
            "FORWARD" => Strand::Forward,
            "REVERSE" => Strand::Reverse,
            "NONE" => Strand::None,
            other => return Err(Error::Io(format!("Unknown strand: {}", other))),
        };
        let intervals = f
            .intervals
            .iter()
            .map(|[s, e]| Interval::new(*s, *e))
            .collect::<Result<Vec<_>>>()?;
        let location = Location::new(intervals, strand, f.partial_start, f.partial_end)?;
        features.push(Feature::new(
            &f.feature_type,
            &f.name,
            location,
            f.qualifiers,
        ));
    }
    Ok(SequenceDocument::new(
        id, name, sequence, topology, features, p.metadata, created,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::genbank;

    fn mini() -> Document {
        let text = include_str!("../../testdata/mini.gb");
        Document::Sequence(genbank::parse(text).unwrap().remove(0))
    }

    #[test]
    fn workflows_speichern() {
        let mut store = ProjectStore::open_in_memory().unwrap();
        assert!(store.workflows().unwrap().is_empty());
        store.save_workflow("b", "{\"x\":1}").unwrap();
        store.save_workflow("a", "{}").unwrap();
        store.save_workflow("b", "{\"x\":2}").unwrap();
        let w = store.workflows().unwrap();
        assert_eq!(
            w,
            vec![
                ("a".to_string(), "{}".to_string()),
                ("b".to_string(), "{\"x\":2}".to_string())
            ]
        );
        store.delete_workflow("a").unwrap();
        store.delete_workflow("gibtsnicht").unwrap();
        assert_eq!(store.workflows().unwrap().len(), 1);
    }

    #[test]
    fn alignment_rundreise_und_alte_baeume() {
        let mut store = ProjectStore::open_in_memory().unwrap();
        let aln = AlignmentDocument::of(
            "aln",
            SequenceType::Protein,
            vec!["a".to_string(), "b".to_string()],
            vec!["AC-T".to_string(), "ACGT".to_string()],
        )
        .unwrap();
        store.save(&Document::Alignment(aln.clone())).unwrap();
        // a phylogenetic tree from an older version: stays in the database, but is no longer shown or loaded
        store
            .conn
            .execute(
                "INSERT INTO documents(id, type, name, created, payload) VALUES('t1', 'tree', 'baum', 1, '{}')",
                [],
            )
            .unwrap();
        match store.load(aln.id()).unwrap() {
            Some(Document::Alignment(b)) => {
                assert_eq!(b.rows(), aln.rows());
                assert_eq!(b.row_names(), aln.row_names());
                assert_eq!(b.seq_type(), SequenceType::Protein);
                assert_eq!(b.name(), "aln");
            }
            other => panic!("unerwartet: {:?}", other),
        }
        assert!(store.load("t1").unwrap().is_none());
        let types: Vec<String> = store
            .list()
            .unwrap()
            .into_iter()
            .map(|d| d.doc_type)
            .collect();
        assert_eq!(types, vec!["alignment".to_string()]);
    }

    #[test]
    fn speichern_schliessen_oeffnen_laden() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.snapious");
        let doc = mini();
        {
            let mut store = ProjectStore::open(&path).unwrap();
            store.save(&doc).unwrap();
        }
        let store = ProjectStore::open(&path).unwrap();
        let back = store.load(doc.id()).unwrap().unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn zweimal_speichern_ergibt_einen_eintrag_mit_neuem_namen() {
        let mut store = ProjectStore::open_in_memory().unwrap();
        let doc = mini();
        store.save(&doc).unwrap();
        let Document::Sequence(s) = &doc else {
            panic!("Sequenzdokument erwartet");
        };
        store.save(&Document::Sequence(s.with_name("neu"))).unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "neu");
        assert_eq!(list[0].doc_type, "sequence");
    }

    #[test]
    fn loeschen() {
        let mut store = ProjectStore::open_in_memory().unwrap();
        let doc = mini();
        store.save(&doc).unwrap();
        store.delete(doc.id()).unwrap();
        assert!(store.load(doc.id()).unwrap().is_none());
        store.delete("gibt-es-nicht").unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn verlauf_reihenfolge_und_filter() {
        let mut store = ProjectStore::open_in_memory().unwrap();
        let mk = |id: &str, t: i64, inp: &str, out: &str| HistoryEntry {
            id: id.into(),
            operation_id: "edit.insert".into(),
            parameters: Params::from([("position".to_string(), "2".to_string())]),
            input_ids: vec![inp.into()],
            output_ids: vec![out.into()],
            time: t,
            summary: "x".into(),
        };
        store.add_history(&mk("h2", 200, "b", "c")).unwrap();
        store.add_history(&mk("h1", 100, "a", "b")).unwrap();
        let all = store.history().unwrap();
        assert_eq!(
            all.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["h1", "h2"]
        );
        assert_eq!(
            all[0].parameters.get("position").map(String::as_str),
            Some("2")
        );
        assert_eq!(store.history_for("a").unwrap().len(), 1);
        assert_eq!(store.history_for("b").unwrap().len(), 2);
        assert_eq!(store.history_for("c").unwrap().len(), 1);
        assert!(store.history_for("z").unwrap().is_empty());
    }

    #[test]
    fn fremde_schema_version_wird_abgelehnt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("alt.snapious");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("PRAGMA user_version = 99;").unwrap();
        }
        assert!(matches!(ProjectStore::open(&path), Err(Error::Io(_))));
    }

    #[test]
    fn unbekannter_dokumenttyp_beim_laden() {
        let store = ProjectStore::open_in_memory().unwrap();
        store
            .conn
            .execute(
                "INSERT INTO documents(id, type, name, created, payload) VALUES('x','foto','n',1,'{}')",
                [],
            )
            .unwrap();
        match store.load("x") {
            Err(Error::Io(m)) => assert!(m.contains("foto")),
            other => panic!("unerwartet: {:?}", other.map(|o| o.is_some())),
        }
    }

    #[test]
    fn kaputte_nutzlast_ist_fehler_kein_absturz() {
        let store = ProjectStore::open_in_memory().unwrap();
        store
            .conn
            .execute(
                "INSERT INTO documents(id, type, name, created, payload) VALUES('x','sequence','n',1,'kaputt')",
                [],
            )
            .unwrap();
        assert!(store.load("x").is_err());
    }

    #[test]
    fn protein_und_rna_round_trip_mit_beschreibung() {
        let mut store = ProjectStore::open_in_memory().unwrap();
        let mut seq = Sequence::new("p", "Prot", "MDEK*", SequenceType::Protein).unwrap();
        seq.set_description("Testprotein");
        let doc =
            Document::Sequence(SequenceDocument::of("Prot", seq).with_topology(Topology::Linear));
        store.save(&doc).unwrap();
        assert_eq!(store.load(doc.id()).unwrap().unwrap(), doc);
    }

    #[test]
    fn datei_wird_beim_zweiten_oeffnen_nicht_ueberschrieben() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.snapious");
        {
            let mut s = ProjectStore::open(&path).unwrap();
            s.save(&mini()).unwrap();
        }
        let s = ProjectStore::open(&path).unwrap();
        assert_eq!(s.list().unwrap().len(), 1);
    }
}
