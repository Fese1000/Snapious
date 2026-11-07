//! Ordner, Notizen und Favoriten für Dokumente. Tabellen: `folders(id, name, parent_id)`, `doc_folder(doc_id, folder_id)`,
//! `notes(doc_id, text)`, `favorites(doc_id)`. Ein leerer Ordnername (`""`) bedeutet die Wurzel.

use rusqlite::{params, OptionalExtension};

use crate::error::{Error, Result};
use crate::store::project_store::{DocumentInfo, ProjectStore};
use crate::util::new_id;

/// Ein Ordner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: String,
    pub name: String,
    /// ID des übergeordneten Ordners, `""` bei einem Ordner in der Wurzel.
    pub parent_id: String,
}

impl ProjectStore {
    /// Legt einen Ordner an (neue ID mit `new_id()`). Name leer (nach `trim`) → `Error::InvalidInput`; `parent_id` nicht leer und
    /// unbekannt → `Error::InvalidInput`. Der Name wird ohne umgebenden Leerraum gespeichert.
    pub fn create_folder(&mut self, name: &str, parent_id: &str) -> Result<Folder> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::InvalidInput(
                "The folder name must not be empty".to_string(),
            ));
        }
        if !parent_id.is_empty() && !self.folder_exists(parent_id)? {
            return Err(Error::InvalidInput(format!(
                "Unknown folder {}",
                parent_id
            )));
        }
        let f = Folder {
            id: new_id(),
            name: name.to_string(),
            parent_id: parent_id.to_string(),
        };
        self.conn.execute(
            "INSERT INTO folders(id, name, parent_id) VALUES(?1, ?2, ?3)",
            params![f.id, f.name, f.parent_id],
        )?;
        Ok(f)
    }

    /// Alle Ordner, sortiert nach Name, dann ID.
    pub fn folders(&self) -> Result<Vec<Folder>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, parent_id FROM folders ORDER BY name, id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Folder {
                id: r.get(0)?,
                name: r.get(1)?,
                parent_id: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Benennt einen Ordner um. Unbekannte ID oder leerer Name → `Error::InvalidInput`.
    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::InvalidInput(
                "The folder name must not be empty".to_string(),
            ));
        }
        let n = self.conn.execute(
            "UPDATE folders SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if n == 0 {
            return Err(Error::InvalidInput(format!("Unknown folder {}", id)));
        }
        Ok(())
    }

    /// Löscht den Ordner und alle seine Unterordner (rekursiv). Dokumente darin: bei `delete_contents` werden sie gelöscht
    /// (`delete`), sonst nur aus dem Ordner gelöst (sie liegen danach in der Wurzel). Unbekannte ID → `Error::InvalidInput`.
    pub fn delete_folder(&mut self, id: &str, delete_contents: bool) -> Result<()> {
        if !self.folder_exists(id)? {
            return Err(Error::InvalidInput(format!("Unknown folder {}", id)));
        }
        // Ordner und alle Unterordner einsammeln
        let all = self.folders()?;
        let mut ids = vec![id.to_string()];
        let mut i = 0;
        while i < ids.len() {
            let current = ids[i].clone();
            for f in &all {
                if f.parent_id == current && !ids.contains(&f.id) {
                    ids.push(f.id.clone());
                }
            }
            i += 1;
        }
        let mut docs = Vec::new();
        for fid in &ids {
            let mut stmt = self
                .conn
                .prepare("SELECT doc_id FROM doc_folder WHERE folder_id = ?1")?;
            for r in stmt.query_map(params![fid], |r| r.get::<_, String>(0))? {
                docs.push(r?);
            }
        }
        for d in &docs {
            self.conn
                .execute("DELETE FROM doc_folder WHERE doc_id = ?1", params![d])?;
            if delete_contents {
                self.delete(d)?;
            }
        }
        for fid in &ids {
            self.conn
                .execute("DELETE FROM folders WHERE id = ?1", params![fid])?;
        }
        Ok(())
    }

    /// Verschiebt ein Dokument in einen Ordner (`folder_id == ""`: in die Wurzel, der Eintrag in `doc_folder` wird entfernt).
    /// Unbekanntes Dokument oder unbekannter Ordner → `Error::InvalidInput`.
    pub fn move_document(&mut self, doc_id: &str, folder_id: &str) -> Result<()> {
        if self.load(doc_id)?.is_none() {
            return Err(Error::InvalidInput(format!(
                "Unknown document {}",
                doc_id
            )));
        }
        if folder_id.is_empty() {
            self.conn
                .execute("DELETE FROM doc_folder WHERE doc_id = ?1", params![doc_id])?;
            return Ok(());
        }
        if !self.folder_exists(folder_id)? {
            return Err(Error::InvalidInput(format!(
                "Unknown folder {}",
                folder_id
            )));
        }
        self.conn.execute(
            "INSERT INTO doc_folder(doc_id, folder_id) VALUES(?1, ?2)
             ON CONFLICT(doc_id) DO UPDATE SET folder_id = ?2",
            params![doc_id, folder_id],
        )?;
        Ok(())
    }

    /// Ordner eines Dokuments (`""` = Wurzel).
    pub fn folder_of(&self, doc_id: &str) -> Result<String> {
        Ok(self
            .conn
            .query_row(
                "SELECT folder_id FROM doc_folder WHERE doc_id = ?1",
                params![doc_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_default())
    }

    /// Dokumente eines Ordners in der Reihenfolge von `list()`. Bei `folder_id == ""` die Dokumente ohne Ordner. Einträge in `doc_folder`
    /// zu gelöschten Dokumenten werden ignoriert.
    pub fn documents_in_folder(&self, folder_id: &str) -> Result<Vec<DocumentInfo>> {
        let mapped: Vec<(String, String)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT doc_id, folder_id FROM doc_folder")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            let mut v = Vec::new();
            for r in rows {
                v.push(r?);
            }
            v
        };
        Ok(self
            .list()?
            .into_iter()
            .filter(|d| {
                let f = mapped
                    .iter()
                    .find(|(doc, _)| doc == &d.id)
                    .map(|(_, f)| f.as_str())
                    .unwrap_or("");
                f == folder_id
            })
            .collect())
    }

    /// Setzt die Notiz zu einem Dokument; ein leerer Text (nach `trim`) löscht die Notiz.
    pub fn set_note(&mut self, doc_id: &str, text: &str) -> Result<()> {
        if text.trim().is_empty() {
            self.conn
                .execute("DELETE FROM notes WHERE doc_id = ?1", params![doc_id])?;
        } else {
            self.conn.execute(
                "INSERT INTO notes(doc_id, text) VALUES(?1, ?2)
                 ON CONFLICT(doc_id) DO UPDATE SET text = ?2",
                params![doc_id, text],
            )?;
        }
        Ok(())
    }

    pub fn note(&self, doc_id: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT text FROM notes WHERE doc_id = ?1",
                params![doc_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?)
    }

    /// Markiert oder entmarkiert ein Dokument als Favorit (mehrfaches Setzen ist kein Fehler).
    pub fn set_favorite(&mut self, doc_id: &str, favorite: bool) -> Result<()> {
        if favorite {
            self.conn.execute(
                "INSERT OR IGNORE INTO favorites(doc_id) VALUES(?1)",
                params![doc_id],
            )?;
        } else {
            self.conn
                .execute("DELETE FROM favorites WHERE doc_id = ?1", params![doc_id])?;
        }
        Ok(())
    }

    /// Favoriten (nur noch vorhandene Dokumente) in der Reihenfolge von `list()`.
    pub fn favorites(&self) -> Result<Vec<DocumentInfo>> {
        let mut stmt = self.conn.prepare("SELECT doc_id FROM favorites")?;
        let mut ids = Vec::new();
        for r in stmt.query_map([], |r| r.get::<_, String>(0))? {
            ids.push(r?);
        }
        Ok(self
            .list()?
            .into_iter()
            .filter(|d| ids.contains(&d.id))
            .collect())
    }
}

