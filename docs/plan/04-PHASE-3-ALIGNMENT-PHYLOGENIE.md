# Phase 3: Alignment und Phylogenie

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: Alignment-Dokumente mit Viewer-Logik, externe Alignment-Programme (MAFFT, MUSCLE, Clustal Omega) mit eigenem Fallback,
Alignment-Werkzeuge, Baumberechnung (NJ/UPGMA, IQ-TREE/FastTree, Bootstrap) und eine interaktive Baumansicht.

Neue Pakete: `align/` (Dokument, Backends, Werkzeuge, Konsens), `tree/` (Dokument, Newick, Operationen, Layout), `view/` (Layout-Logik), `fx/`.
Regeln aus `00-ARBEITSANWEISUNG.md` gelten weiter (Referenzwerte, externe Programme nur mit `Assumptions`).

---

## Aufgabe 3.1: AlignmentDocument und TreeDocument

**Voraussetzung:** Phase 0.

**Dateien:** `model/AlignmentDocument.java`, `model/TreeDocument.java`, Erweiterung von `store/ProjectStore` (neue Dokumenttypen `alignment`, `tree`),
Tests `model/AlignmentDocumentTest.java`, `store/ProjectStoreAlignmentTest.java`.

```java
public final class AlignmentDocument implements Document {
    public AlignmentDocument(String id, String name, Sequence.SequenceType type,
                             List<String> rowNames, List<String> rows, Map<String,String> metadata, Instant created);
    // Alle rows gleich lang (sonst IllegalArgumentException), Zeichen: Sequenzalphabet + '-' (Gap) + 'N'/'X'
    public int rowCount(); public int columnCount();
    public String row(int i); public String rowName(int i);
    public String ungapped(int i);                         // ohne '-'
    public char at(int row, int column);
    public AlignmentDocument withRows(List<String> rowNames, List<String> rows);
    public AlignmentDocument removeRows(Set<Integer> indices);
    public AlignmentDocument removeColumns(BitSet columns);
    public AlignmentDocument subAlignment(int fromColumn, int toColumn);       // exklusiv
    public AlignmentDocument reorder(List<Integer> order);
}

public final class TreeDocument implements Document {
    public TreeDocument(String id, String name, Phylogeny.TreeNode root, boolean rooted,
                        Map<String,String> metadata, Instant created);
    public Phylogeny.TreeNode root(); public boolean rooted();
    public String toNewick();                              // über TreeNode.toNewick() + ";"
}
```

Serialisierung im `ProjectStore`-JSON: Alignment = `{type, rowNames[], rows[]}`; Baum = `{rooted, newick}` (Baum wird beim Laden aus Newick neu aufgebaut, siehe 3.6).

**Tests:** Zeilen ungleicher Länge → Exception. `ungapped(0)` von `AC--GT` = `ACGT`. `removeColumns({2,3})` auf den Zeilen `AC--GT` und `AG--GT` → `ACGT` und `AGGT`.
`subAlignment(1,3)` liefert Spalten 1 und 2. Round-trip durch `ProjectStore` (Zeilen, Namen, Typ identisch).

---

## Aufgabe 3.2: Alignment-Formate (Lesen/Schreiben)

**Voraussetzung:** 3.1, Phase 1.10.

**Dateien:** Erweiterung der Formate `ClustalFormat`, `PhylipFormat`, `NexusFormat` auf `AlignmentDocument`; neu `AlignedFastaFormat` (FASTA mit Gaps), Tests.

- `AlignedFastaFormat.parse(String)`: wie FASTA, aber `-` und `.` bleiben (`.` → `-`); alle Zeilen müssen gleich lang sein, sonst `IOException`.
- `ClustalFormat.write(AlignmentDocument)`: Kopfzeile `CLUSTAL W multiple sequence alignment`, Blöcke à 60 Spalten, Namen auf 16 Zeichen linksbündig, Konservierungszeile (`*` = identisch in allen Zeilen, sonst Leerzeichen).
- `PhylipFormat.write`: interleaved, Namen auf 10 Zeichen gekürzt/aufgefüllt; Namenskonflikte (gleiche ersten 10 Zeichen) → `IOException`, außer `relaxed=true`.
- `NexusFormat.write`: `#NEXUS`, `BEGIN DATA; DIMENSIONS NTAX=n NCHAR=m; FORMAT DATATYPE=DNA MISSING=? GAP=-; MATRIX … ; END;`.

