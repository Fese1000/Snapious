# Karte 1.2: Auswahlmodell (Selection)

Datei: `crates/snapious-core/src/view/selection.rs` (Gerüst und Tests vorhanden)
Voraussetzung: keine (nur `error.rs`)
Ziel: Eine textcursor-artige Auswahl auf einer Sequenz verwalten: Cursorposition (caret), Anker, Bereichsberechnung, Verkleinern auf Sequenzgrenzen.

Funktionen:
  `Selection::none() -> Self`                              // anchor = caret = -1 (oder ein spezieller „leer"-Zustand)
  `Selection::caret_at(position: usize) -> Self`            // anchor = caret = position
  `Selection::range(start: usize, end: usize) -> Self`     // anchor = min, caret = max (Reihenfolge der Argumente egal)
  `is_empty(&self) -> bool`                                // anchor == caret oder initial leer
  `start(&self) -> usize`                                  // min(anchor, caret)
  `end(&self) -> usize`                                    // max(anchor, caret)
  `length(&self) -> usize`                                 // end - start
  `extend_to(&self, position: usize) -> Self`              // anchor bleibt, caret wird position (neues Selection-Objekt)
  `clamp(&self, sequence_length: usize) -> Self`           // anchor und caret auf [0, sequence_length] beschränkt

Randfälle: `range(5, 2)` → start=2, end=5 (Argumente können vertauscht sein); `clamp(10)` auf Bereich [8, 15] → [8, 10]; leere Selection (`none()`) hat `length == 0`; `extend_to` ändert nicht das Original (immutabel).

Fertig, wenn: `cargo test -p snapious-core selection` grün.
Nicht tun: Tests ändern; andere Dateien ändern; `unwrap()`.
