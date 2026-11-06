GENEIOUS-FUNKTIONEN:
- Importiert SFF-Datenformate für 454-Sequenzierung
- Verarbeitet csfasta-Dateiformate für SOLiD-Daten
- Liest qual-Dateien zur Qualitätsbewertung von SOLiD
- Importiert Dokumente aus mehreren Dateien gleichzeitig
- Validiert Lückenfreiheit der Fragmentsequenzen
- Prüft auf nicht-eindeutige Nukleotidsequenzen
- Stellt sicher, dass Sequenzlänge ein Vielfaches von 5 ist
- Sucht zugehörige Basissequenzen in Begleitdateien
- Erkennt und verarbeitet Headerzeilen der Sequenzdaten
- Zeigt Fortschritt beim Importieren definierter Lesungen an
- Prüft Konsistenz zwischen qual- und fasta-Dateien
- Leitet unterstützte Dateitypen über PrimaryFileForMultipleFileImporter

ABGLEICH:
- Importiert SFF-Datenformate für 454-Sequenzierung | Kern: nein | App: nein | Keine SFF-Implementierung vorhanden
- Verarbeitet csfasta-Dateiformate für SOLiD-Daten | Kern: nein | App: nein | Keine csfasta-Implementierung vorhanden
- Liest qual-Dateien zur Qualitätsbewertung von SOLiD | Kern: nein | App: nein | Keine qual-Dateiimplementierung vorhanden (nur FASTQ/Phred)
- Importiert Dokumente aus mehreren Dateien gleichzeitig | Kern: teilweise | App: ja | `import_file` (implizit möglich, aber keine spezifische Multi-File-Logik wie NextGenSequencing)
- Validiert Lückenfreiheit der Fragmentsequenzen | Kern: nein | App: ja | `check_read_file` (prüft Datei, aber keine explizite Lückenfreiheits-Validierung als separate Funktion genannt) -> *Korrektur*: Streng genommen keine explizite "Gap-Free Validation" Funktion im Interface. | App: nein
- Prüft auf nicht-eindeutige Nukleotidsequenzen | Kern: nein | App: nein | Keine spezifische Funktion für "nicht-eindeutige Sequenzen" (IUPAC-Codes werden in `motif.rs` und `cloning/enzymes.rs` unterstützt, aber keine Validierungsfunktion "check_ambiguous" im Interface)
- Stellt sicher, dass Sequenzlänge ein Vielfaches von 5 ist | Kern: nein | App: nein | Keine solche Validierung im Interface
- Sucht zugehörige Basissequenzen in Begleitdateien | Kern: nein | App: nein | Keine Logik für "Base Sequence Lookup" aus Begleitdateien
- Erkennt und verarbeitet Headerzeilen der Sequenzdaten | Kern: teilweise | App: ja | `io/fasta.rs` und `io/fastq.rs` lesen Header; `paste.rs` säubert Text. | `io/fasta.rs`, `io/fastq.rs`, `io/paste.rs`
- Zeigt Fortschritt beim Importieren definierter Lesungen an | Kern: nein | App: ja | `view/progress_format.rs` bietet Fortschrittsanzeige; `import_file` ist die Operation. | `view/progress_format.rs`, `import_file`
- Prüft Konsistenz zwischen qual- und fasta-Dateien | Kern: nein | App: nein | Keine explizite Konsistenzprüfung zwischen separaten qual/fasta Dateien im Interface
- Leitet unterstützte Dateitypen über PrimaryFileForMultipleFileImporter | Kern: nein | App: nein | Keine solche Plugin-Interface-Logik in Snapious vorhanden