**Tests:** je ein Round-trip mit 3 Zeilen × 12 Spalten, mit Gaps; Clustal-Konservierungszeile für Spalten `A/A/A` → `*`, `A/A/C` → Leerzeichen; Phylip mit zu langem Namen → gekürzt, Konflikte erkannt;
Nexus mit Kommentar `[…]` im Kopf wird korrekt gelesen.

---

## Aufgabe 3.3: Alignment-Backends

**Voraussetzung:** 3.1, Phase 0.6 (`ToolRegistry`).

**Dateien:** `align/Aligner.java`, `align/AlignmentRequest.java`, `align/MafftAligner.java`, `align/MuscleAligner.java`, `align/ClustalOmegaAligner.java`,
`align/BuiltinAligner.java`, `align/AlignerFactory.java`, Tests.

```java
public record AlignmentRequest(List<Sequence> sequences, boolean protein, Map<String,String> options) {}

public interface Aligner {
    String name();
    boolean isAvailable();
    AlignmentDocument align(AlignmentRequest request, OperationContext ctx) throws OperationException;
}

public final class AlignerFactory {
    public static Aligner choose(String preferred, ToolRegistry tools);    // "mafft", "muscle", "clustalo", "builtin"; nicht verfügbar → BuiltinAligner
    public static List<Aligner> all(ToolRegistry tools);
}
```

**Aufruf (alle in `Files.createTempDirectory`; Eingabe als FASTA mit eindeutigen Namen `seq1`, `seq2`, … und Namensabbildung zurück):**

| Programm | Kommando | Ausgabe |
|---|---|---|
| MAFFT | `mafft --auto --quiet --thread 1 in.fa` | stdout = aligned FASTA |
| MUSCLE ≥ 5 | `muscle -align in.fa -output out.fa` | `out.fa` |
| MUSCLE 3.x | `muscle -in in.fa -out out.fa` | `out.fa` |
| Clustal Omega | `clustalo -i in.fa -o out.fa --force --outfmt=fa` | `out.fa` |

MUSCLE-Version zuerst mit `muscle -version` erkennen (Text beginnt mit `muscle 5` → neue Syntax). Ausgabe mit `AlignedFastaFormat` lesen und anhand der Namensabbildung
in **Eingabereihenfolge** zurückordnen. Optionen: MAFFT `--maxiterate <n>` über `options["iterations"]`. Exitcode ≠ 0 → `OperationException` mit stderr (ersten 300 Zeichen).
`BuiltinAligner` ruft `MultipleSequenceAlignment.progressiveAlignment` und wandelt in `AlignmentDocument`.

**Tests:**
- `BuiltinAligner` auf `ATGCGTAC`, `ATGAGTAC`, `ATGCGTGC` → 3 Zeilen, alle gleich lang, Entfernen der Gaps ergibt wieder die Eingabe (Invarianz).
- `AlignerFactory.choose("mafft", leereTools)` → `BuiltinAligner`.
- Für jedes tatsächlich installierte Programm (`Assumptions.assumeTrue(isAvailable)`): Alignment von 4 verwandten Sequenzen (je ~60 bp, eine Lücke in einer) → Zeilen gleich lang, `ungapped` entspricht Eingabe, Reihenfolge wie Eingabe.
- Fehlerfall: leere Eingabe → `OperationException`.
- Mit einem **Fake-Programm** (Shell-Skript in `@TempDir`, das FASTA unverändert ausgibt) das Parsen und die Namensabbildung testen, ohne echte Tools.

---

## Aufgabe 3.4: Konsens, Konservierung, Identitätsmatrix, Sequenzlogo

**Voraussetzung:** 3.1.

**Dateien:** `align/Consensus.java`, `align/Conservation.java`, `align/IdentityMatrix.java`, `align/SequenceLogo.java`, Tests.

