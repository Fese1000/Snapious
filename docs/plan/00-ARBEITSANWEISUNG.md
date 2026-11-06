# Arbeitsanweisung (kurz; wird Qwen bei jeder Aufgabe als Systemtext mitgegeben)

Projekt: **Snapious**, ein Programm zum Ansehen, Bearbeiten und Klonieren von DNA-Sequenzen. Logik und Rechnen in **Rust** (`crates/snapious-core`), Oberfläche in Swift (nicht deine Aufgabe).
Referenz: Das Java-Projekt in `src/` (geprüfte Algorithmen und Testwerte). Lies nur die Aufgabe, die dir gegeben wurde.

## Regeln

1. Bearbeite **genau eine Aufgabe**. Ändere nur die Dateien, die in der Aufgabe stehen. Lösche und benenne nichts um.
2. Die Tests der Aufgabe sind vorgegeben. **Ändere sie nicht.** Dein Ziel: `cargo test -p snapious-core <modul>` ist grün.
3. Schreibe zuerst eine Funktion, dann `cargo check`. Erst wenn es kompiliert, die nächste Funktion.
4. Nach der Aufgabe: `cargo fmt`, `cargo clippy -p snapious-core -- -D warnings`, `cargo test -p snapious-core`. Alles muss grün sein.
5. Keine neuen Abhängigkeiten, kein `async`, kein `unsafe`, keine Makros außer `derive`, `vec!`, `format!`, `assert_eq!`.
6. Kein `unwrap()`/`expect()`/`panic!` im Bibliothekscode. Fehler mit `Result` und `?` weitergeben. Indizes immer prüfen (`get(i)`) oder `saturating_sub` verwenden.
7. Besitz einfach halten: Funktionen bekommen `&[u8]`/`&str`/`&T` und geben eigene Werte zurück. Kein `'a` in öffentlichen Strukturen. Im Zweifel `.clone()` oder `.to_vec()`.
8. Ausgaben deterministisch: kein Durchlaufen einer `HashMap` für Ergebnisse (sortiere oder nimm `BTreeMap`); Zufall nur mit festem Seed.
9. Kommentare (`///`) und Fehlermeldungen auf Deutsch, Namen auf Englisch (snake_case; Typen CamelCase).
10. Kein Code aus fremder oder proprietärer Software. Algorithmen nur wie in der Aufgabe beschrieben; Quelle als Kommentar.
11. **Du fragst nicht nach.** Ist etwas unklar: schreibe in `docs/plan/OFFENE_FRAGEN.md` unter „Fragen aus der Umsetzung" eine Zeile (Aufgabe, Frage, deine Annahme) und mache mit der Annahme weiter.
12. **Höchstens 3 Versuche** für einen Compiler- oder Testfehler. Danach: Stand lassen, in `OFFENE_FRAGEN.md` den Fehlertext (erste 20 Zeilen) eintragen und aufhören.

## Antwortformat

Gib am Ende nur diese Liste aus:
- Geänderte Dateien
- `cargo test`-Ergebnis (Anzahl grün/rot)
- Offene Punkte (falls vorhanden)

## Plattformneutral bleiben (wichtig für eine spätere Windows/Linux-Version)

Keine macOS-Aufrufe im Kern. Pfade nur mit `std::path::PathBuf` oder dem Crate `dirs`. Text immer UTF-8, Zeilenenden `\n` und `\r\n` akzeptieren.

## Fortschritt (hakt Claude ab)

