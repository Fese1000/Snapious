GENEIOUS-FUNKTIONEN:
- De-novo-Genomzusammenführung aus verschiedenen Sequenzdaten
- Fehlerkorrektur und Zusammenführung von Lesevorgängen (Reads)
- Unterstützung für Einzelzell-, Multi-Zell- und Metagenom-Daten
- Analyse von RNA-Sequenzdaten mittels rnaSPAdes-Pipeline
- Detektion und Zusammenführung von Plasmiden aus WGS-Daten
- Hybrid-Zusammenführung unter Einbeziehung langer Nanopore/PacBio-Lesungen
- Nutzung vertrauenswürdiger Contigs zur Graphkonstruktion und Lückenschließung
- Integration unzuverlässiger Contigs für Wiederholungsauflösung und Lückenschließung
- Post-Prozessierung mit BWA zur Reduzierung von Fehlanpassungen
- Generierung von Zusammenführungsberichten und Qualitätskontrollen

ABGLEICH:
- De-novo-Genomzusammenführung aus verschiedenen Sequenzdaten | Kern: ja | App: ja | `ngs/spades.rs` (Aufruf von SPAdes, Ergebnis `contigs.fasta`)
- Fehlerkorrektur und Zusammenführung von Lesevorgängen (Reads) | Kern: ja | App: ja | `ngs/spades.rs` (SPAdes integriert Fehlerkorrektur; Snapious ruft SPAdes auf)
- Unterstützung für Einzelzell-, Multi-Zell- und Metagenom-Daten | Kern: ja | App: nein | `ngs/spades.rs` (SPAdes unterstützt dies, Snapious übergibt nur die Datei; keine spezifische Parameter-Steuerung in Snapious sichtbar)
- Analyse von RNA-Sequenzdaten mittels rnaSPAdes-Pipeline | Kern: ja | App: nein | `ngs/spades.rs` (Snapious ruft SPAdes auf, aber keine explizite Unterscheidung/rnaSPAdes-Logik in der Funktionsliste erkennbar)
- Detektion und Zusammenführung von Plasmiden aus WGS-Daten | Kern: ja | App: nein | `ngs/spades.rs` (SPAdes kann Plasmide detektieren, Snapious bietet keine spezifische "Plasmid-Detektion"-Funktion an)
- Hybrid-Zusammenführung unter Einbeziehung langer Nanopore/PacBio-Lesungen | Kern: ja | App: nein | `ngs/spades.rs` (SPAdes unterstützt Hybrid-Assembly, Snapious hat keine spezifische Schnittstelle dafür definiert)
- Nutzung vertrauenswürdiger Contigs zur Graphkonstruktion und Lückenschließung | Kern: ja | App: nein | `ngs/spades.rs` (Interne SPAdes-Logik, keine Snapious-Funktion dafür)
- Integration unzuverlässiger Contigs für Wiederholungsauflösung und Lückenschließung | Kern: ja | App: nein | `ngs/spades.rs` (Interne SPAdes-Logik, keine Snapious-Funktion dafür)
- Post-Prozessierung mit BWA zur Reduzierung von Fehlanpassungen | Kern: ja | App: nein | `ngs/spades.rs` (SPAdes-interner Schritt, Snapious führt keine Post-Processing-Schritte mit BWA nach der Assembly aus)
- Generierung von Zusammenführungsberichten und Qualitätskontrollen | Kern: ja | App: teilweise | `ngs/spades.rs` (Snapious liert das Ergebnis `contigs.fasta`, aber keine explizite Funktion zur Generierung von SPAdes-spezifischen Berichten oder QC-Reports für die Assembly selbst, nur allgemeines `qc_fastq` für Reads)
