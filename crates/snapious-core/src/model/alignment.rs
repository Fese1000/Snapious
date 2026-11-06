//! Alignment-Dokument: gleich lange Zeilen mit Namen.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::sequence::SequenceType;
use crate::util::{new_id, now_secs};

/// Ein Mehrfachalignment. Lücken sind `-` (auch `.` wird als Lücke gezählt).
#[derive(Clone, Debug, PartialEq)]
pub struct AlignmentDocument {
    id: String,
    name: String,
    created: i64,
    seq_type: SequenceType,
    row_names: Vec<String>,
    rows: Vec<String>,
    metadata: BTreeMap<String, String>,
}

fn is_gap(c: char) -> bool {
    c == '-' || c == '.'
}

impl AlignmentDocument {
    /// Prüft: gleich viele Namen wie Zeilen, mindestens eine Zeile, alle Zeilen gleich lang (in Zeichen), keine Leerzeichen in den Zeilen.
    /// Fehler `Error::InvalidInput`.
    pub fn new(
        id: &str,
        name: &str,
        seq_type: SequenceType,
        row_names: Vec<String>,
        rows: Vec<String>,
        metadata: BTreeMap<String, String>,
        created: i64,
    ) -> Result<Self> {
        if rows.is_empty() {
            return Err(Error::InvalidInput(
                "Ein Alignment braucht mindestens eine Zeile".to_string(),
            ));
        }
        if row_names.len() != rows.len() {
            return Err(Error::InvalidInput(format!(
                "{} Namen für {} Zeilen",
                row_names.len(),
                rows.len()
            )));
        }
        let width = rows.first().map_or(0, |r| r.chars().count());
        for (n, r) in row_names.iter().zip(&rows) {
            if r.chars().count() != width {
                return Err(Error::InvalidInput(format!(
                    "Zeile '{}' hat {} statt {} Spalten",
                    n,
                    r.chars().count(),
                    width
                )));
            }
            if r.chars().any(char::is_whitespace) {
                return Err(Error::InvalidInput(format!(
                    "Zeile '{}' enthält Leerraum",
                    n
                )));
            }
        }
        Ok(Self {
            id: id.to_string(),
            name: name.to_string(),
            created,
            seq_type,
            row_names,
            rows,
            metadata,
        })
    }

    /// Neues Dokument mit neuer ID und aktueller Zeit.
    pub fn of(
        name: &str,
        seq_type: SequenceType,
        row_names: Vec<String>,
        rows: Vec<String>,
    ) -> Result<Self> {
        Self::new(
            &new_id(),
            name,
            seq_type,
            row_names,
            rows,
            BTreeMap::new(),
            now_secs(),
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn created(&self) -> i64 {
        self.created
    }

    pub fn seq_type(&self) -> SequenceType {
        self.seq_type
    }

    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }

    pub fn row_names(&self) -> &[String] {
        &self.row_names
    }

    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.rows.first().map_or(0, |r| r.chars().count())
    }

    pub fn row(&self, i: usize) -> Option<&str> {
        self.rows.get(i).map(String::as_str)
    }

    pub fn row_name(&self, i: usize) -> Option<&str> {
        self.row_names.get(i).map(String::as_str)
    }

    /// Zeile ohne Lücken.
    pub fn ungapped(&self, i: usize) -> Option<String> {
        self.rows
            .get(i)
            .map(|r| r.chars().filter(|c| !is_gap(*c)).collect())
    }

    /// Zeichen in Zeile `row`, Spalte `column`.
    pub fn at(&self, row: usize, column: usize) -> Option<char> {
        self.rows.get(row)?.chars().nth(column)
    }

    pub fn with_name(&self, name: &str) -> Self {
        let mut c = self.clone();
        c.name = name.to_string();
        c
    }

    fn derive(&self, row_names: Vec<String>, rows: Vec<String>) -> Result<Self> {
        Self::new(
            &self.id,
            &self.name,
            self.seq_type,
            row_names,
            rows,
            self.metadata.clone(),
            self.created,
        )
    }

    /// Neue Zeilen und Namen (gleiche ID, Typ und Metadaten).
    pub fn with_rows(&self, row_names: Vec<String>, rows: Vec<String>) -> Result<Self> {
        self.derive(row_names, rows)
    }

    /// Entfernt Zeilen; ungültige Indizes → `Error::OutOfRange`; alle Zeilen entfernen → `Error::InvalidInput`.
    pub fn remove_rows(&self, indices: &[usize]) -> Result<Self> {
        if let Some(bad) = indices.iter().find(|i| **i >= self.row_count()) {
            return Err(Error::OutOfRange(format!("Zeile {} gibt es nicht", bad)));
        }
        let keep = |i: &usize| !indices.contains(i);
        let names = (0..self.row_count())
            .filter(keep)
            .filter_map(|i| self.row_names.get(i).cloned())
            .collect();
        let rows = (0..self.row_count())
            .filter(keep)
            .filter_map(|i| self.rows.get(i).cloned())
            .collect();
        self.derive(names, rows)
    }

