# Card 1.2: Selection model

File: `crates/snapious-core/src/view/selection.rs` (skeleton and tests exist)
Prerequisite: none (only `error.rs`)
Goal: Manage a text-cursor-like selection on a sequence: caret position, anchor, range computation, clamping to sequence bounds.

Functions:
  `Selection::none() -> Self`                              // anchor = caret = -1 (or a special "empty" state)
  `Selection::caret_at(position: usize) -> Self`            // anchor = caret = position
  `Selection::range(start: usize, end: usize) -> Self`     // anchor = min, caret = max (argument order does not matter)
  `is_empty(&self) -> bool`                                // anchor == caret or initially empty
  `start(&self) -> usize`                                  // min(anchor, caret)
  `end(&self) -> usize`                                    // max(anchor, caret)
  `length(&self) -> usize`                                 // end - start
  `extend_to(&self, position: usize) -> Self`              // anchor stays, caret becomes position (new Selection object)
  `clamp(&self, sequence_length: usize) -> Self`           // anchor and caret limited to [0, sequence_length]

Edge cases: `range(5, 2)` → start=2, end=5 (arguments may be swapped); `clamp(10)` on range [8, 15] → [8, 10]; an empty selection (`none()`) has `length == 0`; `extend_to` does not modify the original (immutable).

Done when: `cargo test -p snapious-core selection` is green.
Do not: change tests; change other files; use `unwrap()`.
