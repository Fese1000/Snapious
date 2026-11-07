# Snapious

**Snapious** is a hobby project: a re-implementation of the core features of Geneious Prime and SnapGene for molecular biology, native to macOS.
Rust core, SwiftUI app, connected through UniFFI. It is an independent project and not affiliated with Geneious or SnapGene.

The earlier Java project (`src/`, `pom.xml`) is kept for reference only; its old description is in [docs/README-java-referenz.md](docs/README-java-referenz.md).

## Download

Apple-silicon Macs (macOS 14 or newer): get the DMG from the [latest release](https://github.com/Fese1000/Snapious/releases/latest) and drag Snapious into Applications.
The app is not notarized: on first launch right-click → Open, or run `xattr -cr /Applications/Snapious.app`.

## Features

* **Sequence and map in one view**: plasmid ring or linear sequence, smooth zoom (trackpad pinch at the pointer), bases along the ring, endless two-finger rotation, codon blocks with amino acids (CDS, reading frames, selected bases)
* **Editing everywhere**: type, delete, copy/paste; new bases are annotated automatically; undo; prompt to save when leaving
* **Annotations**: list, create, edit, delete, hide by type, click to select the bases
* **Restriction sites**: filters (single cutters, enzyme sets, all known enzymes, custom picks)
* **Digest** with gel simulation, **cloning** (restriction, Gibson/In-Fusion/SLIC, Golden Gate, TA/TOPO, Gateway)
* **CRISPR** guide search, **primer** design and PCR simulation, **analysis** (ORFs, motifs, GC profile, CpG islands, liabilities)
* **Protein** (translation, mass, pI, extinction coefficient, protease digest), **RNA** folding, structures (PDB, XYZ, MOL, SDF)
* **Sequencing**: `.ab1`/`.scf` traces with zoom, quality values, consensus from several reads
* **Alignment** (built-in, MAFFT, MUSCLE, Clustal Omega), local **BLAST**, **NGS** tools (FASTQ, barcodes, coverage, duplicates and more)
* **NCBI** and **UniProt** search with import; import of GenBank, EMBL, FASTA/FASTQ, SnapGene (`.dna`, experimental), GFF3/BED and many other formats
* **Workflows**: save chains of operations as JSON and apply them

Open items and plans: [`docs/plan/OFFENE_FRAGEN.md`](docs/plan/OFFENE_FRAGEN.md); comparison with Geneious: [`docs/plan/15-GENEIOUS-VERGLEICH.md`](docs/plan/15-GENEIOUS-VERGLEICH.md).
Phylogenetic trees, taxonomy and population genetics are deliberately out of scope. The app interface is English by default; German can be selected in Settings (⌘,). The planning documents under `docs/` are in German.

## Layout

| Folder | Contents |
|---|---|
| `crates/snapious-core` | Core: model, formats, algorithms, cloning, alignment, databases, view logic |
| `crates/snapious-ffi` | UniFFI interface (`Project` object) for the Swift app |
| `app/` | SwiftPM package: `SnapiousApp` (UI), `SnapiousFFI` (generated bindings), `SnapiousSelfTest` |
| `scripts/` | `build-ffi.sh`, `bundle-app.sh`, `make-dmg.sh`, Qwen tools (`scripts/qwen`) |
| `docs/plan` | Plans, task cards, open questions |

## Build and test

Requirements: macOS, Rust (stable), Xcode.

```bash
cargo test --workspace                 # all Rust tests
cargo clippy --workspace --all-targets -- -D warnings
scripts/bundle-app.sh                  # builds build/Snapious.app
scripts/make-dmg.sh                    # builds build/Snapious-<version>-arm64.dmg
open build/Snapious.app
cd app && swift run -c release SnapiousSelfTest ../crates/snapious-core/testdata/mini.gb   # Swift <-> Rust
```

Optional external programs (Homebrew): MAFFT, MUSCLE, Clustal Omega, BLAST+, Primer3, ViennaRNA. Tests that need a missing program are skipped.
Render views to PNG without a window: `Snapious --snapshot <project.snapious> <folder>`.

## Working with Qwen

Small Rust tasks are prepared as a skeleton with finished tests; a local Qwen model fills in the bodies (`scripts/qwen/run_card.py`).
Results are checked with `cargo test` and `clippy -D warnings`. Details: [`docs/plan/09-QWEN-ARBEITSWEISE.md`](docs/plan/09-QWEN-ARBEITSWEISE.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: [SECURITY.md](SECURITY.md). License: [MIT](LICENSE).
