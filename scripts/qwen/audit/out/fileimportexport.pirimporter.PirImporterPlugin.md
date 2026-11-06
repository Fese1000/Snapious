GENEIOUS-FUNKTIONEN:
- Importiert PIR-Dateiformate in Geneious Prime
- Erstellt Sequenzen aus PIR-Daten
- Erstellt Alignments basierend auf PIR-Inhalten
- Validiert das Dateiformat der importierten Datei
- Prüft auf vorhandene Sequenzdaten in der Datei
- Überprüft das Vorhandensein eines SequenceBuilders
- Stellt sicher, dass Sequenzen korrekt terminiert sind

ABGLEICH:
- Importiert PIR-Dateiformate in Geneious Prime | Kern: nein | App: nein | io/fastq.rs (PIR ist kein unterstütztes Format; nur FASTA/FASTQ/etc. sind implementiert)
- Erstellt Sequenzen aus PIR-Daten | Kern: nein | App: nein | model/sequence.rs (Kein spezifischer PIR-Parser vorhanden)
- Erstellt Alignments basierend auf PIR-Inhalten | Kern: nein | App: nein | bio/align.rs (Keine automatische Alignment-Erstellung aus PIR-Import)
- Validiert das Dateiformat der importierten Datei | Kern: teilweise | App: ja | io/fasta.rs (Formatvalidierung ist Teil des Leseprozesses, aber kein separater "PIR-Validierer")
- Prüft auf vorhandene Sequenzdaten in der Datei | Kern: ja | App: ja | io/fasta.rs (Sequenzdaten werden beim Lesen extrahiert)
- Überprüft das Vorhandensein eines SequenceBuilders | Kern: ja | App: nein | (Dies ist eine Java/Geneious-interne Klassenstruktur; Snapious nutzt `model/sequence.rs`, aber prüft nicht explizit auf einen "SequenceBuilder")
- Stellt sicher, dass Sequenzen korrekt terminiert sind | Kern: ja | App: teilweise | io/fasta.rs (Terminierung wird beim Lesen geprüft, aber keine automatische Korrektur/Terminierung wie im Plugin beschrieben)
