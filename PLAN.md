# Plan: Expanding Snapious into a Geneious Prime Clone

Hobby project for learning, not for distribution.

## Basis of This Plan

- **Geneious side:** In the installed app (Geneious Prime 2026.1.4) I read only the **names** of the bundled plugins (`Contents/Resources/app/bundledPlugins`). There are around 130 plugins, which shows which functional areas exist. No code was decompiled or copied. We implement the features ourselves from public algorithms and documentation.
- **Snapious side:** The existing code in `GeneiousRebuild` (16 modules, about 8,000 lines, 246 green tests). The plan builds on it and does not replace it.
- **REA:** Is installed and registered as MCP server `rea` in `~/.claude.json` (backup: `~/.claude.json.bak-before-rea`). The tools are available after restarting the app. Native analysis would additionally require Ghidra or Hopper. It was not needed for this plan.

## Current State: Geneious Areas vs. Snapious

| Geneious area (plugins) | Snapious today | Gap |
|---|---|---|
| Sequence viewer, annotations, Feature Finder, Extract/Transfer Annotations | `Sequence`, `AnnotationParser`, text output | Graphical sequence view with annotation tracks, zoom, selection: **missing** |
| Import/export (FASTA, FASTQ, GenBank, EMBL, GFF, BED, SAM, VCF, Newick, Nexus, Phylip, Clustal, PIR, GCG, Mega, ACE, SnapGene, AB1) | `FileIO` (FASTA, FASTQ, GenBank, SAM, VCF, Newick) | EMBL, GFF export, BED, Nexus, Phylip, Clustal, AB1 chromatograms, SnapGene (`.dna`) |
| Alignment (MAFFT/MUSCLE/Clustal Omega, Profile Alignment, Translation Align, Realign Region, Strip Columns, Consensus) | `Alignment` (NW, SW), progressive MSA | Alignment viewer, better heuristics, amino acid alignment, integrating external tools |
| Phylogeny (Tree Builder, Tree Viewer) | `Phylogeny` (NJ, UPGMA, Parsimony), `TreeVisualizer` | Maximum likelihood, bootstrap display, interactive tree |
| Primers (Primer Design, Microsats) | `PrimerDesign` | Primer3-like selection, checking primer pairs against a template, in-silico PCR |
| Cloning (Cloning, Cloning Validation, Gel, Clone Manager, Codon Optimization, CRISPR) | `RestrictionEnzymeDB`, `CloningMethod` | Cloning wizard with construct, gel simulation, codon optimization, gRNA design |
| NGS (Assembler, Mapper, FastQC, BBTools, SPAdes, Coverage, Variants, Chimera Filtering, Expression) | `CoverageAnalyzer`, `QualityScoreConverter` | Read mapping, de novo assembly, variant calling, FastQC-style statistics: **feasible almost only via external tools** |
| Databases (NCBI, UniProt, Pfam, InterPro, BLAST, Identify Organism) | `BLASTLikeSearch` (local) | Online access to the NCBI Entrez/BLAST API, UniProt REST |
| Protein (Protein Statistics/Graphs, Jmol, RNAfold, Molecular Structure) | translation only | Protein properties (MW, pI, hydropathy), 3D view, RNA folding |
| Graphs (GC, sequence logo, similarity, lengths, coverage) | text output | Charts in the UI |
| Document management (local DB, folders, favorites, notes, document history, batch rename, duplicates, operations table) | in-memory `Map` | **Persistent project management**: the biggest structural problem |
| Workflows, plugins, server DB, cloud, tutorial | nothing | Workflow chains, plugin API, possibly a server later |

## Architecture Decisions That Must Be Made First

1. **Document model:** Today there is only `Sequence`. Geneious works with general *documents* (sequence, alignment, tree, chromatogram, list, table). A shared `Document` interface with type and metadata is the basis for everything else.
2. **Persistence:** Store projects locally, preferably SQLite (JDBC) or simply structured files. Without persistence the app is just a calculator.
3. **Operations:** Every analysis (alignment, tree, primers, ...) as an operation with parameters, input documents, and result documents. Undo/history, workflows, and background runs follow almost automatically.
4. **External tools instead of building everything:** Integrate MAFFT, MUSCLE, BLAST+, Primer3, IQ-TREE, BWA/minimap2 via process invocation. That is Geneious' own approach (Clustal, MUSCLE, and BBTools are bundled there). The own Java implementations remain as a learning project and as a fallback without external programs.
5. **UI technology:** Either keep Swing (small startup effort, uglier) or switch to JavaFX (better graphics for sequence/tree views). Recommendation: **JavaFX**. The sequence view is the core of the program and needs canvas drawing.