impl ProjectStore {
    fn folder_exists(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM folders WHERE id = ?1", params![id], |_| {
                Ok(())
            })
            .optional()?
            .is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{Document, SequenceDocument};
    use crate::model::sequence::{Sequence, SequenceType};

    fn add(s: &mut ProjectStore, name: &str) -> String {
        let seq = Sequence::new(name, name, "ACGT", SequenceType::Dna).unwrap();
        let d = Document::Sequence(SequenceDocument::of(name, seq));
        s.save(&d).unwrap();
        d.id().to_string()
    }

    #[test]
    fn ordner_anlegen_und_umbenennen() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        let a = s.create_folder("  Test ", "").unwrap();
        assert_eq!(a.name, "Test");
        assert_eq!(a.parent_id, "");
        let b = s.create_folder("Alt", &a.id).unwrap();
        assert_eq!(b.parent_id, a.id);
        let names: Vec<String> = s.folders().unwrap().into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["Alt", "Test"]);
        s.rename_folder(&a.id, "Neu").unwrap();
        assert_eq!(
            s.folders()
                .unwrap()
                .iter()
                .find(|f| f.id == a.id)
                .unwrap()
                .name,
            "Neu"
        );
        assert!(matches!(
            s.create_folder("  ", ""),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.create_folder("x", "gibtsnicht"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.rename_folder("gibtsnicht", "y"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.rename_folder(&a.id, " "),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn dokumente_verschieben() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        let f = s.create_folder("F", "").unwrap();
        let d1 = add(&mut s, "eins");
        let d2 = add(&mut s, "zwei");
        assert_eq!(s.documents_in_folder("").unwrap().len(), 2);
        s.move_document(&d1, &f.id).unwrap();
        assert_eq!(s.folder_of(&d1).unwrap(), f.id);
        assert_eq!(s.folder_of(&d2).unwrap(), "");
        let in_f = s.documents_in_folder(&f.id).unwrap();
        assert_eq!(in_f.len(), 1);
        assert_eq!(in_f[0].name, "eins");
        assert_eq!(s.documents_in_folder("").unwrap().len(), 1);
        s.move_document(&d1, "").unwrap();
        assert_eq!(s.folder_of(&d1).unwrap(), "");
        assert!(matches!(
            s.move_document("nix", &f.id),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            s.move_document(&d1, "nix"),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn ordner_loeschen() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        let a = s.create_folder("A", "").unwrap();
        let b = s.create_folder("B", &a.id).unwrap();
        let d1 = add(&mut s, "eins");
        let d2 = add(&mut s, "zwei");
        s.move_document(&d1, &a.id).unwrap();
        s.move_document(&d2, &b.id).unwrap();
        s.delete_folder(&a.id, false).unwrap();
        assert!(s.folders().unwrap().is_empty());
        assert_eq!(s.documents_in_folder("").unwrap().len(), 2);
        assert_eq!(s.folder_of(&d2).unwrap(), "");
        // mit Inhalt löschen
        let c = s.create_folder("C", "").unwrap();
        s.move_document(&d1, &c.id).unwrap();
        s.delete_folder(&c.id, true).unwrap();
        assert!(s.load(&d1).unwrap().is_none());
        assert!(s.load(&d2).unwrap().is_some());
        assert!(matches!(
            s.delete_folder("nix", false),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn notizen_und_favoriten() {
        let mut s = ProjectStore::open_in_memory().unwrap();
        let d1 = add(&mut s, "eins");
        let d2 = add(&mut s, "zwei");
        assert_eq!(s.note(&d1).unwrap(), None);
        s.set_note(&d1, "Wichtig: Klon 3").unwrap();
        assert_eq!(s.note(&d1).unwrap().as_deref(), Some("Wichtig: Klon 3"));
        s.set_note(&d1, "Neu").unwrap();
        assert_eq!(s.note(&d1).unwrap().as_deref(), Some("Neu"));
        s.set_note(&d1, "   ").unwrap();
        assert_eq!(s.note(&d1).unwrap(), None);
        s.set_favorite(&d2, true).unwrap();
        s.set_favorite(&d2, true).unwrap();
        let fav = s.favorites().unwrap();
        assert_eq!(fav.len(), 1);
        assert_eq!(fav[0].id, d2);
        s.delete(&d2).unwrap();
        assert!(s.favorites().unwrap().is_empty());
        s.set_favorite(&d1, true).unwrap();
        s.set_favorite(&d1, false).unwrap();
        assert!(s.favorites().unwrap().is_empty());
    }
}
