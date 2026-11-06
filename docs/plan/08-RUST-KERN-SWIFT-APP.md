# Architektur: Rust-Kern + native Swift-App (macOS), später zweite Oberfläche für Windows/Linux

**Entscheidung (Felix):** Alle Logik und alles Rechnen in **Rust** (`snapious-core`), die Oberfläche als **native macOS-App in Swift** (AppKit/SwiftUI, Zeichnen mit Core Graphics/Metal).
Für Windows/Linux kann später eine **zweite Oberfläche** auf demselben Kern entstehen (z. B. Tauri/Web oder egui), ohne den Kern anzufassen.
Dieses Dokument **ersetzt** `08-HYBRID-JAVA-RUST.md` (Java + Rust-Prozess) und die Java/JavaFX-Vorgaben. Das Java-Projekt `GeneiousRebuild/src` bleibt unverändert als **Referenz**
(246 geprüfte Tests liefern Erwartungswerte).

Die Anleitungen `01`–`07` gelten weiter für **Aufgaben, Algorithmen und Testwerte**. Es gilt: *Logik-Aufgaben → Rust* (Abschnitt „Abbildung"), *Oberflächen-Aufgaben → Swift* (Abschnitt „Swift-Aufgaben S.x").

## Schichten

```
┌─────────────────────────────────────────────────────┐
│ Swift-App (macOS)      app/                         │  ← dünn: zeigt an, nimmt Eingaben entgegen
│  Fenster · Ansichten (NSView) · Inspektoren · Menüs │
├─────────────────────────────────────────────────────┤
│ snapious-ffi   (UniFFI → Swift-Bindings)            │  ← kleine, stabile Schnittstelle
├─────────────────────────────────────────────────────┤
│ snapious-core  (reine Rust-Bibliothek)              │  ← ALLE Logik, getestet mit cargo test
│  Modell · Formate · Operationen · Speicher ·        │
│  Algorithmen · Klonierung · Layout- und Presenter-  │
│  Logik · Netz · Werkzeug-Aufrufe                    │
└─────────────────────────────────────────────────────┘
                 snapious-cli (Kommandozeile, nutzt nur den Kern)
```

**Leitregel:** Alles, was nicht unmittelbar Pixel zeichnet oder Betriebssystem-Dialoge öffnet, steht im Kern. Auch Layout-Berechnung (Zeilenumbruch, Spurzuordnung, Kartengeometrie, Baum-Layout),
Auswahlmodell, Undo/Redo, Filter, Presenter-Zustand. Die Swift-Schicht fragt den Kern „was ist in den Zeilen 10–40 zu zeichnen?" und zeichnet es.
So bleibt die zweite Oberfläche später dünn und die Logik ist ohne Fenster testbar.

## Ordnerstruktur

Alles unter `/Users/felixcuntz/Documents/Snapious/GeneiousRebuild/` (neben dem Java-Referenzprojekt):

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
  snapious-ffi/                     # UniFFI-Schicht (Bibliothek `libsnapious_ffi.a` + `.dylib`)
    src/lib.rs  src/api.rs
  snapious-cli/                     # `snapious digest puc19.gb --enzymes EcoRI,HindIII` usw.
app/                                # Swift-Paket (SwiftPM)
  Package.swift
  Sources/SnapiousApp/              # Oberfläche
  Sources/SnapiousFFI/              # generierte Bindings (nicht von Hand ändern)
scripts/
  build-ffi.sh                      # baut Rust-Bibliothek, erzeugt Swift-Bindings
  bundle-app.sh                     # baut .app (Info.plist, Icon, Bibliothek)
GeneiousRebuild/src/                # (bleibt) Java-Referenz
```

## Abhängigkeiten (Rust)

| Zweck | Crate |
|---|---|
| Fehler | `thiserror` (eigenes `Error`-Enum) |
| JSON | `serde`, `serde_json` |
| SQLite | `rusqlite` mit Feature `bundled` |
| HTTP | `ureq` (blockierend) |
| Parallel | `rayon` |
| Zufall | `rand` (nur mit festem Seed: `StdRng::seed_from_u64`) |
| Hashing | `sha2`, `hex` |
| Kompression | `flate2` |
| Zeit | `time` (UTC, Sekunden als i64 speichern) |
| Regex | `regex` |
| FFI | `uniffi` (Proc-Macros: `#[uniffi::export]`, `uniffi::setup_scaffolding!()`) |
| Pfade | `dirs` (Konfigurations-/Cache-Ordner plattformneutral) |
| Tests | `tempfile`, `roxmltree` (SVG/XML prüfen), `criterion` (Benchmarks) |

