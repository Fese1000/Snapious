GENEIOUS-FUNKTIONEN:
- Importiert SAM- und BAM-Sequenzalignments
- Exportiert Daten ins BAM-Format
- Identifiziert gepaarte Lesungen (Reads)
- Erkennt fehlende Basen am Referenzstart
- Prüft Überlappung mit der Referenzsequenz
- Passt Lesungsnamen nach Konvertierung an
- Verarbeitet verschränkte (interleaved) Dateien
- Fügt Informationen zu Lesungsgruppen hinzu
- Berechnet Cigar-Strings für Alignments
- Prüft Orientierung und Offset der Reads
- Extrahiert spezifische Regionen aus Dateien
- Warnet bei falscher Positionierung von Reads

ABGLEICH:
- Importiert SAM- und BAM-Sequenzalignments | Kern: ja | App: ja | io/sam.rs, ngs/bam.rs
- Exportiert Daten ins BAM-Format | Kern: ja | App: nein | io/sam.rs (liest nur), ngs/bam.rs (liest nur) – kein Writer vorhanden
- Identifiziert gepaarte Lesungen (Reads) | Kern: ja | App: nein | ngs/bam.rs (liest Header/Flags, aber keine explizite Paarungslogik in der Schnittstelle)
- Erkennt fehlende Basen am Referenzstart | Kern: ja | App: nein | ngs/bam.rs (liest CIGAR, aber keine spezifische Prüfung auf "missing bases at start" in der API)
- Prüft Überlappung mit der Referenzsequenz | Kern: ja | App: teilweise | ngs/mapper.rs (Mapping), bio/read_map.rs (Abbildung ohne Lücken) – keine explizite "Prüfung" als separate Funktion, aber durch Mapping abgedeckt
- Passt Lesungsnamen nach Konvertierung an | Kern: ja | App: nein | Keine Funktion zur Umbenennung von Read-Namen während des Imports/Exports
- Verarbeitet verschränkte (interleaved) Dateien | Kern: ja | App: teilweise | ngs/fastq_stream.rs (liest Streams), io/sam.rs (liest SAM) – keine explizite "Interleaved-Verarbeitung" als Feature genannt, aber Lesbarkeit gegeben
- Fügt Informationen zu Lesungsgruppen hinzu | Kern: ja | App: nein | Keine Funktion zur Gruppierung von Reads (z.B. nach Read-ID) in der API
- Berechnet Cigar-Strings für Alignments | Kern: ja | App: teilweise | ngs/bam.rs (liest BAM, enthält CIGAR), io/sam.rs (liest SAM) – keine Funktion zur *Berechnung* oder Generierung von CIGAR-Strings aus Sequenzen
- Prüft Orientierung und Offset der Reads | Kern: ja | App: teilweise | ngs/bam.rs (liest Flags/Orientierung), bio/read_map.rs – keine explizite "Prüfung" als API-Funktion, sondern implizit im Leseprozess
- Extrahiert spezifische Regionen aus Dateien | Kern: ja | App: ja | ngs/fai.rs (Teilstücke direkt aus der FASTA lesen), io/sam.rs/io/bed.rs (Regionen)
- Warnet bei falscher Positionierung von Reads | Kern: ja | App: nein | Keine explizite Warnfunktion für "falsche Positionierung" in der API, nur Datenbereitstellung
