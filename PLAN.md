# Plan: Snapious zu einem Geneious-Prime-Nachbau ausbauen

Hobbyprojekt zum Lernen, nicht zum Vertrieb.

## Grundlage dieses Plans

- **Geneious-Seite:** Ich habe in der installierten App (Geneious Prime 2026.1.4) nur die **Namen** der gebündelten Plugins gelesen (`Contents/Resources/app/bundledPlugins`). Das sind rund 130 Plugins und zeigt, welche Funktionsbereiche es gibt. Code wurde **nicht** dekompiliert oder übernommen. Die Funktionen implementieren wir selbst aus öffentlichen Algorithmen und Dokumentationen.
- **Snapious-Seite:** Der vorhandene Code in `GeneiousRebuild` (16 Module, rund 8.000 Zeilen, 246 grüne Tests). Der Plan baut darauf auf und ersetzt ihn nicht.
- **REA:** Ist installiert und als MCP-Server `rea` in `~/.claude.json` eingetragen (Sicherung: `~/.claude.json.bak-before-rea`). Die Tools stehen nach einem Neustart der App bereit. Die native Analyse bräuchte zusätzlich Ghidra oder Hopper. Für diesen Plan war sie nicht nötig.

## Ist-Zustand: Geneious-Bereiche gegen Snapious

| Geneious-Bereich (Plugins) | Snapious heute | Lücke |
|---|---|---|
| Sequenz-Viewer, Annotationen, Feature Finder, Extract/Transfer Annotations | `Sequence`, `AnnotationParser`, Textausgabe | Grafische Sequenzansicht mit Annotationsspuren, Zoom, Auswahl: **fehlt** |
| Import/Export (FASTA, FASTQ, GenBank, EMBL, GFF, BED, SAM, VCF, Newick, Nexus, Phylip, Clustal, PIR, GCG, Mega, ACE, SnapGene, AB1) | `FileIO` (FASTA, FASTQ, GenBank, SAM, VCF, Newick) | EMBL, GFF-Export, BED, Nexus, Phylip, Clustal, AB1-Chromatogramme, SnapGene (`.dna`) |
| Alignment (MAFFT/MUSCLE/Clustal Omega, Profile Alignment, Translation Align, Realign Region, Strip Columns, Consensus) | `Alignment` (NW, SW), progressives MSA | Alignment-Viewer, bessere Heuristik, Aminosäure-Alignment, externe Tools einbinden |
| Phylogenie (Tree Builder, Tree Viewer) | `Phylogeny` (NJ, UPGMA, Parsimony), `TreeVisualizer` | Maximum Likelihood, Bootstrap-Anzeige, interaktiver Baum |
| Primer (Primer Design, Microsats) | `PrimerDesign` | Primer3-ähnliche Auswahl, Primer-Paare gegen Template prüfen, In-silico-PCR |
| Klonierung (Cloning, Cloning Validation, Gel, Clone Manager, Codon Optimization, CRISPR) | `RestrictionEnzymeDB`, `CloningMethod` | Klonierungs-Wizard mit Konstrukt, Gel-Simulation, Codon-Optimierung, gRNA-Design |
| NGS (Assembler, Mapper, FastQC, BBTools, SPAdes, Coverage, Variants, Chimera Filtering, Expression) | `CoverageAnalyzer`, `QualityScoreConverter` | Read-Mapping, De-novo-Assembly, Varianten-Calling, FastQC-artige Statistiken: **fast nur über externe Tools machbar** |
| Datenbanken (NCBI, UniProt, Pfam, InterPro, BLAST, Identify Organism) | `BLASTLikeSearch` (lokal) | Online-Zugriff auf NCBI Entrez/BLAST-API, UniProt-REST |
| Protein (Protein Statistics/Graphs, Jmol, RNAfold, Molecular Structure) | nur Translation | Protein-Eigenschaften (MW, pI, Hydropathie), 3D-Ansicht, RNA-Faltung |
| Graphen (GC, Sequenzlogo, Similarity, Lengths, Coverage) | Textausgaben | Diagramme in der Oberfläche |
| Dokumentverwaltung (Lokale DB, Ordner, Favoriten, Notes, Document History, Batch Rename, Duplikate, Operations Table) | `Map` im Speicher | **Persistente Projektverwaltung**: das größte Strukturproblem |
| Workflows, Plugins, Server-DB, Cloud, Tutorial | nichts | Workflow-Ketten, Plugin-API, später evtl. Server |