```java
public final class Consensus {
    /** Pro Spalte: häufigstes Zeichen, wenn sein Anteil (ohne Gaps gerechnet gegen alle Zeilen) >= threshold, sonst IUPAC-Mehrdeutigkeitscode (DNA) bzw. 'X' (Protein); reine Gap-Spalte → '-'. */
    public static String compute(AlignmentDocument aln, double threshold, boolean ignoreGaps);
    public static char iupac(Set<Character> bases);        // {A,G}→R, {C,T}→Y, {G,T}→K, {A,C}→M, {C,G}→S, {A,T}→W, {C,G,T}→B, {A,G,T}→D, {A,C,T}→H, {A,C,G}→V, alle vier→N
}
public final class Conservation {
    public static double[] identity(AlignmentDocument aln);          // Anteil der Zeilen mit häufigstem Nicht-Gap-Zeichen, 0..1
    public static double[] entropyBits(AlignmentDocument aln);       // Shannon-Entropie in Bit je Spalte (ohne Gaps)
}
public final class IdentityMatrix {
    public static double[][] percent(AlignmentDocument aln);         // Paarweise Identität in % (identische Nicht-Gap-Spalten / Spalten, in denen beide kein Gap haben)
}
public final class SequenceLogo {
    public record Column(char[] symbols, double[] bits) {}           // sortiert aufsteigend nach Höhe
    public static List<Column> compute(AlignmentDocument aln, boolean applySmallSampleCorrection);
}
```

Logo: `R_i = log2(s) − (H_i + e_n)` mit `s` = 4 (DNA) bzw. 20 (Protein), `H_i` Entropie, `e_n` Kleinprobenkorrektur `(s−1)/(2 ln2 · n)`; Höhe des Symbols = Häufigkeit × `R_i`
(Schneider & Stephens 1990). Ohne Korrektur `e_n = 0`.

**Tests:**
- Zeilen `ACGT`, `ACGT`, `ACGA`: In Spalte 4 hat `T` den Anteil 2/3 ≈ 0,667. `Consensus.compute(aln, 0.7, true)` = `ACGW` (T und A → IUPAC `W`), `compute(aln, 0.5, true)` = `ACGT`.
- `Consensus.iupac(Set.of('A','G'))='R'`; `Set.of('A','C','G','T')='N'`.
- Identität: Spalte mit `A,A,A` → 1.0; `A,A,C` → 0.6667 (±1e-4); `A,-,-` → 1.0.
- Entropie: `A,C,G,T` (je einmal) → 2.0 Bit; `A,A,A,A` → 0.0.
- Identitätsmatrix: Diagonale 100; `ACGT` gegen `ACGA` → 75; Spalten mit Gap in einer Zeile werden nicht gezählt (`AC-T` vs `ACGT` → 100).
- Logo: vier gleich häufige Basen → Gesamthöhe 0 (ohne Korrektur); eine einzige Base → Höhe 2 Bit.

---

## Aufgabe 3.5: Alignment-Werkzeuge

**Voraussetzung:** 3.3, 3.4.

**Dateien:** `align/tools/ProfileAlignment.java`, `align/tools/TranslationAlignment.java`, `align/tools/StripColumns.java`,
`align/tools/RealignRegion.java`, `align/tools/ExtractConsensus.java` + Operationen (`align.profile`, `align.translate`, `align.strip`, `align.realign`, `align.consensus`), Tests.

- **ProfileAlignment:** zwei bestehende Alignments zu einem verbinden, ohne die Gap-Struktur innerhalb der Alignments zu ändern.
  Die vorhandene private Methode `MultipleSequenceAlignment.mergeProfiles` wird zu `public static AlignmentDocument alignProfiles(AlignmentDocument a, AlignmentDocument b)`
  (Verhalten gleich; bestehende Tests grün lassen). Gap-Spalten entstehen nur als reine Einfügungen in der jeweils anderen Gruppe.
- **TranslationAlignment:** Nukleotidsequenzen (Länge Vielfaches von 3, sonst Exception mit Sequenzname) → translatieren (Rahmen 1, `Sequence.translate`) → Proteinalignment (über gewählten `Aligner`)
  → Rückübersetzung: jede Aminosäure wird durch ihr Original-Codon ersetzt, jede Protein-Gap-Spalte durch `---`. Ergebnis: DNA-Alignment (Spaltenzahl = 3 × Protein-Spalten).
- **StripColumns:** Parameter `gapFractionThreshold` (0..1): entferne Spalten, in denen der Anteil der Gaps ≥ Schwelle ist (Schwelle 1.0 = nur reine Gap-Spalten; 0.0 = alle Spalten mit mindestens einem Gap).
- **RealignRegion:** Spaltenbereich `[from,to)` und Zeilenauswahl → die ungappten Teilsequenzen dieser Zeilen im Bereich neu mit gewähltem `Aligner` ausrichten und zurückschreiben; Spaltenanzahl des Alignments darf sich ändern
  (alle übrigen Zeilen im Bereich werden mit Gap-Spalten aufgefüllt, links/rechts bleibt alles unverändert).