## Phases (order based on the comparison with SnapGene)

Effort figures are rough estimates for one person in their spare time (approx. 8-10 h/week). Each phase ends with a runnable app, green tests, and a short entry in the README. New features get tests before they appear in the UI. Test data are own small files and public plasmids (e.g. pUC19, from GenBank).

### Phase 0: Foundation (approx. 3 weeks)

**Goal:** The architecture on which everything else builds.

| Task | Details |
|---|---|
| Package structure | `model` (documents), `ops` (operations), `store` (persistence), `ui` (JavaFX), `bio` (existing algorithms), `tools` (external programs) |
| `Document` interface | Types: `SequenceDocument`, `AlignmentDocument`, `TreeDocument`, `TableDocument`. Common fields: ID, name, timestamp, metadata, parent documents. `Sequence` becomes the core of `SequenceDocument` (with annotation list and linear/circular topology). |
| Annotations | Own class `Feature` (type, position(s), strand, qualifiers). The `AnnotationParser` populates it, `FileIO` writes it back to GenBank. |
| Operations | `Operation<I,O>` with parameters, input documents, result documents, progress, and cancellation. Each execution creates a `HistoryEntry` (operation, parameters, input/output IDs). |
| Persistence | SQLite via JDBC: tables for documents, features, history. One project = one file `*.snapious`. Import/export remain file formats. |
| External tools | `ToolRegistry`: configurable paths, version check, start a process with timeout, capture stdout/stderr. Tests with a dummy program. |
| UI decision | Introduce JavaFX (module path in the Maven configuration). The Swing UI stays runnable until Phase 1. |

**Done when:** A sequence can be imported, saved, reloaded after restart, and exported to GenBank, including annotations. The history shows the import. ≥ 40 new tests.

### Phase 1: Sequence and Map View (approx. 5 weeks)

**Goal:** The SnapGene-style main window. First truly visible result.

| Task | Details |
|---|---|
| Sequence view | `Canvas`-based, virtualized (draw only visible rows so that 1 Mb sequences stay smooth). Line wrapping, base numbers, complementary strand, color coding, zoom. |
| Selection | Ranges via mouse and keyboard, copying (bases, FASTA, reverse complement), position display, length, live GC content and Tm of the selection. |
| Annotation tracks | Features as arrows with color and label, overlaps on multiple tracks, click selects the bases. |
| Translation | Amino acids below the sequence for the chosen frame or a feature, ORF display (existing `findORFs`). |
| Map | Circular (plasmid) and linear, with features, enzyme cut sites, and selection marker. Map and sequence are synchronized. |
| Enzyme display | Single and double cutters (from `RestrictionEnzymeDB`), filterable by supplier/name, shown in map and sequence. |
| Editing | Insert, delete, replace with undo/redo (builds on `SequenceEditor` and the history), shift features along when editing. |
| Feature detection | Own feature database (`features.json`: promoters, resistance genes, tags, origins) from free sources, search of both strands with tolerance for small deviations. |
| File formats | Import/export EMBL, GFF3, BED, Nexus, Phylip, Clustal; read AB1 chromatograms. Each with a test file. |

**Done when:** pUC19 can be loaded, displayed as map and sequence, a feature edited, an enzyme cut seen, and saved as GenBank. A 1 Mb sequence scrolls without stuttering.

### Phase 2: Cloning and Primers (approx. 7 weeks)

**Goal:** The shared core of both role models. Build constructs virtually instead of only printing text.