Keine weiteren Crates ohne Eintrag in `OFFENE_FRAGEN.md`. Kein `async`, kein `unsafe` (außer in generierter FFI-Schicht), keine eigenen prozeduralen Makros.
`[profile.release] lto = "fat"`, `codegen-units = 1`, `panic = "abort"` — **Achtung:** `panic = "abort"` beendet die App bei einem Panic; die FFI-Schicht fängt deshalb Fehler als `Result` ab, Panics gelten als Bugs und bekommen Tests.

## Abbildung Java-Anleitung → Rust

| Java-Anleitung | Rust |
|---|---|
| `record Foo(a, b)` | `#[derive(Clone, Debug, PartialEq)] pub struct Foo { pub a: A, pub b: B }` |
| `enum` | `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum …` |
| `interface` | `pub trait …` (austauschbar: `Box<dyn Trait>`) |
| `List<T>` / `Map<K,V>` | `Vec<T>` / `BTreeMap<K,V>` (geordnete Ausgabe), `HashMap` nur intern |
| `Optional<T>` | `Option<T>` |
| `IllegalArgumentException` | `Err(Error::InvalidInput(String))` |
| `IllegalStateException` | `Err(Error::InvalidState(String))` |
| `IOException` | `Err(Error::Io(String))` (+ `From<std::io::Error>`) |
| `OperationException` | `Err(Error::Operation(String))` (deutsche Nutzermeldung) |
| `IndexOutOfBoundsException` | `Err(Error::OutOfRange(String))` |
| Intervall `[start, end)` | `std::ops::Range<usize>` |
| `with…()` bei unveränderlichen Dokumenten | `fn with_…(&self, …) -> Self` |
| Sequenz als `String` | **`Vec<u8>`/`&[u8]` (ASCII-Großbuchstaben)**, `String` nur für Anzeige/Dateien |
| `@Test`, `assertEquals` | `#[test]`, `assert_eq!`; Gleitkomma: `assert!((a - b).abs() < eps)` |
| `Assumptions.assumeTrue(tool)` | `if !tools::is_available("mafft") { eprintln!("übersprungen"); return; }` |
| `@Tag("network")` | `#[ignore = "network"]` (laufen mit `cargo test -- --ignored`) |
| `@TempDir` | `tempfile::tempdir()` |
| `Operation`-Interface | `pub trait Operation { fn id(&self) -> &str; fn label(&self) -> &str; fn run(&self, inputs: &[Document], params: &Params, ctx: &mut dyn OperationContext) -> Result<OperationResult>; }` |
| `Document`-Interface | `pub enum Document { Sequence(SequenceDocument), Alignment(AlignmentDocument), Tree(TreeDocument) }` |
| `ProjectStore` (JDBC) | `ProjectStore` mit `rusqlite`; JSON-Payload mit `serde_json` (gleiches Schema wie in Phase 0.5) |
| Methoden `findAllCutSites` | `find_all_cut_sites` (snake_case, Typen `CamelCase`) |
| „Presenter" (testbar ohne Fenster) | Struktur im Kern mit reinen Methoden (`view/presenters/…`) |
| Java-Paritätstest | entfällt; stattdessen übernimmt der Rust-Test die **Erwartungswerte der Java-Tests** |

Namen werden mechanisch übertragen, damit Qwen die Java-Anleitung direkt lesen kann.

## Leistungsvorgaben (verbindlich, mit Benchmarks)

