# Architecture: Rust core + native Swift app (macOS), later a second UI for Windows/Linux

**Decision (Felix):** All logic and computation in **Rust** (`snapious-core`), the user interface as a **native macOS app in Swift** (AppKit/SwiftUI, drawing with Core Graphics/Metal).
For Windows/Linux a **second UI** can be built later on the same core (e.g. Tauri/web or egui) without touching the core.
This document **replaces** `08-HYBRID-JAVA-RUST.md` (Java + Rust process) and the Java/JavaFX requirements. The Java project `GeneiousRebuild/src` remains unchanged as a **reference**
(246 verified tests provide expected values).

The instructions `01`–`07` continue to apply for **tasks, algorithms and test values**. The rule is: *logic tasks → Rust* (section "Mapping"), *UI tasks → Swift* (section "Swift tasks S.x").

## Layers

```
┌─────────────────────────────────────────────────────┐
│ Swift app (macOS)      app/                         │  ← thin: displays, accepts input
│  Windows · views (NSView) · inspectors · menus      │
├─────────────────────────────────────────────────────┤
│ snapious-ffi   (UniFFI → Swift bindings)            │  ← small, stable interface
├─────────────────────────────────────────────────────┤
│ snapious-core  (pure Rust library)                  │  ← ALL logic, tested with cargo test
│  model · formats · operations · storage ·           │
│  algorithms · cloning · layout and presenter        │
│  logic · network · tool invocation                  │
└─────────────────────────────────────────────────────┘
                 snapious-cli (command line, uses only the core)
```

**Guiding rule:** Everything that does not directly draw pixels or open operating-system dialogs belongs in the core. This includes layout computation (line wrapping, lane assignment, map geometry, tree layout),
selection model, undo/redo, filters, presenter state. The Swift layer asks the core "what has to be drawn in lines 10–40?" and draws it.
This keeps the second UI thin later on, and the logic is testable without a window.

## Folder structure

Everything under `/Users/felixcuntz/Documents/Snapious/GeneiousRebuild/` (next to the Java reference project):

```
Cargo.toml                          # [workspace] members = ["crates/snapious-core", "crates/snapious-ffi", "crates/snapious-cli"]
crates/
  snapious-core/
    src/
      lib.rs  error.rs
      model/      sequence.rs  feature.rs  document.rs  alignment_doc.rs  tree_doc.rs
      bio/        align.rs  msa.rs  distance.rs  tree.rs  newick.rs  nn_tm.rs  protein.rs  codon.rs
      cloning/    enzymes.rs  digest.rs  ligation.rs  overlap.rs  golden_gate.rs  gateway.rs  topo.rs  pcr.rs
      primer/     picker.rs  checks.rs
      io/         genbank.rs  fasta.rs  embl.rs  gff3.rs  bed.rs  clustal.rs  phylip.rs  nexus.rs  ab1.rs
      ops/        operation.rs  runner.rs  history.rs  registry.rs  workflow.rs
      store/      project_store.rs  versions.rs  settings.rs
      view/       seq_layout.rs  selection.rs  feature_layout.rs  map_layout.rs  tree_layout.rs  translation.rs  edit_session.rs  presenters/
      net/        http.rs  cache.rs  ratelimit.rs  ncbi.rs  uniprot.rs  blast.rs
      tools.rs  gel.rs  crispr.rs  codon_opt.rs  seqcheck.rs  search.rs
    tests/  benches/  testdata/
  snapious-ffi/                     # UniFFI layer (library `libsnapious_ffi.a` + `.dylib`)
    src/lib.rs  src/api.rs
  snapious-cli/                     # `snapious digest puc19.gb --enzymes EcoRI,HindIII` etc.
app/                                # Swift package (SwiftPM)
  Package.swift
  Sources/SnapiousApp/              # user interface
  Sources/SnapiousFFI/              # generated bindings (do not edit by hand)
scripts/
  build-ffi.sh                      # builds the Rust library, generates Swift bindings
  bundle-app.sh                     # builds .app (Info.plist, icon, library)
GeneiousRebuild/src/                # (remains) Java reference
```

## Dependencies (Rust)

| Purpose | Crate |
|---|---|
| Errors | `thiserror` (own `Error` enum) |
| JSON | `serde`, `serde_json` |
| SQLite | `rusqlite` with feature `bundled` |
| HTTP | `ureq` (blocking) |
| Parallelism | `rayon` |
| Randomness | `rand` (only with a fixed seed: `StdRng::seed_from_u64`) |
| Hashing | `sha2`, `hex` |
| Compression | `flate2` |
| Time | `time` (UTC, store seconds as i64) |
| Regex | `regex` |
| FFI | `uniffi` (proc macros: `#[uniffi::export]`, `uniffi::setup_scaffolding!()`) |
| Paths | `dirs` (platform-neutral config/cache folders) |
| Tests | `tempfile`, `roxmltree` (check SVG/XML), `criterion` (benchmarks) |

