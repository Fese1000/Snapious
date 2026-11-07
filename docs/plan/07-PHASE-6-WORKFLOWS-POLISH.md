# Phase 6: Workflows, Plugins, Document Management, and Polish

> **Implementation note:** The signatures in this document are written in Java notation. Implementation is in **Rust** (mapping: `08-RUST-CORE-SWIFT-APP.md`, table "Mapping Java guide → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-WORKING-INSTRUCTIONS.md` first.


Goal: chain operations into workflows, load plugins, manage documents (folders, search, versions), settings, export, installer package, and quality tooling.

New packages: `workflow/`, `plugin/`, `project/` (management), `settings/`, `export/`.

---

## Task 6.1: Registering Operations (Operation Registry)

**Prerequisite:** Phases 0–5 (as far as implemented).

**Files:** `ops/OperationRegistry.java`, test.

```java
public final class OperationRegistry {
    public static OperationRegistry withBuiltins(ToolRegistry tools, HttpFetcher fetcher);    // registers all existing operations
    public void register(Operation op);                          // duplicate ID → IllegalArgumentException
    public Optional<Operation> find(String id);
    public List<Operation> all();                                // sorted by id
}
```

`withBuiltins` registers all operations created in Phases 0–5 (only those that already exist; keep the list as a constant in `OperationRegistry` and extend it with every new operation).

**Tests:** Registering two operations with the same ID → exception; `find("edit.insert")` present; `all()` sorted; `withBuiltins` contains no duplicate IDs.

---

## Task 6.2: Workflows

**Prerequisite:** 6.1.

**Files:** `workflow/Workflow.java`, `workflow/WorkflowStep.java`, `workflow/WorkflowRunner.java`, `workflow/WorkflowJson.java`, tests.

```java
public record StepInput(String reference) {}     // "$in:0" (input document 0), "$step:2:1" (output 1 of step 2, 0-based)
public record WorkflowStep(String id, String operationId, Map<String,String> parameters, List<StepInput> inputs) {}
public record Workflow(String name, String description, int inputCount, List<WorkflowStep> steps, List<String> outputs /* references as in StepInput */) {}

public final class WorkflowRunner {
    public WorkflowRunner(OperationRegistry registry, OperationRunner runner);
    public WorkflowResult run(Workflow wf, List<Document> inputs, OperationContext ctx) throws OperationException;
    public List<WorkflowResult> runBatch(Workflow wf, List<Document> inputs, OperationContext ctx);   // one document as input each, an error in one run does not stop the others
}
public record WorkflowResult(List<Document> outputs, List<HistoryEntry> history, List<String> errors) {}

public final class WorkflowJson {
    public static String write(Workflow wf);
    public static Workflow read(String json);
}
```

**Rules:** Steps run in order; references only to earlier steps (forward reference → `IllegalArgumentException` during validation). Validate before execution: all `operationId`s known, references valid,
`inputCount` matches the number of documents passed. Each step creates its `HistoryEntry` via `OperationRunner`. Error in a step → `OperationException` with step number; `runBatch` collects errors per input.
Define and document the JSON format yourself (`{"name":…, "steps":[{"id":…,"operation":…,"parameters":{…},"inputs":["$in:0"]}], "outputs":["$step:2:0"]}`), using the `Json` helper class from Phase 0/4.

**Tests:**
- Workflow `edit.reverseComplement` → `edit.insert` (position=1, bases=GG) on `AATT`: result `rc(AATT)`=`AATT` → insert `A` `GG` `ATT` → `AGGATT`.
- Two inputs with `runBatch` (`ATGC`, `GGGG`) → two results.
- Unknown operation → `IllegalArgumentException` during validation (before any execution; no history).
- JSON round trip: `read(write(wf)).equals(wf)`.
- Error in the second step (invalid parameter) → exception mentions `Schritt 2` (the message text itself stays German); `runBatch` reports the error for that input, the others continue.

---

## Task 6.3: Plugin API

> **→ replaced:** Plugins are **external programs** (`plugin.json`), see `08-RUST-CORE-SWIFT-APP.md`, section "Deviations per phase". `ServiceLoader`/JAR is dropped.

**Prerequisite:** 6.1.

**Files:** `plugin/SnapiousPlugin.java`, `plugin/PluginLoader.java`, `plugin/Importer.java`, `plugin/Exporter.java`, example plugin under `examples/hello-plugin/` (its own small Maven project), tests.

```java
public interface SnapiousPlugin {
    String name();
    String version();
    default List<Operation> operations() { return List.of(); }
    default List<Importer> importers() { return List.of(); }
    default List<Exporter> exporters() { return List.of(); }
}
public interface Importer { String name(); List<String> extensions(); List<Document> read(Path file) throws IOException; }
public interface Exporter { String name(); String extension(); boolean accepts(Document d); void write(Document d, Path file) throws IOException; }

public final class PluginLoader {
    public static List<SnapiousPlugin> load(Path directory);             // all *.jar in the folder via URLClassLoader + ServiceLoader<SnapiousPlugin>
    public static void register(List<SnapiousPlugin> plugins, OperationRegistry ops, FormatRegistry formats);
}
```

Faulty JARs are skipped (warning in the log interface `PluginLog`), never a crash. Plugin folder: `~/.snapious/plugins`. Plugins run **without a sandbox** — note in the README and in the settings dialog.

**Tests:** Build a mini JAR in the test (`javax.tools.JavaCompiler` compiles a plugin class in the `@TempDir`; `META-INF/services/com.snapious.plugin.SnapiousPlugin` with the class name; package with `JarOutputStream`) →
`load` finds the plugin with `name()`, and its operation is in the registry after `register`. Defective JAR (random bytes) → skipped, no error. Duplicate operation ID from a plugin → skipped, warning.

---

## Task 6.4: Document Management (Folders, Search, Batch Actions)

**Prerequisite:** Phase 0.5.

**Files:** Extension of `store/ProjectStore` (schema version 2: column `folder_id`, table `folders`, table `notes`, table `favorites`), `project/ProjectService.java`, `project/DocumentSearch.java`, `project/BatchRename.java`, `project/Duplicates.java`, tests.

```java
public record Folder(String id, String name, String parentId) {}
public final class ProjectService {
    public ProjectService(ProjectStore store);
    public Folder createFolder(String name, String parentId);
    public void moveDocument(String documentId, String folderId);          // null = root
    public void renameFolder(String id, String name); public void deleteFolder(String id, boolean deleteContents);
    public void setNote(String documentId, String text); public Optional<String> note(String documentId);
    public void setFavorite(String documentId, boolean favorite); public List<DocumentInfo> favorites();
    public List<DocumentInfo> list(String folderId);
}
public final class DocumentSearch {
    public static List<DocumentInfo> byName(ProjectStore store, String text);                 // contains, case-insensitive
    public static List<DocumentInfo> byType(ProjectStore store, String type);
    public static List<DocumentInfo> byFeatureText(ProjectStore store, String text);          // in feature name/type/qualifiers
    public static List<DocumentInfo> byMetadata(ProjectStore store, String key, String value);
    public static List<DocumentInfo> bySequenceMotif(ProjectStore store, String motifIupac);   // motif (IUPAC) in sequences, both strands
}
public final class BatchRename {
    /** Pattern: {name} original name, {n} counter from 1, {n:03} counter with leading zeros, {type}. */
    public static Map<String,String> plan(List<DocumentInfo> docs, String pattern);           // id → new name
}
public final class Duplicates {
    public static List<List<DocumentInfo>> findSequenceDuplicates(ProjectStore store);        // same bases (SHA-256 of the normalized uppercase sequence), same type
}
```

Migration from schema 1 to 2 on opening: `ALTER TABLE documents ADD COLUMN folder_id TEXT`, create new tables, `PRAGMA user_version = 2`. Schema 1 files must remain openable (test with a version 1 file created in the test).

**Tests:** Create a folder, move a document, `list(folderId)` shows it; deleting a folder with `deleteContents=false` moves contents to the root. `byFeatureText("lacZ")` finds the document with feature `lacZα`. `bySequenceMotif` finds `GAATTC` also in the reverse complement
(document contains `GAATTC`, search for `GAATTC` finds it; document contains only the reverse complement `GAATCC` of `GGATTC`, search for `GGATTC` finds it as well). Motif `RGATCY` finds a document with `GGATCC` and one with `AGATCT`.
`BatchRename.plan` with pattern `Construct_{n:03}` for 3 documents → `Construct_001` … `Construct_003`. `findSequenceDuplicates` groups two documents with the same sequence (different names), but not DNA with identical text as RNA.

---

## Task 6.5: Document Versions

**Prerequisite:** 6.4.

**Files:** Extension of `store/ProjectStore` (table `versions(doc_id, version, time, payload, summary)`), `project/Versions.java`, tests.

- On every `save(doc)` whose `payload` differs from the last saved one, the **previous** version is stored in `versions` (at most 50 per document, the oldest drop off).
- `List<VersionInfo> versions(String docId)`; `Document loadVersion(String docId, int version)`; `void restore(String docId, int version)` (the current version itself becomes a version in the process).

**Tests:** Save a document, change it, save → one version (the first version); `restore` brings back the original state and stores the intermediate version as another version; 60 changes → 50 versions.
Saving the same version again creates no new version.

---

## Task 6.6: Settings

**Prerequisite:** 0.6.

**Files:** `settings/Settings.java`, `settings/SettingsStore.java`, test.

```java
public record Settings(Map<String,String> toolPaths, String ncbiEmail, String ncbiApiKey, String defaultOrganism, boolean darkMode,
                       int undoLimit, Path pluginDirectory, Path cacheDirectory) {
    public static Settings defaults();
}
public final class SettingsStore {
    public SettingsStore(Path file);                 // JSON file ~/.snapious/settings.json
    public Settings load(); public void save(Settings s);
}
```

File with permissions for the user only (`PosixFilePermissions rw-------`), if the file system allows it (otherwise ignore). Never print the API key to the log (`toString` masks it).
Settings dialog in JavaFX (presenter testable): tool paths with a "Check" button (`ToolRegistry.run(tool, ["--version"])`) and status display.

**Tests:** Round trip; missing file → default values; corrupt file → default values and the defective file is backed up as `.bak`; `toString()` does not contain the key.

---

## Task 6.7: Export (Image, SVG, Text)

**Prerequisite:** Phase 1, 3.8.

**Files:** `export/MapSvg.java`, `export/SequenceTextExport.java`, `export/ReportBuilder.java`, tests.

- `MapSvg.circular(SequenceDocument, CircularMapLayout, EnzymeMapModel, int size)` and `MapSvg.linear(...)`: SVG with features (arrows/arcs), enzyme labels, title (name, length). Output is well-formed XML.
- `SequenceTextExport.format(doc, int basesPerLine, boolean numbers, boolean features)`: text representation with position numbers, optionally translation and feature list.
- `ReportBuilder`: generates a text report from a `SequenceDocument` (length, GC content, features table, single-cutter enzymes) as Markdown; for cloning results additionally the history (`HistoryEntry`) with parameters.
- PDF: deliberately **not** with a new library; instructions in the README: print the SVG to PDF from the browser or Preview.

**Tests:** Parse the SVG of the circular map (`DocumentBuilder`), number of `<path`/`<text` plausible (one feature → at least one `path`); `SequenceTextExport` at 10 bases per line and position numbers: line 1 starts with `1`, line 2 with `11`.
Markdown report contains the headings `# <Name>` and `## Features`.

---

## Task 6.8: User Interface (Polish)

> **→ replaced:** User interface in Swift (**S.9**); presenter logic in the core.

**Prerequisite:** all previous.

- **Main window:** left project bar with folder tree, search field, favorites; tabs for open documents; "Operations" menu (from `OperationRegistry`, grouped by category via the ID prefix); "Workflow" menu (record from history, run, import/export); "Plugins" menu.
- **History view:** Table of `HistoryEntry` per document; "Restore" calls `Versions.restore`.
- **Keyboard shortcuts:** Ctrl/Cmd+O open, S save, Z/Y undo/redo, F search, +/− zoom, E enzyme sidebar, L feature sidebar.
- **Dark mode:** two CSS files (`light.css`, `dark.css`) in `src/main/resources/fx/`; toggle in the settings; obtain drawing colors (canvas) from a `Theme` class, not hard-coded.
- **Error display:** `OperationException` → dialog with a German message; unexpected errors → dialog with a short message + log file `~/.snapious/log.txt` (simple file logger, no new framework).

**Presenter tests:** Menu construction from the registry (number of entries per category), workflow recording (3 history entries yield a workflow with 3 steps and matching references).

---

## Task 6.9: Quality and Packaging

> **→ replaced:** JaCoCo → `cargo llvm-cov`; `jpackage` → `scripts/bundle-app.sh` (`.app`); TestFX is dropped.

- **Test coverage:** JaCoCo (`org.jacoco:jacoco-maven-plugin`, report `target/site/jacoco`). Goal: ≥ 80 % line coverage in `model`, `ops`, `store`, `cloning`, `primer`, `align`, `tree`; record gaps in a report `docs/plan/ABDECKUNG.md`.
- **Performance:** Tests with a time limit (generous, e.g. 5 s): `SequenceDocument.insert` on a 1 Mb sequence, `findAllCutSites` on 1 Mb, NJ tree with 100 sequences of 500 bp, gel for 1000 fragments.
- **Language of messages:** Uniformly German; a test checks that `OperationException` messages are not empty.
- **UI smoke tests** (optional): TestFX only locally, not in the default run (`-Pui`). At least one test starts `SnapiousFxApp`, opens `mini.gb`, and checks that the document list shows one entry.
- **Installer package:** `jpackage --type dmg --name Snapious --input target --main-jar snapious-biotools-1.0.0.jar --main-class com.snapious.fx.SnapiousFxApp --module-path <javafx-jmods> --add-modules javafx.controls`
  (macOS); as a script `scripts/package-mac.sh`. The script first checks `jpackage -version` and prints an understandable message if JavaFX JMODs are missing. **No** signing/notarization (private project) — note in the README.
- **README:** Feature list per phase with screenshots (optional), section "External programs" (which, for what, where to configure), section "Limitations" (e.g. no replacement for Geneious/SnapGene).

---

## Overall Acceptance of Phase 6

Scenario (manual, with a log in `docs/plan/ABNAHME.md`):
1. New project, create folder "Test", import `puc19.gb`.
2. Record a workflow from the history: digest → cloning → export; run it in batch on three insert files.
3. Restore a version of a document.
4. Load the example plugin; its operation appears in the menu and runs.
5. Dark mode on/off, save settings and restart.
6. `mvn clean package` green, `scripts/package-mac.sh` produces a DMG (if JavaFX JMODs are present).