## Architekturentscheidungen, die zuerst fallen müssen

1. **Dokumentmodell:** Heute gibt es nur `Sequence`. Geneious arbeitet mit allgemeinen *Dokumenten* (Sequenz, Alignment, Baum, Chromatogramm, Liste, Tabelle). Ein gemeinsames `Document`-Interface mit Typ und Metadaten ist die Basis für alles andere.
2. **Persistenz:** Projekte lokal speichern, am besten SQLite (JDBC) oder einfach strukturierte Dateien. Ohne Persistenz ist die App nur ein Rechner.
3. **Operationen:** Jede Analyse (Alignment, Baum, Primer …) als Operation mit Parametern, Eingabedokumenten und Ergebnisdokumenten. Daraus ergeben sich Undo/Historie, Workflows und Hintergrundläufe fast von allein.
4. **Externe Tools statt Eigenbau:** MAFFT, MUSCLE, BLAST+, Primer3, IQ-TREE, BWA/minimap2 über Prozessaufruf einbinden. Das ist Geneious' eigener Weg (Clustal, MUSCLE und BBTools liegen dort gebündelt). Die eigenen Java-Implementierungen bleiben als Lernprojekt und Fallback ohne externe Programme.
5. **UI-Technik:** Entweder Swing beibehalten (Startaufwand klein, hässlicher) oder auf JavaFX wechseln (bessere Grafik für Sequenz-/Baumansichten). Empfehlung: **JavaFX**. Die Sequenzansicht ist der Kern des Programms und braucht Canvas-Zeichnung.

## Phasen (Reihenfolge nach dem Vergleich mit SnapGene)

Aufwandsangaben sind grobe Schätzungen für eine Person in der Freizeit (ca. 8-10 h/Woche). Jede Phase endet mit einer lauffähigen App, grünen Tests und einem kurzen Eintrag im README. Neue Funktionen bekommen Tests, bevor sie in der Oberfläche auftauchen. Als Testdaten dienen eigene kleine Dateien und öffentliche Plasmide (z. B. pUC19, aus GenBank).

### Phase 0: Fundament (ca. 3 Wochen)

**Ziel:** Die Architektur, auf der alles andere aufbaut.

| Aufgabe | Details |
|---|---|
| Paketstruktur | `model` (Dokumente), `ops` (Operationen), `store` (Persistenz), `ui` (JavaFX), `bio` (bestehende Algorithmen), `tools` (externe Programme) |
| `Document`-Interface | Typen: `SequenceDocument`, `AlignmentDocument`, `TreeDocument`, `TableDocument`. Gemeinsame Felder: ID, Name, Zeitstempel, Metadaten, Eltern-Dokumente. `Sequence` wird zum Kern von `SequenceDocument` (mit Annotationsliste und Topologie linear/zirkulär). |
| Annotationen | Eigene Klasse `Feature` (Typ, Position(en), Strang, Qualifier). Der `AnnotationParser` liefert daraus gespeist, `FileIO` schreibt sie zurück nach GenBank. |
| Operationen | `Operation<I,O>` mit Parametern, Eingabedokumenten, Ergebnisdokumenten, Fortschritt und Abbruch. Jede Ausführung erzeugt einen `HistoryEntry` (Operation, Parameter, Ein-/Ausgabe-IDs). |
| Persistenz | SQLite via JDBC: Tabellen für Dokumente, Features, Verlauf. Ein Projekt = eine Datei `*.snapious`. Import/Export bleiben Dateiformate. |
| Externe Tools | `ToolRegistry`: Pfade konfigurierbar, Version prüfen, Prozess mit Timeout starten, stdout/stderr erfassen. Tests mit einem Dummy-Programm. |
| UI-Entscheidung | JavaFX einführen (Modulpfad in der Maven-Konfiguration). Swing-Oberfläche bleibt bis Phase 1 lauffähig. |