No further crates without an entry in `OPEN-QUESTIONS.md`. No `async`, no `unsafe` (except in the generated FFI layer), no custom procedural macros.
`[profile.release] lto = "fat"`, `codegen-units = 1`, `panic = "abort"` — **Caution:** `panic = "abort"` terminates the app on a panic; the FFI layer therefore catches errors as `Result`, panics count as bugs and get tests.

## Mapping Java instructions → Rust

| Java instructions | Rust |
|---|---|
| `record Foo(a, b)` | `#[derive(Clone, Debug, PartialEq)] pub struct Foo { pub a: A, pub b: B }` |
| `enum` | `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum …` |
| `interface` | `pub trait …` (interchangeable: `Box<dyn Trait>`) |
| `List<T>` / `Map<K,V>` | `Vec<T>` / `BTreeMap<K,V>` (ordered output), `HashMap` only internally |
| `Optional<T>` | `Option<T>` |
| `IllegalArgumentException` | `Err(Error::InvalidInput(String))` |
| `IllegalStateException` | `Err(Error::InvalidState(String))` |
| `IOException` | `Err(Error::Io(String))` (+ `From<std::io::Error>`) |
| `OperationException` | `Err(Error::Operation(String))` (German user message) |
| `IndexOutOfBoundsException` | `Err(Error::OutOfRange(String))` |
| Interval `[start, end)` | `std::ops::Range<usize>` |
| `with…()` on immutable documents | `fn with_…(&self, …) -> Self` |
| Sequence as `String` | **`Vec<u8>`/`&[u8]` (ASCII uppercase)**, `String` only for display/files |
| `@Test`, `assertEquals` | `#[test]`, `assert_eq!`; floating point: `assert!((a - b).abs() < eps)` |
| `Assumptions.assumeTrue(tool)` | `if !tools::is_available("mafft") { eprintln!("übersprungen"); return; }` |
| `@Tag("network")` | `#[ignore = "network"]` (run with `cargo test -- --ignored`) |
| `@TempDir` | `tempfile::tempdir()` |
| `Operation` interface | `pub trait Operation { fn id(&self) -> &str; fn label(&self) -> &str; fn run(&self, inputs: &[Document], params: &Params, ctx: &mut dyn OperationContext) -> Result<OperationResult>; }` |
| `Document` interface | `pub enum Document { Sequence(SequenceDocument), Alignment(AlignmentDocument), Tree(TreeDocument) }` |
| `ProjectStore` (JDBC) | `ProjectStore` with `rusqlite`; JSON payload with `serde_json` (same schema as in Phase 0.5) |
| Methods `findAllCutSites` | `find_all_cut_sites` (snake_case, types `CamelCase`) |
| "Presenter" (testable without a window) | Struct in the core with pure methods (`view/presenters/…`) |
| Java parity test | dropped; instead the Rust test adopts the **expected values of the Java tests** |

Names are transferred mechanically so that Qwen can read the Java instructions directly.

## Performance requirements (binding, with benchmarks)

| Area | Technique | Target (Apple M chip, release) |
|---|---|---|
| Alignment (NW/SW) | two `i32` rows for scores, traceback 2 bits/cell (packed) | 5,000 × 5,000 bases < 0.3 s |
| Restriction scan | IUPAC bitmasks (A=1, C=2, G=4, T=8), enzymes in parallel (`rayon`) | 1 Mb × 79 enzymes < 50 ms |
| Distances/bootstrap | column counting via tables, `rayon` | 100 seq. × 1,000 columns, 100 bootstrap < 5 s |
| Seed-and-extend | 2-bit k-mers in `u64`, `HashMap<u64, Vec<u32>>` | 1 Mb DB, 1 kb query < 20 ms |
| Sequence view layout | O(visible lines) | 1 Mb: < 1 ms per call |
| GenBank/FASTA parser | `&str` splitting, sequence directly into `Vec<u8>` | 100 MB in < 1 s (FASTA) |
| FFI call | data in large chunks (line ranges, not characters) | < 0.2 ms for a layout query |

Every task with a row in this table gets a `criterion` benchmark and a coarse `#[test]` with `Instant` (limit = 10 × target).

## Rules for Rust code (especially for Qwen)

