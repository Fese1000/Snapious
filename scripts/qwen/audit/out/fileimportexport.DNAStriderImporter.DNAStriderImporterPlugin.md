GENEIOUS-FUNKTIONEN:
- Importiert DNA-Sequenzen aus Strider-Dateien
- Verarbeitet DNA Strider Sequenzdaten
- Prüft auf negative Sequenzlängen
- Prüft auf zu lange Sequenzen
- Validiert das DNA Stride-Dateiformat

ABGLEICH:
- Importiert DNA-Sequenzen aus Strider-Dateien | Kern: nein | App: nein | io/fastq.rs, io/fasta.rs (Strider-Format nicht implementiert)
- Verarbeitet DNA Strider Sequenzdaten | Kern: nein | App: nein | io/fastq.rs, io/fasta.rs (Strider-Format nicht implementiert)
- Prüft auf negative Sequenzlängen | Kern: ja | App: ja | model/sequence.rs (Grundoperationen enthalten Validierung)
- Prüft auf zu lange Sequenzen | Kern: ja | App: ja | model/sequence.rs (Grundoperationen enthalten Validierung)
- Validiert das DNA Stride-Dateiformat | Kern: nein | App: nein | io/fastq.rs, io/fasta.rs (Strider-Format nicht implementiert)