| Task | Details |
|---|---|
| Virtual digest | Fragments from `RestrictionEnzymeDB.cut` with real overhangs, fragment list with length, selection of individual fragments. |
| Restriction cloning | Wizard: vector + insert + enzyme(s) → construct as a new `SequenceDocument` with carried-over features. Check for compatibility (`isCompatibleWith`), orientation, reading frame. |
| Gibson / In-Fusion / SLIC | Overlap detection and design, multiple inserts, output primers with overhangs, compute the construct. |
| Golden Gate | Choose a type IIS enzyme, check overhangs for uniqueness (no duplicates), compute the construct, warn on internal sites. |
| Gateway and TOPO | `attB/attP/attL/attR` recombination, TOPO/TA with overhangs. |
| Gel simulation | Lanes, agarose percentage, markers (e.g. 1 kb ladder), migration distance from fragment size (log-linear model). |
| PCR simulation | Check primer binding (3' end exact, mismatches allowed in the 5' region), product as a document. |
| Primer design | Own selection (Tm, GC, length, GC clamp, hairpin/dimer) and optionally Primer3 as a backend. Existing `PrimerDesign` class as a fallback. |
| Codon optimization | From codon usage tables (`CodonUsageAnalyzer`), with organism selection, avoiding restriction sites. |
| CRISPR | gRNA search next to a PAM (NGG and others), off-target counting with mismatches in the genome/plasmid, score. |
| Sequencing comparison | Align a Sanger result (AB1/FASTA) against the construct, mark deviations (builds on the alignment from Phase 3, so initially with the existing `Alignment`). |

**Done when:** A standard case works end to end: digest pUC19 with EcoRI/BamHI, clone an insert, view the construct as a map, simulate the gel, and generate primers for the control PCR. The results match a hand-calculated reference (tests with fixed expected fragment lengths).

### Phase 3: Alignment and Phylogeny (approx. 5 weeks)

| Task | Details |
|---|---|
| Alignment viewer | Column view, color schemes (nucleotides, amino acids), consensus, identity bars, sequence logo, selection of columns and rows, sorting. |
| Backends | MAFFT, MUSCLE, and Clustal Omega via `ToolRegistry`; own `MultipleSequenceAlignment` as a fallback without external programs. |
| Alignment tools | Profile alignment, translation alignment (codons), remove columns, realign region, extract consensus. |
| Tree computation | NJ/UPGMA (existing), plus IQ-TREE or FastTree as a backend with bootstrap. Distance models JC69/K80/p-distance. |
| Tree view | Interactive: set root, rotate/collapse branches, support values, scale bar, export as PNG/SVG and Newick. Extension of the existing `TreeVisualizer`. |
| Similarity graph | Identity matrix as a heatmap. |

**Done when:** An alignment and a tree are produced from ten public 16S sequences, both with MAFFT and with the own fallback. Both results agree in essence.

### Phase 4: Databases and Protein (approx. 4 weeks)

| Task | Details |
|---|---|
| NCBI | Entrez E-utilities: search and download of GenBank entries, respect the rate limit, cache responses locally. |
| BLAST | Remote via the NCBI BLAST API and local via BLAST+ (`makeblastdb`, `blastn/blastp`). Hit list as a table with alignment view. The own `BLASTLikeSearch` remains for small data sets. |
| UniProt / Pfam / InterPro | Retrieval via REST, domains as features on the protein. |
| Protein properties | Molecular weight, isoelectric point, extinction coefficient, hydropathy (Kyte-Doolittle) as a chart, amino acid composition. |
| RNA folding | ViennaRNA (`RNAfold`) as a backend, display of the structure as dot-bracket notation and a simple diagram. |
| 3D structure | Only display a PDB file (embed a WebView with 3Dmol.js or Jmol), no own modeling. |
| Organism identification | BLAST-based, when a hit provides a taxonomy. |

**Done when:** A GenBank entry can be loaded via search, the protein sequence translated, mapped in UniProt, and properties displayed.

### Phase 5: NGS Basics (approx. 6+ weeks, optional)

| Task | Details |
|---|---|
| Quality report | FastQC-style statistics: base quality per position, GC distribution, duplicates, adapters. Builds on `QualityScoreConverter`. |
| Trimming | Quality/adapter trimming (`BBDuk`/`fastp` as a backend or an own simple variant). |
| Mapping | minimap2/BWA as a backend; read BAM/SAM, coverage chart (existing `CoverageAnalyzer`), reads in the sequence view. |
| Variants | Read VCF from `bcftools`, mark variants in the map, table with filter. Own calling only as a simple pileup example. |
| De novo assembly | Only invoke SPAdes/Flye, result as contig documents. |
| Large files | Streaming and indexes (`.fai`, `.bai`); test memory limits. |

