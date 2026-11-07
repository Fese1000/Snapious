# Snapious

**Snapious** ist ein Hobbyprojekt: ein Nachbau der wichtigsten Funktionen von Geneious Prime und SnapGene für die Molekularbiologie, native für macOS.
*A hobby re-implementation of core Geneious Prime / SnapGene features: Rust core, native SwiftUI app for macOS.*

Rechenkern in **Rust**, Oberfläche in **SwiftUI**, verbunden über UniFFI. Ein unabhängiges Projekt, nicht mit Geneious oder SnapGene verbunden.
Das frühere Java-Projekt (`src/`, `pom.xml`) bleibt nur als Referenz; seine alte Beschreibung steht in [docs/README-java-referenz.md](docs/README-java-referenz.md).

## Was die App kann

* **Sequenz & Karte in einer Ansicht**: Plasmidring oder lineare Sequenz, stufenloses Zoomen (Trackpad-Pinch am Mauszeiger), Basen am Ring, endloses Drehen mit zwei Fingern, Codon-Blöcke mit Aminosäuren (CDS, Leseraster, markierte Basen)
* **Bearbeiten** überall: Tippen, Löschen, Kopieren/Einfügen, neue Basen werden automatisch annotiert, Rückgängig, Rückfrage beim Verlassen
* **Annotationen**: Liste, anlegen, ändern, löschen, ausblenden, per Klick Basen markieren
* **Schnittstellen**: Filter (Einzelschneider, Enzymsets, alle bekannten Enzyme, eigene Auswahl)
* **Verdau** mit Gel-Simulation, **Klonieren** (Restriktion, Gibson/In-Fusion/SLIC, Golden Gate, TA/TOPO, Gateway)
* **CRISPR**-gRNA-Suche, **Primer**-Design und PCR-Simulation, **Analyse** (ORFs, Motive, GC-Verlauf, CpG-Inseln, Schwachstellen)
* **Protein** (Translation, Masse, pI, Extinktion, Proteaseverdau), **RNA** (Faltung), Strukturen (PDB, XYZ, MOL, SDF)
* **Sequenzierung**: `.ab1`/`.scf`-Spuren mit Zoom, Qualitäten, Konsens aus mehreren Reads
* **Alignment** (eingebaut, MAFFT, MUSCLE, Clustal Omega), lokales **BLAST**, **NGS**-Werkzeuge (FASTQ, Barcodes, Coverage, Duplikate u. a.)
* **NCBI**- und **UniProt**-Suche mit Import; Import von GenBank, EMBL, FASTA/FASTQ, SnapGene (`.dna`, experimentell), GFF3/BED und vielen weiteren Formaten
* **Workflows**: Abfolgen von Operationen als JSON speichern und anwenden

Offenes und Pläne: [`docs/plan/OFFENE_FRAGEN.md`](docs/plan/OFFENE_FRAGEN.md), Vergleich mit Geneious: [`docs/plan/15-GENEIOUS-VERGLEICH.md`](docs/plan/15-GENEIOUS-VERGLEICH.md).
Stammbäume, Taxonomie und Populationsgenetik sind bewusst nicht Teil des Projekts.

## Aufbau

| Ordner | Inhalt |
|---|---|
| `crates/snapious-core` | Rechenkern: Modell, Formate, Algorithmen, Klonierung, Alignment, Datenbanken, Ansichtslogik |
| `crates/snapious-ffi` | UniFFI-Schnittstelle (`Project`-Objekt) für die Swift-App |
| `app/` | SwiftPM-Paket: `SnapiousApp` (Oberfläche), `SnapiousFFI` (erzeugte Bindings), `SnapiousSelfTest` |
| `scripts/` | `build-ffi.sh`, `bundle-app.sh`, Qwen-Werkzeuge (`scripts/qwen`) |
| `docs/plan` | Pläne, Aufgabenkarten, offene Fragen |

## Bauen und testen

Voraussetzungen: macOS, Rust (stable), Xcode.

```bash
cargo test --workspace                 # alle Rust-Tests
cargo clippy --workspace --all-targets -- -D warnings
scripts/bundle-app.sh                  # baut build/Snapious.app
open build/Snapious.app
cd app && swift run -c release SnapiousSelfTest ../crates/snapious-core/testdata/mini.gb   # Swift ↔ Rust
```

Optionale externe Programme (Homebrew): MAFFT, MUSCLE, Clustal Omega, BLAST+, Primer3, ViennaRNA. Fehlende Programme überspringen die zugehörigen Tests.
Ansichten ohne Fenster als PNG: `Snapious --snapshot <Projekt.snapious> <Ordner>`.

## Arbeitsweise mit Qwen

Kleine Rust-Aufgaben werden als Gerüst mit fertigen Tests vorbereitet; ein lokales Qwen füllt die Rümpfe (`scripts/qwen/run_card.py`).
Ergebnisse werden mit `cargo test` und `clippy -D warnings` geprüft. Details: [`docs/plan/09-QWEN-ARBEITSWEISE.md`](docs/plan/09-QWEN-ARBEITSWEISE.md).

## Mitmachen

Siehe [CONTRIBUTING.md](CONTRIBUTING.md). Sicherheitsprobleme: [SECURITY.md](SECURITY.md). Lizenz: [MIT](LICENSE).
