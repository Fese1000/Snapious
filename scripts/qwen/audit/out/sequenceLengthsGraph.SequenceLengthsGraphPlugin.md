GENEIOUS-FUNKTIONEN:
- Zeigt Histogramme der Sequenzlängen ausgewählter Sequenzen an
- Berechnet und visualisiert Insert Sizes für Paired Reads
- Extrahiert Sequenzen mit Längen in einem angegebenen Bereich
- Exportiert die zugrunde liegenden Längendaten als CSV-Datei
- Zeigt tatsächliche Paargedistanzen von Reads in Contigs an
- Ignoriert falsch orientierte oder zu weit entfernte Paare
- Stellt Längen von Contigs gesondert dar
- Zeigt tatsächliche Paargedistanzen für Reads in Contigs an

ABGLEICH:
- Zeigt Histogramme der Sequenzlängen ausgewählter Sequenzen an | Kern: nein | App: ja | `summary` (bietet Zusammenfassungen, aber kein spezifisches Längen-Histogramm wie beschrieben; `seq_stats` bietet GC/Statistiken, aber kein Längen-Histogramm für Auswahlen)
- Berechnet und visualisiert Insert Sizes für Paired Reads | Kern: nein | App: nein | Keine Funktion zur Berechnung von Insert Sizes für Paired-Ends-Reads vorhanden
- Extrahiert Sequenzen mit Längen in einem angegebenen Bereich | Kern: nein | App: ja | `selection_stats` (zeigt Stats, aber keine direkte Filter-/Extraktionsfunktion nach Längencode; `edit_delete`/`edit_insert` sind manuell) -> Streng: nein (keine automatische Extraktion nach Längenbereich implementiert)
- Exportiert die zugrunde liegenden Längendaten als CSV-Datei | Kern: nein | App: ja | `export_text` (kann Text exportieren, aber kein spezifischer CSV-Export für Längendaten definiert) -> Streng: nein
- Zeigt tatsächliche Paargedistanzen von Reads in Contigs an | Kern: nein | App: ja | `assemble_reads` (baut Contigs, aber keine explizite Distanzvisualisierung für Paired Reads wie im Plugin) -> Streng: nein
- Ignoriert falsch orientierte oder zu weit entfernte Paare | Kern: nein | App: nein | Keine Logik zur Filterung von Paaren basierend auf Orientierung/Distanz im Assembly-Prozess sichtbar
- Stellt Längen von Contigs gesondert dar | Kern: nein | App: ja | `summary` (kann Contig-Info zeigen, aber keine gesonderte visuelle Darstellung der Längenverteilung wie im Plugin) -> Streng: nein
- Zeigt tatsächliche Paargedistanzen für Reads in Contigs an | Kern: nein | App: nein | Keine spezifische Funktion zur Anzeige von Paargedistanzen in Contigs vorhanden