**Fertig, wenn:** Eine Sequenz kann importiert, gespeichert, nach Neustart wieder geladen und nach GenBank exportiert werden, einschließlich Annotationen. Der Verlauf zeigt den Import. ≥ 40 neue Tests.

### Phase 1: Sequenz- und Kartenansicht (ca. 5 Wochen)

**Ziel:** Das SnapGene-artige Hauptfenster. Erstes wirklich sichtbares Ergebnis.

| Aufgabe | Details |
|---|---|
| Sequenzansicht | `Canvas`-basiert, virtualisiert (nur sichtbare Zeilen zeichnen, damit 1 Mb-Sequenzen flüssig bleiben). Zeilenumbruch, Basennummern, Komplementärstrang, Farbcodierung, Zoom. |
| Auswahl | Bereiche per Maus und Tastatur, Kopieren (Basen, FASTA, reverse-komplement), Positionsanzeige, Länge, GC-Gehalt und Tm der Auswahl live. |
| Annotationsspuren | Features als Pfeile mit Farbe und Beschriftung, Überlappungen auf mehrere Spuren, Klick selektiert die Basen. |
| Translation | Aminosäuren unter der Sequenz für gewählten Rahmen oder ein Feature, ORF-Anzeige (bestehendes `findORFs`). |
| Karte | Kreisförmig (Plasmid) und linear, mit Features, Enzymschnittstellen und Markierung der Auswahl. Karte und Sequenz sind synchron. |
| Enzym-Anzeige | Einzel- und Zweifachschneider (aus `RestrictionEnzymeDB`), filterbar nach Anbieter/Name, Anzeige in Karte und Sequenz. |
| Bearbeiten | Einfügen, Löschen, Ersetzen mit Undo/Redo (baut auf `SequenceEditor` und dem Verlauf auf), Features beim Editieren mitverschieben. |
| Feature-Erkennung | Eigene Feature-Datenbank (`features.json`: Promotoren, Resistenzgene, Tags, Origins) aus freien Quellen, Suche beider Stränge mit Toleranz für kleine Abweichungen. |
| Dateiformate | EMBL, GFF3, BED, Nexus, Phylip, Clustal importieren/exportieren, AB1-Chromatogramme lesen. Jeweils mit Testdatei. |

**Fertig, wenn:** pUC19 lässt sich laden, als Karte und Sequenz anzeigen, ein Feature bearbeiten, ein Enzym schneiden sehen und als GenBank speichern. Eine 1-Mb-Sequenz scrollt ohne Ruckeln.

### Phase 2: Klonierung und Primer (ca. 7 Wochen)

**Ziel:** Der gemeinsame Kern beider Vorbilder. Konstrukte virtuell bauen statt nur Texte auszugeben.

| Aufgabe | Details |
|---|---|
| Virtueller Verdau | Fragmente aus `RestrictionEnzymeDB.cut` mit echten Überhängen, Fragmentliste mit Länge, Auswahl einzelner Fragmente. |
| Restriktionsklonierung | Wizard: Vektor + Insert + Enzym(e) → Konstrukt als neues `SequenceDocument` mit übernommenen Features. Prüfung auf Kompatibilität (`isCompatibleWith`), Orientierung, Leserahmen. |
| Gibson / In-Fusion / SLIC | Overlap-Erkennung und -Design, mehrere Inserts, Primer mit Überhängen ausgeben, Konstrukt berechnen. |
| Golden Gate | Typ-IIS-Enzym wählen, Überhänge eindeutig prüfen (keine Doppelten), Konstrukt berechnen, Warnung bei internen Schnittstellen. |
| Gateway und TOPO | Rekombination `attB/attP/attL/attR`, TOPO/TA mit Überhängen. |
| Gel-Simulation | Spuren, Agarose-Prozent, Marker (z. B. 1 kb Ladder), Laufstrecke aus Fragmentgröße (log-lineares Modell). |
| PCR-Simulation | Primer binden prüfen (3'-Ende exakt, Mismatches im 5'-Bereich erlaubt), Produkt als Dokument. |
| Primer-Design | Eigene Auswahl (Tm, GC, Länge, GC-Klammer, Hairpin/Dimer) und optional Primer3 als Backend. Bestehende `PrimerDesign`-Klasse als Fallback. |
| Codon-Optimierung | Aus Codon-Nutzungstabellen (`CodonUsageAnalyzer`), mit Organismenauswahl, Vermeidung von Restriktionsstellen. |
| CRISPR | gRNA-Suche neben PAM (NGG und weitere), Off-Target-Zählung mit Mismatches im Genom/Plasmid, Score. |
| Sequenzierungsabgleich | Sanger-Ergebnis (AB1/FASTA) gegen das Konstrukt ausrichten, Abweichungen markieren (baut auf Alignment aus Phase 3, daher zuerst mit dem vorhandenen `Alignment`). |