1. `cargo check` after every file, `cargo test` after every task, `cargo clippy -- -D warnings` and `cargo fmt` before finishing.
2. Functions take `&[u8]`/`&str`/`&T` and return their own values. **No lifetime parameters in public structs**; clone instead of storing references.
3. No `unwrap()`/`expect()` outside of tests; propagate errors with `?`.
4. Indices checked or validated beforehand; `saturating_sub`/`checked_sub` for `usize`.
5. No panics on invalid input (empty sequence, length 0, index at the boundary): always `Result`/empty results, plus one test each.
6. Deterministic output (do not iterate over `HashMap` for results; use `BTreeMap` or sort); randomness only with a fixed seed.
7. Tests as `#[cfg(test)] mod tests` below the file; integration tests in `crates/snapious-core/tests/`; test names in German without umlauts (`fn verdau_ecori_zwei_fragmente()`).
8. `///` documentation in German with the source of the algorithm.
9. **Stay platform-neutral:** no macOS calls in the core; paths only with `std::path`/`dirs`; look up program file names without a fixed extension (`.exe` on Windows);
   accept line endings `\n` and `\r\n`; text always UTF-8.

## Deviations per phase (Rust instead of Java)

- **Phase 0:** 0.1–0.6 in Rust (`model`, `io::genbank`, `ops`, `store`, `tools`); JSON with `serde_json`. 0.6 `tools.rs`: `std::process::Command`, stdout/stderr in two threads, timeout with a `try_wait` loop (20 ms), then `kill`.
  **0.7 is dropped** (JavaFX) and replaced by **S.0**. New **task 0.0:** create the workspace, run `cargo fmt`/`clippy`/`test`, `.gitignore`.
  New **task 0.9 (port of the reference algorithms)** in this order, each with the test values of the Java tests:
  `model::sequence` → `bio::align` → `bio::msa` → `bio::tree`/`newick` → `cloning::enzymes` (79 enzymes from `RestrictionEnzymeDB.java`) → `primer` → `search` → `io` (FASTA/FASTQ/GenBank/SAM/VCF) → `coverage`.
- **Phase 1:** `view/*` and `features/*` as described, in the core. 1.11 is dropped as a JavaFX task; presenters stay in the core, drawing is done by Swift (S.2–S.4).
- **Phase 2/3:** purely algorithmic, unchanged; bootstrap and distances with `rayon`; trees/gel/SVG export return `String`/number lists. UI parts (2.14, 3.9) become Swift tasks.
- **Phase 4:** `ureq` behind an `HttpFetcher` trait (fakes in tests). 4.7: 3D view via a temporary HTML file with 3Dmol.js in the default browser (`open`), no embedded WebView.
- **Phase 5:** postponed.
- **Phase 6:** 6.3 **plugins** as external programs (`~/.snapious/plugins/<name>/plugin.json`; invocation with `--in <file> --out <folder> --params <json>`), because Rust has no stable ABI.
  6.9 Packaging: `scripts/bundle-app.sh` produces the `.app`; coverage with `cargo llvm-cov`. Settings: JSON with `serde`, file permissions `0o600`.

---

## FFI layer (`snapious-ffi`)

**Task F.0 (Claude): set up UniFFI.**

- Crate type `["staticlib", "cdylib"]`, `uniffi::setup_scaffolding!()`, Swift bindings generated with `uniffi-bindgen-swift` (script `scripts/build-ffi.sh`: `cargo build --release -p snapious-ffi`, then bindings to `app/Sources/SnapiousFFI/`).
- **Interface design (binding):**
  - Object `Project` (Arc, internally `Mutex<ProjectState>`): `open(path)`, `create(path)`, `documents()`, `import_file(path)`, `document(id)`, `delete(id)`, `save()`, `history()`, `undo(id)`, `redo(id)`.
  - Object `SequenceSession` (an opened sequence document with `EditSession`): `length()`, `topology()`, `line_slices(bases_per_line, first_line, last_line) -> Vec<LineSlice>`, `features_in(range) -> Vec<PlacedFeatureDto>`,
    `translation(frame, range)`, `enzyme_sites(filter, names)`, `select(start, end)`, `selection_stats()`, `insert/delete/replace`, `undo/redo`, `circular_map(size) -> MapGeometry`, `linear_map(width) -> MapGeometry`.
  - Operations: `run_operation(id, params_json, input_ids, progress: Box<dyn ProgressListener>) -> OperationOutcome`, cancellation via a `CancelToken` object.
  - All types as simple records/enums (UniFFI-compatible): `u32/u64/f64/String/Vec/Option`; sequences as `String` (ASCII) or `Vec<u8>`.
  - Errors as `enum ApiError { InvalidInput(String), Io(String), Operation(String), Internal(String) }` (becomes `throws` in Swift).
  - **No** callbacks per character or per pixel; data in blocks (`LineSlice` contains line index, start position and the base string of the line).
