# Vergleich mit Geneious Prime (Plugin für Plugin)

Quelle: die 113 mitgelieferten Plugins unter `/Applications/Geneious Prime.app/Contents/Resources/app/bundledPlugins`.
Vorgehen: Je Plugin die Klassen- und Beschriftungsliste aus dem Jar (Inventar, mit REA am Klonierungs-Plugin geprüft: 365 Klassen,
dieselben Namen) → Qwen Schritt A (nur die Funktionen des Plugins beschreiben, ohne Snapious zu nennen) → Qwen Schritt B (jede Funktion gegen die
Funktionsübersicht und Schnittstellenliste von Snapious prüfen). Skript: `scripts/qwen/audit/geneious_audit.py`, Rohausgaben
`scripts/qwen/audit/out/*.md`, alle Kern-Lücken `scripts/qwen/audit/kern_luecken.txt` (249 Zeilen, viele davon Oberflächen-Kleinkram).
Überwacht: 94 Plugins ausgewertet (Cloud, Server, Teilen, Tutorial u. ä. übersprungen).

**Verlässlichkeit:** Qwen irrt in Einzelfällen (z. B. meldete es „Stapelumbenennung fehlt“, obwohl `project/search.rs` sie hat). Die Liste unten
ist deshalb von mir gegen den Code geprüft (grep): bestätigt sind Rückübersetzung, Sequenz zusammenfügen, Enzymsets, Silent-Mutations-Analyse
(keine Treffer im Quelltext). Gateway/TOPO: Kern vorhanden (`cloning/gateway.rs`, `cloning/topo.rs`), aber keine Schnittstelle zur App.

## Lücken nach Wichtigkeit

### A. Molekularbiologie (Kern)
1. **Rückübersetzung** Protein → DNA (EMBOSS backtranseq, mit Codon-Tabelle) – fehlt.
2. **Gateway (BP/LR) und TOPO/TA in der App** – Kern da, Schnittstelle und Oberfläche fehlen; att-Sequenzen nicht eingetragen.
3. **Silent-Mutations-Analyse** (Geneious: SilentMutationGenerator) – fehlt.
4. **Enzymsets** (Gruppen von Enzymen speichern und verwenden), Schnitt nach Annotation/Index – fehlt bzw. teilweise.
5. **Sequenzen zusammenfügen** (Concatenate) – fehlt.
6. ~~Codon-Optimierung ohne bestimmte Schnittstellen~~ – Qwens Fehlurteil: `bio/codon_optimize.rs` hat `avoid_sites` schon; es fehlt nur der Zugriff aus der App.
7. **Mutationsfinder**: gefundene Varianten auf die Referenz anwenden und neue Sequenz erzeugen – fehlt.
8. **DNA-Faltung** und Primer-Sekundärstruktur (nur RNA vorhanden).
9. **Alignment-Spalten entfernen/maskieren** (Lücken, konstante Stellen, Codonposition 1/2/3, NEXUS-CHARSETs) – fehlt.
10. **Annotationen**: Satzvergleich (nur in A / in allen), Extrahieren aus Konsens, Feature-Finder nach Datenbankähnlichkeit,
    Annotationstabelle als Ansicht mit Filter/CSV – teilweise.
11. **Lokale BLAST-Datenbank aus Sequenzliste erstellen** (`makeblastdb`) und BLAST-Taxonomie – prüfen/teilweise.
12. **Identify Organism** mit mehreren Loci – fehlt.

### B. Sequenzdaten (NGS)
13. Barcode/MID-Aufteilung, Paired-Read-Verwaltung, Insert-Size-Auswertung.
14. Genexpression (RPKM/FPKM/TPM, DESeq2, Volcano/PCA), Chimärenfilter (UCHIME), Referenz-Assembly mit Introns/Fusionen.
15. Mikrosatelliten (Peak-Calling, Ladder, Allel-Binning).
16. Sequenzlängen-Histogramm, Alignment-Ähnlichkeitsgraph.

### C. Proteine und Strukturen
17. Domänen: Pfam, InterProScan (Netzwerk), Domänenansicht.
18. Antikörper/TCR-Annotation (VDJ, CDR, IMGT), Liabilities, klonale Gruppierung (Geneious „Biologics“).
19. 3D-Struktur: nur Cα-Spur; kein Ribbon/Ball-and-Stick, keine Zustände.
20. Strukturformate CML, MOL, XYZ, HIN, GPR, NWO.