**Fertig, wenn:** Ein Standardfall funktioniert komplett: pUC19 mit EcoRI/BamHI verdauen, ein Insert klonieren, das Konstrukt als Karte sehen, das Gel simulieren und Primer für die Kontroll-PCR erzeugen. Die Ergebnisse stimmen mit einer von Hand berechneten Referenz überein (Tests mit festen erwarteten Fragmentlängen).

### Phase 3: Alignment und Phylogenie (ca. 5 Wochen)

| Aufgabe | Details |
|---|---|
| Alignment-Viewer | Spaltenansicht, Farbschemata (Nukleotide, Aminosäuren), Konsens, Identitätsbalken, Sequenzlogo, Auswahl von Spalten und Zeilen, Sortieren. |
| Backends | MAFFT, MUSCLE und Clustal Omega über `ToolRegistry`; eigenes `MultipleSequenceAlignment` als Fallback ohne externe Programme. |
| Alignment-Werkzeuge | Profil-Alignment, Translation-Alignment (Codons), Spalten entfernen, Region neu ausrichten, Konsensus extrahieren. |
| Baumberechnung | NJ/UPGMA (vorhanden), zusätzlich IQ-TREE oder FastTree als Backend mit Bootstrap. Distanzmodelle JC69/K80/p-Distanz. |
| Baumansicht | Interaktiv: Wurzel setzen, Äste drehen/zusammenklappen, Support-Werte, Maßstab, Export als PNG/SVG und Newick. Ausbau des bestehenden `TreeVisualizer`. |
| Ähnlichkeitsgraph | Identitätsmatrix als Heatmap. |

**Fertig, wenn:** Aus zehn öffentlichen 16S-Sequenzen entsteht ein Alignment und ein Baum, beides mit MAFFT und mit dem eigenen Fallback. Beide Ergebnisse stimmen im Wesentlichen überein.

### Phase 4: Datenbanken und Protein (ca. 4 Wochen)

| Aufgabe | Details |
|---|---|
| NCBI | Entrez-E-Utilities: Suche und Download von GenBank-Einträgen, Rate-Limit beachten, Antworten lokal zwischenspeichern. |
| BLAST | Remote über die NCBI-BLAST-API und lokal über BLAST+ (`makeblastdb`, `blastn/blastp`). Trefferliste als Tabelle mit Alignment-Ansicht. Der eigene `BLASTLikeSearch` bleibt für kleine Datenmengen. |
| UniProt / Pfam / InterPro | Abruf per REST, Domänen als Features auf dem Protein. |
| Protein-Eigenschaften | Molekulargewicht, isoelektrischer Punkt, Extinktionskoeffizient, Hydropathie (Kyte-Doolittle) als Diagramm, Aminosäurezusammensetzung. |
| RNA-Faltung | ViennaRNA (`RNAfold`) als Backend, Anzeige der Struktur als Punkt-Klammer-Notation und einfaches Diagramm. |
| 3D-Struktur | Nur PDB-Datei anzeigen (Einbettung eines WebView mit 3Dmol.js oder Jmol), keine eigene Modellierung. |
| Organismus-Identifikation | BLAST-basiert, wenn ein Treffer eine Taxonomie liefert. |

