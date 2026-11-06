//! Rückgängig/Wiederholen für Dokumentzustände (begrenzte Tiefe).

/// Zwei Stapel: Zustände zum Zurückgehen und zum Wiederholen.
#[derive(Clone, Debug)]
pub struct UndoStack<T: Clone> {
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T: Clone> UndoStack<T> {
    /// Leerer Stapel; `limit` ist die größte Zahl gespeicherter Rückgängig-Schritte (mindestens 1: `limit.max(1)`).
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Merkt `state` (den Zustand VOR einer Änderung) für Rückgängig, leert den Wiederholen-Stapel und wirft bei
    /// Überschreiten des Limits den ältesten Eintrag weg.
    pub fn push(&mut self, state: T) {
        self.redo.clear();
        self.undo.push(state);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// Geht einen Schritt zurück: nimmt den obersten Rückgängig-Zustand und gibt ihn zurück; `current` (der Zustand jetzt)
    /// kommt auf den Wiederholen-Stapel. Ist nichts zum Zurückgehen da → `None`, nichts ändert sich.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    /// Wie `undo`, aber umgekehrt: nimmt den obersten Wiederholen-Zustand, `current` kommt auf den Rückgängig-Stapel
    /// (ohne das Limit zu verletzen: ältester Eintrag fällt weg). Nichts zum Wiederholen → `None`.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Zahl der möglichen Rückgängig-Schritte.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_und_wege() {
        let mut s = UndoStack::new(2);
        assert!(!s.can_undo() && !s.can_redo());
        s.push(1);
        s.push(2);
        s.push(3); // 1 fällt weg, Rückgängig hält [2, 3]
        assert_eq!(s.undo_len(), 2);
        assert_eq!(s.undo(4), Some(3));
        assert!(s.can_redo());
        assert_eq!(s.undo(3), Some(2));
        assert_eq!(s.undo(2), None);
        assert_eq!(s.undo_len(), 0);
        assert_eq!(s.redo(2), Some(3));
        assert_eq!(s.redo(3), Some(4));
        assert_eq!(s.redo(4), None);
        assert_eq!(s.undo_len(), 2);
    }

    #[test]
    fn push_leert_wiederholen() {
        let mut s = UndoStack::new(5);
        s.push("a");
        assert_eq!(s.undo("b"), Some("a"));
        assert!(s.can_redo());
        s.push("c");
        assert!(!s.can_redo());
        assert_eq!(UndoStack::<i32>::new(0).undo_len(), 0);
        let mut z = UndoStack::new(0);
        z.push(1);
        z.push(2);
        assert_eq!(z.undo_len(), 1); // Limit mindestens 1
    }
}