| Bereich | Technik | Ziel (Apple M-Chip, Release) |
|---|---|---|
| Alignment (NW/SW) | zwei `i32`-Zeilen für Scores, Traceback 2 Bit/Zelle (gepackt) | 5 000 × 5 000 Basen < 0,3 s |
| Restriktionsscan | IUPAC-Bitmasken (A=1, C=2, G=4, T=8), Enzyme parallel (`rayon`) | 1 Mb × 79 Enzyme < 50 ms |
| Distanzen/Bootstrap | Spaltenzählung über Tabellen, `rayon` | 100 Seq. × 1 000 Spalten, 100 Bootstrap < 5 s |
| Seed-and-Extend | 2-Bit-k-mere in `u64`, `HashMap<u64, Vec<u32>>` | 1 Mb DB, 1 kb Query < 20 ms |
| Sequenzansicht-Layout | O(sichtbare Zeilen) | 1 Mb: < 1 ms je Aufruf |
| GenBank/FASTA-Parser | `&str`-Zerlegung, Sequenz direkt in `Vec<u8>` | 100 MB in < 1 s (FASTA) |
| FFI-Aufruf | Daten in großen Stücken (Zeilenbereiche, nicht Zeichen) | < 0,2 ms für Layout-Abfrage |

Jede Aufgabe mit einer Zeile aus dieser Tabelle bekommt einen `criterion`-Benchmark und einen groben `#[test]` mit `Instant` (Grenze = 10 × Ziel).

## Regeln für Rust-Code (besonders für Qwen)

1. `cargo check` nach jeder Datei, `cargo test` nach jeder Aufgabe, `cargo clippy -- -D warnings` und `cargo fmt` vor Abschluss.
2. Funktionen nehmen `&[u8]`/`&str`/`&T` und geben eigene Werte zurück. **Keine Lebensdauer-Parameter in öffentlichen Strukturen**; klonen statt Referenzen speichern.
3. Kein `unwrap()`/`expect()` außerhalb von Tests; Fehler mit `?`.
4. Indizes geprüft oder vorab validiert; `saturating_sub`/`checked_sub` bei `usize`.
5. Keine Panics bei ungültigen Eingaben (leere Sequenz, Länge 0, Index am Rand): immer `Result`/leere Ergebnisse, plus je ein Test.
6. Ausgaben deterministisch (kein Iterieren über `HashMap` für Ergebnisse; `BTreeMap` oder sortieren); Zufall nur mit festem Seed.
7. Tests als `#[cfg(test)] mod tests` unter der Datei; Integrationstests in `crates/snapious-core/tests/`; Testnamen auf Deutsch ohne Umlaute (`fn verdau_ecori_zwei_fragmente()`).
8. `///`-Doku auf Deutsch mit Quelle des Algorithmus.
9. **Plattformneutral bleiben:** keine macOS-Aufrufe im Kern; Pfade nur mit `std::path`/`dirs`; Dateinamen von Programmen ohne feste Endung suchen (unter Windows `.exe`);
   Zeilenenden `\n` und `\r\n` akzeptieren; Text immer UTF-8.

## Abweichungen pro Phase (Rust statt Java)

- **Phase 0:** 0.1–0.6 in Rust (`model`, `io::genbank`, `ops`, `store`, `tools`); JSON mit `serde_json`. 0.6 `tools.rs`: `std::process::Command`, stdout/stderr in zwei Threads, Timeout mit `try_wait`-Schleife (20 ms), dann `kill`.
  **0.7 entfällt** (JavaFX) und wird durch **S.0** ersetzt. Neue **Aufgabe 0.0:** Workspace anlegen, `cargo fmt`/`clippy`/`test` laufen, `.gitignore`.
  Neue **Aufgabe 0.9 (Port der Referenzalgorithmen)** in dieser Reihenfolge, jeweils mit den Testwerten der Java-Tests:
  `model::sequence` → `bio::align` → `bio::msa` → `bio::tree`/`newick` → `cloning::enzymes` (79 Enzyme aus `RestrictionEnzymeDB.java`) → `primer` → `search` → `io` (FASTA/FASTQ/GenBank/SAM/VCF) → `coverage`.
- **Phase 1:** `view/*` und `features/*` wie beschrieben, im Kern. 1.11 entfällt als JavaFX-Aufgabe; Presenter bleiben im Kern, Zeichnen übernimmt Swift (S.2–S.4).
- **Phase 2/3:** rein algorithmisch, unverändert; Bootstrap und Distanzen mit `rayon`; Bäume/Gel/SVG-Export liefern `String`/Zahlenlisten. UI-Teile (2.14, 3.9) werden Swift-Aufgaben.
- **Phase 4:** `ureq` hinter einem `HttpFetcher`-Trait (Fakes in Tests). 4.7: 3D-Ansicht über temporäre HTML-Datei mit 3Dmol.js im Standardbrowser (`open`), keine eingebettete WebView.
- **Phase 5:** zurückgestellt.
- **Phase 6:** 6.3 **Plugins** als externe Programme (`~/.snapious/plugins/<name>/plugin.json`; Aufruf mit `--in <datei> --out <ordner> --params <json>`), weil Rust keine stabile ABI hat.
  6.9 Verpackung: `scripts/bundle-app.sh` erzeugt die `.app`; Abdeckung mit `cargo llvm-cov`. Einstellungen: JSON mit `serde`, Dateirechte `0o600`.

