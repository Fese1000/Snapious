//! Verknüpfte externe Dateien (SAM/BAM, FASTQ, VCF …): Pfade werden im Projekt vermerkt, die Dateien selbst nicht kopiert.

use rusqlite::{params, OptionalExtension};

use crate::error::{Error, Result};
use crate::store::project_store::ProjectStore;
use crate::util::{new_id, now_secs};

/// Eine verknüpfte Datei.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalFile {
    pub id: String,
    /// Pfad wie angegeben.
    pub path: String,
    /// Art, z. B. `sam`, `bam`, `fastq`, `vcf`.
    pub kind: String,
    /// Dokument (Referenzsequenz), zu dem die Datei gehört; leer, wenn keins.
    pub doc_id: String,
    /// Anzeigename.
    pub label: String,
    /// Zeitpunkt des Eintrags (Sekunden seit 1970).
    pub added: i64,
}

impl ProjectStore {
    /// Trägt eine Datei in die Tabelle `external_files` ein (Spalten `id`, `path`, `kind`, `doc_id`, `label`, `added`).
    /// Neue ID (`new_id()`), `added = now_secs()`. Ein leerer `path` oder `kind` → `Error::InvalidInput`. Ist `label` leer, wird der
    /// letzte Teil des Pfades (nach dem letzten `/` oder `\\`) als Name genommen. Gleicher `path` mit gleichem `kind` und `doc_id` existiert schon:
    /// keine zweite Zeile, sondern der vorhandene Eintrag wird zurückgegeben (und `label` aktualisiert, wenn eines angegeben war).
    /// Rückgabe: der Eintrag.
    pub fn add_external_file(
        &mut self,
        path: &str,
        kind: &str,
        doc_id: &str,
        label: &str,
    ) -> Result<ExternalFile> {
        if path.trim().is_empty() || kind.trim().is_empty() {
            return Err(Error::InvalidInput(
                "Pfad und Art der Datei dürfen nicht leer sein".to_string(),
            ));
        }
        let default_label = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();
        let existing: Option<ExternalFile> = self
            .conn
            .query_row(
                "SELECT id, path, kind, doc_id, label, added FROM external_files
                 WHERE path = ?1 AND kind = ?2 AND doc_id = ?3",
                params![path, kind, doc_id],
                row_to_file,
            )
            .optional()?;
        if let Some(mut f) = existing {
            if !label.is_empty() && f.label != label {
                self.conn.execute(
                    "UPDATE external_files SET label = ?1 WHERE id = ?2",
                    params![label, f.id],
                )?;
                f.label = label.to_string();
            }
            return Ok(f);
        }
        let f = ExternalFile {
            id: new_id(),
            path: path.to_string(),
            kind: kind.to_string(),
            doc_id: doc_id.to_string(),
            label: if label.is_empty() {
                default_label
            } else {
                label.to_string()
            },
            added: now_secs(),
        };
        self.conn.execute(
            "INSERT INTO external_files(id, path, kind, doc_id, label, added) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![f.id, f.path, f.kind, f.doc_id, f.label, f.added],
        )?;
        Ok(f)
    }

    /// Alle Einträge, sortiert nach `added`, dann nach `id`. Ist `doc_id` nicht leer, nur die Einträge dieses Dokuments.
    pub fn external_files(&self, doc_id: &str) -> Result<Vec<ExternalFile>> {
        let mut out = Vec::new();
        if doc_id.is_empty() {
            let mut stmt = self.conn.prepare(
                "SELECT id, path, kind, doc_id, label, added FROM external_files ORDER BY added, id",
            )?;
            for r in stmt.query_map([], row_to_file)? {
                out.push(r?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, path, kind, doc_id, label, added FROM external_files
                 WHERE doc_id = ?1 ORDER BY added, id",
            )?;
            for r in stmt.query_map(params![doc_id], row_to_file)? {
                out.push(r?);
            }
        }
        Ok(out)
    }

    /// Entfernt den Eintrag mit dieser ID (die Datei selbst bleibt). Unbekannte ID → `Error::InvalidInput`.
    pub fn remove_external_file(&mut self, id: &str) -> Result<()> {
        let n = self
            .conn
            .execute("DELETE FROM external_files WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(Error::InvalidInput(format!(
                "Keine verknüpfte Datei mit der ID {}",
                id
            )));
        }
        Ok(())
    }
}

fn row_to_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<ExternalFile> {
    Ok(ExternalFile {
        id: r.get(0)?,
        path: r.get(1)?,
        kind: r.get(2)?,
        doc_id: r.get(3)?,
        label: r.get(4)?,
        added: r.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eintragen_und_lesen() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        assert!(s.external_files("").unwrap().is_empty());
        let a = s
            .add_external_file("/daten/lauf1/reads.bam", "bam", "doc1", "")
            .unwrap();
        assert_eq!(a.label, "reads.bam");
        assert_eq!(a.kind, "bam");
        assert_eq!(a.doc_id, "doc1");
        assert!(!a.id.is_empty());
        assert!(a.added > 1_700_000_000);
        let b = s
            .add_external_file("C:\\daten\\x.sam", "sam", "doc2", "Lauf 2")
            .unwrap();
        assert_eq!(b.label, "Lauf 2");
        let all = s.external_files("").unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(s.external_files("doc1").unwrap(), vec![a.clone()]);
        assert_eq!(
            s.external_files("doc2").unwrap()[0].path,
            "C:\\daten\\x.sam"
        );
        assert!(s.external_files("unbekannt").unwrap().is_empty());
        let win = s
            .add_external_file("C:\\daten\\y.vcf", "vcf", "", "")
            .unwrap();
        assert_eq!(win.label, "y.vcf");
    }

    #[test]
    fn doppelte_eintraege() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        let a = s.add_external_file("/a/x.sam", "sam", "d", "").unwrap();
        let b = s.add_external_file("/a/x.sam", "sam", "d", "Neu").unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(b.label, "Neu");
        assert_eq!(s.external_files("").unwrap().len(), 1);
        assert_eq!(s.external_files("").unwrap()[0].label, "Neu");
        // anderer Typ oder anderes Dokument ist ein neuer Eintrag
        let c = s.add_external_file("/a/x.sam", "bam", "d", "").unwrap();
        assert_ne!(a.id, c.id);
        let d = s.add_external_file("/a/x.sam", "sam", "e", "").unwrap();
        assert_ne!(a.id, d.id);
        assert_eq!(s.external_files("").unwrap().len(), 3);
    }

    #[test]
    fn entfernen_und_fehler() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        let a = s.add_external_file("/a/x.sam", "sam", "d", "").unwrap();
        s.remove_external_file(&a.id).unwrap();
        assert!(s.external_files("").unwrap().is_empty());
        assert!(matches!(
            s.remove_external_file(&a.id),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.add_external_file("", "sam", "d", ""),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.add_external_file("/a/x.sam", "", "d", ""),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn bleibt_in_der_projektdatei() {
        let dir = std::env::temp_dir().join(format!("snapious-ext-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("p.snapious");
        {
            let mut s = ProjectStore::open(&file).unwrap();
            s.add_external_file("/a/x.sam", "sam", "d", "Eins").unwrap();
        }
        let s = ProjectStore::open(&file).unwrap();
        let v = s.external_files("d").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].label, "Eins");
        let _ = std::fs::remove_dir_all(dir);
    }
}
