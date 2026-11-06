GENEIOUS-FUNKTIONEN:
- Zeigt Domänen-Sequenzen grafisch an
- Ermöglicht Drag-Selektion von Sequenzbereichen
- Markiert ausgewählte Bereiche mit grauem Rechteck
- Zoomt auf Details der Domänen-Sequenz
- Extrahiert Domänen in separate Dokumente
- Findet ähnliche Sequenzen in UniProt
- Verwaltet Domain-, Clan- und Sequenzdokumente
- Bietet spezialisierte Viewer für Domänen-Ansichten

ABGLEICH:
- Zeigt Domänen-Sequenzen grafisch an | Kern: nein | App: nein | Keine DOMänen-spezifische Grafik; `view/map_svg.rs` und `view/feature_layout.rs` zeichnen nur Nukleotid-/Protein-Features, keine Protein-Domänen-Hierarchie
- Ermöglicht Drag-Selektion von Sequenzbereichen | Kern: ja | App: ja | `view/selection.rs` (Auswahl zwischen Basen), jedoch keine DOMÄNEN-spezifische Drag-Logik, nur generische Sequenzauswahl
- Markiert ausgewählte Bereiche mit grauem Rechteck | Kern: ja | App: ja | `view/selection.rs` und `view/edit_session.rs` (visuelle Markierung von Selektionen), aber nicht spezifisch für "Domänen" als Objekttyp
- Zoomt auf Details der Domänen-Sequenz | Kern: ja | App: teilweise | `view/map_layout.rs` und `view/seq_layout.rs` ermöglichen Zoom auf Sequenzabschnitte, aber keine DOMÄNEN-spezifische "Zoom-to-Domain"-Funktion
- Extrahiert Domänen in separate Dokumente | Kern: ja | App: teilweise | `ops/builtin.rs` (edit_replace/edit_delete) und `model/document.rs` können Teile extrahieren, aber keine automatische "Domäne zu Dokument"-Konvertierung wie im Plugin
- Findet ähnliche Sequenzen in UniProt | Kern: ja | App: ja | `db/uniprot.rs` (Suche und FASTA-Abruf), `ops/network.rs` (Datensatz laden), aber keine DOMÄNEN-spezifische Ähnlichkeitssuche (nur Sequenz-ähnlichkeit)
- Verwaltet Domain-, Clan- und Sequenzdokumente | Kern: ja | App: nein | `store/project_store.rs` verwaltet generische Dokumente, aber keine spezifische Hierarchie oder Typisierung für "Domain/Clan" wie im Plugin
- Bietet spezialisierte Viewer für Domänen-Ansichten | Kern: ja | App: nein | `view/map_svg.rs` und `view/tree_layout.rs` bieten Standard-Ansichten, aber keine spezialisierte "Domänen-Viewer"-Klasse