- **ExtractConsensus:** erzeugt ein `SequenceDocument` (Name `<Alignment> Konsens`) aus `Consensus.compute` ohne Gaps.

**Tests:**
- Profile: `A = {ACGT, ACGT}`, `B = {AGT}` → Ergebnis hat 3 Zeilen, `ungapped` jeder Zeile unverändert; die beiden ersten Zeilen sind weiterhin gleich zueinander.
- Translation Alignment: `ATGAAATAG`, `ATGAAGTAG` → Spaltenzahl 9, Rückübersetzung jeder Zeile ergibt die Eingabe; Länge nicht Vielfaches von 3 → `OperationException` mit Namen.
- Strip: Alignment mit Spalte 2 nur Gaps → bei Schwelle 1.0 entfernt; bei einer Spalte mit 1 Gap von 4 Zeilen und Schwelle 0.5 bleibt sie, bei 0.25 wird sie entfernt.
- RealignRegion mit `BuiltinAligner`: Zeilen unverändert außerhalb; `ungapped` jeder Zeile gleich dem Original.
- ExtractConsensus enthält keine `-`.

---

## Aufgabe 3.6: Newick-Parser, TreeDocument-Werkzeuge

**Voraussetzung:** 3.1.

**Dateien:** `tree/Newick.java`, `tree/TreeOps.java`, Tests. (Die vorhandene `Phylogeny.TreeNode` wird weiterverwendet; Ergänzungen dort sind erlaubt:
`setSupport`, `getSupport` sind vorhanden.)

```java
public final class Newick {
    public static Phylogeny.TreeNode parse(String text);          // ein Baum, endet mit ';'
    public static List<Phylogeny.TreeNode> parseAll(String text); // mehrere
    public static String write(Phylogeny.TreeNode root, int decimals);   // Locale.ROOT
}
```

Grammatik: `node := ( '(' node (',' node)* ')' )? label? (':' length)?`; Labels dürfen in `'…'` stehen (mit `''` als Escape), Unterstriche bleiben, Kommentare `[ … ]`
werden ignoriert; ein Label an einem **inneren** Knoten wird als Support-Wert (`setSupport`, falls numerisch) bzw. Knotenname gelesen. Fehlerhafte Klammerung →
`IllegalArgumentException` mit Position.

```java
public final class TreeOps {
    public static Phylogeny.TreeNode midpointRoot(Phylogeny.TreeNode unrootedRoot);
    public static Phylogeny.TreeNode rootWithOutgroup(Phylogeny.TreeNode root, Set<String> outgroupLeaves);
    public static Phylogeny.TreeNode ladderize(Phylogeny.TreeNode root, boolean ascending);
    public static Phylogeny.TreeNode rotate(Phylogeny.TreeNode root, Phylogeny.TreeNode node);   // Kinder umkehren
    public static Phylogeny.TreeNode collapseBelowSupport(Phylogeny.TreeNode root, double minSupport);
    public static List<Set<String>> clades(Phylogeny.TreeNode root);       // Blattmengen aller inneren Knoten (ohne Wurzel, ohne triviale)
    public static double robinsonFoulds(Phylogeny.TreeNode a, Phylogeny.TreeNode b);   // Anzahl unterschiedlicher Bipartitionen
}
```

**Tests:**
- `parse("((A:0.1,B:0.2)90:0.05,C:0.3);")` → Wurzel mit zwei Kindern; erster Innenknoten hat Support 90 und Astlänge 0,05; `write(...)` liefert denselben Text zurück (Dezimalstellen 2 oder 1 passend wählen) — Round-trip über `parse(write(x))` ist gleich in Topologie und Längen.
- Quoted: `('Homo sapiens':1,B:2);` → Blattname `Homo sapiens`.
- Fehler: `(A,B` → `IllegalArgumentException`.
- Midpoint-Root: Baum `((A:1,B:1):1,C:5);` (als ungewurzelt behandeln). Der längste Abstand ist A–C = 7, der Mittelpunkt liegt 3,5 von C entfernt, also auf dem Ast zu C (1,5 vom Verzweigungspunkt). Nach dem Rooten haben Wurzel→C und Wurzel→A beide den Abstand 3,5 (±1e-9).
- `rootWithOutgroup({"C"})`: C ist Schwester des Rests.
- `collapseBelowSupport(…, 95)` entfernt den Innenknoten mit Support 90 (A und B werden direkt Kinder der Wurzel), Astlängen addieren sich.
- `robinsonFoulds` zweier gleicher Bäume = 0; `((A,B),(C,D))` vs `((A,C),(B,D))` = 2.
- Ladderize ordnet nach Anzahl Blätter.

