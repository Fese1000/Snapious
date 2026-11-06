GENEIOUS-FUNKTIONEN:
- Erstellen einzelner Sequenzgraphen-Fabriken
- Bereitstellen von Hydrophobizitätsgraphen pro Rest
- Bereitstellen von PI-Graphen pro Rest
- Berechnung der Hydrophobizität an jedem Rest
- Berechnung des isoelektrischen Punkts (PI) pro Rest
- Durchschnittliche Hydrophobizität über alle Sequenzen
- Durchschnittlicher isoelektrischer Punkt (PI) über alle Sequenzen
- Generierung von Proteingraphen

ABGLEICH:
- Erstellen einzelner Sequenzgraphen-Fabriken | Kern: nein | App: nein | Keine Graphen-Objektfabrik implementiert; Snapious berechnet nur Werte (`bio/protein.rs`) oder exportiert SVG (`view/map_svg.rs`, `view/tree_layout.rs`), aber keine interaktiven Graphen-Fabriken.
- Bereitstellen von Hydrophobizitätsgraphen pro Rest | Kern: ja | App: nein | Snapious berechnet Hydropathie (`bio/protein.rs`), stellt aber keine Graphen-Datenstruktur bereit; die UI zeigt nur Text/Status (`view/selection_stats.rs`), keine Graphen.
- Bereitstellen von PI-Graphen pro Rest | Kern: ja | App: nein | Snapious berechnet den isoelektrischen Punkt (`bio/protein.rs`), stellt aber keine Graphen-Datenstruktur bereit; die UI zeigt nur Text/Status (`view/selection_stats.rs`), keine Graphen.
- Berechnung der Hydrophobizität an jedem Rest | Kern: ja | App: ja | `bio/protein.rs` (Hydropathie)
- Berechnung des isoelektrischen Punkts (PI) pro Rest | Kern: ja | App: ja | `bio/protein.rs` (isoelektrischer Punkt)
- Durchschnittliche Hydrophobizität über alle Sequenzen | Kern: ja | App: nein | Snapious berechnet Eigenschaften pro Sequenz (`bio/protein.rs`), bietet aber keine Funktion zur Durchschnittsbildung über mehrere Sequenzen/Alignment-Zeilen an.
- Durchschnittlicher isoelektrischer Punkt (PI) über alle Sequenzen | Kern: ja | App: nein | Snapious berechnet PI pro Sequenz (`bio/protein.rs`), bietet aber keine Funktion zur Durchschnittsbildung über mehrere Sequenzen/Alignment-Zeilen an.
- Generierung von Proteingraphen | Kern: ja | App: nein | Snapious exportiert statische SVG-Karten (`view/map_svg.rs`) oder Sequenztext, aber keine dynamischen Proteingraphen (wie Hydrophobizitätsverläufe als Graphenobjekte).
