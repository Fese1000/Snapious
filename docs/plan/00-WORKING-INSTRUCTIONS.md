# Work Instructions (short; given to Qwen as system text with every task)

Project: **Snapious**, a program for viewing, editing, and cloning DNA sequences. Logic and computation in **Rust** (`crates/snapious-core`), user interface in Swift (not your task).
Reference: The Java project in `src/` (verified algorithms and test values). Read only the task you were given.

## Rules

1. Work on **exactly one task**. Change only the files named in the task. Do not delete or rename anything.
2. The tests of the task are given. **Do not change them.** Your goal: `cargo test -p snapious-core <module>` is green.
3. Write one function first, then `cargo check`. Only when it compiles, the next function.
4. After the task: `cargo fmt`, `cargo clippy -p snapious-core -- -D warnings`, `cargo test -p snapious-core`. Everything must be green.
5. No new dependencies, no `async`, no `unsafe`, no macros except `derive`, `vec!`, `format!`, `assert_eq!`.
6. No `unwrap()`/`expect()`/`panic!` in library code. Propagate errors with `Result` and `?`. Always check indices (`get(i)`) or use `saturating_sub`.
7. Keep ownership simple: functions take `&[u8]`/`&str`/`&T` and return owned values. No `'a` in public structs. When in doubt, `.clone()` or `.to_vec()`.
8. Deterministic output: do not iterate over a `HashMap` for results (sort or use `BTreeMap`); randomness only with a fixed seed.
9. Comments (`///`) and error messages in German, names in English (snake_case; types CamelCase).
10. No code from third-party or proprietary software. Algorithms only as described in the task; source as a comment.
11. **You do not ask questions.** If something is unclear: write a line under "Questions from implementation" in `docs/plan/OPEN-QUESTIONS.md` (task, question, your assumption) and continue with the assumption.
12. **At most 3 attempts** for a compiler or test error. After that: leave the state as is, enter the error text (first 20 lines) in `OPEN-QUESTIONS.md`, and stop.

## Response Format

At the end, output only this list:
- Changed files
- `cargo test` result (number green/red)
- Open points (if any)

## Stay Platform-Neutral (important for a later Windows/Linux version)

No macOS calls in the core. Paths only with `std::path::PathBuf` or the crate `dirs`. Text always UTF-8, accept line endings `\n` and `\r\n`.

## Progress (checked off by Claude)