**Done when:** A small example data set (phage genome, a few thousand reads) yields a coverage view with one detected variant.

### Phase 6: Workflows, Plugins, and Polish (approx. 4 weeks)

| Task | Details |
|---|---|
| Workflows | Save operations from the history as a repeatable chain, with parameters, batch processing on multiple documents, export as JSON. |
| Plugin API | `ServiceLoader`-based: plugins provide operations, importers/exporters, and views. One example plugin in the repo. |
| Document management | Folders, search, filters, batch rename, find duplicates, favorites, notes on documents. |
| Document history | Display the history per document with access to earlier versions. |
| Polish | Keyboard shortcuts, settings, printing/PDF export of the map, dark mode, installer package (`jpackage`) for macOS. |
| Quality | Evaluate test coverage (JaCoCo), UI smoke tests (TestFX), performance tests for large sequences. |

**Done when:** A workflow "Import → Digest → Ligate → Export map" runs at the push of a button on three files.

## Timeline and Dependencies

| Phase | Duration | Depends on | Can run in parallel with |
|---|---|---|---|
| 0 Foundation | 3 weeks | - | - |
| 1 Sequence/map view | 5 weeks | 0 | Parts of 3 (alignment viewer) |
| 2 Cloning/primers | 7 weeks | 0, 1 | - |
| 3 Alignment/phylogeny | 5 weeks | 0, 1 | 2 |
| 4 Databases/protein | 4 weeks | 0, 1 | 3 |
| 5 NGS | 6+ weeks | 0, 1, 3 | - |
| 6 Workflows/polish | 4 weeks | all | - |

Total about **9-12 months** in spare time. After Phase 2 a SnapGene-like core is done, after Phases 3 and 4 the Geneious breadth of analysis.

## Quality Rules for All Phases

- Every function gets tests with **known reference values** (fixed fragment lengths, Tm values from the literature, results from MAFFT/BLAST), no tests that merely check that nothing is `null`. That was the biggest problem in the old code.
- External programs are used in tests only if they are installed (`Assumptions.assumeTrue`), otherwise the test is skipped.
- The build must be green after every task (`mvn test`).
- Algorithms with a source in a comment (paper, textbook, specification), so it stays clear where they come from.

## The Best of Geneious and SnapGene