- [x] Phase 0 Kern 0.1–0.6 (Claude, 25.10.2026): Feature-Modell, SequenceDocument, GenBank, Operationen/Verlauf, SQLite-Speicher, Programmaufrufe; 101 Tests grün
- [x] F.0 UniFFI (25.10.2026): `crates/snapious-ffi` (Objekt `Project`, 8 Tests), `scripts/build-ffi.sh`, Swift-Selbsttest `SnapiousSelfTest` (11 Prüfungen grün)
- [x] S.0 Swift-Fenster (25.10.2026): SwiftUI-App `app/Sources/SnapiousApp` (Projekt anlegen/öffnen, Import per Dialog oder Drag & Drop, Dokumentliste, Umbenennen/Löschen, Detailansicht mit Eckdaten, Feature-Tabelle, Sequenzvorschau); gebaut mit `scripts/bundle-app.sh` (nutzt die Xcode-Toolchain per DEVELOPER_DIR), von Hand an pUC19 geprüft
- [x] 0.9 Port der Referenzalgorithmen (Claude, 25.10.2026): Sequence, Alignment (global/lokal/semiglobal), MSA, Distanzen, Newick, Bäume (NJ, UPGMA, Parsimonie, Bootstrap, Baum-Werkzeuge), Restriktionsenzyme (79, beide Stränge, IUPAC), Primer (NN-Tm, Prüfungen, Auswahl), BLAST-ähnliche Suche, FASTA/FASTQ/SAM/VCF/Qualität, Coverage; 274 Tests grün. **Noch nicht portiert:** `AnnotationParser` (GFF3), `CodonUsageAnalyzer`, `SequenceEditor` (Teile stecken in `SequenceDocument`), Text-Planer `CloningMethod` (wird durch Phase 2 ersetzt).
- Dadurch bereits erledigt (nicht mehr an Qwen vergeben): Karten 2.8 (Tm/Prüfungen), 2.9 (Primer-Auswahl), 3.6 (Newick, Baum-Werkzeuge), 3.7 (Distanzen, NJ/UPGMA, Bootstrap, Parsimonie), 1.1 (Qwen), Teile von 2.1 (Enzymsuche inkl. Überhänge) und 4.? (lokale Suche).
- [x] Phase 1 Kern 1.1–1.8 (Qwen, von Claude geprüft und nachgebessert, 25.10.2026): Zeilenlayout, Selection, FeatureLayout, Translation, SelectionStats, Kartengeometrie, EnzymeMapModel, EditSession; 331 Tests grün, clippy sauber. 1.9 Feature-Erkennung (von Claude geschrieben, weil Qwen nur "denkt" und keinen Code ausgibt). 1.10 GFF3-Parser (Qwen, 2 Clippy-Zeilen von Claude). Offen: 1.11 Codon-Nutzung, Swift-Sequenzansicht S.2
- [x] Nacht 25./26.10.2026 (Qwen mit Claude-Nacharbeit, alle Tests grün, clippy sauber): Kern 1.9–1.11 (Feature-Erkennung, GFF3, Codon-Nutzung), 2.1 Fragmente/Verdau, 2.2 Ligation, 2.3 Gibson (`overlap.rs`, `overlap_primers.rs`), 2.4 Golden Gate, 2.5 Gateway/TOPO/TA, 2.6 Gel, 2.7 PCR, 2.10 Klonierungsprimer, 2.11 Codon-Optimierung, 2.12 CRISPR, 2.13 Sequenzierungsabgleich, 3.1 Alignment-/Baum-Dokumente samt Speicher, 3.2 Clustal/PHYLIP, 3.4 Alignment-Kennzahlen, 3.8 Baumlayout/SVG, 4.5 Proteineigenschaften, Genetische Codes, Sequenzstatistik, Proteinverdau, Primer-Annotation; Swift: Sequenzansicht mit Bearbeiten/Rückgängig, Karte, Verdau mit Gel, CRISPR, Klonieren, Primer, Analyse (ORFs/Motive), Alignment- und Baumansicht, Gibson/Golden Gate über Mehrfachauswahl.
- [x] Nachtlauf Teil 2 (26.10.2026, 540 Rust-Tests grün, clippy -D warnings sauber, Swift-Selbsttest grün): Datenbanken (`db/ncbi.rs`, `db/uniprot.rs`, Abruf über `curl`), ABIF-Spuren (`io/abif.rs`), Sequenzierabgleich (`seqcheck.rs`), BLAST lokal (`bio/blast.rs`), externe Aligner/Baumprogramme (`bio/backends.rs`: MAFFT, MUSCLE, Clustal Omega, FastTree, IQ-TREE), RNA-Faltung (`bio/rna_fold.rs`), Motivsuche (`bio/motif.rs`), Karten-SVG (`view/map_svg.rs`), Alignment/Baum als Projektdokumente samt Export; Integrationstest mit echtem pUC19 (`tests/cloning_puc19.rs`). Oberfläche: Tabs Sequenz (Suche, Aminosäurezeile, Bearbeiten), Karte, Verdau+Gel, CRISPR, Klonieren, Primer, Analyse, Protein, Sequenzierung, RNA; NCBI-/UniProt-Dialog; Alignment-Bearbeitung. Bildschirmfotos ohne Fenster: `Snapious --snapshot <Projekt> <Ordner>`.
- [x] Nachtlauf Teil 3 (26.10.2026, 565 Rust-Tests grün): Operationsverzeichnis mit 14 Operationen (`ops/registry.rs`, `ops/more.rs`), Workflows mit JSON, Speicherung im Projekt und Stapellauf (`ops/workflow.rs`, Dialog „Workflows“ in der App), SnapGene-Import `.dna` (`io/snapgene.rs`, nur nach Blockbeschreibung geprüft, nicht an echten Dateien), GFF3-Export und GFF3/BED-Import als Features, Features aus der Auswahl eintragen/löschen in der App.
- [x] Nachtlauf Teil 4 (26.10.2026, 603 Rust-Tests grün): Laborrechner (`bio/calc.rs`), Mutagenese-Primer (`primer/mutagenesis.rs`), Alignment-Varianten (`bio/variants.rs`), Wiederholungen (`bio/repeats.rs`), Dotplot (`bio/dotplot.rs`), EMBL (`io/embl.rs`), Contig-Zusammenbau (`bio/assembly.rs`), Read-Abbildung (`bio/read_map.rs`); in der App: Tab „Werkzeuge“ (Mutagenese, Wiederholungen, Rechner), Varianten-Tabelle im Alignment, Contigs/Abbildung in der Mehrfachauswahl, EMBL-Export, Reiter-Überlaufmenü.
- [x] Phase 0 (Abgleich 26.10.2026): 0.1–0.6 erledigt; 0.7 JavaFX entfällt (ersetzt durch S.0); 0.8 Abschluss: README ist neu geschrieben, Testzahl weit über 40.
- [x] Phase 1 (Abgleich und Lücken geschlossen 26.10.2026): 1.1–1.11 erledigt. NEXUS (`io/nexus.rs`, Export in der App), Feature-Datenbank mit 17 Vorlagen aus `view/standard-features.json` (inkl. AmpR, ori, lacZα aus pUC19), 1-Mb-Erkennung 0,08 s. Offen nur die Handprüfung des Scrollens einer 1-Mb-Sequenz in der Oberfläche.
- [x] Phase 2 Abgleich (26.10.2026): Lücken geschlossen: `model/rotate.rs` (Ursprung drehen), `bio/kazusa.rs` (Kazusa-Import, Codon-Tabellen als JSON), `ops/phase2.rs` (clone.restriction, pcr.simulate, codon.optimize; Registry jetzt 17 Operationen). Qwen lieferte alle drei im Kern, Claude ergänzte Importe/Kleinigkeiten. Offen: Gateway-Operationen (echte att-Sequenzen fehlen), primer.pick/crispr.find/seq.check als Operationen, ClonePipelineIT.
- [x] Phase 3 Abgleich (26.10.2026): Lücken geschlossen: `bio/align_tools.rs` (Profile verbinden, Translations-Alignment, Bereich neu ausrichten), `ops/phase3.rs` (align.profile, align.translate, align.realign, tree.reroot, tree.collapse; Registry jetzt 22 Operationen). Qwen scheiterte hier an beiden Karten (Zeitüberschreitung, Denkzeit), Claude schrieb beide. Danach: Identitäts-Heatmap und Sequenzlogo (Alignment-Menü, `AlignmentCharts.swift`) und `tests/align_tree.rs` (AlignTreeIT) ergänzt.
- [x] Phase 4 Abgleich (26.10.2026, alle fünf Karten von Qwen, grün): `db/cache.rs` (Zwischenspeicher, Anfragenbremse), `bio/blast_remote.rs` (NCBI-BLAST-API mit Wartezeiten), `db/uniprot_entry.rs` (Eintrag → Protein-Dokument mit Features), `io/pdb.rs` (PDB-Parser), `ops/phase4.rs` (protein.properties, rna.fold; Registry 24 Operationen). Danach erledigt: 3D-Spur (`StructureView`, ohne Netz), Cache und Remote-BLAST in FFI/Oberfläche, db.ncbi.fetch und db.uniprot.fetch (Qwen).
- [x] Phase 5 NGS (26.10.2026, 696+ Rust-Tests grün): `ngs/` mit FASTQ-Strom (`fastq_stream`), FASTA-Index (`fai`), QC-Bericht (`qc`), Trimming (`trim`), BAM-Leser über gzip (`bam`), SNV-Erkennung (`pileup`), Mapper-/SPAdes-Aufrufe (`mapper`, `spades`); `tests/ngs_pipeline.rs`. Qwen schaffte trim und spades, die übrigen sechs schrieb Claude nach drei Fehlversuchen. Oberfläche: Tab „NGS“ (QC-Diagramm mit Ampeln, Trimmen, Varianten aus SAM/BAM mit VCF). Offen: Mapping-Dialog, Coverage-Spur unter der Sequenz, externe-Dateien-Tabelle.
- [x] Phase 6 (27.10.2026, 736 Rust-Tests + Zufallstests + Selbsttest grün): Kern `store/folders.rs` (Ordner, Notizen, Favoriten), `store/versions.rs` (50 Versionen je Dokument, Wiederherstellen), `store/external.rs` (verknüpfte Dateien), `project/search.rs` (Suche, Stapelumbenennung, Dubletten), `settings.rs`, `export/text.rs`, `plugin.rs` (Plugins als Programme), `ops/record.rs` (Workflow aus Verlauf), `ops/analysis.rs`, `cloning/site_check.rs`, `bio/codon_tables.rs` (E. coli, Mensch, Hefe, Maus aus Kazusa). Oberfläche: Fenster „Verwaltung“ (Ordner, Suche, Dokument mit Notiz/Versionen/Workflow-Aufzeichnung/Export, Einstellungen, Plugins), NGS-Tab mit Mapping (minimap2/BWA), verknüpften Dateien und Coverage-Spur. Qwen schaffte in Phase 6 nur Einstellungen und Codon-Tabellen, den Rest schrieb Claude nach drei Fehlversuchen.\n- [x] Qualitätsdurchgang (27.10.2026): Zufallstests `tests/fuzz_parser.rs` (22 Dateileser mit beschädigten Eingaben), `tests/fuzz_compute.rs` (Rechenfunktionen), FFI-Test mit extremen Werten; gefundene und behobene Fehler: PDB-Leser bei Mehrbyte-Zeichen, Überlauf und Endlosschleife bei Wiederholungssuche mit riesigen Parametern; echter Mapping-Test mit minimap2 (`tests/ngs_mapping.rs`).

## Testdaten

Unter `crates/snapious-core/testdata/`: `mini.gb` (60 bp, siehe Phase 0.3), `puc19.gb` (NCBI `L09137`, 2686 bp; Herkunft in `testdata/README.md`).
Eigene Testsequenzen stehen immer direkt in der Aufgabe.
