GENEIOUS-FUNKTIONEN:
- Entfernt chimäre Sequenzen mittels UCHIME/USEARCH
- Nutzt Referenzdatenbank für Chimera-Erkennung
- Verarbeitet Nukleotid-Sequenzlisten und Alignments
- Unterstützt Paired-End-Leseverarbeitung (getrennt/konkateniert)
- Speichert chimäre Reads in separater Liste
- Filtert Sequenzen basierend auf Länge und Divergenz
- Bietet Optionen für Reverse-Komplement-Betrachtung
- Exportiert und importiert Ergebnisse aus UCHIME/USEARCH

ABGLEICH:
- Entfernt chimäre Sequenzen mittels UCHIME/USEARCH | Kern: nein | App: nein | Keine Implementierung von UCHIME/USEARCH oder Chimera-Erkennungsalgorithmen vorhanden
- Nutzt Referenzdatenbank für Chimera-Erkennung | Kern: nein | App: nein | Keine spezifische Referenzdatenbank für Chimera-Filterung implementiert
- Verarbeitet Nukleotid-Sequenzlisten und Alignments | Kern: ja | App: ja | `bio/align.rs`, `model/document.rs` (Allgemeine Verarbeitung von Sequenzlisten/Alignments, aber keine Chimera-Logik)
- Unterstützt Paired-End-Leseverarbeitung (getrennt/konkateniert) | Kern: ja | App: ja | `io/fastq.rs`, `ngs/fastq_stream.rs` (Lesen/Schreiben von FASTQ, keine spezifische Paired-End-Chimera-Behandlung)
- Speichert chimäre Reads in separater Liste | Kern: nein | App: nein | Keine Funktion zum Speichern von "chimären" Reads als separater Kategorie vorhanden
- Filtert Sequenzen basierend auf Länge und Divergenz | Kern: ja | App: teilweise | `bio/seq_stats.rs`, `model/sequence.rs` (Statistiken und Grundoperationen vorhanden, aber kein dedizierter Filter für Chimerie basierend auf Divergenz)
- Bietet Optionen für Reverse-Komplement-Betrachtung | Kern: ja | App: ja | `ops/builtin.rs` (enthält "Reverse-Komplement"), `bio/genetic_code.rs`
- Exportiert und importiert Ergebnisse aus UCHIME/USEARCH | Kern: nein | App: nein | Keine spezifischen Import/Export-Formate für UCHIME/USEARCH-Ergebnisse vorhanden