SnapGene is not installed on this machine. The basis is the public pages of [SnapGene](https://snapgene.com/features) and the release notes. The two programs have different profiles:

| | Geneious Prime | SnapGene |
|---|---|---|
| Strength | Breadth: alignment, phylogeny, NGS, databases, workflows | Depth in cloning: simulation, plasmid map, documentation |
| Data model | Documents in folders, operations with parameters | One sequence file with history, every manipulation is logged |
| Usability | Many windows and dialogs | One central map plus sequence, very straightforward |
| Weakness | Cloning less convenient | Hardly any analysis beyond cloning |

**What I would take from each:**

| From Geneious | From SnapGene |
|---|---|
| Document and operation model (Phase 0) | Plasmid map (circular and linear) as the main view |
| Alignment with MAFFT/MUSCLE, phylogeny, tree view | Cloning simulation: restriction, Gibson (multiple inserts), Golden Gate, Gateway, TOPO, In-Fusion |
| NCBI/UniProt/BLAST integration | Virtual gel (lanes, agarose percentage, markers) |
| Workflows and batch processing | Automatic feature detection from a feature database (own curated list) |
| Variety of import formats | History of every change, older constructs retrievable |
| | "Align to Reference": check Sanger results against the simulated construct |
| | PCR simulation and automatic primer design |

**Consequences for the phases:**
- **Phase 1** gets the SnapGene-style main view: map and sequence side by side, selection synchronized in both. This is more important than a Geneious-style document tree.
- **Phase 2** is cloning and thus comes before alignment/phylogeny (Phase 3). It is the shared core of both programs and builds directly on `RestrictionEnzymeDB` and `CloningMethod`.
- **History:** The operation model from Phase 0 stores every change (edit, digest, ligation) with input and result. Undo, history, and later workflows follow from it.
- **File format:** SnapGene's `.dna` format is proprietary. Reading it would only be possible via publicly documented parts. Not planned; GenBank/FASTA are enough.
- **Feature database:** Collect own annotations (promoters, resistance genes, tags, origins) from free sources such as Addgene or GenBank entries. SnapGene's curated database is protected by copyright and will not be copied.

## Detailed Specifications for Implementation

For each phase there is a detailed guide with numbered tasks (files, signatures, algorithms, tests with reference values). They are written so that a
language model like Qwen can implement the tasks one after another without follow-up questions:

| File | Content |
|---|---|
| [docs/plan/00-WORKING-INSTRUCTIONS.md](docs/plan/00-WORKING-INSTRUCTIONS.md) | Rules, technical framework, existing code, progress list |
| [docs/plan/01-PHASE-0-FOUNDATION.md](docs/plan/01-PHASE-0-FOUNDATION.md) | Document model, GenBank with features, operations/history, SQLite, external tools, JavaFX skeleton |
| [docs/plan/02-PHASE-1-VIEWS.md](docs/plan/02-PHASE-1-VIEWS.md) | Sequence view, annotations, map, enzymes, editing/undo, feature detection, further formats |
| [docs/plan/03-PHASE-2-CLONING.md](docs/plan/03-PHASE-2-CLONING.md) | Virtual digest, restriction/Gibson/Golden Gate/Gateway/TOPO, gel, PCR, primers, codons, CRISPR, sequencing comparison |
| [docs/plan/04-PHASE-3-ALIGNMENT-PHYLOGENY.md](docs/plan/04-PHASE-3-ALIGNMENT-PHYLOGENY.md) | Alignment document, MAFFT/MUSCLE/Clustal, consensus/logo, trees, bootstrap, tree view |
| [docs/plan/05-PHASE-4-DATABASES-PROTEIN.md](docs/plan/05-PHASE-4-DATABASES-PROTEIN.md) | NCBI, BLAST, UniProt, protein properties, RNA folding, 3D |
| [docs/plan/06-PHASE-5-NGS.md](docs/plan/06-PHASE-5-NGS.md) | FASTQ/BAM, QC, trimming, mapping, coverage, variants, assembly |
| [docs/plan/07-PHASE-6-WORKFLOWS-POLISH.md](docs/plan/07-PHASE-6-WORKFLOWS-POLISH.md) | Workflows, plugins, document management, versions, settings, export, packaging |
| [docs/plan/08-RUST-CORE-SWIFT-APP.md](docs/plan/08-RUST-CORE-SWIFT-APP.md) | **Architecture:** Rust core (all logic) + native Swift app for macOS; mapping Java→Rust, FFI (UniFFI), Swift tasks S.0–S.9, later second UI for Windows/Linux |
| [docs/plan/09-QWEN-WORKFLOW.md](docs/plan/09-QWEN-WORKFLOW.md) | **Working with Qwen3.6-35B-A3B:** task cards, test-first, decomposition rules, Rust error cheat sheet, checklist, verified example card 1.1 |
| [docs/plan/OPEN-QUESTIONS.md](docs/plan/OPEN-QUESTIONS.md) | Collected questions and assumptions made |

## Deliberately Left Out

Cloud, server database, sharing feature, tutorial plugin, commercial import formats (Vector NTI, DNAStar), and all license management. For a private learning project they add nothing.

## Next Concrete Step

Start Phase 0 (see above).

## Risks and Notes

- **License:** Geneious' terms of use usually prohibit reverse engineering. This plan therefore uses only plugin names and publicly known algorithms. Anyone who looks deeper into Geneious or copies code enters a legal gray area.
- **Scope:** A whole team developed Geneious over many years. Phases 1-3 yield a usable sequence editor with cloning. NGS is achievable with reasonable effort only as an integration of external tools.
- **External tools** are themselves licensed (BLAST+ public domain, MAFFT BSD, Primer3 GPL). Uncritical for a private project.