---

## Aufgabe 3.7: Baumberechnung

**Voraussetzung:** 3.1, 3.3, 3.6.

**Dateien:** `tree/TreeBuilder.java`, `tree/DistanceModel.java`, `tree/Bootstrap.java`, `tree/IqTreeBackend.java`, `tree/FastTreeBackend.java`, Operation `tree.build`, Tests.

```java
public enum DistanceModel { P_DISTANCE, JUKES_CANTOR, KIMURA_2P, POISSON }      // POISSON für Proteine
public enum TreeMethod { NEIGHBOR_JOINING, UPGMA, IQTREE, FASTTREE }

public final class TreeBuilder {
    public TreeBuilder(ToolRegistry tools);
    public TreeDocument build(AlignmentDocument aln, TreeMethod method, DistanceModel model,
                              int bootstrapReplicates, long seed, OperationContext ctx) throws OperationException;
}
public final class Bootstrap {
    /** Felsenstein-Bootstrap für Distanzmethoden: Spalten mit Zurücklegen ziehen, Baum bauen, Bipartitionshäufigkeit in % auf den Referenzbaum schreiben. */
    public static void annotate(Phylogeny.TreeNode referenceTree, AlignmentDocument aln,
                                java.util.function.Function<AlignmentDocument, Phylogeny.TreeNode> builder,
                                int replicates, long seed);
}
```

**Distanzen** (Alignment-Spalten mit Gap in einer der beiden Zeilen werden paarweise ausgelassen): `p = Unterschiede / verglichene Spalten`;
JC69: `d = −3/4 ln(1 − 4/3 p)` (bei `p ≥ 0.75` → `Double.POSITIVE_INFINITY`, dann auf 10 begrenzen); K2P: `d = −1/2 ln(1 − 2P − Q) − 1/4 ln(1 − 2Q)` mit Transitionen `P` und Transversionen `Q`;
Poisson (Protein): `d = −ln(1 − p)`. Die vorhandenen Methoden in `Phylogeny` (`jukesCantorDistance`, `kimura2ParamDistance`) dürfen verwendet werden, wenn ihre Tests die Werte bestätigen — sonst neu implementieren.

**Backends:**
- IQ-TREE: `iqtree2 -s aln.fa -m MFP -B <n> --prefix out -nt 1 -redo` (ohne Bootstrap ohne `-B`); Ergebnis `out.treefile` (Newick, Support an inneren Knoten). Programmname in dieser Reihenfolge suchen: `iqtree3`, `iqtree2`, `iqtree` (auf dem Entwicklungsrechner ist `iqtree3` 3.1.4 per Homebrew installiert; Kommandozeilenoptionen sind kompatibel).
- FastTree: `FastTree -nt -gtr -quiet aln.fa` (DNA) bzw. `FastTree -quiet aln.fa` (Protein); Newick auf stdout, Support als Wert 0..1 → ×100.

**Tests:**
- 4 Sequenzen mit klarer Struktur `A1`,`A2` (fast gleich) und `B1`,`B2` (fast gleich, anders als A): NJ und UPGMA liefern `((A1,A2),(B1,B2))` (Vergleich per `robinsonFoulds == 0` mit dem erwarteten Baum aus `Newick.parse`).
- Distanzwerte: `p` für `ACGT` vs `ACGA` = 0,25; JC69 = `−0.75·ln(1 − 4/3·0.25)` = 0,3041 (±1e-4). K2P für eine Transition in 4 Basen (`ACGT` vs `GCGT`): `P=0.25`, `Q=0` → `−0.5·ln(0.5) − 0.25·ln(1)` = 0,3466 (±1e-4).
- Bootstrap mit klarer Trennung und 100 Wiederholungen: Support der Hauptverzweigung ≥ 95; Seed gleich → gleiche Werte.
- Fake-Backends (Shell-Skripte, die eine feste Newick-Datei ausgeben) testen den Ablauf ohne IQ-TREE/FastTree; echte Programme nur mit `Assumptions`.

---

## Aufgabe 3.8: Baum-Layout und Export

**Voraussetzung:** 3.6.

**Dateien:** `view/TreeLayout.java`, `view/SvgWriter.java`, Tests.

