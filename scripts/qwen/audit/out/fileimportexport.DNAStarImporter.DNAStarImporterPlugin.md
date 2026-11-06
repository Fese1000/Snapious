GENEIOUS-FUNKTIONEN:
- Importiert DNAStar-Sequenzdateien
- Verarbeitet SBD-Dateiformate (SequenceShape)
- Handhabt SeqAndPro-Sequenztypen
- Liest Genbank-Sektionen aus DNAStar ein
- Identifiziert Sequenztypen automatisch
- Erstellt neue Sequenzobjekte aus Daten
- Zeigt Fehler bei fehlenden Daten an
- Meldet Speicherfehler während des Imports
- Bietet keine spezifischen biologischen Analysen
- Keine Visualisierung oder Annotation-Funktion sichtbar

ABGLEICH:
- Importiert DNAStar-Sequenzdateien | Kern: nein | App: nein | Keine Datei für `.sbd` oder DNAStar-Format vorhanden (nur `.ab1`, `.fasta`, `.fastq`, `.genbank`, `.embl`, `.bed`, `.gff3`, `.sam`, `.bam`, `.vcf`, `.nexus`, `.newick`, `.pdb`, `.snapgene` (.dna), `.alignment_formats`)
- Verarbeitet SBD-Dateiformate (SequenceShape) | Kern: nein | App: nein | Keine Datei oder Funktion für SBD/SequenceShape vorhanden
- Handhabt SeqAndPro-Sequenztypen | Kern: nein | App: nein | Keine spezifische Handhabung für "SeqAndPro" Typen in den beschriebenen Modulen (`model/sequence.rs` behandelt nur DNA/RNA/Protein)
- Liest Genbank-Sektionen aus DNAStar ein | Kern: teilweise | App: ja | `io/genbank.rs` kann GenBank lesen, aber kein spezifischer DNAStar-Parser; SnapGene-Import ist in `io/snapgene.rs`, aber nicht für DNAStar
- Identifiziert Sequenztypen automatisch | Kern: ja | App: ja | `io/paste.rs` ("erkart der Art der Sequenz") und `model/sequence.rs` (Grundoperationen für DNA, RNA, Protein)
- Erstellt neue Sequenzobjekte aus Daten | Kern: ja | App: ja | `model/sequence.rs` und `ops/builtin.rs` (create_sequence via App-Schnittstelle)
- Zeigt Fehler bei fehlenden Daten an | Kern: ja | App: ja | `view/error_hint.rs` ("übersetzt technische Meldungen in Titel und Lösungsvorschlag")
- Meldet Speicherfehler während des Imports | Kern: ja | App: nein | `util.rs` oder allgemeine Fehlerbehandlung vorhanden, aber keine spezifische "Speicherfehler-Meldung während Import" als separate Funktion; `io/` Module lesen Dateien, Fehler werden standardmäßig propagiert
- Bietet keine spezifischen biologischen Analysen | Kern: ja | App: n/a | Dies ist eine Eigenschaft des Plugins, keine Funktion zur Prüfung
- Keine Visualisierung oder Annotation-Funktion sichtbar | Kern: ja | App: nein | Snapious hat umfangreiche Visualisierung (`view/map_svg.rs`, `view/seq_layout.rs`) und Annotation-Funktionen (`model/feature.rs`, `import_annotations`)
