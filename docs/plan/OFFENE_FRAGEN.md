# Open Questions and Assumptions

This document collects everything that was unclear during planning. The questions were **not** asked individually but bundled here.
The assumptions made apply until they are answered. Whoever implements the plans enters new ambiguities here (task, question, assumption made).

## Answered Questions (Felix, 25.10.2026)

1. **Qwen:** Qwen3.6-35B-A3B (MoE, about 3B active parameters), local via Ollama/LM Studio, context window **64k or more**; working mode in `09-QWEN-ARBEITSWEISE.md`. → One task together with `00-ARBEITSANWEISUNG.md` and the affected source files fits in the context;
   process at most one task (or two small ones) per session.
2. **UI/architecture:** first JavaFX, then hybrid (Java + Rust process), **final: Rust core + native Swift app (AppKit/SwiftUI)** for macOS. A second UI for Windows/Linux (Tauri or egui) is possible later and does not change the core. See `08-RUST-KERN-SWIFT-APP.md`.
3. **Phase 5 (NGS):** **postponed**, to be decided later. The specification `06-PHASE-5-NGS.md` is kept in reserve; Phase 6 must **not** depend on it
   (operations from Phase 5 are registered in `OperationRegistry.withBuiltins` only if they exist).
4. **Platform:** macOS only (Apple Silicon).
5. **Downloads:** allowed (pUC19, codon tables, Gateway att sequences, 3Dmol.js); document the origin.
6. **Phase 0:** done by Claude; Qwen starts with Phase 1.
7. **Language:** German (assumption, not contradicted).
8. **External programs:** all installed (Homebrew): MAFFT 7.526, BLAST+ 2.17.0, Primer3 2.6.1, IQ-TREE 3.1.4 (`iqtree3`), MUSCLE 5.3, Clustal Omega 1.2.4, FastTree 2.1.11, ViennaRNA 2.7.0 (the last four via the tap `brewsci/bio`). Rust 1.99.1 (Homebrew) is also installed.

## Open Points After the Swift Decision

- **Xcode:** Felix is installing it right now (useful for previews, Instruments, signing). Building also works with the Command Line Tools (Swift 6.4, AppKit, SwiftUI, Metal available).
- **Who does what:** core tasks (Rust) Qwen, except Phase 0 and 0.9 (Claude); all Swift tasks (S.x) Claude. *Assumption:* so.
- **Java project:** stays as a reference in the folder `src/`; no longer maintained after 0.9 is complete. *Assumption:* do not delete.
- **Second UI (Windows/Linux):** Tauri or egui, to be decided later.

## Open After Reviewing the Guides

- **Pilot run:** Run card 1.1 (see `09-QWEN-ARBEITSWEISE.md`) once with Qwen before handing out further cards. Then adjust the card size.
- **Guides `01`–`07` are still in Java notation** and will be implemented **card by card** as Rust cards with skeleton and tests (Claude, one card in advance). This was deliberately not done up front because skeletons and tests must be verified to compile.
- **Qwen behavior unverified:** Statements about the model's suitability are assessments, not measurements.

## Known Gaps in the Specifications (deliberately left open)

- **Gateway sequences** (`gateway-sites.json`): real arm/core sequences must be entered from the manufacturer's documentation (2.5).
- **Codon tables** for E. coli, human, yeast: generate from Kazusa (2.11).
- **Feature database** (`standard-features.json`): only entries whose sequence can be substantiated from free sources (1.9).
- **3Dmol.js** to be placed manually in the resources (4.7).
- **PDF export** not planned (6.7).
- **Instability index** (4.5) only if the dipeptide table is taken from the original source.
- **Real Sanger files (.ab1)** are not placed in the repository; tests create synthetic files.

## Questions from Implementation

- **Night run 25/26.10.2026 (Claude):**
  - The Swift views (sequence, map, digest with gel, CRISPR, cloning, primers) build without errors but were **not looked at by hand** (no screen access during the night). Please click through once and report anything unusual.
  - Codon tables (E. coli, human, yeast) are still missing: `CodonTable::from_counts` can build them from counts, but the Kazusa data still has to be obtained.
  - Gibson/overlap assembly, Golden Gate, ligation, PCR, gel, CRISPR, and codon optimization are finished in the core; the UI so far handles digest, ligation (restriction cloning), CRISPR, and primer selection.
  - Wallace/NN Tm: overlap Tm uses `nearest_neighbor(250 nM, 50 mM)`.
  - Deviation from plan 2.2: The warning "insert contains internal site" is not implemented.
  - Plan 2.6 claims a denser gel lets fragments migrate further; the formula yields the opposite (correct for agarose). Test and core follow the formula.
  - Qwen queue stopped (file `scripts/qwen/STOP`); to continue, `rm scripts/qwen/STOP` and `nohup scripts/qwen/worker.sh &`, tasks as lines in `scripts/qwen/queue.txt`.
  - UI checked via screenshots without a window (`Snapious --snapshot`): sequence, map, alignment, and tree views look right; the views with tables, input fields, and buttons (digest, CRISPR, cloning, primers, analysis, protein, sequencing, RNA) could not be rendered this way and need to be looked at by hand once.
  - macOS asked for screen recording permission for Claude during testing; this was not confirmed (security setting).
  - BLAST uses `-task blastn` (or `blastn-short` below 30 bases) so that short sequences find hits.
  - SnapGene import (`.dna`) was tested only with self-built files following the known block structure. Please try it with a real `.dna` file and report deviations (flags, feature XML).
  - Workflows: The dialog shows the JSON directly; a graphical editor is missing.
