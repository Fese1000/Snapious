//! Selection in the sequence view: positions lie BETWEEN the bases (0 to length).

/// Selection with anchor point and caret. `none()` means: nothing selected, no caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    anchor: Option<usize>,
    caret: Option<usize>,
}

impl Selection {
    pub fn none() -> Self {
        Selection {
            anchor: None,
            caret: None,
        }
    }

    /// Just a caret at `position` (empty selection).
    pub fn caret_at(position: usize) -> Self {
        Selection {
            anchor: Some(position),
            caret: Some(position),
        }
    }

    /// Range from `anchor` to `caret` (in either order).
    pub fn range(anchor: usize, caret: usize) -> Self {
        Selection {
            anchor: Some(anchor),
            caret: Some(caret),
        }
    }

    pub fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    pub fn caret(&self) -> Option<usize> {
        self.caret
    }

    /// Nothing selected and no caret.
    pub fn is_none(&self) -> bool {
        self.anchor.is_none() && self.caret.is_none()
    }

    /// No bases selected (`none()` or anchor equals caret).
    pub fn is_empty(&self) -> bool {
        self.is_none() || self.anchor == self.caret
    }

    /// Smaller of the two positions (0 for `none()`).
    pub fn start(&self) -> usize {
        self.anchor
            .unwrap_or_default()
            .min(self.caret.unwrap_or_default())
    }

    /// Larger of the two positions (0 for `none()`).
    pub fn end(&self) -> usize {
        self.anchor
            .unwrap_or_default()
            .max(self.caret.unwrap_or_default())
    }

    /// Number of selected bases (`end - start`).
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    /// Is the base with index `position` in the selection (`start <= position < end`)?
    pub fn contains(&self, position: usize) -> bool {
        let s = self.start();
        let e = self.end();
        s <= position && position < e
    }

    /// Anchor stays, the caret moves to `position`. For `none()` this becomes a caret at `position`.
    pub fn extend_to(&self, position: usize) -> Selection {
        if self.is_none() {
            return Selection::caret_at(position);
        }
        Selection {
            anchor: self.anchor,
            caret: Some(position),
        }
    }

    /// Both positions at most `sequence_length`.
    pub fn clamp(&self, sequence_length: usize) -> Selection {
        if self.is_none() {
            return Self::none();
        }
        Selection {
            anchor: self.anchor.map(|a| a.min(sequence_length)),
            caret: self.caret.map(|c| c.min(sequence_length)),
        }
    }

    /// Move caret by `delta`, clamped to `0..=sequence_length` (for `none()` the caret starts at 0).
    /// With `extend = true` the anchor stays, otherwise a plain caret results at the new position.
    pub fn move_caret(&self, delta: isize, sequence_length: usize, extend: bool) -> Selection {
        let current_pos = self.caret.unwrap_or_default();
        let new_pos = (current_pos as isize + delta)
            .max(0)
            .min(sequence_length as isize);
        let new_pos = (new_pos as usize).min(sequence_length);
        if extend {
            self.extend_to(new_pos)
        } else {
            Selection::caret_at(new_pos)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bereich_in_beliebiger_reihenfolge() {
        let s = Selection::range(5, 2);
        assert_eq!((s.start(), s.end(), s.len()), (2, 5, 3));
        assert_eq!(s.anchor(), Some(5));
        assert_eq!(s.caret(), Some(2));
        assert!(!s.is_empty() && !s.is_none());
    }

    #[test]
    fn none_und_cursor() {
        let n = Selection::none();
        assert!(n.is_none() && n.is_empty());
        assert_eq!((n.start(), n.end(), n.len()), (0, 0, 0));
        assert_eq!(n.anchor(), None);
        let c = Selection::caret_at(4);
        assert!(!c.is_none() && c.is_empty());
        assert_eq!((c.start(), c.end()), (4, 4));
        assert_eq!(c.caret(), Some(4));
    }

    #[test]
    fn enthaelt_prueft_basen_index() {
        let s = Selection::range(2, 5);
        assert!(!s.contains(1));
        assert!(s.contains(2));
        assert!(s.contains(4));
        assert!(!s.contains(5));
        assert!(!Selection::none().contains(0));
        assert!(!Selection::caret_at(3).contains(3));
    }

    #[test]
    fn erweitern() {
        let s = Selection::caret_at(3).extend_to(9);
        assert_eq!((s.start(), s.end()), (3, 9));
        assert_eq!(s.anchor(), Some(3));
        let rueck = s.extend_to(1);
        assert_eq!((rueck.start(), rueck.end()), (1, 3));
        assert_eq!(Selection::none().extend_to(7), Selection::caret_at(7));
    }

    #[test]
    fn begrenzen() {
        let s = Selection::range(2, 10).clamp(4);
        assert_eq!((s.start(), s.end()), (2, 4));
        assert_eq!(Selection::none().clamp(4), Selection::none());
        assert_eq!(Selection::range(8, 9).clamp(4), Selection::range(4, 4));
    }

    #[test]
    fn cursor_bewegen() {
        let s = Selection::caret_at(3);
        assert_eq!(s.move_caret(2, 10, false), Selection::caret_at(5));
        assert_eq!(s.move_caret(-5, 10, false), Selection::caret_at(0));
        assert_eq!(s.move_caret(100, 10, false), Selection::caret_at(10));
        let e = s.move_caret(2, 10, true);
        assert_eq!((e.start(), e.end()), (3, 5));
        assert_eq!(e.anchor(), Some(3));
        // without Shift the selection collapses to the caret
        assert_eq!(
            Selection::range(2, 6).move_caret(1, 10, false),
            Selection::caret_at(7)
        );
        assert_eq!(
            Selection::none().move_caret(3, 10, false),
            Selection::caret_at(3)
        );
        assert_eq!(
            Selection::none().move_caret(-3, 10, false),
            Selection::caret_at(0)
        );
    }
}