### D. Dateiformate
21. Alignments/Sequenzen: MEGA, PileUp, GCG, PIR, RSF, ACE, DNA Strider, DNAStar, Vector NTI, Clone Manager (.cm5), Artemis-SQ.
22. Sequenzdaten: SFF, csfasta+QUAL, SCF, CAF, MAQ.
23. Export: QUAL, CSV/TSV (Tabellen), komprimiertes FASTA/FASTQ.
24. Tabellenimport (CSV/TSV/XLSX von Sequenzen und Primern).

### E. Weniger wichtig
- Gel-Bandenintensität, Chromatogramm-Skalierung nach Konfidenz, Alignment-Sortierung/Filter, Dokumentfilter nach Feldern.

## Nächste Schritte
Karten für Qwen (Rust-Kern mit Tests): 1, 3, 5, 6, 7, 9, 10 (Satzvergleich), 16, 21–23. Danach Schnittstelle und Oberfläche
(Claude) für 2, 4, 8, 11, 12. NGS- und Proteinpakete (13–20) als eigene Phase.

## Stand der Karten (Gruppe A, Rust-Kern)
Gerüste mit Tests liegen für: Rückübersetzung (`bio/backtranslate.rs`), Silent-Mutationen (`bio/silent_mutations.rs`), Alignment-Spalten (`bio/align_columns.rs`), Annotationsvergleich (`bio/annotation_compare.rs`), Varianten anwenden (`bio/apply_variants.rs`), Enzymsets (`cloning/enzyme_sets.rs`), Sequenzen zusammenfügen (`cloning/concatenate.rs`). Gateway/TOPO in der App und DNA-Faltung stehen noch aus.

## Stand der Karten (zweite Runde: Formate, NGS, Hilfsfunktionen)
Rust-Kern mit Tests (Qwen oder von Hand), noch nicht an Schnittstelle und App angeschlossen:
- Formate lesen: MEGA (`io/mega.rs`), PIR (`io/pir.rs`), GCG (`io/gcg.rs`), XYZ (`io/xyz.rs`), MOL V2000 (`io/mol.rs`), Tabellen-Import (`io/table_import.rs`).
- Schreiben: QUAL (`io/qual.rs`), CSV/TSV (`export/table.rs`, mit Lesen).
- NGS: Barcode-Zuordnung (`ngs/barcode.rs`), RPKM/TPM/log2 (`ngs/expression.rs`), Paired Reads (`ngs/pairs.rs`), Längenhistogramm (`ngs/histogram.rs`).
- Sonstiges: Fragmente aus Schnittpositionen/Annotationen (`cloning/cut_positions.rs`), Allel-Binning (`bio/allele_binning.rs`).
Qwen-Runner: `run_card.py` nutzt jetzt den Roh-Endpunkt ohne Denkphase (ca. 30 s statt 10 Minuten je Versuch); `QWEN_THINK=1` schaltet auf die Chat-Schnittstelle mit Denkphase zurück.

## Stand der Karten (dritte Runde)
Rust-Kern mit Tests, noch nicht an Schnittstelle und App angeschlossen:
- Formate: csfasta/QUAL (`io/csfasta.rs`), ACE (`io/ace.rs`), SCF v3 (`io/scf.rs`), SFF (`io/sff.rs`), CML (`io/cml.rs`).
- Auswertung: Ähnlichkeitsverlauf (`bio/similarity_graph.rs`), Annotationen ausschneiden (`bio/annotation_extract.rs`), Chimären (`bio/chimera.rs`), Mikrosatelliten-Peaks und Ladder (`bio/microsat.rs`), Identify Organism mit Gewichten (`bio/organism_loci.rs`), Feature-Treffer mit Fehlpaarungen (`bio/feature_match.rs`).
Qwen (ohne Denkphase) schaffte in dieser Runde nur 2 von 11 (`annotation_extract`, `microsat`); die übrigen neun habe ich selbst geschrieben. Muster: Randfälle der Spezifikation (NaN, Gleichstand, Grenzen) und Binärformate gelingen Qwen ohne Denkphase selten.

