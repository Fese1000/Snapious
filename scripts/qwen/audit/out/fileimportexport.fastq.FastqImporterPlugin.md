GENEIOUS-FUNKTIONEN:
- Importiert FASTQ-Sequenzdateien in Geneious Prime
- Generiert QC-Berichte (sofern FastQC-Plugin verfügbar)
- Verarbeitet Illumina-Qualitätsdaten (Version 1.3–1.7)
- Importiert Dateien als gepaarte Lesevorgänge (Paired Reads)
- Bietet Option zum Importieren als ungepaarte Lesevorgänge
- Stellt FASTQ-Importoptionen und Qualitäts-Einstellungen bereit

ABGLEICH:
- Importiert FASTQ-Sequenzdateien in Geneious Prime | Kern: ja | App: ja | io/fastq.rs, import_file
- Generiert QC-Berichte (sofern FastQC-Plugin verfügbar) | Kern: ja | App: ja | ngs/qc.rs, qc_fastq
- Verarbeitet Illumina-Qualitätsdaten (Version 1.3–1.7) | Kern: ja | App: ja | io/quality.rs (Phred+33 Logik abgedeckt)
- Importiert Dateien als gepaarte Lesevorgänge (Paired Reads) | Kern: ja | App: nein | io/fastq.rs liest Einträge, aber keine explizite "Paired-End"-Struktur im Import-Wizard oder `import_file` genannt; `ngs/fastq_stream.rs` lielt stromweise, aber Paarung ist nicht als spezifische Import-Option in der Schnittstelle aufgeführt.
- Bietet Option zum Importieren als ungepaarte Lesevorgänge | Kern: ja | App: ja | io/fastq.rs, import_file (implizit durch Standard-Import)
- Stellt FASTQ-Importoptionen und Qualitäts-Einstellungen bereit | Kern: ja | App: ja | io/fastq.rs, settings_load (Qualitätskodierung Phred+33 ist fest implementiert)
