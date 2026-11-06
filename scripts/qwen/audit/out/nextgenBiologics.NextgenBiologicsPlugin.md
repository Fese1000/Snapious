GENEIOUS-FUNKTIONEN:
- Antikörper- und TCR-Sequenzannotation (VDJ/VJ)
- Erstellung und Verwaltung von Biologics-Datenbanken
- Vergleich verschiedener Annotation-Algorithmen (IgBLAST, ANARCI)
- Klonale Clusterung und Häufigkeitsanalyse von Sequenzen
- Einzelzell-Analyse (SingleCell) mit UMI-Deduplizierung
- Identifikation posttranslationaler Modifikationen (Deamidation, Glykosylierung)
- Erkennung von Sequenzfehlern und Qualitätsmängeln (Liabilities)
- Import von IMGT-Fasta-Daten für Referenzsequenzen
- Generierung von Zusammenfassungs- und Statistikdokumenten
- Analyse von CDR-Regionen und Framework-Fehlern
- Unterstützung für schwere und leichte Ketten sowie TCRs

ABGLEICH:
- Antikörper- und TCR-Sequenzannotation (VDJ/VJ) | Kern: nein | App: nein | Keine VDJ/Rekombinationslogik oder IMGT-Referenzdatenbank vorhanden
- Erstellung und Verwaltung von Biologics-Datenbanken | Kern: nein | App: ja (store/project_store.rs, store/folders.rs) | Projektstruktur und Ordnerverwaltung existieren, aber keine spezifische "Biologics"-Datenbank-Struktur
- Vergleich verschiedener Annotation-Algorithmen (IgBLAST, ANARCI) | Kern: nein | App: nein | Keine spezialisierten Antikörper-Annotationstools implementiert
- Klonale Clusterung und Häufigkeitsanalyse von Sequenzen | Kern: nein | App: ja (project/search.rs) | "Dublettensuche" und "Stapelumbenennung" existieren, aber keine klonale Clusterung nach Sequenzähnlichkeit oder Häufigkeitsverteilung
- Einzelzell-Analyse (SingleCell) mit UMI-Deduplizierung | Kern: nein | App: nein | Keine SingleCell-Logik oder UMI-Behandlung vorhanden
- Identifikation posttranslationaler Modifikationen (Deamidation, Glykosylierung) | Kern: nein | App: ja (model/feature.rs, ops/more.rs) | Features können Modifikationen speichern und Protein-Eigenschaften berechnet werden, aber keine spezifische PTM-Identifikation oder -Verwaltung für Biologics
- Erkennung von Sequenzfehlern und Qualitätsmängeln (Liabilities) | Kern: nein | App: ja (ngs/qc.rs, primer/checks.rs) | QC-Berichte und Primer-Qualitätsprüfungen existieren, aber keine spezifische "Liability"-Analyse für Biologics
- Import von IMGT-Fasta-Daten für Referenzsequenzen | Kern: nein | App: ja (io/fasta.rs, db/ncbi.rs) | Fasta-Import und NCBI-Zugriff existieren, aber keine IMGT-spezifische Parserlogik oder Referenzdatenbank
- Generierung von Zusammenfassungs- und Statistikdokumenten | Kern: ja | App: ja (export/text.rs, bio/align_stats.rs) | "Markdown-Bericht" und "Kennzahlen zu Alignments" (Konsens, Entropie etc.) sind vorhanden
- Analyse von CDR-Regionen und Framework-Fehlern | Kern: nein | App: ja (bio/motif.rs, primer/picker.rs) | Motivsuche und Primerdesign existieren, aber keine spezifische CDR/Framework-Definition oder Fehleranalyse für Antikörper
- Unterstützung für schwere und leichte Ketten sowie TCRs | Kern: nein | App: ja (model/sequence.rs, bio/genetic_code.rs) | DNA/RNA/Protein-Sequenzen und Translation werden unterstützt, aber keine spezifische Logik für Kettentypen (Heavy/Light/TCR)
