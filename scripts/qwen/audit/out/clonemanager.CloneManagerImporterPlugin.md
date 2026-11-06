GENEIOUS-FUNKTIONEN:
- Importieren von Clone Manager .cm5 Dateien
- Erkennen und Validieren des CM5 Dateiformats
- Prüfen der Anwesenheit von 5SES Header
- Parsen und Extrahieren von Sequenzdaten
- Validieren der Sequenzlänge im Header
- Prüfen auf Vorhandensein einer Sequenz
- Erkennen von abgeschnittenen Dateien
- Validieren der Blockanzahl im Header
- Erstellen neuer Dokumente aus importierten Daten
- Anzeigen von Import-Fehlermeldungen

ABGLEICH:
- Importieren von Clone Manager .cm5 Dateien | Kern: nein | App: nein | Keine Datei oder Funktion für `.cm5`-Import vorhanden (Snapious unterstützt FASTA, GenBank, EMBL, GFF3, SAM/BAM, VCF, ABIF, PDB, Newick, NEXUS, Clustal, PHYLIP, SnapGene `.dna`, aber nicht Clone Manager Format)
- Erkennen und Validieren des CM5 Dateiformats | Kern: nein | App: nein | Keine spezifische Parser-Logik für CM5-Header oder Formatvalidierung in den oben genannten `io/`-Modulen vorhanden
- Prüfen der Anwesenheit von 5SES Header | Kern: nein | App: nein | Keine Referenz auf "5SES" oder spezifische CM5-Header-Struktur in `io/`-Modulen vorhanden
- Parsen und Extrahieren von Sequenzdaten | Kern: teilweise | App: ja | Generische FASTA/GenBank-Parsing-Funktionen (`io/fasta.rs`, `io/genbank.rs`) existieren, aber kein spezifischer CM5-Parser; Extraktion ist nur möglich, wenn das Format bereits als unterstütztes Standardformat vorliegt
- Validieren der Sequenzlänge im Header | Kern: nein | App: nein | Keine spezifische Validierung einer "Header-Sequenzlänge" im Kontext eines proprietären Formats wie CM5 vorhanden; allgemeine Längenprüfung ist Teil der allgemeinen Sequenzvalidierung (`model/sequence.rs`), aber nicht als separater Import-Schritt für CM5 implementiert
- Prüfen auf Vorhandensein einer Sequenz | Kern: teilweise | App: ja | Generische Prüfung, ob ein importiertes Dokument eine Sequenz enthält (`model/document.rs`, `io/fasta.rs` etc.), aber keine spezifische CM5-spezifische Logik
- Erkennen von abgeschnittenen Dateien | Kern: nein | App: nein | Keine spezifische Logik zur Erkennung von "abgeschnittenen" CM5-Dateien vorhanden; allgemeine Fehlerbehandlung (`view/error_hint.rs`) existiert, aber keine format-spezifische Erkennung
- Validieren der Blockanzahl im Header | Kern: nein | App: nein | Keine Referenz auf "Blockanzahl" oder CM5-spezifische Blockstruktur in den importierten Dateimodulen vorhanden
- Erstellen neuer Dokumente aus importierten Daten | Kern: ja | App: ja | Funktion `create_sequence` und `import_file` existieren in der Schnittstelle; jedoch fehlt die spezifische Brücke von CM5-Daten zu diesen Funktionen, da der Import selbst fehlt
- Anzeigen von Import-Fehlermeldungen | Kern: ja | App: ja | Funktion `view/error_hint.rs` und allgemeine Fehlerbehandlung existieren; jedoch bezieht sich dies auf generische App-Fehler, nicht spezifisch auf CM5-Importfehler
