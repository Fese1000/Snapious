GENEIOUS-FUNKTIONEN:
- Umbenennung von Sequenznamen basierend auf Feldinhalten
- Ersetzung ganzer Felder durch neue Inhalte
- Teilweise Ersetzung mittels regulärer Ausdrücke (Regex)
- Einfügen von Feldwerten in neue Namen (z.B. Organismus)
- Extraktion von Zeichen oder Teilstrings aus bestehenden Feldern
- Nutzung von Annotationseigenschaften für Namenskonstruktion
- Zählung und Duplikatserkennung bei der Umbenennung
- Batch-Kopie von Annotationsnamen oder -eigenschaften
- Umbenennung von Sequenzen in Listen und Alignments
- Verfolgung von Links zu referenzierten Sequenzalignments
- Umbenennung von Nukleotid- und Aminosäuresequenzen

ABGLEICH:
- Umbenennung von Sequenznamen basierend auf Feldinhalten | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Ersetzung ganzer Felder durch neue Inhalte | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Teilweise Ersetzung mittels regulärer Ausdrücke (Regex) | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Einfügen von Feldwerten in neue Namen (z.B. Organismus) | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Extraktion von Zeichen oder Teilstrings aus bestehenden Feldern | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Nutzung von Annotationseigenschaften für Namenskonstruktion | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Zählung und Duplikatserkennung bei der Umbenennung | Kern: ja | App: ja | `find_duplicates` (Datei: `project/search.rs`)
- Batch-Kopie von Annotationsnamen oder -eigenschaften | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Umbenennung von Sequenzen in Listen und Alignments | Kern: ja | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Verfolgung von Links zu referenzierten Sequenzalignments | Kern: nein | App: ja | `batch_rename` (Datei: `project/search.rs`)
- Umbenennung von Nukleotid- und Aminosäuresequenzen | Kern: ja | App: ja | `batch_rename` (Datei: `project/search.rs`)