    /// Entfernt Spalten; ungültige Indizes → `Error::OutOfRange`.
    pub fn remove_columns(&self, columns: &[usize]) -> Result<Self> {
        if let Some(bad) = columns.iter().find(|c| **c >= self.column_count()) {
            return Err(Error::OutOfRange(format!("Spalte {} gibt es nicht", bad)));
        }
        let rows = self
            .rows
            .iter()
            .map(|r| {
                r.chars()
                    .enumerate()
                    .filter(|(i, _)| !columns.contains(i))
                    .map(|(_, c)| c)
                    .collect()
            })
            .collect();
        self.derive(self.row_names.clone(), rows)
    }

    /// Spalten `[from, to)`; `from > to` oder `to` hinter dem Ende → `Error::OutOfRange`.
    pub fn sub_alignment(&self, from: usize, to: usize) -> Result<Self> {
        if from > to || to > self.column_count() {
            return Err(Error::OutOfRange(format!(
                "Spaltenbereich {}-{} bei {} Spalten",
                from,
                to,
                self.column_count()
            )));
        }
        let rows = self
            .rows
            .iter()
            .map(|r| r.chars().skip(from).take(to - from).collect())
            .collect();
        self.derive(self.row_names.clone(), rows)
    }

    /// Ordnet die Zeilen um: `order` muss eine Permutation von `0..row_count` sein, sonst `Error::InvalidInput`.
    pub fn reorder(&self, order: &[usize]) -> Result<Self> {
        let mut sorted = order.to_vec();
        sorted.sort_unstable();
        if sorted != (0..self.row_count()).collect::<Vec<_>>() {
            return Err(Error::InvalidInput(
                "Die Reihenfolge ist keine Permutation der Zeilen".to_string(),
            ));
        }
        let names = order
            .iter()
            .filter_map(|i| self.row_names.get(*i).cloned())
            .collect();
        let rows = order
            .iter()
            .filter_map(|i| self.rows.get(*i).cloned())
            .collect();
        self.derive(names, rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aln(rows: &[&str]) -> AlignmentDocument {
        let names = (0..rows.len()).map(|i| format!("s{}", i + 1)).collect();
        AlignmentDocument::of(
            "aln",
            SequenceType::Dna,
            names,
            rows.iter().map(|r| r.to_string()).collect(),
        )
        .unwrap()
    }

    #[test]
    fn pruefung() {
        let n = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(
            AlignmentDocument::of("x", SequenceType::Dna, n(&["a", "b"]), n(&["AC", "ACG"]))
                .is_err()
        );
        assert!(
            AlignmentDocument::of("x", SequenceType::Dna, n(&["a"]), n(&["AC", "AC"])).is_err()
        );
        assert!(AlignmentDocument::of("x", SequenceType::Dna, vec![], vec![]).is_err());
        assert!(AlignmentDocument::of("x", SequenceType::Dna, n(&["a"]), n(&["A C"])).is_err());
    }

    #[test]
    fn zugriff() {
        let a = aln(&["AC--GT", "AG--GT"]);
        assert_eq!((a.row_count(), a.column_count()), (2, 6));
        assert_eq!(a.row(0), Some("AC--GT"));
        assert_eq!(a.row_name(1), Some("s2"));
        assert_eq!(a.ungapped(0).unwrap(), "ACGT");
        assert_eq!(a.at(1, 1), Some('G'));
        assert_eq!(a.at(1, 9), None);
        assert_eq!(a.row(5), None);
    }

    #[test]
    fn umbauen() {
        let a = aln(&["AC--GT", "AG--GT"]);
        let b = a.remove_columns(&[2, 3]).unwrap();
        assert_eq!(b.rows(), &["ACGT".to_string(), "AGGT".to_string()]);
        assert_eq!(b.id(), a.id());
        assert!(a.remove_columns(&[6]).is_err());
        let c = a.sub_alignment(1, 3).unwrap();
        assert_eq!(c.rows(), &["C-".to_string(), "G-".to_string()]);
        assert!(a.sub_alignment(3, 2).is_err() && a.sub_alignment(0, 7).is_err());
        let d = a.remove_rows(&[0]).unwrap();
        assert_eq!(d.row_count(), 1);
        assert_eq!(d.row_name(0), Some("s2"));
        assert!(a.remove_rows(&[0, 1]).is_err());
        assert!(matches!(a.remove_rows(&[2]), Err(Error::OutOfRange(_))));
        let e = a.reorder(&[1, 0]).unwrap();
        assert_eq!(e.row(0), Some("AG--GT"));
        assert_eq!(e.row_name(0), Some("s2"));
        assert!(a.reorder(&[0, 0]).is_err() && a.reorder(&[0]).is_err());
        assert_eq!(a.with_name("neu").name(), "neu");
    }
}