**Fertig, wenn:** Ein GenBank-Eintrag lässt sich per Suche laden, die Proteinsequenz translatieren, in UniProt zuordnen und Eigenschaften anzeigen.

### Phase 5: NGS-Grundlagen (ca. 6+ Wochen, optional)

| Aufgabe | Details |
|---|---|
| Qualitätsbericht | FastQC-artige Statistik: Basenqualität pro Position, GC-Verteilung, Duplikate, Adapter. Baut auf `QualityScoreConverter` auf. |
| Trimming | Qualitäts-/Adapter-Trimming (`BBDuk`/`fastp` als Backend oder eigene einfache Variante). |
| Mapping | minimap2/BWA als Backend; BAM/SAM lesen, Coverage-Diagramm (bestehender `CoverageAnalyzer`), Reads in der Sequenzansicht. |
| Varianten | VCF aus `bcftools` lesen, Varianten in der Karte markieren, Tabelle mit Filter. Eigenes Calling nur als einfaches Pileup-Beispiel. |
| De-novo-Assembly | Nur Aufruf von SPAdes/Flye, Ergebnis als Contigs-Dokumente. |
| Große Dateien | Streaming und Indizes (`.fai`, `.bai`); Speichergrenzen testen. |

**Fertig, wenn:** Aus einem kleinen Beispiel-Datensatz (Phagen-Genom, einige tausend Reads) entsteht eine Coverage-Ansicht mit einer erkannten Variante.

### Phase 6: Workflows, Plugins und Politur (ca. 4 Wochen)

| Aufgabe | Details |
|---|---|
| Workflows | Operationen aus dem Verlauf zu einer wiederholbaren Kette speichern, mit Parametern, Stapelverarbeitung auf mehrere Dokumente, Export als JSON. |
| Plugin-API | `ServiceLoader`-basiert: Plugins liefern Operationen, Importer/Exporter und Ansichten. Ein Beispiel-Plugin im Repo. |
| Dokumentverwaltung | Ordner, Suche, Filter, Umbenennen im Stapel, Duplikate finden, Favoriten, Notizen zu Dokumenten. |
| Dokumenthistorie | Anzeige des Verlaufs pro Dokument mit Rückgriff auf frühere Versionen. |
| Politur | Tastenkürzel, Einstellungen, Drucken/PDF-Export der Karte, Dark-Mode, Installationspaket (`jpackage`) für macOS. |
| Qualität | Testabdeckung auswerten (JaCoCo), UI-Smoketests (TestFX), Performance-Tests für große Sequenzen. |

**Fertig, wenn:** Ein Workflow „Importieren → Verdauen → Ligieren → Karte exportieren" läuft per Knopfdruck auf drei Dateien.

## Zeitübersicht und Abhängigkeiten

| Phase | Dauer | Abhängig von | Parallel möglich mit |
|---|---|---|---|
| 0 Fundament | 3 Wochen | - | - |
| 1 Sequenz-/Kartenansicht | 5 Wochen | 0 | Teile von 3 (Alignment-Viewer) |
| 2 Klonierung/Primer | 7 Wochen | 0, 1 | - |
| 3 Alignment/Phylogenie | 5 Wochen | 0, 1 | 2 |
| 4 Datenbanken/Protein | 4 Wochen | 0, 1 | 3 |
| 5 NGS | 6+ Wochen | 0, 1, 3 | - |
| 6 Workflows/Politur | 4 Wochen | alle | - |

Gesamt etwa **9-12 Monate** in der Freizeit. Nach Phase 2 ist ein SnapGene-ähnlicher Kern fertig, nach Phase 3 und 4 die Geneious-Analysebreite.

## Qualitätsregeln für alle Phasen

- Jede Funktion bekommt Tests mit **bekannten Referenzwerten** (feste Fragmentlängen, Tm-Werte aus der Literatur, Ergebnisse von MAFFT/BLAST), keine Tests, die nur prüfen, dass nichts `null` ist. Das war das größte Problem im alten Code.
- Externe Programme werden in Tests nur verwendet, wenn sie installiert sind (`Assumptions.assumeTrue`), sonst wird übersprungen.
- Der Build muss nach jeder Aufgabe grün sein (`mvn test`).
- Algorithmen mit Quelle im Kommentar (Paper, Lehrbuch, Spezifikation), damit klar bleibt, woher sie stammen.

