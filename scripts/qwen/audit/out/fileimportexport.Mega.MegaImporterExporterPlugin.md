GENEIOUS-FUNKTIONEN:
- MEGA-Alignments importieren
- MEGA-Dokumente exportieren
- Dateiimport über MegaImporterExporterPlugin
- Dateiausgabe über MegaImporterExporterPlugin
- MEGA-Dateiformate lesen
- MEGA-Dateiformate schreiben
- MEGA-Alignment-Dokumente laden
- MEGA-Alignment-Dokumente speichern

ABGLEICH:
- MEGA-Alignments importieren | Kern: nein | App: nein | Keine Datei für MEGA-Format (`.meg`/`.mega`) vorhanden; `io/alignment_formats.rs` unterstützt nur Clustal und PHYLIP.
- MEGA-Dokumente exportieren | Kern: nein | App: nein | Keine Datei für MEGA-Export vorhanden; `export/text.rs` unterstützt nur Text und Markdown.
- Dateiimport über MegaImporterExporterPlugin | Kern: nein | App: nein | Keine spezifische Import-Logik für MEGA vorhanden; `io/` enthält keine MEGA-Parses.
- Dateiausgabe über MegaImporterExporterPlugin | Kern: nein | App: nein | Keine spezifische Export-Logik für MEGA vorhanden; `export/` enthält keine MEGA-Generatoren.
- MEGA-Dateiformate lesen | Kern: nein | App: nein | Keine Datei, die das MEGA-Dateiformat liest; `io/` listet keine MEGA-Schnittstelle.
- MEGA-Dateiformate schreiben | Kern: nein | App: nein | Keine Datei, die das MEGA-Dateiformat schreibt; `io/` listet keine MEGA-Schnittstelle.
- MEGA-Alignment-Dokumente laden | Kern: nein | App: nein | Keine Funktion zum Laden von MEGA-spezifischen Alignment-Dokumenten; `io/alignment_formats.rs` deckt nur Clustal/PHYLIP ab.
- MEGA-Alignment-Dokumente speichern | Kern: nein | App: nein | Keine Funktion zum Speichern von MEGA-spezifischen Alignment-Dokumenten; `io/` bietet keinen MEGA-Writer.
