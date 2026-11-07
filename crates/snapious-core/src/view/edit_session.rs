//! Editing session with undo/redo for a sequence document.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::Feature;

/// Maximum number of undo steps.
pub const MAX_UNDO: usize = 200;

/// Holds the current document plus the undo and redo stacks.
/// Every change produces a new document via the immutable methods of `SequenceDocument`.
#[derive(Clone, Debug)]
pub struct EditSession {
    current: SequenceDocument,
    saved: SequenceDocument,
    undo: Vec<SequenceDocument>,
    redo: Vec<SequenceDocument>,
}

impl EditSession {
    /// New session; the document counts as saved (`is_modified() == false`).
    pub fn new(doc: SequenceDocument) -> Self {
        EditSession {
            current: doc.clone(),
            saved: doc,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The current document.
    pub fn document(&self) -> &SequenceDocument {
        &self.current
    }

    /// Sets `doc` as the new state: old state goes onto the undo stack, redo stack is cleared.
    /// If the undo stack then has more than `MAX_UNDO` entries, the oldest is dropped.
    /// If `doc` equals the current document, nothing happens.
    pub fn apply(&mut self, doc: SequenceDocument) {
        if self.current == doc {
            return;
        }
        self.undo.push(self.current.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.current = doc;
    }

    /// Inserts bases (`SequenceDocument::insert`). On error the session stays unchanged.
    pub fn insert(&mut self, position: usize, bases: &str) -> Result<()> {
        let new_doc = self.current.insert(position, bases)?;
        self.apply(new_doc);
        Ok(())
    }

    /// Deletes `[start, end)` (`SequenceDocument::delete`). On error the session stays unchanged.
    pub fn delete(&mut self, start: usize, end: usize) -> Result<()> {
        let new_doc = self.current.delete(start, end)?;
        self.apply(new_doc);
        Ok(())
    }

    /// Replaces `[start, end)` with `bases` (`SequenceDocument::replace`) as a single step.
    pub fn replace(&mut self, start: usize, end: usize, bases: &str) -> Result<()> {
        let new_doc = self.current.replace(start, end, bases)?;
        self.apply(new_doc);
        Ok(())
    }

    /// Appends a feature to the end of the feature list. If it extends beyond the end of the document
    /// (`location().end() > len()`), returns `Error::OutOfRange` and makes no change.
    pub fn add_feature(&mut self, feature: Feature) -> Result<()> {
        let mut features = self.current.features().to_vec();
        if feature.location().end() > self.current.len() {
            return Err(Error::OutOfRange(format!(
                "Feature end {} lies beyond the document length {}",
                feature.location().end(),
                self.current.len()
            )));
        }
        features.push(feature);
        let new_doc = self.current.with_features(features);
        self.apply(new_doc);
        Ok(())
    }

    /// Removes the feature with this index. Invalid index: `Error::OutOfRange`, no change.
    pub fn remove_feature(&mut self, index: usize) -> Result<()> {
        let features = self.current.features();
        if index >= features.len() {
            return Err(Error::OutOfRange(format!(
                "Invalid feature index {} (max {})",
                index,
                features.len().saturating_sub(1)
            )));
        }
        let mut new_features: Vec<Feature> = features.to_vec();
        new_features.remove(index);
        let new_doc = self.current.with_features(new_features);
        self.apply(new_doc);
        Ok(())
    }

    /// Changes the topology. If it is already set, nothing happens (no undo step).
    pub fn set_topology(&mut self, topology: Topology) {
        if self.current.topology() != topology {
            let new_doc = self.current.with_topology(topology);
            self.apply(new_doc);
        }
    }

    /// Renames the document (`with_name`); same name: nothing.
    pub fn rename(&mut self, name: &str) {
        if self.current.name() != name {
            let new_doc = self.current.with_name(name);
            self.apply(new_doc);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of possible undo steps.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// Undoes the last step; `false` if there is nothing to do.
    pub fn undo(&mut self) -> bool {
        let prev = match self.undo.pop() {
            Some(doc) => doc,
            None => return false,
        };
        self.redo.push(self.current.clone());
        self.current = prev;
        true
    }

    /// Restores the most recently undone step; `false` if there is nothing to do.
    pub fn redo(&mut self) -> bool {
        let next = match self.redo.pop() {
            Some(doc) => doc,
            None => return false,
        };
        self.undo.push(self.current.clone());
        self.current = next;
        true
    }

    /// `true` if the current document differs from the last saved one (compared with `==`).
    /// After undoing back to the saved state it is `false` again.
    pub fn is_modified(&self) -> bool {
        self.current != self.saved
    }

    /// Marks the current state as saved.
    pub fn mark_saved(&mut self) {
        self.saved = self.current.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Interval, Location, Strand};
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(bases: &str) -> SequenceDocument {
        let seq = Sequence::new("s", "s", bases, SequenceType::Dna).unwrap();
        SequenceDocument::of("s", seq)
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
    fn neue_sitzung() {
        let s = EditSession::new(doc("ATGC"));
        assert_eq!(s.document().bases(), "ATGC");
        assert!(!s.can_undo() && !s.can_redo() && !s.is_modified());
        assert_eq!(s.undo_depth(), 0);
    }

    #[test]
    fn einfuegen_undo_redo() {
        let mut s = EditSession::new(doc("ATGC"));
        s.insert(2, "TT").unwrap();
        assert_eq!(s.document().bases(), "ATTTGC");
        assert!(s.is_modified() && s.can_undo() && !s.can_redo());
        assert!(s.undo());
        assert_eq!(s.document().bases(), "ATGC");
        assert!(!s.is_modified() && !s.can_undo() && s.can_redo());
        assert!(s.redo());
        assert_eq!(s.document().bases(), "ATTTGC");
        assert!(!s.redo());
    }

    #[test]
    fn loeschen_und_ersetzen() {
        let mut s = EditSession::new(doc("AAAACCCC"));
        s.delete(0, 2).unwrap();
        assert_eq!(s.document().bases(), "AACCCC");
        s.replace(0, 2, "GGG").unwrap();
        assert_eq!(s.document().bases(), "GGGCCCC");
        assert_eq!(s.undo_depth(), 2);
        s.undo();
        assert_eq!(s.document().bases(), "AACCCC");
    }

    #[test]
    fn fehler_aendern_nichts() {
        let mut s = EditSession::new(doc("ATGC"));
        assert!(s.insert(99, "A").is_err());
        assert!(s.delete(3, 99).is_err());
        assert!(s.replace(5, 6, "A").is_err());
        assert!(!s.can_undo() && !s.is_modified());
        assert_eq!(s.document().bases(), "ATGC");
    }

    #[test]
    fn neue_aenderung_leert_redo() {
        let mut s = EditSession::new(doc("ATGC"));
        s.insert(0, "A").unwrap();
        s.undo();
        assert!(s.can_redo());
        s.insert(0, "G").unwrap();
        assert!(!s.can_redo());
        assert_eq!(s.document().bases(), "GATGC");
    }

    #[test]
    fn features_hinzufuegen_und_entfernen() {
        let mut s = EditSession::new(doc("ATGCATGC"));
        s.add_feature(feat(0, 4)).unwrap();
        s.add_feature(feat(4, 8)).unwrap();
        assert_eq!(s.document().features().len(), 2);
        assert!(matches!(
            s.add_feature(feat(4, 9)),
            Err(Error::OutOfRange(_))
        ));
        assert_eq!(s.document().features().len(), 2);
        s.remove_feature(0).unwrap();
        assert_eq!(s.document().features().len(), 1);
        assert_eq!(s.document().features()[0].location().start(), 4);
        assert!(matches!(s.remove_feature(5), Err(Error::OutOfRange(_))));
        s.undo();
        assert_eq!(s.document().features().len(), 2);
    }

    #[test]
    fn topologie_und_name() {
        let mut s = EditSession::new(doc("ATGC"));
        s.set_topology(Topology::Linear);
        assert!(!s.can_undo());
        s.set_topology(Topology::Circular);
        assert_eq!(s.document().topology(), Topology::Circular);
        assert!(s.is_modified());
        s.rename("neu");
        assert_eq!(s.document().name(), "neu");
        s.rename("neu");
        assert_eq!(s.undo_depth(), 2);
        s.undo();
        s.undo();
        assert_eq!(s.document().topology(), Topology::Linear);
        assert!(!s.is_modified());
    }

    #[test]
    fn speichern_markiert() {
        let mut s = EditSession::new(doc("ATGC"));
        s.insert(0, "A").unwrap();
        assert!(s.is_modified());
        s.mark_saved();
        assert!(!s.is_modified());
        s.undo();
        assert!(s.is_modified());
        s.redo();
        assert!(!s.is_modified());
    }

    #[test]
    fn undo_grenze() {
        let mut s = EditSession::new(doc("A"));
        for _ in 0..(MAX_UNDO + 20) {
            s.insert(0, "A").unwrap();
        }
        assert_eq!(s.undo_depth(), MAX_UNDO);
        let mut n = 0;
        while s.undo() {
            n += 1;
        }
        assert_eq!(n, MAX_UNDO);
        assert_eq!(s.document().len(), 1 + 20);
    }

    #[test]
    fn apply_gleiches_dokument_ist_nichts() {
        let mut s = EditSession::new(doc("ATGC"));
        let d = s.document().clone();
        s.apply(d);
        assert!(!s.can_undo());
    }
}
