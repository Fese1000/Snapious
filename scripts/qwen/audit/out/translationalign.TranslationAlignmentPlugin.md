GENEIOUS-FUNKTIONEN:
- Führt eine Übersetzungsalignment-Analyse durch
- Bietet Optionen für Proteinalignments zur Translation
- Aligniert die proteinübersetzten Sequenzen der Eingabe
- Führt ein Back-Translation der alignierten Sequenzen durch
- Prüft auf unübersetzte Suffixe in den Sequenzen
- Erkennt unerwartete Sequenzen im Translation Alignment
- Validiert, dass mindestens 2 Sequenzen ausgewählt sind
- Nutzt verfügbare Proteinalignments für die Translation
- Prüft, ob Sequenzen mindestens 3 Basen lang sind
- Verwendet Proteinalignments zur Erstellung des Alignments

ABGLEICH:
- Führt eine Übersetzungsalignment-Analyse durch | Kern: ja | App: nein | bio/align_tools.rs (Translations-Alignment), view/translation.rs
- Bietet Optionen für Proteinalignments zur Translation | Kern: ja | App: nein | bio/align_tools.rs (implizit durch Translations-Alignment), view/translation.rs
- Aligniert die proteinübersetzten Sequenzen der Eingabe | Kern: ja | App: nein | bio/align_tools.rs (Translations-Alignment)
- Führt ein Back-Translation der alignierten Sequenzen durch | Kern: ja | App: nein | bio/align_tools.rs (implizit im Prozess des Translations-Alignments)
- Prüft auf unübersetzte Suffixe in den Sequenzen | Kern: ja | App: nein | bio/align_tools.rs (implizit in der Validierung des Translations-Alignments)
- Erkennt unerwartete Sequenzen im Translation Alignment | Kern: ja | App: nein | bio/align_tools.rs (implizit in der Validierung)
- Validiert, dass mindestens 2 Sequenzen ausgewählt sind | Kern: ja | App: nein | bio/align_tools.rs (implizit in der Logik des Alignments)
- Nutzt verfügbare Proteinalignments für die Translation | Kern: ja | App: nein | bio/align_tools.rs (Translations-Alignment)
- Prüft, ob Sequenzen mindestens 3 Basen lang sind | Kern: ja | App: nein | bio/align_tools.rs (implizit in der Validierung)
- Verwendet Proteinalignments zur Erstellung des Alignments | Kern: ja | App: nein | bio/align_tools.rs (Translations-Alignment)