Angeschlossen an Schnittstelle und App (Runde 2 und 3): Import von csfasta, ACE (Contigs mit `read`-Features), SCF, SFF und der Formate der zweiten Runde; CML im Struktur-Dialog; Ähnlichkeitsverlauf und Chimärenprüfung im Alignment-Menü „Bearbeiten“; Features extrahieren und Feature-Suche per Ähnlichkeit im Analyse-Reiter; Fragmentgrößen (Größenstandard, Peaks) und Organismen-Rangliste im Werkzeuge-Reiter.

## Stand der Karten (vierte Runde) und Oberfläche
Qwen (8 grün: Schwachstellen, hmmscan-/InterProScan-/BLAST-Tabellen, Abdeckung, Duplikate, zuletzt geöffnet, SDF; 8 rot, von mir gelöst: Proteinkennzahlen, Statistik, Fragmentlängen, k-mer, Feature-Tabelle, Kürzel-Darstellung, HIN, Nussinov). Dabei fand ich einen Fehler in einer früheren Qwen-Karte (`normalize_keys` erkannte `alt` nicht).
Alles ist an Schnittstelle und App angeschlossen: Schwachstellen und Aliphatic Index/GRAVY im Protein-Reiter, Domänen und BLAST-Tabelle und Statistik im Werkzeuge-Reiter, Abdeckung, Duplikate, Fragmentlängen und k-mer im NGS-Reiter, einfache Faltung im RNA-Reiter, Filter und Sortierung der Feature-Tabelle, HIN und SDF im Struktur-Dialog.
Oberfläche: Befehlspalette (⌘K), aufgeräumte Toolbar, Aufgabengruppen statt 13 Reitern, Inspektor, Startseite mit zuletzt geöffneten Projekten, verständliche Fehlermeldungen, Tastenkürzel-Übersicht (⌘/).
Klonieren: TA und gerichtetes TOPO sowie Gateway (BP/LR, att-Stellen aus dem Kit-Handbuch einzutragen) in der Klonierungsansicht.

## Abschluss (letzte Runde)
- Konsensbaum nach der Mehrheitsregel (`bio/treeops.rs::consensus_tree`) und Bootstrap-Bäume in der App (Alignment-Menü „Bootstrap“, Mehrfachauswahl von Bäumen).
- Workflow-Editor mit Schritten (Operation wählen, ordnen, Parameter) neben der JSON-Ansicht.
- Hinweisleiste unten für Rückmeldungen, Beschriftungen für Bedienhilfen an den Symbolknöpfen, Aminosäuren ohne Basenfarben.
- In der App durchgeklickt: Protein-Reiter (Schwachstellen, Aliphatic Index, GRAVY), Werkzeuge (p-Wert-Korrektur), NGS (Abdeckung, k-mer), Klonieren (TA/TOPO, Gateway), Workflow-Editor.

### Bewusst offen geblieben
- Vector NTI, DNA Strider (kein offenes Format bekannt), Clone Manager .cm5.
- Pfam- und InterPro-Datenbank selbst (Netz bzw. große externe Daten); gelesen werden nur die Ausgaben der Programme.
- Antikörper-Nummerierung (IMGT), 3D mit Ribbon, Instabilitätsindex (400-Werte-Tabelle), att-Sequenzen für Gateway (aus dem Kit-Handbuch einzutragen), DNA-Faltung mit Energieparametern (nur die einfache Variante nach Nussinov).
- Fortschritt mit Abbrechen für lange Läufe, Rückgängig für Änderungen außerhalb der Sequenzansicht (Versionen in der Verwaltung gibt es).
- SnapGene-`.dna` mit einer echten Datei (keine vorhanden).

## Entfernt (Entscheidung des Nutzers)
Stammbäume (NJ/UPGMA/FastTree/IQ-TREE, Bootstrap, Konsensbaum, Newick), Taxonomie/Organismus-Ranglisten, Populationsgenetik
(Allel-Binning, Fragmentgrößen/Mikrosatelliten) und Chimären-Prüfung sind aus Kern, Schnittstelle und App entfernt.
Alte Projekte mit Baum-Dokumenten bleiben ladbar; die Bäume werden nur nicht mehr angezeigt. Kapitel in 04/08 sind historisch.