- [x] Phase 0 core 0.1–0.6 (Claude, 25.10.2026): feature model, SequenceDocument, GenBank, operations/history, SQLite storage, program invocations; 101 tests green
- [x] F.0 UniFFI (25.10.2026): `crates/snapious-ffi` (object `Project`, 8 tests), `scripts/build-ffi.sh`, Swift self-test `SnapiousSelfTest` (11 checks green)
- [x] S.0 Swift window (25.10.2026): SwiftUI app `app/Sources/SnapiousApp` (create/open project, import via dialog or drag & drop, document list, rename/delete, detail view with key data, feature table, sequence preview); built with `scripts/bundle-app.sh` (uses the Xcode toolchain via DEVELOPER_DIR), checked by hand against pUC19
- [x] 0.9 Port of the reference algorithms (Claude, 25.10.2026): Sequence, alignment (global/local/semi-global), MSA, distances, Newick, trees (NJ, UPGMA, parsimony, bootstrap, tree tools), restriction enzymes (79, both strands, IUPAC), primers (NN Tm, checks, selection), BLAST-like search, FASTA/FASTQ/SAM/VCF/quality, coverage; 274 tests green. **Not yet ported:** `AnnotationParser` (GFF3), `CodonUsageAnalyzer`, `SequenceEditor` (parts are in `SequenceDocument`), text planner `CloningMethod` (will be replaced by Phase 2).
- Already done as a result (no longer to be given to Qwen): cards 2.8 (Tm/checks), 2.9 (primer selection), 3.6 (Newick, tree tools), 3.7 (distances, NJ/UPGMA, bootstrap, parsimony), 1.1 (Qwen), parts of 2.1 (enzyme search incl. overhangs) and 4.? (local search).
- [x] Phase 1 core 1.1–1.8 (Qwen, checked and improved by Claude, 25.10.2026): line layout, Selection, FeatureLayout, translation, SelectionStats, map geometry, EnzymeMapModel, EditSession; 331 tests green, clippy clean. 1.9 feature detection (written by Claude because Qwen only "thinks" and outputs no code). 1.10 GFF3 parser (Qwen, 2 clippy lines by Claude). Open: 1.11 codon usage, Swift sequence view S.2
- [x] Night of 25/26.10.2026 (Qwen with Claude follow-up, all tests green, clippy clean): core 1.9–1.11 (feature detection, GFF3, codon usage), 2.1 fragments/digest, 2.2 ligation, 2.3 Gibson (`overlap.rs`, `overlap_primers.rs`), 2.4 Golden Gate, 2.5 Gateway/TOPO/TA, 2.6 gel, 2.7 PCR, 2.10 cloning primers, 2.11 codon optimization, 2.12 CRISPR, 2.13 sequencing comparison, 3.1 alignment/tree documents including storage, 3.2 Clustal/PHYLIP, 3.4 alignment metrics, 3.8 tree layout/SVG, 4.5 protein properties, genetic codes, sequence statistics, protein digest, primer annotation; Swift: sequence view with editing/undo, map, digest with gel, CRISPR, cloning, primers, analysis (ORFs/motifs), alignment and tree view, Gibson/Golden Gate via multi-selection.
- [x] Night run part 2 (26.10.2026, 540 Rust tests green, clippy -D warnings clean, Swift self-test green): databases (`db/ncbi.rs`, `db/uniprot.rs`, retrieval via `curl`), ABIF traces (`io/abif.rs`), sequencing comparison (`seqcheck.rs`), local BLAST (`bio/blast.rs`), external aligners/tree programs (`bio/backends.rs`: MAFFT, MUSCLE, Clustal Omega, FastTree, IQ-TREE), RNA folding (`bio/rna_fold.rs`), motif search (`bio/motif.rs`), map SVG (`view/map_svg.rs`), alignment/tree as project documents including export; integration test with real pUC19 (`tests/cloning_puc19.rs`). UI: tabs Sequence (search, amino acid row, editing), Map, Digest+Gel, CRISPR, Cloning, Primers, Analysis, Protein, Sequencing, RNA; NCBI/UniProt dialog; alignment editing. Screenshots without a window: `Snapious --snapshot <project> <folder>`.
- [x] Night run part 3 (26.10.2026, 565 Rust tests green): operation registry with 14 operations (`ops/registry.rs`, `ops/more.rs`), workflows with JSON, storage in the project and batch run (`ops/workflow.rs`, "Workflows" dialog in the app), SnapGene import `.dna` (`io/snapgene.rs`, checked only against the block description, not on real files), GFF3 export and GFF3/BED import as features, adding/deleting features from the selection in the app.
- [x] Night run part 4 (26.10.2026, 603 Rust tests green): lab calculator (`bio/calc.rs`), mutagenesis primers (`primer/mutagenesis.rs`), alignment variants (`bio/variants.rs`), repeats (`bio/repeats.rs`), dot plot (`bio/dotplot.rs`), EMBL (`io/embl.rs`), contig assembly (`bio/assembly.rs`), read mapping (`bio/read_map.rs`); in the app: tab "Tools" (mutagenesis, repeats, calculator), variants table in the alignment, contigs/mapping in the multi-selection, EMBL export, tab overflow menu.
- [x] Phase 0 (reconciliation 26.10.2026): 0.1–0.6 done; 0.7 JavaFX dropped (replaced by S.0); 0.8 wrap-up: README rewritten, test count well above 40.
- [x] Phase 1 (reconciliation and gaps closed 26.10.2026): 1.1–1.11 done. NEXUS (`io/nexus.rs`, export in the app), feature database with 17 templates from `view/standard-features.json` (incl. AmpR, ori, lacZα from pUC19), 1 Mb detection 0.08 s. Only the manual check of scrolling a 1 Mb sequence in the UI is open.
- [x] Phase 2 reconciliation (26.10.2026): gaps closed: `model/rotate.rs` (rotate origin), `bio/kazusa.rs` (Kazusa import, codon tables as JSON), `ops/phase2.rs` (clone.restriction, pcr.simulate, codon.optimize; registry now 17 operations). Qwen delivered all three in the core, Claude added imports/small things. Open: Gateway operations (real att sequences missing), primer.pick/crispr.find/seq.check as operations, ClonePipelineIT.
- [x] Phase 3 reconciliation (26.10.2026): gaps closed: `bio/align_tools.rs` (join profiles, translation alignment, realign region), `ops/phase3.rs` (align.profile, align.translate, align.realign, tree.reroot, tree.collapse; registry now 22 operations). Qwen failed on both cards here (timeout, thinking time), Claude wrote both. Afterwards added: identity heatmap and sequence logo (Alignment menu, `AlignmentCharts.swift`) and `tests/align_tree.rs` (AlignTreeIT).
- [x] Phase 4 reconciliation (26.10.2026, all five cards by Qwen, green): `db/cache.rs` (cache, request throttle), `bio/blast_remote.rs` (NCBI BLAST API with wait times), `db/uniprot_entry.rs` (entry → protein document with features), `io/pdb.rs` (PDB parser), `ops/phase4.rs` (protein.properties, rna.fold; registry 24 operations). Afterwards done: 3D track (`StructureView`, offline), cache and remote BLAST in FFI/UI, db.ncbi.fetch and db.uniprot.fetch (Qwen).
- [x] Phase 5 NGS (26.10.2026, 696+ Rust tests green): `ngs/` with FASTQ stream (`fastq_stream`), FASTA index (`fai`), QC report (`qc`), trimming (`trim`), BAM reader via gzip (`bam`), SNV detection (`pileup`), mapper/SPAdes invocations (`mapper`, `spades`); `tests/ngs_pipeline.rs`. Qwen managed trim and spades, Claude wrote the other six after three failed attempts. UI: tab "NGS" (QC chart with traffic lights, trimming, variants from SAM/BAM with VCF). Open: mapping dialog, coverage track below the sequence, external files table.
- [x] Phase 6 (27.10.2026, 736 Rust tests + random tests + self-test green): core `store/folders.rs` (folders, notes, favorites), `store/versions.rs` (50 versions per document, restore), `store/external.rs` (linked files), `project/search.rs` (search, batch rename, duplicates), `settings.rs`, `export/text.rs`, `plugin.rs` (plugins as programs), `ops/record.rs` (workflow from history), `ops/analysis.rs`, `cloning/site_check.rs`, `bio/codon_tables.rs` (E. coli, human, yeast, mouse from Kazusa). UI: "Management" window (folders, search, document with note/versions/workflow recording/export, settings, plugins), NGS tab with mapping (minimap2/BWA), linked files, and coverage track. In Phase 6 Qwen managed only settings and codon tables, Claude wrote the rest after three failed attempts.\n- [x] Quality pass (27.10.2026): random tests `tests/fuzz_parser.rs` (22 file readers with corrupted inputs), `tests/fuzz_compute.rs` (computation functions), FFI test with extreme values; bugs found and fixed: PDB reader with multi-byte characters, overflow and infinite loop in repeat search with huge parameters; real mapping test with minimap2 (`tests/ngs_mapping.rs`).

## Test Data

Under `crates/snapious-core/testdata/`: `mini.gb` (60 bp, see Phase 0.3), `puc19.gb` (NCBI `L09137`, 2686 bp; origin in `testdata/README.md`).
Own test sequences are always given directly in the task.
