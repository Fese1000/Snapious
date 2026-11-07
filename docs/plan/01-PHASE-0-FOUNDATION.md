# Phase 0: Foundation

> **Implementation note:** The signatures in this document are written in Java notation. The implementation is in **Rust** (mapping: `08-RUST-CORE-SWIFT-APP.md`, table "Abbildung Java-Anleitung → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-WORKING-INSTRUCTIONS.md` first.


Goal: document model, annotations, GenBank with features, operations with history, project store (SQLite),
integration of external programs, JavaFX skeleton. After this, all later phases can build on it.

New packages (all under `src/main/java/com/snapious/`):

```
model/   Strand, Location, Feature, Topology, Document, SequenceDocument
io/      GenBankFormat (new, alongside the existing FileIO)
ops/     Operation, OperationContext, OperationResult, HistoryEntry, OperationRunner
store/   ProjectStore
tools/   ToolRegistry, ToolResult
fx/      SnapiousFxApp (JavaFX entry point, still minimal)
```

---

## Task 0.1: Feature model

**Prerequisite:** none.

**Files:** `model/Strand.java`, `model/Location.java`, `model/Feature.java`, tests `model/FeatureTest.java`.

```java
public enum Strand { FORWARD, REVERSE, NONE }      // + / - / unknown

/** A contiguous range, 0-based, end exclusive. */
public record Interval(int start, int end) {
    public Interval { if (start < 0 || end < start) throw new IllegalArgumentException(...); }
    public int length() { return end - start; }
    public boolean overlaps(Interval o) { return start < o.end && o.start < end; }
}

/** Position of a feature: one or more intervals (join) on one strand. */
public record Location(List<Interval> intervals, Strand strand, boolean partialStart, boolean partialEnd) {
    public Location { intervals = List.copyOf(intervals); if (intervals.isEmpty()) throw ...; }
    public static Location of(int start, int end, Strand strand);
    public int start();          // smallest start of all intervals
    public int end();            // largest end
    public int length();         // sum of the interval lengths
    public boolean contains(int position);
    public Location shifted(int delta);    // shift all intervals by delta
}

public final class Feature {
    public Feature(String type, String name, Location location, Map<String, List<String>> qualifiers);
    // Getters: type() e.g. "CDS", "gene", "promoter", name(), location(), qualifiers()
    public String qualifier(String key);                // first value or null
    public Feature withLocation(Location l);            // copy with new position
    // equals/hashCode over type, name, location, qualifiers
}
```

The qualifier map is copied immutably (`Map.copyOf` with `List.copyOf` of the values); insertion order
is preserved via `LinkedHashMap` on creation.

**Tests (reference values):**
- `Location.of(10, 20, FORWARD).length() == 10`, `.contains(10)` is true, `.contains(20)` is false.
- Join of `[0,10)` and `[20,25)`: `length() == 15`, `start() == 0`, `end() == 25`, `contains(15)` is false.
- `shifted(5)` on `[10,20)` gives `[15,25)`.
- Invalid interval `new Interval(5, 3)` throws `IllegalArgumentException`.
- `Feature.qualifier("gene")` returns the first value; a missing key returns `null`.

**Done when:** tests green.

---

## Task 0.2: SequenceDocument

**Prerequisite:** 0.1.

**Files:** `model/Topology.java`, `model/Document.java`, `model/SequenceDocument.java`, test `model/SequenceDocumentTest.java`.

```java
public enum Topology { LINEAR, CIRCULAR }

public interface Document {
    String id();                         // unique within the project (UUID string), random for new documents
    String name();
    Instant created();
    Map<String, String> metadata();      // free-form, e.g. "organism", "source", "accession"
    String type();                       // "sequence", "alignment", "tree", "table"
}

public final class SequenceDocument implements Document {
    public SequenceDocument(String id, String name, Sequence sequence, Topology topology,
                            List<Feature> features, Map<String, String> metadata, Instant created);
    public static SequenceDocument of(String name, Sequence sequence);       // linear, no features, new UUID
    public Sequence sequence();
    public Topology topology();
    public List<Feature> features();                                        // immutable
    public int length();
    public String bases();

    // All modifications return a NEW document (same id!); the original stays unchanged:
    public SequenceDocument withName(String name);
    public SequenceDocument withTopology(Topology t);
    public SequenceDocument withFeatures(List<Feature> features);
    public SequenceDocument insert(int position, String bases);     // shifts features
    public SequenceDocument delete(int start, int end);             // removes/trims features
    public SequenceDocument replace(int start, int end, String bases);
    public SequenceDocument subDocument(int start, int end);        // excerpt with contained features, positions -start
    public SequenceDocument reverseComplement();                    // features mirrored, strand flipped
}
```

**Rules for features on modification** (binding):
- `insert(pos, bases)` of length `n`: intervals with `start >= pos` are shifted by `n`; an interval with
  `start < pos < end` is extended by `n` (`end += n`); intervals with `end <= pos` stay.
  Exception: `end == pos` stays unchanged (inserting directly after the feature does not extend it).
- `delete(start, end)` with `d = end - start`: intervals entirely inside the deleted range disappear; partially
  overlapping ones are trimmed; intervals after it are shifted by `d`. If all intervals of a
  feature disappear, the feature is dropped.
- `replace` = `delete` followed by `insert` at `start`.
- `subDocument(s, e)`: only features that lie **entirely** within `[s, e)` are kept (intervals shifted by `-s`).
- `reverseComplement()`: interval `[a, b)` at length `L` becomes `[L-b, L-a)`, strand `FORWARD`↔`REVERSE`,
  order of the join intervals reversed.
- Invalid ranges: `IndexOutOfBoundsException` (like `Sequence.subsequence`).
- `Sequence` is mutable (has setters) → copy defensively in the constructor
  (`new Sequence(id, name, bases, type)`).

**Tests (reference values):**
- Document `ACGTACGTAC` (length 10), feature `gene` `[2,6)` FORWARD:
  - `insert(0, "TT")` → bases `TTACGTACGTAC`, feature `[4,8)`.
  - `insert(4, "TT")` → feature `[2,8)` (inserting in the middle extends it).
  - `insert(6, "TT")` → feature unchanged `[2,6)`.
  - `delete(0, 3)` → bases `TACGTAC`, feature `[0,3)`.
  - `delete(2, 6)` → feature is dropped.
  - `delete(4, 8)` → feature `[2,4)`.
- `reverseComplement()` of `AACCGGTT` (feature `[0,2)` FORWARD) → bases `AACCGGTT`, feature `[6,8)` REVERSE.
- `subDocument(2, 8)` of the first example contains the feature as `[0,4)`.
- The original stays unchanged after every modification (compare `bases()` and `features()`).

**Done when:** tests green.

---

## Task 0.3: GenBank with features (read and write)

**Prerequisite:** 0.2.

**Files:** `io/GenBankFormat.java`, test `io/GenBankFormatTest.java`, test resource `src/test/resources/mini.gb`.

```java
public final class GenBankFormat {
    public static List<SequenceDocument> parse(String text) throws IOException;     // multiple entries possible ("//")
    public static List<SequenceDocument> read(Path file) throws IOException;
    public static String format(SequenceDocument doc);
    public static void write(SequenceDocument doc, Path file) throws IOException;
}
```

**Format (reading):**
- `LOCUS` line: name, length, `bp`, molecule type (`DNA`, `RNA`, `mRNA` → type DNA/RNA), topology (`circular` or `linear`).
  Example: `LOCUS       mini                      60 bp    DNA     circular   SYN 01-JAN-2024`.
- `DEFINITION`, `ACCESSION`, `ORGANISM` → `metadata` (`definition`, `accession`, `organism`).
- `FEATURES` section: line starts with 5 spaces + type (columns 6-20) + position (from column 22). Qualifier lines
  start with 21 spaces and `/key="value"` (the value may span several lines, a closing quote ends it),
  or `/key=value` (without quotes) or `/key` (without value → value `""`).
- Position grammar (GenBank is 1-based, inclusive → internally 0-based, end exclusive):
  - `5` → `[4,5)`; `5..10` → `[4,10)`; `<5..10` → partialStart; `5..>10` → partialEnd;
  - `complement(5..10)` → REVERSE; `join(1..3,7..9)` → two intervals; `complement(join(1..3,7..9))` → REVERSE with
    two intervals (store in ascending position order);
  - `join(complement(1..3),complement(7..9))` may be rejected with `IOException("not supported: …")`.
- The feature name is the first qualifier present from the order `label`, `gene`, `product`, `note`, otherwise the type.
- `ORIGIN` section: lines `        1 acgtacgtac gtacgtacgt ...` → strip digits and spaces, uppercase.
- The sequence type from LOCUS determines `SequenceType` (DNA if unclear). Illegal characters → `IOException` with line number.

**Format (writing):**
- LOCUS line with correct length and `circular`/`linear`; `FEATURES` in input order; positions converted back to 1-based,
  `complement(...)` and `join(...)` correctly nested; qualifier values in quotes, wrapped at 58 characters
  (21-space indent); `ORIGIN` with 60 bases per line in 6 blocks of 10, line start right-aligned to 9 positions;
  terminated by `//`.

**Test resource `mini.gb`** (60 bp circular, 2 features; use the sequence exactly as given):

```
LOCUS       mini                      60 bp    DNA     circular   SYN 01-JAN-2024
DEFINITION  Testplasmid.
ACCESSION   TEST0001
FEATURES             Location/Qualifiers
     gene            5..20
                     /gene="abc"
     CDS             complement(30..50)
                     /gene="xyz"
                     /note="zwei Zeilen
                     Fortsetzung"
ORIGIN
        1 atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc
//
```

**Tests:**
- `parse(...)` returns one document with `length() == 60`, `topology() == CIRCULAR`, `metadata().get("accession").equals("TEST0001")`.
- Feature 1: type `gene`, name `abc`, position `[4,20)` FORWARD. Feature 2: type `CDS`, name `xyz`, `[29,50)` REVERSE,
  qualifier `note` = `zwei Zeilen Fortsetzung` (line break + indent → one space).
- Round trip: `parse(format(doc))` yields the same bases, features (incl. qualifiers) and topology.
- `join(1..3,7..9)` becomes two intervals `[0,3)` and `[6,9)`.
- Invalid character in `ORIGIN` (e.g. `x`) → `IOException`.
- Two entries in one text → list of length 2.

**Done when:** tests green; `FileIO.readGenBank` remains unchanged.

---

## Task 0.4: Operations and history

**Prerequisite:** 0.2.

**Files:** `ops/Operation.java`, `ops/OperationContext.java`, `ops/OperationResult.java`, `ops/HistoryEntry.java`,
`ops/OperationRunner.java`, tests `ops/OperationRunnerTest.java`.

```java
public interface Operation {
    String id();                         // stable name, e.g. "edit.insert", "clone.restriction"
    String label();                      // display name (German)
    OperationResult run(List<Document> inputs, Map<String, String> parameters, OperationContext ctx)
        throws OperationException;
}

public interface OperationContext {      // progress and cancellation
    void progress(double fraction, String message);   // 0.0 .. 1.0
    boolean isCancelled();
}

public record OperationResult(List<Document> outputs, String summary) {}

public class OperationException extends Exception { ... }   // checked, with a message for the UI

public record HistoryEntry(String id, String operationId, Map<String,String> parameters,
                           List<String> inputIds, List<String> outputIds,
                           Instant time, String summary) {}

public final class OperationRunner {
    public OperationRunner(HistoryListener listener);                       // listener.add(HistoryEntry)
    public OperationResult execute(Operation op, List<Document> inputs,
                                   Map<String,String> parameters, OperationContext ctx) throws OperationException;
}
```

Behavior of `execute`: runs the operation, on success creates a `HistoryEntry` (new UUID, current time,
IDs of the input and output documents) and reports it to the listener. On `OperationException` there is **no** entry.
If `ctx.isCancelled()` becomes true after the run, the results are discarded and `OperationException("abgebrochen")` is thrown.

In addition, two simple operations as examples and for tests:
`ops/builtin/ReverseComplementOperation` (`id = "edit.reverseComplement"`, takes one `SequenceDocument`,
returns the reverse-complemented one named `"<old> (rev-kom)"`) and `ops/builtin/InsertOperation`
(`edit.insert`, parameters `position`, `bases`).

**Tests:**
- Successful run of `edit.reverseComplement` with `AACCGGTT` → result `AACCGGTT`; test with `ATGC` → `GCAT`.
- A `HistoryEntry` was reported: `operationId == "edit.reverseComplement"`, `inputIds` contains the input ID.
- `InsertOperation` with `position=2`, `bases=GG` on `AATT` → `AAGGTT`.
- Invalid parameter (`position=abc`) → `OperationException`, no history entry.
- Cancellation: an operation that sets `ctx.isCancelled()` to `true` after starting → `OperationException`, no entry.

**Done when:** tests green.

---

## Task 0.5: Project store (SQLite)

> **→ replaced:** In Rust with `rusqlite` (feature `bundled`) and `serde_json` instead of JDBC and a custom Json class; schema and tests stay the same. Ignore the Maven line.

**Prerequisite:** 0.2, 0.3, 0.4.

**Maven:** add dependency `org.xerial:sqlite-jdbc:3.45.3.0` (scope compile) to `pom.xml`.

**Files:** `store/ProjectStore.java`, test `store/ProjectStoreTest.java`.

```java
public final class ProjectStore implements AutoCloseable {
    public static ProjectStore open(Path file) throws IOException;      // creates file and schema if new
    public void save(Document doc) throws IOException;                   // insert or update by id
    public Optional<Document> load(String id) throws IOException;
    public List<DocumentInfo> list() throws IOException;                 // id, name, type, created
    public void delete(String id) throws IOException;
    public void addHistory(HistoryEntry e) throws IOException;
    public List<HistoryEntry> history() throws IOException;              // ascending by time
    public List<HistoryEntry> historyFor(String documentId) throws IOException;   // entries where the ID appears as input or output
    @Override public void close() throws IOException;
}
public record DocumentInfo(String id, String name, String type, Instant created) {}
```

**Schema** (on open with `CREATE TABLE IF NOT EXISTS`, plus `PRAGMA user_version = 1`):

```sql
CREATE TABLE documents(id TEXT PRIMARY KEY, type TEXT NOT NULL, name TEXT NOT NULL,
                       created INTEGER NOT NULL, payload TEXT NOT NULL);
CREATE TABLE history(id TEXT PRIMARY KEY, time INTEGER NOT NULL, operation TEXT NOT NULL,
                     parameters TEXT NOT NULL, inputs TEXT NOT NULL, outputs TEXT NOT NULL, summary TEXT);
```

`payload` is self-generated JSON (do not add a new Jackson; a small private helper class `Json` in `store/`
with `String write(Object)` / `Object parse(String)` for Map/List/String/Number/Boolean/null is enough).
For `SequenceDocument` the JSON contains: `name`, `type: "DNA|RNA|PROTEIN"`, `bases`, `topology`, `metadata`,
`features` (type, name, strand, partialStart, partialEnd, intervals as `[[s,e],...]`, qualifiers).
Unknown document types on load: `IOException("Unbekannter Dokumenttyp: …")`.

**Tests (temporary directory `@TempDir`):**
- Save a document, close the store, reopen, load → same bases, features, topology, metadata.
- `save` twice with the same ID and a changed name → `list()` has 1 entry with the new name.
- `delete` removes the document; `load` returns `Optional.empty()`.
- History: save two entries → `history()` in chronological order; `historyFor(id)` finds the entry as input and as output.
- Open a file with a different `user_version` (e.g. 99) → `IOException`.

**Done when:** tests green.

---

## Task 0.6: Integration of external programs

**Prerequisite:** none.

**Files:** `tools/ToolRegistry.java`, `tools/ToolResult.java`, test `tools/ToolRegistryTest.java`.

```java
public record ToolResult(int exitCode, String stdout, String stderr, Duration duration) {
    public boolean ok() { return exitCode == 0; }
}

public final class ToolRegistry {
    public static ToolRegistry defaults();                       // searches PATH and common paths (/opt/homebrew/bin, /usr/local/bin)
    public ToolRegistry withExecutable(String tool, Path executable);   // set path manually (immutable: new instance)
    public boolean isAvailable(String tool);
    public Optional<Path> executable(String tool);
    public ToolResult run(String tool, List<String> args, Path workingDir, String stdin, Duration timeout)
        throws IOException;                                      // IOException on missing tool, timeout, start failures
    public static boolean isAvailableStatic(String tool);        // shorthand for assumptions: defaults().isAvailable(tool)
}
```

Known tool names (search names only, nothing is installed): `mafft`, `muscle`, `clustalo`, `blastn`, `blastp`, `makeblastdb`,
`primer3_core`, `iqtree3`, `iqtree2`, `iqtree`, `FastTree`, `minimap2`, `bwa`, `samtools`, `bcftools`, `RNAfold`, `spades.py`.

Implementation with `ProcessBuilder`: read stdout and stderr in two threads (otherwise it blocks on large output),
timeout with `Process.waitFor(timeout)`, on expiry `destroyForcibly()` and `IOException("Zeitüberschreitung")`.

**Tests (without real bio tools, platform-independent using `sh`, Unix only; otherwise `Assumptions.assumeTrue`):**
- `withExecutable("echo", Path.of("/bin/echo")).run("echo", List.of("hallo"), ...)` → `stdout` is `hallo\n`, `ok()`.
- `run("cat", ..., stdin="abc")` → `stdout == "abc"`.
- `run("sh", List.of("-c", "exit 3"), ...)` → `exitCode == 3`.
- `run("sleep", List.of("5"), ..., Duration.ofMillis(200))` → `IOException`.
- Unknown tool `"gibtsnicht"` → `isAvailable` false, `run` throws `IOException`.
- Large output (`sh -c 'yes | head -n 200000'`) does not block.

**Done when:** tests green.

---

## Task 0.7: JavaFX skeleton

> **→ replaced:** by task **S.0** (Swift window) in `08-RUST-CORE-SWIFT-APP.md`. Do not implement.

**Prerequisite:** 0.5 (for "Open project"), otherwise none.

**Maven (`pom.xml`):**
- Properties: `<javafx.version>21.0.5</javafx.version>`.
- Dependencies `org.openjfx:javafx-controls` and `org.openjfx:javafx-graphics` (version `${javafx.version}`).
  Maven resolves the platform classifier automatically via the OpenJFX artifact's profile.
- Plugin `org.openjfx:javafx-maven-plugin:0.0.8` with `<mainClass>com.snapious.fx.SnapiousFxApp</mainClass>`.
  Run: `mvn javafx:run`.
- **No** `module-info.java` (runs on the classpath). The existing Swing `SnapiousApp` remains the main class in the JAR.

**Files:** `fx/SnapiousFxApp.java` (extends `javafx.application.Application`), `fx/ProjectModel.java`, test `fx/ProjectModelTest.java`.

Window 1280×800, title "Snapious", on the left a `ListView<DocumentInfo>` (document list), in the middle an empty
placeholder ("Select a document"), at the top a "File" menu: "Open project…", "New project…", "Import sequence…"
(FASTA or GenBank via `FileChooser`), "Quit". Imported documents are saved in the `ProjectStore` and
appear in the list.

**Important:** The logic lives in `ProjectModel` (no JavaFX types except `ObservableList`, or none at all), so it can be tested
without a window:

```java
public final class ProjectModel {
    public void open(Path file) throws IOException;
    public void importFile(Path file) throws IOException;     // .fa/.fasta/.gb/.gbk → save documents
    public List<DocumentInfo> documents();
    public Optional<Document> load(String id) throws IOException;
}
```

**Tests:** `ProjectModel` with `@TempDir`: open project, import `mini.gb` → `documents()` has 1 entry named `mini`.
Import FASTA with two entries → 2 documents. Unknown file extension → `IOException`. **No** test that opens a window.

**Done when:** `mvn test` green and `mvn javafx:run` opens the window; importing `mini.gb` shows the entry in the list.

---

## Task 0.8: Closing Phase 0

- Add all new packages with a short description to the README (section "Architektur").
- Section "Fortschritt" in `00-WORKING-INSTRUCTIONS.md`: tick off Phase 0.
- `mvn clean package` runs through completely, `java -jar target/snapious-biotools-1.0.0.jar` starts the Swing UI as before.
- At least 40 new tests in this phase (sum over all tasks).
