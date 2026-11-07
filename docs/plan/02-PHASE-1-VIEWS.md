# Phase 1: Sequence and Map View

> **Implementation note:** The signatures in this document are written in Java notation. The implementation is in **Rust** (mapping: `08-RUST-CORE-SWIFT-APP.md`, table "Mapping Java instructions → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-WORKING-INSTRUCTIONS.md` first.


Goal: the main window with sequence view, annotation tracks, map (circular/linear), enzyme display, editing with
undo, automatic feature detection and more file formats.

**Guiding principle:** All computation (layout, geometry, statistics) lives in pure Java classes without JavaFX and is
tested with numbers. The JavaFX classes only draw what these classes have computed. There are **no** UI tests with windows.

New packages: `view/` (pure layout logic), `fx/` (drawing), `features/` (feature database and detection),
`io/` (additional formats), `edit/` (editing with undo).

---

## Task 1.1: Line layout of the sequence view

**Prerequisite:** Phase 0.

**Files:** `view/SequenceLayout.java`, test `view/SequenceLayoutTest.java`.

```java
public final class SequenceLayout {
    public SequenceLayout(int sequenceLength, int basesPerLine);          // basesPerLine >= 1
    public int lineCount();                                               // ceil(length / basesPerLine), at least 1; for length 0 → 0
    public int lineOf(int position);                                      // position / basesPerLine
    public int columnOf(int position);                                    // position % basesPerLine
    public int lineStart(int line);                                       // line * basesPerLine
    public int lineEnd(int line);                                         // min(lineStart + basesPerLine, length), exclusive
    public int firstVisibleLine(double scrollY, double lineHeight);      // floor(scrollY / lineHeight)
    public int lastVisibleLine(double scrollY, double viewHeight, double lineHeight); // inclusive, capped at lineCount()-1
    public static int basesPerLineFor(double viewWidth, double charWidth, double marginWidth, int groupSize);
}
```

`basesPerLineFor`: available width `viewWidth - marginWidth`, one character is `charWidth`, after every `groupSize` characters
an additional gap of `charWidth`. The result is a **multiple of `groupSize`**, at least `groupSize`.
Example: `viewWidth=800, charWidth=10, marginWidth=80, groupSize=10` → group width 110 px (10 characters + 1 gap) →
`floor(720/110) = 6` groups → 60 bases per line.

**Tests:**
- Length 125, 60 per line: `lineCount()=3`, `lineOf(59)=0`, `lineOf(60)=1`, `columnOf(61)=1`, `lineEnd(2)=125`.
- Length 0: `lineCount()=0`.
- `firstVisibleLine(250, 20)=12`; `lastVisibleLine(250, 100, 20)` with 100 lines = 17 (pixels 250..350 → lines 12..17).
- `basesPerLineFor(800,10,80,10)=60` and `basesPerLineFor(50,10,0,10)=10` (minimum).

---

## Task 1.2: Selection model

**Prerequisite:** none.

**Files:** `view/Selection.java`, test `view/SelectionTest.java`.

```java
public record Selection(int anchor, int caret) {            // both 0-based, positions BETWEEN the bases (0..length)
    public static Selection none();                         // anchor=caret=-1
    public static Selection caretAt(int position);
    public static Selection range(int start, int end);
    public boolean isEmpty();                               // anchor == caret or none
    public int start();  public int end();                  // min / max
    public int length();
    public Selection extendTo(int position);                // anchor stays
    public Selection clamp(int sequenceLength);
}
```

**Tests:** `range(5,2).start()==2`, `.end()==5`, `.length()==3`; `extendTo(9)` on `caretAt(3)` → `start 3, end 9`;
`clamp(4)` on `range(2,10)` → `end 4`; `none().isEmpty()` is true.

---

## Task 1.3: Annotation tracks (lane assignment)

**Prerequisite:** 0.1.

**Files:** `view/FeatureLayout.java`, test `view/FeatureLayoutTest.java`.

```java
public record PlacedFeature(Feature feature, Interval segment, int lane) {}

public final class FeatureLayout {
    /** Distributes the segments of all features onto lanes so that no two overlap. */
    public static List<PlacedFeature> assignLanes(List<Feature> features);
    public static int laneCount(List<PlacedFeature> placed);
}
```

Algorithm (interval coloring, greedy): sort all segments (every interval of every feature) by `start`, then by length descending;
place each segment into the first lane whose last end is `<= segment.start`; otherwise create a new lane.
All segments of **one** feature end up in the same lane (join): for the feature, choose the smallest lane in which **all**
of its segments fit. (Then sort features instead of segments: by `location().start()`, then by length descending.)

**Tests:**
- Three features `[0,10)`, `[5,15)`, `[10,20)` → lanes 0, 1, 0 (the third fits behind the first).
- Two disjoint features → both lane 0, `laneCount=1`.
- Join feature `[0,5)`+`[20,25)` and feature `[10,15)`: no segment overlaps another, the join feature occupies lane 0, `[10,15)` also fits into lane 0. Expected: both lane 0.
- Empty list → empty output, `laneCount=0`.

---

## Task 1.4: Translation and frame track

**Prerequisite:** 0.2.

**Files:** `view/TranslationTrack.java`, test `view/TranslationTrackTest.java`.

```java
public record AminoAcidCell(int basePosition, char aminoAcid, boolean isStart, boolean isStop) {}   // basePosition = 1st base of the codon

public final class TranslationTrack {
    /** Translation in frame 0, 1, 2 (forward strand) over [from, to). Incomplete codons at the end are dropped. */
    public static List<AminoAcidCell> frame(String bases, int frame, int from, int to);
    /** Translation along a feature (CDS): follows the intervals (join) and the strand, frame from qualifier /codon_start (default 1). */
    public static List<AminoAcidCell> forFeature(String bases, Feature cds);
}
```

Uses the codon table from `Sequence` — for this, extract a **public static** method
`Sequence.aminoAcidFor(String codon)` (uppercase, T instead of U) from the existing private method `codonToAminoAcid`
(behavior unchanged; existing tests must stay green).

**Tests:**
- `frame("ATGAAATAG", 0, 0, 9)` → `M`(start), `K`, `*`(stop); `basePosition` 0, 3, 6.
- `frame("AATGAAATAG", 1, 0, 10)` → the same three cells with `basePosition` 1, 4, 7.
- `forFeature` with CDS `[0,9)` REVERSE on `CTATTTCAT` → reverse complement `ATGAAATAG` → `M K *`.
- CDS with join `[0,3)`+`[6,9)` on `ATGCCCTAG` → codons `ATG`, `TAG` → `M *`.

---

## Task 1.5: Statistics and clipboard for the selection

**Prerequisite:** 1.2.

**Files:** `view/SelectionStats.java`, `view/SequenceClipboard.java`, tests for them.

```java
public record SelectionStats(int length, double gcPercent, double tmCelsius) {
    public static SelectionStats of(String bases);        // Tm: up to 13 bases Wallace (2*(A+T)+4*(G+C)),
                                                          // above that 64.9 + 41*(G+C-16.4)/N   (Marmur/Doty, cite source)
}
public final class SequenceClipboard {
    public static String asText(String bases);                    // as is
    public static String asFasta(String name, String bases);      // via Sequence.toFASTA format, 60 per line
    public static String asReverseComplement(String bases);
}
```

**Tests:** `of("ATGC")` → `length 4`, `gcPercent 50.0`, `tm 12.0`. `of("GGGGGGGGGGGGGGGGGGGG")` (20 G) → `gc 100`,
`tm = 64.9 + 41*(20-16.4)/20 = 72.28`. `asReverseComplement("ATGC")="GCAT"`. `asFasta("x", 70 characters)` → 2 sequence lines (60 + 10).

---

## Task 1.6: Plasmid map geometry

**Prerequisite:** 0.1.

**Files:** `view/CircularMapLayout.java`, `view/LinearMapLayout.java`, tests.

```java
public final class CircularMapLayout {
    public CircularMapLayout(int sequenceLength, double centerX, double centerY, double radius);
    /** Angle in degrees, 0° = top (12 o'clock), clockwise; position 0 lies at 0°. */
    public double angleOf(double position);
    public double[] pointAt(double position, double radiusOffset);       // {x, y}
    public double sweepOf(int start, int end);                           // (end-start)/length*360
    /** Label positions without overlap: returns an angle for each anchor position, minimum spacing minAngleDeg. */
    public double[] spreadLabels(double[] anglesDeg, double minAngleDeg);
}
public final class LinearMapLayout {
    public LinearMapLayout(int sequenceLength, double x0, double x1);
    public double xOf(double position);                                  // x0 + position/length*(x1-x0)
    public int positionAt(double x);                                     // inverse, rounded, capped to 0..length
}
```

`pointAt`: `x = cx + (r+off)*sin(a)`, `y = cy - (r+off)*cos(a)` (a in radians).
`spreadLabels`: sort angles, then from left to right push each to `max(angle, previous + minAngle)`,
finally check cyclically (resolve remaining overlap with the start + 360° by shifting back evenly); preserve the order of the input array in the result.

**Tests:** Length 1000, center (0,0), r=100: `angleOf(0)=0`, `angleOf(250)=90`, `angleOf(500)=180`;
`pointAt(0,0)` = (0,−100) (±1e-9), `pointAt(250,0)` = (100, 0); `sweepOf(0,250)=90`.
`LinearMapLayout(1000, 50, 550)`: `xOf(500)=300`, `positionAt(300)=500`, `positionAt(0)=0`.
`spreadLabels([10,11,12], 5)` → `[10,15,20]` (±1e-9). Two labels far apart remain unchanged.

---

## Task 1.7: Enzyme display logic

**Prerequisite:** Phase 0 (`RestrictionEnzymeDB` exists).

**Files:** `view/EnzymeMapModel.java`, test.

```java
public enum EnzymeFilter { ALL, UNIQUE_CUTTERS, DOUBLE_CUTTERS, NON_CUTTERS }

public final class EnzymeMapModel {
    public EnzymeMapModel(RestrictionEnzymeDB db, SequenceDocument doc);
    public Map<String, List<CutSite>> sitesByEnzyme();                           // all enzymes that cut
    public List<CutSite> sites(EnzymeFilter filter, Set<String> onlyNames);     // onlyNames empty = all
    public List<String> nonCutters();
    public boolean isCircular();                                                  // from doc.topology()
}
```

Note: For circular sequences, recognition sites can span the origin (end+start). Do **not** change
`RestrictionEnzymeDB.findAllCutSites` for this; instead, in `EnzymeMapModel` search the sequence extended by `maxRecognitionLength-1`
bases from the start and map hits with `position >= length` to `position - length` (remove duplicates).

**Tests:** Sequence `GAATTC` + 20×`A` + `GGATCC` (linear): EcoRI and BamHI are unique cutters; `UNIQUE_CUTTERS`
contains both, `DOUBLE_CUTTERS` neither. Circular `TTCAAAAAAGAA` (length 12; the recognition site `GAATTC` spans the origin: `GAA` at the end + `TTC` at the start)
→ EcoRI found at position 9; viewed as linear it is not found. Sequence without an EcoRI site → `nonCutters()` contains `EcoRI`.

---

## Task 1.8: Editing with undo/redo

**Prerequisite:** 0.2.

**Files:** `edit/EditSession.java`, test `edit/EditSessionTest.java`.

```java
public final class EditSession {
    public EditSession(SequenceDocument initial);
    public SequenceDocument current();
    public void insert(int position, String bases);
    public void delete(int start, int end);
    public void replace(int start, int end, String bases);
    public void addFeature(Feature f);
    public void removeFeature(Feature f);
    public void setTopology(Topology t);
    public boolean canUndo(); public boolean canRedo();
    public void undo(); public void redo();
    public boolean isModified();               // current != last saved state
    public void markSaved();
    public int historyLimit();                 // 200 steps, oldest are dropped
}
```

Every change pushes the previous `SequenceDocument` (immutable, no copying needed) onto the undo stack and clears
the redo stack. Bases are checked with the validation of `Sequence` before insertion (invalid → `IllegalArgumentException`,
state unchanged).

**Tests:** Insert → undo → original; redo → changed again; after undo and a new change, `canRedo()` is false;
`isModified()` is false after `markSaved()`, true after a change, false again after undo;
invalid insertion (`"X"`) throws and changes nothing; 250 changes → more than 200 undo steps are not possible.

---

## Task 1.9: Feature database and detection

**Prerequisite:** 0.1.

**Files:** `features/FeatureDefinition.java`, `features/FeatureDatabase.java`, `features/FeatureDetector.java`,
`src/main/resources/features/standard-features.json`, tests.

```java
public record FeatureDefinition(String name, String type, String sequence, String color, boolean isProtein) {}
public final class FeatureDatabase {
    public static FeatureDatabase standard();                  // loads standard-features.json from the classpath
    public static FeatureDatabase fromJson(String json);
    public List<FeatureDefinition> all();
}
public final class FeatureDetector {
    public FeatureDetector(FeatureDatabase db, int maxMismatches);          // 0 = exact
    public List<Feature> detect(SequenceDocument doc, boolean circular);     // both strands, no duplicates
}
```

**Contents of `standard-features.json`:** Only features whose sequence is **public and short**; write them yourself from
freely accessible sources (e.g. GenBank entry of pUC19 `L09137`, textbook consensus sequences). At least (name, type, sequence for each):

| Name | Type | Note |
|---|---|---|
| `lac promoter` / `lacZα` fragment | promoter | from the pUC19 entry |
| `AmpR` | CDS | sequence of the bla gene (861 bp) from L09137 |
| `ori` (pMB1/ColE1) | rep_origin | from L09137 |
| `T7 promoter` | promoter | `TAATACGACTCACTATAG` |
| `T7 terminator` | terminator | `CTAGCATAACCCCTTGGGGCCTCTAAACGGGTCTTGAGGGGTTTTTTG` |
| `SP6 promoter` | promoter | `ATTTAGGTGACACTATAG` |
| `M13 fwd` / `M13 rev` | primer_bind | `GTAAAACGACGGCCAGT` / `CAGGAAACAGCTATGAC` |
| `6xHis` | misc_feature | `CATCATCATCATCATCAC` |
| `FLAG` | misc_feature | `GACTACAAGGACGACGATGACAAG` |
| `HA` | misc_feature | `TACCCATACGATGTTCCAGATTACGCT` |
| `loxP` | misc_recomb | `ATAACTTCGTATAGCATACATTATACGAAGTTAT` |
| `attB1` / `attB2` | misc_recomb | see `CloningMethod.ATT_B1/ATT_B2` |

Each entry gets a color (hex). **Omit** entries whose sequence you cannot reliably document from a free source,
and note them in `OPEN-QUESTIONS.md`.

**Detection:** sliding window over both strands, comparison with at most `maxMismatches` deviations (Hamming),
IUPAC not needed. Circular: extend the sequence by `maxLength-1` and report hits across the origin as a join location
(`[start,length)`+`[0,rest)`). Remove duplicate hits (same definition, same position); a hit with fewer
mismatches displaces overlapping ones of the same definition. The generated `Feature` has `type`, `name`, strand and the qualifier
`note="automatisch erkannt"`.

**Tests:** Sequence with an embedded `T7 promoter` sequence at position 10 → one feature `[10,28)` FORWARD. The same sequence
embedded reverse-complemented → REVERSE. With one mismatch and `maxMismatches=0` not found, with `1` found.
Feature across the origin in a circular sequence: join with two intervals. Empty database → empty list.

---

## Task 1.10: Additional file formats

**Prerequisite:** 0.2, 0.3.

**Files (each with reading and, where sensible, writing):**
`io/EmblFormat.java`, `io/Gff3Format.java`, `io/BedFormat.java`, `io/NexusFormat.java`, `io/PhylipFormat.java`,
`io/ClustalFormat.java`, `io/Ab1Reader.java`, tests with small example files under `src/test/resources/`.

All formats return documents from Phase 0. Alignment formats (Nexus, Phylip, Clustal) initially return only
`List<Sequence>` with gaps (`-`) **preserved** (class `AlignedSequences(List<String> names, List<String> rows)`); an
`AlignmentDocument` comes in Phase 3.

- **EMBL:** like GenBank, but line prefixes `ID`, `FT` (features), `SQ` (sequence with numbers at the end of the line), `//`. Position syntax identical.
- **GFF3:** columns `seqid, source, type, start, end, score, strand, phase, attributes` (tab-separated, 1-based, inclusive).
  `Name=`/`ID=` → feature name, remaining attributes → qualifiers. Lines `##FASTA` introduce sequences. Features belong to the
  sequence with the matching `seqid`. Writing: `format(SequenceDocument)`.
- **BED:** `chrom start end name score strand` (0-based, end exclusive!) → attach features to an existing document:
  `List<Feature> read(Path, String sequenceName)`.
- **Phylip (sequential and interleaved):** first line `n  length`, names 10 characters (or separated by spaces, relaxed).
- **Clustal:** header line `CLUSTAL`, blocks `Name  sequence`, ignore conservation lines (`*`, `:`, `.`).
- **Nexus:** `#NEXUS`, `BEGIN DATA;`/`MATRIX`…`;`, remove comments in `[ ]`, `FORMAT … GAP=- MISSING=?`.
- **AB1 (ABIF, big-endian):**
  - File header: bytes 0..3 = `ABIF`, version `short` at 4.
  - Directory entry (28 bytes each: name 4 characters, number `int`, element type `short`, element size `short`,
    element count `int`, data size `int`, data offset `int`, handle `int`): the first entry at offset 6 describes the
    directory itself – `element count` at offset 18, `data offset` at offset 26.
  - If the data size is ≤ 4, the data are stored **in the data offset field itself**.
  - Required tags: `PBAS`(no. 1; bases as ASCII), `PCON`(1; quality values as bytes), `PLOC`(1; peak positions as `short`),
    `FWO_`(1; base order, 4 characters, e.g. `GATC`), `DATA`(no. 9–12; trace data `short` for the four colors in `FWO_` order),
    `SMPL`(1; sample name as Pascal string: 1st byte = length).
  - Result: `record Chromatogram(String sampleName, String bases, int[] quality, int[] peakLocations, Map<Character,int[]> traces)`.

**Tests:** One small example file per format (5–10 lines, self-written) with expected values, round-trip where
writing is possible. For AB1: a mini ABIF file **generated synthetically in the test** (assemble the bytes in the test: header,
two entries `PBAS1` with `ACGT` and `PCON1` with `{30,40,50,60}`) → bases `ACGT`, quality `[30,40,50,60]`. Do not put real
.ab1 files into the repository (privacy).

---

## Task 1.11: Assemble the main view (JavaFX)

> **→ replaced** by **S.2–S.5** (Swift). The presenter part stays in the Rust core (`view/presenters`); drawing is done by Swift. Do not implement as JavaFX.

**Prerequisite:** 1.1–1.10 and 0.7.

**Files:** `fx/SequenceCanvas.java`, `fx/MapCanvas.java`, `fx/EditorView.java` (SplitPane: map on the left, sequence on the right),
`fx/EditorPresenter.java`. The **presenter** connects `EditSession`, `Selection`, `EnzymeMapModel`, `FeatureLayout`;
it is testable without a window.

Behavior:
- Click/drag in the sequence sets the `Selection`; clicking a feature selects its range; the selection is highlighted in the map
  as an arc and vice versa (click in the map → `LinearMapLayout.positionAt` or angle → position).
- Keyboard: arrows move the cursor, Shift extends, Ctrl/Cmd+C copies (`SequenceClipboard.asText`), Ctrl/Cmd+Z/Y undo/redo,
  Del/Backspace deletes the selection, letters `A C G T` insert bases (only in edit mode, toggle at the top).
- Status bar: `SelectionStats` of the selection and cursor position.
- Sidebar "Enzymes": `EnzymeFilter` selection, list with number of cut sites.
- Sidebar "Features": list, double-click jumps to the position, add/delete via `EditSession`.
- Button "Detect features" calls `FeatureDetector` and shows suggestions to accept.
- Draw only visible lines (`firstVisibleLine`…`lastVisibleLine`).

**Tests:** `EditorPresenter`: set selection → `SelectionStats` correct; deleting the selection via the presenter changes the
document and the selection collapses to the deletion position; map click at angle 90° with length 1000 → position 250.

**Done when (check manually):** opening `puc19.gb` → map and sequence visible, EcoRI unique cutter marked,
selection in sync, editing and undo work, a 1-Mb random sequence (generate in the test and save via `Sequence.toFASTA`)
scrolls smoothly. All `view/` tests green.
