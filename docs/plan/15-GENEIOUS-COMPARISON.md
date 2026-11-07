# Comparison with Geneious Prime (Plugin by Plugin)

Source: the 113 bundled plugins under `/Applications/Geneious Prime.app/Contents/Resources/app/bundledPlugins`.
Approach: For each plugin, the class and label list from the jar (inventory, checked with REA on the cloning plugin: 365 classes,
same names) → Qwen step A (describe only the plugin's functions, without mentioning Snapious) → Qwen step B (check each function against the
feature overview and interface list of Snapious). Script: `scripts/qwen/audit/geneious_audit.py`, raw outputs
`scripts/qwen/audit/out/*.md`, all core gaps `scripts/qwen/audit/kern_luecken.txt` (249 lines, many of them minor UI items).
Supervised: 94 plugins evaluated (cloud, server, sharing, tutorial etc. skipped).

**Reliability:** Qwen errs in individual cases (e.g. it reported "batch rename missing" although `project/search.rs` has it). The list below
has therefore been checked by me against the code (grep): confirmed are back-translation, concatenating sequences, enzyme sets, silent mutation analysis
(no hits in the source). Gateway/TOPO: core exists (`cloning/gateway.rs`, `cloning/topo.rs`), but no interface to the app.

## Gaps by Importance

### A. Molecular Biology (core)
1. **Back-translation** protein → DNA (EMBOSS backtranseq, with codon table) – missing.
2. **Gateway (BP/LR) and TOPO/TA in the app** – core exists, interface and UI missing; att sequences not entered.
3. **Silent mutation analysis** (Geneious: SilentMutationGenerator) – missing.
4. **Enzyme sets** (save and use groups of enzymes), cutting by annotation/index – missing or partial.
5. **Concatenate sequences** – missing.
6. ~~Codon optimization without specific sites~~ – Qwen's misjudgment: `bio/codon_optimize.rs` already has `avoid_sites`; only access from the app is missing.
7. **Mutation finder**: apply found variants to the reference and generate a new sequence – missing.
8. **DNA folding** and primer secondary structure (only RNA exists).
9. **Remove/mask alignment columns** (gaps, constant sites, codon position 1/2/3, NEXUS CHARSETs) – missing.
10. **Annotations**: set comparison (only in A / in all), extracting from consensus, feature finder by database similarity,
    annotation table as a view with filter/CSV – partial.
11. **Create a local BLAST database from a sequence list** (`makeblastdb`) and BLAST taxonomy – check/partial.
12. **Identify Organism** with multiple loci – missing.

### B. Sequence Data (NGS)
13. Barcode/MID splitting, paired-read management, insert size evaluation.
14. Gene expression (RPKM/FPKM/TPM, DESeq2, volcano/PCA), chimera filter (UCHIME), reference assembly with introns/fusions.
15. Microsatellites (peak calling, ladder, allele binning).
16. Sequence length histogram, alignment similarity graph.

### C. Proteins and Structures
17. Domains: Pfam, InterProScan (network), domain view.
18. Antibody/TCR annotation (VDJ, CDR, IMGT), liabilities, clonal grouping (Geneious "Biologics").
19. 3D structure: only Cα trace; no ribbon/ball-and-stick, no states.
20. Structure formats CML, MOL, XYZ, HIN, GPR, NWO.

### D. File Formats
21. Alignments/sequences: MEGA, PileUp, GCG, PIR, RSF, ACE, DNA Strider, DNAStar, Vector NTI, Clone Manager (.cm5), Artemis-SQ.
22. Sequence data: SFF, csfasta+QUAL, SCF, CAF, MAQ.
23. Export: QUAL, CSV/TSV (tables), compressed FASTA/FASTQ.
24. Table import (CSV/TSV/XLSX of sequences and primers).

### E. Less Important
- Gel band intensity, chromatogram scaling by confidence, alignment sorting/filter, document filter by fields.

## Next Steps
Cards for Qwen (Rust core with tests): 1, 3, 5, 6, 7, 9, 10 (set comparison), 16, 21–23. Then interface and UI
(Claude) for 2, 4, 8, 11, 12. NGS and protein packages (13–20) as a separate phase.

## Status of the Cards (Group A, Rust core)
Skeletons with tests exist for: back-translation (`bio/backtranslate.rs`), silent mutations (`bio/silent_mutations.rs`), alignment columns (`bio/align_columns.rs`), annotation comparison (`bio/annotation_compare.rs`), apply variants (`bio/apply_variants.rs`), enzyme sets (`cloning/enzyme_sets.rs`), concatenate sequences (`cloning/concatenate.rs`). Gateway/TOPO in the app and DNA folding are still pending.