- **Tests:** Rust tests call the FFI functions the way Swift would (without Swift). In addition, a Swift smoke test in S.0 that calls `ping`/`Project.create`.

---

## Swift tasks (user interface)

All Swift tasks are implemented by **Claude** (feedback by screenshot/description from Felix), because they must be checked visually and Swift is harder for a local Qwen.
Technology: **SwiftPM** (`app/Package.swift`), target macOS 14+, Swift 6, SwiftUI for the frame (sidebar, inspectors, dialogs), **AppKit `NSView`** for the large drawing surfaces (Core Graphics; Metal only when needed).
Built with `swift build -c release` (Command Line Tools suffice; Xcode is **not** required), bundled via `scripts/bundle-app.sh`.

| No. | Task | Prerequisite |
|---|---|---|
| **S.0** | SwiftPM project, UniFFI bindings integrated, window with document list (open/new project, import sequence). Menu bar, `.app` bundling. Smoke test: `Project.create` + import of `mini.gb` shows one entry. | F.0, Phase 0 |
| **S.1** | Main window frame: sidebar (folders/documents), tabs for opened documents, inspector on the right (SwiftUI). | S.0 |
| **S.2** | `SequenceView` (`NSView`): draws only visible lines based on `line_slices`; number margin, groups of 10, complementary strand, colors; zoom; mouse and keyboard selection (passed to `select`). | S.1, Phase 1.1–1.2 |
| **S.3** | Annotation tracks and translation in the `SequenceView` (`features_in`, `translation`). | S.2, 1.3–1.4 |
| **S.4** | `MapView` (circular/linear): features, enzyme cuts, selection arc; click sets the selection (passed to the core). | S.2, 1.6–1.7 |
| **S.5** | Inspectors: selection statistics, enzyme list (filter), feature list (add/delete), feature detection. Undo/redo, edit mode. | S.3, 1.5/1.8/1.9 |
| **S.6** | Digest view with gel (`GelView`), cloning wizard, PCR/primer dialog, sequencing comparison. | S.5, Phase 2 |
| **S.7** | `AlignmentView` (columns, colors, consensus, identity, logo) and `TreeView` (layout from the core, export PNG/SVG). | S.5, Phase 3 |
| **S.8** | NCBI/BLAST/UniProt dialogs, protein view with charts (Swift Charts), RNA arc diagram, 3D in the browser. | S.5, Phase 4 |
| **S.9** | Workflows menu, plugin management, settings (check tool paths), dark mode, printing, PDF export of the map (macOS provides PDF generation: `NSView.dataWithPDF`), dock icon, `.app` signing not needed (private). | S.6–S.8, Phase 6 |

**Rules for the Swift layer:** no domain logic (no computing with bases, no layout computation); any computation that would be needed twice in the second UI belongs in the core. `@MainActor` for UI; long operations on background tasks with `ProgressListener` and cancellation.

---

## Later: Windows/Linux

Yes, this is feasible **without touching the core**. You then write only a second, thin UI that calls the same functions:

| Option | Effort | Note |
|---|---|---|
| **Tauri + web UI** (Svelte/React, canvas) | medium | Calls `snapious-core` directly (Rust); canvas drawing based on the data from `view::*`; best look, many ready-made building blocks |
| **egui/eframe** (pure Rust) | medium | Same data structures, drawing with `Painter`; quick start, plainer look |
| **Qt/QML** | high | Mature desktop toolkit, but a C++ binding is needed |

The effort for the second UI roughly corresponds to the Swift tasks S.1–S.9 (about 30–40 % of the total project), no more.
To keep it that way: follow the guiding rule above, no Apple APIs in the core, occasionally check `cargo build --target x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` (core without UI), keep path handling and program lookup platform-neutral.
External programs (MAFFT etc.) also exist for Windows/Linux; the path lookup is centralized in `tools.rs`.

---

## Order

1. **Task 0.0** (workspace) and **0.9** (port reference algorithms) – creates the foundation with the Java test values.
2. **Phase 0** in Rust (0.1–0.6), **F.0** (UniFFI), **S.0** (Swift window with document list). After that a vertical slice runs: import, save, display.
3. **Phase 1** (core: Qwen; UI S.1–S.5: Claude).
4. **Phase 2 → 3 → 4 → 6** as in `00`–`07`; core tasks Qwen, Swift tasks Claude.
5. After that optionally Phase 5 and the second UI.
