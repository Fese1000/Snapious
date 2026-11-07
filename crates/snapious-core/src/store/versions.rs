//! Document versions: on saving, the previous revision is kept in the table `versions` (at most 50 per document).
//! Table `versions(doc_id, version, time, type, name, created, payload, summary)`; `payload`, `type`, `name`, `created` are the columns
//! of `documents` at the time of the revision.

use rusqlite::{params, OptionalExtension};

use crate::error::{Error, Result};
use crate::model::document::Document;
use crate::store::project_store::ProjectStore;
use crate::util::now_secs;

/// Maximum number of retained versions per document.
pub const MAX_VERSIONS: usize = 50;

/// Short info about a version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionInfo {
    /// Sequential number per document, starting at 1.
    pub version: i64,
    pub time: i64,
    pub summary: String,
}

impl ProjectStore {
    /// Saves like `save`, but first stores the old revision: if the document already exists (row in `documents`) and the new payload
    /// (`payload` after saving) differs from the old one, the old row is recorded as a new version (number = largest existing + 1,
    /// `time = now_secs()`, `summary` as passed). Then the oldest versions are deleted so that at most `MAX_VERSIONS` remain.
    /// If the payload is the same or the document is new, no version is created. Hint: first read the old row, then `save`, then read the new
    /// payload and compare; if the payload is the same there is nothing to do.
    pub fn save_versioned(&mut self, doc: &Document, summary: &str) -> Result<()> {
        let prev: Option<String> = self
            .conn
            .query_row(
                "SELECT payload FROM documents WHERE id = ?1",
                params![doc.id()],
                |r| r.get(0),
            )
            .optional()?;
        let prev_row: Option<(String, String, i64, String)> = self
            .conn
            .query_row(
                "SELECT type, name, created, payload FROM documents WHERE id = ?1",
                params![doc.id()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        self.save(doc)?;
        let (Some(old_payload), Some((ty, name, created, _))) = (prev, prev_row) else {
            return Ok(());
        };
        let new_payload: String = self.conn.query_row(
            "SELECT payload FROM documents WHERE id = ?1",
            params![doc.id()],
            |r| r.get(0),
        )?;
        if new_payload == old_payload {
            return Ok(());
        }
        self.insert_version(doc.id(), &ty, &name, created, &old_payload, summary)?;
        Ok(())
    }

    /// Versions of a document, newest first.
    pub fn versions(&self, doc_id: &str) -> Result<Vec<VersionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT version, time, summary FROM versions WHERE doc_id = ?1 ORDER BY version DESC",
        )?;
        let rows = stmt.query_map(params![doc_id], |r| {
            Ok(VersionInfo {
                version: r.get(0)?,
                time: r.get(1)?,
                summary: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Loads a version as a document (same ID as the document). Hint: copy the row into an empty `ProjectStore::open_in_memory()`
    /// (`INSERT INTO documents(id, type, name, created, payload)`) and call `load` there. Unknown version -> `Error::InvalidInput`.
    pub fn load_version(&self, doc_id: &str, version: i64) -> Result<Document> {
        let row: Option<(String, String, i64, String)> = self
            .conn
            .query_row(
                "SELECT type, name, created, payload FROM versions WHERE doc_id = ?1 AND version = ?2",
                params![doc_id, version],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((ty, name, created, payload)) = row else {
            return Err(Error::InvalidInput(format!(
                "Unknown version {} of {}",
                version, doc_id
            )));
        };
        let tmp = ProjectStore::open_in_memory()?;
        tmp.conn.execute(
            "INSERT INTO documents(id, type, name, created, payload) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![doc_id, ty, name, created, payload],
        )?;
        tmp.load(doc_id)?
            .ok_or_else(|| Error::InvalidInput("Version not readable".to_string()))
    }

    /// Restores a version: the current revision is (if it differs) itself stored as a new version
    /// (`summary = "before restore"`), then the row in `documents` (`type`, `name`, `created`, `payload`) is replaced by that of the version.
    /// The version stays in the list. Unknown version or unknown document -> `Error::InvalidInput`.
    pub fn restore_version(&mut self, doc_id: &str, version: i64) -> Result<()> {
        let row: Option<(String, String, i64, String)> = self
            .conn
            .query_row(
                "SELECT type, name, created, payload FROM versions WHERE doc_id = ?1 AND version = ?2",
                params![doc_id, version],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((ty, name, created, payload)) = row else {
            return Err(Error::InvalidInput(format!(
                "Unknown version {} of {}",
                version, doc_id
            )));
        };
        let current: Option<(String, String, i64, String)> = self
            .conn
            .query_row(
                "SELECT type, name, created, payload FROM documents WHERE id = ?1",
                params![doc_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((cty, cname, ccreated, cpayload)) = current else {
            return Err(Error::InvalidInput(format!(
                "Unknown document {}",
                doc_id
            )));
        };
        if cpayload != payload {
            self.insert_version(
                doc_id,
                &cty,
                &cname,
                ccreated,
                &cpayload,
                "before restore",
            )?;
        }
        self.conn.execute(
            "UPDATE documents SET type = ?2, name = ?3, created = ?4, payload = ?5 WHERE id = ?1",
            params![doc_id, ty, name, created, payload],
        )?;
        Ok(())
    }
}

impl ProjectStore {
    /// Records a version and trims the list to `MAX_VERSIONS`.
    fn insert_version(
        &mut self,
        doc_id: &str,
        ty: &str,
        name: &str,
        created: i64,
        payload: &str,
        summary: &str,
    ) -> Result<()> {
        let next: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM versions WHERE doc_id = ?1",
            params![doc_id],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO versions(doc_id, version, time, type, name, created, payload, summary)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                doc_id,
                next,
                now_secs(),
                ty,
                name,
                created,
                payload,
                summary
            ],
        )?;
        self.conn.execute(
            "DELETE FROM versions WHERE doc_id = ?1 AND version <= ?2",
            params![doc_id, next - MAX_VERSIONS as i64],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::SequenceDocument;
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(id_from: &SequenceDocument, bases: &str) -> Document {
        let seq = Sequence::new("s", "s", bases, SequenceType::Dna).unwrap();
        Document::Sequence(id_from.with_name("s").with_bases_for_test(seq))
    }

    trait WithBases {
        fn with_bases_for_test(&self, seq: Sequence) -> SequenceDocument;
    }

    impl WithBases for SequenceDocument {
        fn with_bases_for_test(&self, seq: Sequence) -> SequenceDocument {
            SequenceDocument::new(
                self.id(),
                self.name(),
                seq,
                self.topology(),
                self.features().to_vec(),
                self.metadata().clone(),
                self.created(),
            )
        }
    }

    fn start() -> (ProjectStore, SequenceDocument) {
        let seq = Sequence::new("s", "s", "AAAA", SequenceType::Dna).unwrap();
        (
            ProjectStore::open_in_memory().unwrap(),
            SequenceDocument::of("s", seq),
        )
    }

    fn bases_of(d: &Document) -> String {
        match d {
            Document::Sequence(s) => s.bases().to_string(),
            _ => panic!("keine Sequenz"),
        }
    }

    #[test]
    fn erste_fassung_wird_version() {
        let (mut s, base) = start();
        let id = base.id().to_string();
        s.save_versioned(&Document::Sequence(base.clone()), "neu")
            .unwrap();
        assert!(s.versions(&id).unwrap().is_empty());
        s.save_versioned(&doc(&base, "CCCC"), "geändert").unwrap();
        let v = s.versions(&id).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, 1);
        assert_eq!(v[0].summary, "geändert");
        assert_eq!(bases_of(&s.load_version(&id, 1).unwrap()), "AAAA");
        assert_eq!(bases_of(&s.load(&id).unwrap().unwrap()), "CCCC");
        // same revision again: no new version
        s.save_versioned(&doc(&base, "CCCC"), "nochmal").unwrap();
        assert_eq!(s.versions(&id).unwrap().len(), 1);
    }

    #[test]
    fn wiederherstellen() {
        let (mut s, base) = start();
        let id = base.id().to_string();
        s.save_versioned(&Document::Sequence(base.clone()), "")
            .unwrap();
        s.save_versioned(&doc(&base, "CCCC"), "").unwrap();
        s.restore_version(&id, 1).unwrap();
        assert_eq!(bases_of(&s.load(&id).unwrap().unwrap()), "AAAA");
        let v = s.versions(&id).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].version, 2);
        assert_eq!(v[0].summary, "before restore");
        assert_eq!(bases_of(&s.load_version(&id, 2).unwrap()), "CCCC");
        assert!(matches!(
            s.restore_version(&id, 9),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.load_version(&id, 9),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.restore_version("nix", 1),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn hoechstens_fuenfzig() {
        let (mut s, base) = start();
        let id = base.id().to_string();
        s.save_versioned(&Document::Sequence(base.clone()), "")
            .unwrap();
        for i in 0..60usize {
            let bases = "ACGT".repeat(1 + i % 7) + &"A".repeat(i);
            s.save_versioned(&doc(&base, &bases), &format!("v{}", i))
                .unwrap();
        }
        let v = s.versions(&id).unwrap();
        assert_eq!(v.len(), MAX_VERSIONS);
        // newest first, the oldest were dropped
        assert_eq!(v[0].version, 60);
        assert_eq!(v[MAX_VERSIONS - 1].version, 11);
    }
}