---

## FFI-Schicht (`snapious-ffi`)

**Aufgabe F.0 (Claude): UniFFI einrichten.**

- Crate-Typ `["staticlib", "cdylib"]`, `uniffi::setup_scaffolding!()`, Swift-Bindings erzeugt mit `uniffi-bindgen-swift` (Skript `scripts/build-ffi.sh`: `cargo build --release -p snapious-ffi`, dann Bindings nach `app/Sources/SnapiousFFI/`).
- **Schnittstellenentwurf (verbindlich):**
  - Objekt `Project` (Arc, intern `Mutex<ProjectState>`): `open(path)`, `create(path)`, `documents()`, `import_file(path)`, `document(id)`, `delete(id)`, `save()`, `history()`, `undo(id)`, `redo(id)`.
  - Objekt `SequenceSession` (ein geöffnetes Sequenzdokument mit `EditSession`): `length()`, `topology()`, `line_slices(bases_per_line, first_line, last_line) -> Vec<LineSlice>`, `features_in(range) -> Vec<PlacedFeatureDto>`,
    `translation(frame, range)`, `enzyme_sites(filter, names)`, `select(start, end)`, `selection_stats()`, `insert/delete/replace`, `undo/redo`, `circular_map(size) -> MapGeometry`, `linear_map(width) -> MapGeometry`.
  - Operationen: `run_operation(id, params_json, input_ids, progress: Box<dyn ProgressListener>) -> OperationOutcome`, Abbruch über `CancelToken`-Objekt.
  - Alle Typen als einfache Records/Enums (UniFFI-fähig): `u32/u64/f64/String/Vec/Option`; Sequenzen als `String` (ASCII) oder `Vec<u8>`.
  - Fehler als `enum ApiError { InvalidInput(String), Io(String), Operation(String), Internal(String) }` (wird in Swift zu `throws`).
  - **Keine** Rückrufe pro Zeichen oder pro Pixel; Daten in Blöcken (`LineSlice` enthält Zeilenindex, Startposition und den Basen-String der Zeile).
- **Tests:** Rust-Tests rufen die FFI-Funktionen wie Swift es täte (ohne Swift). Dazu ein Swift-Smoketest in S.0, der `ping`/`Project.create` aufruft.

---

## Swift-Aufgaben (Oberfläche)

Alle Swift-Aufgaben setzt **Claude** um (Rückmeldung per Screenshot/Beschreibung durch Felix), weil sie visuell geprüft werden müssen und Swift für ein lokales Qwen schwerer ist.
Technik: **SwiftPM** (`app/Package.swift`), Ziel macOS 14+, Swift 6, SwiftUI für Rahmen (Seitenleiste, Inspektoren, Dialoge), **AppKit-`NSView`** für die großen Zeichenflächen (Core Graphics; Metal erst bei Bedarf).
Gebaut wird mit `swift build -c release` (Command Line Tools genügen; Xcode ist **nicht** erforderlich), Bündelung per `scripts/bundle-app.sh`.