## Status of the Cards (second round: formats, NGS, helper functions)
Rust core with tests (Qwen or by hand), not yet connected to the interface and app:
- Reading formats: MEGA (`io/mega.rs`), PIR (`io/pir.rs`), GCG (`io/gcg.rs`), XYZ (`io/xyz.rs`), MOL V2000 (`io/mol.rs`), table import (`io/table_import.rs`).
- Writing: QUAL (`io/qual.rs`), CSV/TSV (`export/table.rs`, with reading).
- NGS: barcode assignment (`ngs/barcode.rs`), RPKM/TPM/log2 (`ngs/expression.rs`), paired reads (`ngs/pairs.rs`), length histogram (`ngs/histogram.rs`).
- Miscellaneous: fragments from cut positions/annotations (`cloning/cut_positions.rs`), allele binning (`bio/allele_binning.rs`).
Qwen runner: `run_card.py` now uses the raw endpoint without a thinking phase (about 30 s instead of 10 minutes per attempt); `QWEN_THINK=1` switches back to the chat interface with a thinking phase.

## Status of the Cards (third round)
Rust core with tests, not yet connected to the interface and app:
- Formats: csfasta/QUAL (`io/csfasta.rs`), ACE (`io/ace.rs`), SCF v3 (`io/scf.rs`), SFF (`io/sff.rs`), CML (`io/cml.rs`).
- Analysis: similarity profile (`bio/similarity_graph.rs`), cut out annotations (`bio/annotation_extract.rs`), chimeras (`bio/chimera.rs`), microsatellite peaks and ladder (`bio/microsat.rs`), Identify Organism with weights (`bio/organism_loci.rs`), feature hits with mismatches (`bio/feature_match.rs`).
Qwen (without a thinking phase) managed only 2 of 11 in this round (`annotation_extract`, `microsat`); I wrote the other nine myself. Pattern: edge cases of the specification (NaN, ties, boundaries) and binary formats rarely succeed for Qwen without a thinking phase.

Connected to the interface and app (rounds 2 and 3): import of csfasta, ACE (contigs with `read` features), SCF, SFF, and the formats of the second round; CML in the structure dialog; similarity profile and chimera check in the alignment menu "Edit"; extract features and feature search by similarity in the Analysis tab; fragment sizes (size standard, peaks) and organism ranking in the Tools tab.

## Status of the Cards (fourth round) and UI
Qwen (8 green: weak points, hmmscan/InterProScan/BLAST tables, coverage, duplicates, recently opened, SDF; 8 red, solved by me: protein metrics, statistics, fragment lengths, k-mer, feature table, abbreviation display, HIN, Nussinov). In the process I found a bug in an earlier Qwen card (`normalize_keys` did not recognize `alt`).
Everything is connected to the interface and app: weak points and aliphatic index/GRAVY in the Protein tab, domains and BLAST table and statistics in the Tools tab, coverage, duplicates, fragment lengths, and k-mer in the NGS tab, simple folding in the RNA tab, filter and sorting of the feature table, HIN and SDF in the structure dialog.
UI: command palette (⌘K), tidied toolbar, task groups instead of 13 tabs, inspector, start page with recently opened projects, understandable error messages, keyboard shortcut overview (⌘/).
Cloning: TA and directional TOPO as well as Gateway (BP/LR, att sites to be entered from the kit manual) in the cloning view.

## Wrap-up (final round)
- Consensus tree by the majority rule (`bio/treeops.rs::consensus_tree`) and bootstrap trees in the app (Alignment menu "Bootstrap", multi-selection of trees).
- Workflow editor with steps (choose operation, order, parameters) next to the JSON view.
- Notice bar at the bottom for feedback, accessibility labels on the icon buttons, amino acids without base colors.
- Clicked through in the app: Protein tab (weak points, aliphatic index, GRAVY), Tools (p-value correction), NGS (coverage, k-mer), Cloning (TA/TOPO, Gateway), workflow editor.

### Deliberately Left Open
- Vector NTI, DNA Strider (no open format known), Clone Manager .cm5.
- The Pfam and InterPro databases themselves (network or large external data); only the programs' outputs are read.
- Antibody numbering (IMGT), 3D with ribbon, instability index (400-value table), att sequences for Gateway (to be entered from the kit manual), DNA folding with energy parameters (only the simple Nussinov variant).
- Progress with cancel for long runs, undo for changes outside the sequence view (versions in the management window exist).
- SnapGene `.dna` with a real file (none available).

## Removed (user's decision)
Phylogenetic trees (NJ/UPGMA/FastTree/IQ-TREE, bootstrap, consensus tree, Newick), taxonomy/organism rankings, population genetics
(allele binning, fragment sizes/microsatellites), and chimera checking have been removed from the core, interface, and app.
Old projects with tree documents remain loadable; the trees are just no longer displayed. Chapters in 04/08 are historical.
