GENEIOUS-FUNKTIONEN:
- Importiert FASTA-Dateien mit Nukleotid- und Proteinsequenzen
- Exportiert Sequenzen oder Alignments im FASTA-Format
- Komprimiert exportierte Sequenzen oder Alignments
- Setzt Leerzeichen in Sequenznamen durch Unterstriche
- Ersetzt benutzerdefinierte Zeichen in Namen durch Unterstriche
- Umbricht Sequenzzeilen alle 80 Zeichen für Kompatibilität
- Behandelt vollständig aus Lücken bestehende Sequenzen im Alignment
- Exportiert fehlende Enden von Alignments als FASTA
- Behält Sequenzbeschreibung nach dem ersten Leerzeichen bei
- Ignoriert Beschreibung nach Namen für inkompatible Programme
- Wendet verschiedene Groß-/Kleinschreibung auf Editier-Historie an
- KEINE

ABGLEICH:
- Importiert FASTA-Dateien mit Nukleotid- und Proteinsequenzen | Kern: ja | App: ja | io/fasta.rs, import_file
- Exportiert Sequenzen oder Alignments im FASTA-Format | Kern: ja | App: ja | io/fasta.rs, export_document
- Komprimiert exportierte Sequenzen oder Alignments | Kern: nein | App: nein | Keine Kompressionsfunktion im Export beschrieben (nur `io/fasta.rs` schreibt Text)
- Setzt Leerzeichen in Sequenznamen durch Unterstriche | Kern: ja | App: nein | Keine automatische Namensbereinigung im Import/Export dokumentiert
- Ersetzt benutzerdefinierte Zeichen in Namen durch Unterstriche | Kern: ja | App: nein | Keine automatische Namensbereinigung im Import/Export dokumentiert
- Umbricht Sequenzzeilen alle 80 Zeichen für Kompatibilität | Kern: ja | App: teilweise | io/fasta.rs (Standard FASTA-Formatierung impliziert Zeilenumbruch, aber explizite "80er"-Regel als konfigurierbare Option fehlt)
- Behandelt vollständig aus Lücken bestehende Sequenzen im Alignment | Kern: ja | App: teilweise | bio/align.rs (Alignment-Datenstruktur existiert, spezifische Handhabung "vollständig leerer" Sequenzen als Sonderfall nicht explizit genannt)
- Exportiert fehlende Enden von Alignments als FASTA | Kern: ja | App: nein | Keine spezifische Funktion zum Exportieren von "fehlenden Enden" (Trimmen/Padding) im Kontext des Exports beschrieben
- Behält Sequenzbeschreibung nach dem ersten Leerzeichen bei | Kern: ja | App: ja | io/fasta.rs (Standard FASTA-Header-Parsing)
- Ignoriert Beschreibung nach Namen für inkompatible Programme | Kern: ja | App: nein | Keine explizite Logik zur Ignorierung von Headern basierend auf Zielprogrammen im Export beschrieben
- Wendet verschiedene Groß-/Kleinschreibung auf Editier-Historie an | Kern: ja | App: nein | Keine spezifische Groß-/Kleinschreibungs-Logik in der Editier-Historie (Undo/Redo) dokumentiert
