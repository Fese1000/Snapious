GENEIOUS-FUNKTIONEN:
- Trennt Sequenzen nach Barcode (MID) oder fester Länge
- Gruppiert Sequenzen zu einem einzigen Dokument zusammen
- Setzt Paired Reads durch Namen oder Felder
- Trimmt Adapter, Primer oder Linker von Sequenzenden
- Extrahiert Sequenzen basierend auf Barcode-Sets
- Validiert und korrigiert Paarungsstatus von Reads
- Konfiguriert Sequenzierungstechnologie für Fehlermodelle
- Filtert Barcodes basierend auf Mindest-Leseanzahl
- Verarbeitet 454 Rapid/Standard MIDs und Illumina Daten
- Prüft auf Duplikate, fehlende Primer oder Linker
- Erstellt neue Subfolder für extrahierte Sequenzdaten
- Keine spezifischen biologischen Analysefunktionen erkennbar

ABGLEICH:
- Trennt Sequenzen nach Barcode (MID) oder fester Länge | Kern: nein | App: nein | Keine Funktion zum Splitting nach Barcode/Länge vorhanden (nur `edit_delete`/`edit_insert` manuell)
- Gruppiert Sequenzen zu einem einzigen Dokument zusammen | Kern: ja | App: ja | `align_documents` (kann zur Zusammenführung genutzt werden) oder manuelle Kopie/Einfügung via `edit_insert`
- Setzt Paired Reads durch Namen oder Felder | Kern: nein | App: nein | Keine explizite Paired-Ende-Manipulation oder Feld-basierte Paarungsfunktion vorhanden
- Trimmt Adapter, Primer oder Linker von Sequenzenden | Kern: ja | App: ja | `trim_fastq` (in `ngs/trim.rs`)
- Extrahiert Sequenzen basierend auf Barcode-Sets | Kern: nein | App: nein | Keine automatische Extraktion nach Barcode-Set vorhanden (nur manuelle Auswahl via `selection_text`/`edit_delete`)
- Validiert und korrigiert Paarungsstatus von Reads | Kern: nein | App: nein | Keine Funktion zur Validierung/Korrektur von Read-Paarungsstatus vorhanden
- Konfiguriert Sequenzierungstechnologie für Fehlermodelle | Kern: nein | App: nein | Keine Konfiguration von Fehlermodellen pro Technologie vorhanden
- Filtert Barcodes basierend auf Mindest-Leseanzahl | Kern: nein | App: nein | Keine Filterung nach Read-Anzahl pro Barcode vorhanden
- Verarbeitet 454 Rapid/Standard MIDs und Illumina Daten | Kern: nein | App: nein | Keine spezifische Verarbeitung für 454 MIDs oder Illumina-spezifische Metadaten vorhanden
- Prüft auf Duplikate, fehlende Primer oder Linker | Kern: ja | App: teilweise | `find_duplicates` (App) für Duplikate; fehlende Primer/Linker müssen manuell geprüft werden (`motif` oder `primer/checks`)
- Erstellt neue Subfolder für extrahierte Sequenzdaten | Kern: ja | App: ja | `create_folder` (App) und manuelle Verschiebung via `move_document`
