GENEIOUS-FUNKTIONEN:
- Erstellen von Konsensusbäumen aus mehreren Stammbäumen
- Generieren phylogenetischer Bäume aus Sequenzalignments
- Berechnung genetischer Distanzmodelle für Baumkonstruktion
- Resampling von Bäumen zur Schätzung der Support-Werte
- Sortierung und Analyse der häufigsten Topologien
- Filterung von Bäumen basierend auf Burn-in-Prozent
- Anzeige und Verwaltung von Stammbaum-Dokumenten
- Import und Export von Baumdateiformaten
- Visualisierung von Clade-Häufigkeiten in Konsensusbäumen
- Auswahl verschiedener Algorithmen zur Baumkonstruktion

ABGLEICH:
- Erstellen von Konsensusbäumen aus mehreren Stammbäumen | Kern: ja | App: nein | bio/newick.rs (Lesen/Schreiben), bio/tree.rs (Datenstruktur) – *Fehlende Implementierung der Konsensus-Algorithmen (z.B. Strict/Majority-Rule) und die Verknüpfung mehrerer Bäume zu einem Konsensusbaum in der Benutzeroberfläche.*
- Generieren phylogenetischer Bäume aus Sequenzalignments | Kern: ja | App: ja | bio/tree.rs (Neighbor-Joining, UPGMA), bio/backends.rs (Aufruf von FastTree, IQ-TREE über externe Programme)
- Berechnung genetischer Distanzmodelle für Baumkonstruktion | Kern: ja | App: ja | bio/distance.rs (Berechnung evolutionärer Distanzen)
- Resampling von Bäumen zur Schätzung der Support-Werte | Kern: ja | App: nein | bio/tree.rs (Basis-Algorithmen vorhanden) – *Fehlende Implementierung von Bootstrapping-Verfahren und die Berechnung/Anzeige von Support-Werten (Bootstrap-Support) für Knoten.*
- Sortierung und Analyse der häufigsten Topologien | Kern: ja | App: teilweise | bio/treeops.rs (Sortieren, Aufspaltungen) – *Fehlende spezifische Analyse-Funktion zur Häufigkeitsbestimmung von Topologien über mehrere Bäume hinweg.*
- Filterung von Bäumen basierend auf Burn-in-Prozent | Kern: ja | App: nein | bio/treeops.rs (Filterung/Manipulation) – *Fehlende spezifische Burn-in-Logik und UI-Filterung basierend auf diesem statistischen Wert.*
- Anzeige und Verwaltung von Stammbaum-Dokumenten | Kern: ja | App: ja | model/tree_doc.rs, view/tree_layout.rs (Layout/SVG), store/project_store.rs (Speicherung)
- Import und Export von Baumdateiformaten | Kern: ja | App: ja | bio/newick.rs (Newick-Format lesen/schreiben), io/nexus.rs (NEXUS-Format lesen/schreiben)
- Visualisierung von Clade-Häufigkeiten in Konsensusbäumen | Kern: ja | App: nein | view/tree_layout.rs (Layout) – *Fehlende visuelle Darstellung von Clade-Häufigkeiten oder Support-Werten innerhalb der Baumvisualisierung.*