| Nr. | Aufgabe | Voraussetzung |
|---|---|---|
| **S.0** | SwiftPM-Projekt, UniFFI-Bindings eingebunden, Fenster mit Dokumentliste (Projekt öffnen/neu, Sequenz importieren). Menüleiste, `.app`-Bündelung. Smoketest: `Project.create` + Import von `mini.gb` zeigt einen Eintrag. | F.0, Phase 0 |
| **S.1** | Hauptfenster-Rahmen: Seitenleiste (Ordner/Dokumente), Tabs für geöffnete Dokumente, Inspektor rechts (SwiftUI). | S.0 |
| **S.2** | `SequenceView` (`NSView`): zeichnet nur sichtbare Zeilen anhand `line_slices`; Zahlenrand, Gruppen à 10, Komplementärstrang, Farben; Zoom; Maus- und Tastaturauswahl (an `select` übergeben). | S.1, Phase 1.1–1.2 |
| **S.3** | Annotationsspuren und Translation in der `SequenceView` (`features_in`, `translation`). | S.2, 1.3–1.4 |
| **S.4** | `MapView` (zirkulär/linear): Features, Enzymschnitte, Auswahlbogen; Klick setzt Auswahl (an Kern). | S.2, 1.6–1.7 |
| **S.5** | Inspektoren: Auswahlstatistik, Enzymliste (Filter), Featureliste (hinzufügen/löschen), Feature-Erkennung. Undo/Redo, Bearbeiten-Modus. | S.3, 1.5/1.8/1.9 |
| **S.6** | Verdau-Ansicht mit Gel (`GelView`), Klonierungs-Assistent, PCR-/Primer-Dialog, Sequenzierungsabgleich. | S.5, Phase 2 |
| **S.7** | `AlignmentView` (Spalten, Farben, Konsens, Identität, Logo) und `TreeView` (Layout aus dem Kern, Export PNG/SVG). | S.5, Phase 3 |
| **S.8** | NCBI-/BLAST-/UniProt-Dialoge, Protein-Ansicht mit Diagrammen (Swift Charts), RNA-Bogendiagramm, 3D im Browser. | S.5, Phase 4 |
| **S.9** | Workflows-Menü, Plugin-Verwaltung, Einstellungen (Werkzeugpfade prüfen), Dunkelmodus, Drucken, PDF-Export der Karte (macOS bringt PDF-Erzeugung mit: `NSView.dataWithPDF`), Dock-Icon, `.app`-Signierung entfällt (privat). | S.6–S.8, Phase 6 |

**Regeln für die Swift-Schicht:** keine Fachlogik (nichts Rechnen mit Basen, keine Layout-Berechnung); jede Berechnung, die doppelt in der zweiten Oberfläche nötig wäre, gehört in den Kern. `@MainActor` für UI; lange Operationen auf Hintergrund-Tasks mit `ProgressListener` und Abbruch.

---

## Später: Windows/Linux

Ja, das ist machbar, **ohne den Kern anzufassen**. Man schreibt dann nur eine zweite, dünne Oberfläche, die dieselben Funktionen aufruft:

| Option | Aufwand | Hinweis |
|---|---|---|
| **Tauri + Web-UI** (Svelte/React, Canvas) | mittel | Ruft `snapious-core` direkt (Rust) auf; Canvas-Zeichnung nach den Daten aus `view::*`; bestes Aussehen, viele fertige Bausteine |
| **egui/eframe** (reines Rust) | mittel | Gleiche Datenstrukturen, Zeichnen mit `Painter`; schneller Einstieg, schlichtere Optik |
| **Qt/QML** | hoch | Reife Desktop-Toolkit, aber C++-Bindung nötig |

Der Aufwand für die zweite Oberfläche entspricht grob den Swift-Aufgaben S.1–S.9 (ca. 30–40 % des Gesamtprojekts), nicht mehr.
Damit das so bleibt: Leitregel oben einhalten, keine Apple-APIs im Kern, `cargo build --target x86_64-pc-windows-msvc` und `x86_64-unknown-linux-gnu` gelegentlich prüfen (Kern ohne Oberfläche), Pfade und Programmsuche plattformneutral.
Externe Programme (MAFFT usw.) gibt es auch für Windows/Linux; die Pfadsuche ist in `tools.rs` zentral.

---

## Reihenfolge

1. **Aufgabe 0.0** (Workspace) und **0.9** (Referenzalgorithmen portieren) – schafft die Basis mit den Java-Testwerten.
2. **Phase 0** in Rust (0.1–0.6), **F.0** (UniFFI), **S.0** (Swift-Fenster mit Dokumentliste). Danach läuft ein vertikaler Schnitt: importieren, speichern, anzeigen.
3. **Phase 1** (Kern: Qwen; Oberfläche S.1–S.5: Claude).
4. **Phase 2 → 3 → 4 → 6** wie in `00`–`07`; Kern-Aufgaben Qwen, Swift-Aufgaben Claude.
5. Danach optional Phase 5 und die zweite Oberfläche.