## Das Beste aus Geneious und SnapGene

SnapGene ist auf diesem Rechner nicht installiert. Grundlage sind die öffentlichen Seiten von [SnapGene](https://snapgene.com/features) und die Release Notes. Beide Programme haben ein unterschiedliches Profil:

| | Geneious Prime | SnapGene |
|---|---|---|
| Stärke | Breite: Alignment, Phylogenie, NGS, Datenbanken, Workflows | Tiefe bei Klonierung: Simulation, Plasmidkarte, Dokumentation |
| Datenmodell | Dokumente in Ordnern, Operationen mit Parametern | Eine Sequenzdatei mit Verlauf, jede Manipulation wird protokolliert |
| Bedienung | Viele Fenster und Dialoge | Eine zentrale Karte plus Sequenz, sehr geradlinig |
| Schwäche | Klonierung weniger bequem | Kaum Analyse jenseits der Klonierung |

**Was ich aus jedem übernehmen würde:**

| Aus Geneious | Aus SnapGene |
|---|---|
| Dokument- und Operationsmodell (Phase 0) | Plasmidkarte (kreisförmig und linear) als Hauptansicht |
| Alignment mit MAFFT/MUSCLE, Phylogenie, Baumansicht | Klonierungssimulation: Restriktion, Gibson (mehrere Inserts), Golden Gate, Gateway, TOPO, In-Fusion |
| NCBI/UniProt/BLAST-Anbindung | Virtuelles Gel (Spuren, Agarose-Prozent, Marker) |
| Workflows und Stapelverarbeitung | Automatische Feature-Erkennung aus einer Feature-Datenbank (eigene kuratierte Liste) |
| Importformat-Vielfalt | Verlauf jeder Änderung, ältere Konstrukte abrufbar |
| | „Align to Reference": Sanger-Ergebnisse gegen das simulierte Konstrukt prüfen |
| | PCR-Simulation und automatisches Primer-Design |

**Folgen für die Phasen:**
- **Phase 1** bekommt die SnapGene-artige Hauptansicht: Karte und Sequenz nebeneinander, Auswahl wird in beiden synchronisiert. Das ist wichtiger als ein Geneious-artiger Dokumentbaum.
- **Phase 2** ist die Klonierung und steht damit vor Alignment/Phylogenie (Phase 3). Sie ist der gemeinsame Kern beider Programme und baut direkt auf `RestrictionEnzymeDB` und `CloningMethod` auf.
- **Verlauf:** Das Operationsmodell aus Phase 0 speichert jede Änderung (Edit, Verdau, Ligation) mit Eingabe und Ergebnis. Daraus entstehen Undo, Historie und später Workflows.
- **Dateiformat:** SnapGenes `.dna`-Format ist proprietär. Lesen würde nur über öffentlich dokumentierte Teile gehen. Nicht eingeplant, GenBank/FASTA reichen.
- **Feature-Datenbank:** Sammle eigene Annotationen (Promotoren, Resistenzgene, Tags, Origins) aus freien Quellen wie Addgene- oder GenBank-Einträgen. SnapGenes kuratierte Datenbank ist urheberrechtlich geschützt und wird nicht kopiert.

## Detaillierte Spezifikationen für die Umsetzung

Für jede Phase gibt es eine ausführliche Anleitung mit nummerierten Aufgaben (Dateien, Signaturen, Algorithmen, Tests mit Referenzwerten). Sie sind so geschrieben, dass ein
Sprachmodell wie Qwen die Aufgaben der Reihe nach ohne Rückfragen umsetzen kann:

| Datei | Inhalt |
|---|---|
| [docs/plan/00-ARBEITSANWEISUNG.md](docs/plan/00-ARBEITSANWEISUNG.md) | Regeln, technischer Rahmen, vorhandener Code, Fortschrittsliste |
| [docs/plan/01-PHASE-0-FUNDAMENT.md](docs/plan/01-PHASE-0-FUNDAMENT.md) | Dokumentmodell, GenBank mit Features, Operationen/Verlauf, SQLite, externe Tools, JavaFX-Grundgerüst |
| [docs/plan/02-PHASE-1-ANSICHTEN.md](docs/plan/02-PHASE-1-ANSICHTEN.md) | Sequenzansicht, Annotationen, Karte, Enzyme, Bearbeiten/Undo, Feature-Erkennung, weitere Formate |
| [docs/plan/03-PHASE-2-KLONIERUNG.md](docs/plan/03-PHASE-2-KLONIERUNG.md) | Virtueller Verdau, Restriktion/Gibson/Golden Gate/Gateway/TOPO, Gel, PCR, Primer, Codons, CRISPR, Sequenzierungsabgleich |
| [docs/plan/04-PHASE-3-ALIGNMENT-PHYLOGENIE.md](docs/plan/04-PHASE-3-ALIGNMENT-PHYLOGENIE.md) | Alignment-Dokument, MAFFT/MUSCLE/Clustal, Konsens/Logo, Bäume, Bootstrap, Baumansicht |
| [docs/plan/05-PHASE-4-DATENBANKEN-PROTEIN.md](docs/plan/05-PHASE-4-DATENBANKEN-PROTEIN.md) | NCBI, BLAST, UniProt, Proteineigenschaften, RNA-Faltung, 3D |
| [docs/plan/06-PHASE-5-NGS.md](docs/plan/06-PHASE-5-NGS.md) | FASTQ/BAM, QC, Trimming, Mapping, Coverage, Varianten, Assembly |
| [docs/plan/07-PHASE-6-WORKFLOWS-POLITUR.md](docs/plan/07-PHASE-6-WORKFLOWS-POLITUR.md) | Workflows, Plugins, Dokumentverwaltung, Versionen, Einstellungen, Export, Verpackung |
| [docs/plan/08-RUST-KERN-SWIFT-APP.md](docs/plan/08-RUST-KERN-SWIFT-APP.md) | **Architektur:** Rust-Kern (alle Logik) + native Swift-App für macOS; Abbildung Java→Rust, FFI (UniFFI), Swift-Aufgaben S.0–S.9, spätere zweite Oberfläche für Windows/Linux |
| [docs/plan/09-QWEN-ARBEITSWEISE.md](docs/plan/09-QWEN-ARBEITSWEISE.md) | **Arbeitsweise mit Qwen3.6-35B-A3B:** Aufgabenkarten, Test-zuerst, Zerlegungsregeln, Rust-Fehler-Spickzettel, Prüfliste, verifizierte Beispielkarte 1.1 |
| [docs/plan/OFFENE_FRAGEN.md](docs/plan/OFFENE_FRAGEN.md) | Gesammelte Fragen und gewählte Annahmen |

## Bewusst weggelassen

Cloud, Server-Datenbank, Teilen-Funktion, Tutorial-Plugin, kommerzielle Importformate (Vector NTI, DNAStar) und die gesamte Lizenzverwaltung. Für ein privates Lernprojekt bringen sie nichts.

## Nächster konkreter Schritt

Phase 0 beginnen (siehe oben).

## Risiken und Hinweise

- **Lizenz:** Geneious' Nutzungsbedingungen verbieten üblicherweise Reverse Engineering. Dieser Plan nutzt deshalb nur Plugin-Namen und öffentlich bekannte Algorithmen. Wer tiefer in Geneious hineinschaut oder Code übernimmt, begibt sich in rechtliches Grau.
- **Umfang:** Geneious hat über viele Jahre ein ganzes Team entwickelt. Phase 1-3 ergeben einen brauchbaren Sequenz-Editor mit Klonierung. NGS ist mit vertretbarem Aufwand nur als Anbindung externer Tools zu haben.
- **Externe Tools** sind selbst lizenziert (BLAST+ gemeinfrei, MAFFT BSD, Primer3 GPL). Für ein privates Projekt unkritisch.
