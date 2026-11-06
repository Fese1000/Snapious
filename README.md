# Snapious

Hobbyprojekt (nicht zum Vertrieb): ein Nachbau der wichtigsten Funktionen von Geneious Prime und SnapGene.
Rechenkern in **Rust**, Oberfläche in **SwiftUI** (macOS). Das frühere Java-Projekt (`src/`, `pom.xml`) bleibt nur als Referenz;
seine alte Beschreibung steht in [docs/README-java-referenz.md](docs/README-java-referenz.md).

## Aufbau

| Ordner | Inhalt |
|---|---|
| `crates/snapious-core` | Rechenkern: Modell, Formate, Algorithmen, Klonierung, Alignment/Bäume, Datenbanken, Ansichtslogik |
| `crates/snapious-ffi` | UniFFI-Schnittstelle (`Project`-Objekt) für die Swift-App |
| `app/` | SwiftPM-Paket: `SnapiousApp` (Oberfläche), `SnapiousFFI` (erzeugte Bindings), `SnapiousSelfTest` |
| `scripts/` | `build-ffi.sh`, `bundle-app.sh`, Qwen-Werkzeuge (`scripts/qwen`) |
| `docs/plan` | Pläne, Aufgabenkarten, Offene Fragen |

## Bauen und testen

```bash
cargo test --workspace                 # alle Rust-Tests
cargo clippy --workspace --all-targets -- -D warnings
scripts/bundle-app.sh                  # baut build/Snapious.app (braucht Xcode; nutzt DEVELOPER_DIR)
open build/Snapious.app
cd app && swift run -c release SnapiousSelfTest ../crates/snapious-core/testdata/mini.gb   # Swift ↔ Rust, auch NCBI/UniProt
```

Externe Programme (Homebrew): MAFFT, MUSCLE, Clustal Omega, FastTree, IQ-TREE, BLAST+, Primer3, ViennaRNA. Fehlende Programme überspringen die
zugehörigen Tests.

## Was die App kann

* Projekte (SQLite), Import FASTA/FASTQ/GenBank (Drag & Drop), Export GenBank/FASTA/Clustal/PHYLIP/Newick/SVG
* **Sequenz**: Ansicht mit Features, Auswahl, Tippen/Löschen mit Rückgängig, Statistik (GC, Tm), Kopieren (FASTA, Rev.-Komplement)
* **Karte**: Kreis und linear mit Enzymstellen
* **Verdau** mit Gel-Simulation, **Klonieren** (Restriktion, Gibson/In-Fusion/SLIC, Golden Gate; Gateway/TOPO im Kern)
* **CRISPR**-gRNA-Suche, **Primer**-Design und PCR-Simulation (Kern), **Analyse** (ORFs, bekannte Motive, GC-Verlauf, CpG-Inseln)
* **Protein**: Translation, Masse, pI, Extinktion, Proteaseverdau
* **Sequenzierung**: .ab1-Spuren ansehen und gegen eine Referenz abgleichen
* **Alignment** (eingebaut, MAFFT, MUSCLE, Clustal Omega) und **Bäume** (NJ, UPGMA, FastTree, IQ-TREE), lokales **BLAST**
* **NCBI**- und **UniProt**-Suche mit Import; Import auch von SnapGene-Dateien (.dna, experimentell) sowie GFF3/BED-Annotationen
* **Workflows**: Abfolgen von Operationen als JSON speichern und auf eine Auswahl anwenden

Nicht umgesetzt: NGS-Teil (zurückgestellt), Workflows/Plugins, 3D-Strukturen, ViennaRNA-Oberfläche. Siehe `docs/plan/OFFENE_FRAGEN.md`.

Ansichten ohne Fenster als PNG: `app/.build/release/Snapious --snapshot <Projekt.snapious> <Ordner>`
(Demo-Projekt: `SnapiousSelfTest --make-demo <Datei> <Sequenzdateien…>`).

## Arbeitsweise mit Qwen

Kleine Rust-Aufgaben werden als Gerüst mit fertigen Tests vorbereitet; ein lokales Qwen füllt die Rümpfe
(`scripts/qwen/run_card.py`, Warteschlange `worker.sh`). Ergebnisse werden mit `cargo test` und `clippy -D warnings` geprüft;
scheitert Qwen, wird nachgebessert (`scripts/qwen/salvage.py`, `wt.sh`). Details: `docs/plan/09-QWEN-ARBEITSWEISE.md`.
