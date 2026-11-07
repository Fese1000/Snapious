//! Undo/redo for document states (limited depth).

/// Two stacks: states to go back to and states to redo.
#[derive(Clone, Debug)]
pub struct UndoStack<T: Clone> {
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T: Clone> UndoStack<T> {
    /// Empty stack; `limit` is the largest number of stored undo steps (at least 1: `limit.max(1)`).
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Remembers `state` (the state BEFORE a change) for undo, clears the redo stack and drops the oldest entry when
    /// the limit is exceeded.
    pub fn push(&mut self, state: T) {
        self.redo.clear();
        self.undo.push(state);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// Goes back one step: takes the top undo state and returns it; `current` (the state now)
    /// goes onto the redo stack. If there is nothing to go back to → `None`, nothing changes.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    /// Like `undo`, but reversed: takes the top redo state, `current` goes onto the undo stack
    /// (without violating the limit: the oldest entry is dropped). Nothing to redo → `None`.
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

    /// Number of possible undo steps.
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
        s.push(3); // 1 is dropped, undo holds [2, 3]
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
        assert_eq!(z.undo_len(), 1); // limit is at least 1
    }
}
