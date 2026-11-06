GENEIOUS-FUNKTIONEN:
- Importiert Dateien, die von Artemis exportiert wurden
- Verarbeitet GenBank-Dateien aus Artemis
- Liest EMBL- und FASTA-Dateien aus Artemis ein
- Importiert unformatierte Dateien aus Artemis
- Analysiert SQ-Zeilen auf BASE COUNT Informationen
- Parst FEATURE-Lines für Lokalisierung und Qualifizierer
- Validiert die Token-Anzahl in SQ-Zeilen
- Prüft das Vorhandensein von Sequenzen in der Datei

ABGLEICH:
- Importiert Dateien, die von Artemis exportiert wurden | Kern: ja | App: ja | io/genbank.rs, io/embl.rs, io/fasta.rs
- Verarbeitet GenBank-Dateien aus Artemis | Kern: ja | App: ja | io/genbank.rs
- Liest EMBL- und FASTA-Dateien aus Artemis ein | Kern: ja | App: ja | io/embl.rs, io/fasta.rs
- Importiert unformatierte Dateien aus Artemis | Kern: ja | App: ja | io/paste.rs (Säuberung und Erkennung der Art), io/fasta.rs
- Analysiert SQ-Zeilen auf BASE COUNT Informationen | Kern: nein | App: nein | Keine spezifische Parser-Logik für Artemis-SQ-Basenzählung vorhanden; Snapious nutzt `io/genbank.rs` für Feature/Sequenz-Parsing, aber keine explizite SQ-Validierung der Base Counts als separater Schritt.
- Parst FEATURE-Lines für Lokalisierung und Qualifizierer | Kern: ja | App: ja | io/genbank.rs, io/embl.rs (Parsing von Features und Positionen)
- Validiert die Token-Anzahl in SQ-Zeilen | Kern: nein | App: nein | Keine explizite Validierung der Token-Anzahl in SQ-Zeilen wie im Artemis-Importer spezifiziert; Snapious verlässt sich auf das robuste Parsing von `io/genbank.rs` und `io/embl.rs`.
- Prüft das Vorhandensein von Sequenzen in der Datei | Kern: ja | App: ja | io/genbank.rs, io/embl.rs (Prüfung auf Sequenzdaten beim Lesen)