```java
public enum TreeStyle { RECTANGULAR_PHYLOGRAM, RECTANGULAR_CLADOGRAM, CIRCULAR }
public record NodeBox(Phylogeny.TreeNode node, double x, double y, double parentX, double parentY, boolean leaf, String label) {}
public final class TreeLayout {
    public TreeLayout(TreeStyle style, double width, double height, double margin);
    public List<NodeBox> layout(Phylogeny.TreeNode root);
}
```

**Regeln:** Rechteckig: Blätter erhalten `y = margin + i · (height − 2·margin)/(n−1)` in Blattreihenfolge (bei `n=1` Mitte); innere Knoten `y` = Mittel der Kind-`y`-Werte.
`x` Phylogramm = `margin + (Abstand von der Wurzel / größter Abstand) · (width − 2·margin)`; Kladogramm: `x` nach Tiefe (Blätter ganz rechts, `x` = margin + (1 − Höhe/Wurzelhöhe) · …, wobei Höhe = größte Anzahl Kanten bis zu einem Blatt). Kreisförmig: Blattwinkel gleichmäßig `0…2π`, Radius = Abstand (Phylogramm).
`SvgWriter.write(List<NodeBox>, ...)` erzeugt SVG (Linien, Beschriftungen, Maßstab), `String` ohne externe Bibliothek.

**Tests:** Baum `((A:1,B:1):1,C:2);` Phylogramm 400×300, margin 10: `x` von A, B, C = 10 + (2/2)·380 = 390 (Abstand 2 / max 2 = 1); `y` der drei Blätter = 10, 150, 290; innerer Knoten `y` = 80, `x` = 10 + (1/2)·380 = 200;
Wurzel `x` = 10. Kladogramm: alle Blätter `x` = 390. SVG enthält genau 3 `<text`-Elemente für die Blätter und ist wohlgeformt (mit `javax.xml.parsers` parsen).

---

## Aufgabe 3.9: Operationen und Oberfläche

**Voraussetzung:** 3.1–3.8, 1.11.

- **Operationen:** `align.run`, `align.profile`, `align.translate`, `align.strip`, `align.realign`, `align.consensus`, `tree.build`, `tree.reroot`, `tree.collapse`. Je ein Test Standardfall + Fehlerfall.
- **Alignment-Ansicht (`fx/AlignmentView`, Presenter `AlignmentPresenter`):** Zeilen mit Namen links, Spalten rechts; Farbschemata (Nukleotide: A grün, C blau, G orange, T rot;
  Aminosäuren nach Gruppen hydrophob/polar/geladen/aromatisch/speziell – selbst gewählte Farben), Konsenszeile, Identitätsbalken (`Conservation.identity`), optional Sequenzlogo,
  Auswahl von Zeilen/Spalten, Sortieren der Zeilen, Doppelklick auf einen Zeilennamen öffnet die Sequenz. Zeichnen virtualisiert (nur sichtbare Zeilen/Spalten).
- **Baum-Ansicht (`fx/TreeView`, Presenter `TreePresenter`):** `TreeLayout`, Zoom, Klick selektiert Knoten/Clade, Kontextaktionen (Wurzel hier setzen, drehen, einklappen, Unterbaum exportieren),
  Support-Werte anzeigen/verbergen, Export PNG (`WritableImage`) und SVG (`SvgWriter`) und Newick.
- **Identitäts-Heatmap:** `IdentityMatrix.percent` als Farbfläche mit Zahlen.
- **Presenter-Tests:** Spaltenauswahl `[3,6)` → `selectedColumns()`; Zeilen sortieren nach Identität zum Konsens liefert erwartete Reihenfolge; Klick auf Knoten liefert korrektes `clades`-Set.

---

## Gesamtabnahme Phase 3 (Integrationstest `AlignTreeIT`)

1. Zehn kurze, selbst erzeugte verwandte Sequenzen (Mutation von einer Ahnensequenz mit festem `Random`-Seed, einige Indels) als Dokumente.
2. Alignment mit `BuiltinAligner` (und zusätzlich mit MAFFT, falls installiert): alle Zeilen gleich lang, `ungapped` = Original.
3. NJ-Baum mit 100 Bootstrap-Wiederholungen: alle Blätter vorhanden, Support-Werte 0–100, Newick round-trip.
4. Wurzel per Mittelpunkt, Layout berechnen, SVG schreiben (wohlgeformt).
5. Alles in einem Projekt speichern, neu laden, identisch.
