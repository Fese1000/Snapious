GENEIOUS-FUNKTIONEN:
- Generiert Qualitätskontrollberichte für Sequenzdaten
- Analysiert Lesungen mit Qualitätsinformationen (Read Quality)
- Verarbeitet Listen von Nukleotidsequenzen
- Zeigt FastQC-Qualitätskontrollberichte an
- Erstellt QC-Berichte für bearbeitete Sequenzdaten
- Führt FastQC-Analyseoperationen aus
- Stellt Dokumentansichten für FastQC-Ergebnisse bereit
- Zeigt Warnungen bei Berichtsgenerierungsproblemen an
- Signalisiert erfolgreichen Abschluss der FastQC-Analyse
- Unterstützt HTML-formatierte Ergebnisberichte

ABGLEICH:
- Generiert Qualitätskontrollberichte für Sequenzdaten | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Analysiert Lesungen mit Qualitätsinformationen (Read Quality) | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Verarbeitet Listen von Nukleotidsequenzen | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Zeigt FastQC-Qualitätskontrollberichte an | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Erstellt QC-Berichte für bearbeitete Sequenzdaten | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Führt FastQC-Analyseoperationen aus | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Stellt Dokumentansichten für FastQC-Ergebnisse bereit | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Zeigt Warnungen bei Berichtsgenerierungsproblemen an | Kern: ja | App: ja | error_hint (view/error_hint.rs)
- Signalisiert erfolgreichen Abschluss der FastQC-Analyse | Kern: ja | App: ja | qc_fastq (ngs/qc.rs)
- Unterstützt HTML-formatierte Ergebnisberichte | Kern: nein | App: nein | export_text (export/text.rs) bietet nur Markdown/Text, keine HTML-Generierung für QC-Berichte
