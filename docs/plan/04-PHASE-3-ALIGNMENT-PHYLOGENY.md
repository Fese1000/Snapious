# Phase 3: Alignment and Phylogeny

> **Implementation note:** The signatures in this document are written in Java notation. The implementation is in **Rust** (mapping: `08-RUST-CORE-SWIFT-APP.md`, table "Mapping Java instructions → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-WORKING-INSTRUCTIONS.md` first.


Goal: alignment documents with viewer logic, external alignment programs (MAFFT, MUSCLE, Clustal Omega) with a built-in fallback,
alignment tools, tree building (NJ/UPGMA, IQ-TREE/FastTree, bootstrap) and an interactive tree view.

New packages: `align/` (document, backends, tools, consensus), `tree/` (document, Newick, operations, layout), `view/` (layout logic), `fx/`.
The rules from `00-WORKING-INSTRUCTIONS.md` continue to apply (reference values, external programs only with `Assumptions`).

---

## Task 3.1: AlignmentDocument and TreeDocument

**Prerequisite:** Phase 0.

**Files:** `model/AlignmentDocument.java`, `model/TreeDocument.java`, extension of `store/ProjectStore` (new document types `alignment`, `tree`),
tests `model/AlignmentDocumentTest.java`, `store/ProjectStoreAlignmentTest.java`.

```java
public final class AlignmentDocument implements Document {
    public AlignmentDocument(String id, String name, Sequence.SequenceType type,
                             List<String> rowNames, List<String> rows, Map<String,String> metadata, Instant created);
    // All rows have equal length (otherwise IllegalArgumentException), characters: sequence alphabet + '-' (gap) + 'N'/'X'
    public int rowCount(); public int columnCount();
    public String row(int i); public String rowName(int i);
    public String ungapped(int i);                         // without '-'
    public char at(int row, int column);
    public AlignmentDocument withRows(List<String> rowNames, List<String> rows);
    public AlignmentDocument removeRows(Set<Integer> indices);
    public AlignmentDocument removeColumns(BitSet columns);
    public AlignmentDocument subAlignment(int fromColumn, int toColumn);       // exclusive
    public AlignmentDocument reorder(List<Integer> order);
}

public final class TreeDocument implements Document {
    public TreeDocument(String id, String name, Phylogeny.TreeNode root, boolean rooted,
                        Map<String,String> metadata, Instant created);
    public Phylogeny.TreeNode root(); public boolean rooted();
    public String toNewick();                              // via TreeNode.toNewick() + ";"
}
```

Serialization in the `ProjectStore` JSON: alignment = `{type, rowNames[], rows[]}`; tree = `{rooted, newick}` (the tree is rebuilt from Newick on load, see 3.6).

**Tests:** Rows of unequal length → exception. `ungapped(0)` of `AC--GT` = `ACGT`. `removeColumns({2,3})` on the rows `AC--GT` and `AG--GT` → `ACGT` and `AGGT`.
`subAlignment(1,3)` returns columns 1 and 2. Round-trip through `ProjectStore` (rows, names, type identical).

---

## Task 3.2: Alignment formats (read/write)

**Prerequisite:** 3.1, Phase 1.10.

**Files:** extension of the formats `ClustalFormat`, `PhylipFormat`, `NexusFormat` to `AlignmentDocument`; new `AlignedFastaFormat` (FASTA with gaps), tests.

- `AlignedFastaFormat.parse(String)`: like FASTA, but `-` and `.` are kept (`.` → `-`); all rows must have equal length, otherwise `IOException`.
- `ClustalFormat.write(AlignmentDocument)`: header line `CLUSTAL W multiple sequence alignment`, blocks of 60 columns, names left-aligned to 16 characters, conservation line (`*` = identical in all rows, otherwise space).
- `PhylipFormat.write`: interleaved, names truncated/padded to 10 characters; name conflicts (same first 10 characters) → `IOException`, unless `relaxed=true`.
- `NexusFormat.write`: `#NEXUS`, `BEGIN DATA; DIMENSIONS NTAX=n NCHAR=m; FORMAT DATATYPE=DNA MISSING=? GAP=-; MATRIX … ; END;`.

**Tests:** one round-trip each with 3 rows × 12 columns, with gaps; Clustal conservation line for columns `A/A/A` → `*`, `A/A/C` → space; Phylip with a name that is too long → truncated, conflicts detected;
Nexus with a comment `[…]` in the header is read correctly.

---

## Task 3.3: Alignment backends

**Prerequisite:** 3.1, Phase 0.6 (`ToolRegistry`).

**Files:** `align/Aligner.java`, `align/AlignmentRequest.java`, `align/MafftAligner.java`, `align/MuscleAligner.java`, `align/ClustalOmegaAligner.java`,
`align/BuiltinAligner.java`, `align/AlignerFactory.java`, tests.

```java
public record AlignmentRequest(List<Sequence> sequences, boolean protein, Map<String,String> options) {}

public interface Aligner {
    String name();
    boolean isAvailable();
    AlignmentDocument align(AlignmentRequest request, OperationContext ctx) throws OperationException;
}

public final class AlignerFactory {
    public static Aligner choose(String preferred, ToolRegistry tools);    // "mafft", "muscle", "clustalo", "builtin"; not available → BuiltinAligner
    public static List<Aligner> all(ToolRegistry tools);
}
```

**Invocation (all in `Files.createTempDirectory`; input as FASTA with unique names `seq1`, `seq2`, … and a name mapping back):**

| Program | Command | Output |
|---|---|---|
| MAFFT | `mafft --auto --quiet --thread 1 in.fa` | stdout = aligned FASTA |
| MUSCLE ≥ 5 | `muscle -align in.fa -output out.fa` | `out.fa` |
| MUSCLE 3.x | `muscle -in in.fa -out out.fa` | `out.fa` |
| Clustal Omega | `clustalo -i in.fa -o out.fa --force --outfmt=fa` | `out.fa` |

Detect the MUSCLE version first with `muscle -version` (text starts with `muscle 5` → new syntax). Read the output with `AlignedFastaFormat` and use the name mapping to reorder
into **input order**. Options: MAFFT `--maxiterate <n>` via `options["iterations"]`. Exit code ≠ 0 → `OperationException` with stderr (first 300 characters).
`BuiltinAligner` calls `MultipleSequenceAlignment.progressiveAlignment` and converts the result to an `AlignmentDocument`.

**Tests:**
- `BuiltinAligner` on `ATGCGTAC`, `ATGAGTAC`, `ATGCGTGC` → 3 rows, all of equal length, removing the gaps yields the input again (invariance).
- `AlignerFactory.choose("mafft", emptyTools)` → `BuiltinAligner`.
- For each program that is actually installed (`Assumptions.assumeTrue(isAvailable)`): alignment of 4 related sequences (~60 bp each, one gap in one of them) → rows of equal length, `ungapped` matches the input, order as in the input.
- Error case: empty input → `OperationException`.
- Test parsing and the name mapping with a **fake program** (shell script in `@TempDir` that outputs the FASTA unchanged), without real tools.

---

## Task 3.4: Consensus, conservation, identity matrix, sequence logo

**Prerequisite:** 3.1.

**Files:** `align/Consensus.java`, `align/Conservation.java`, `align/IdentityMatrix.java`, `align/SequenceLogo.java`, tests.

```java
public final class Consensus {
    /** Per column: most frequent character if its fraction (gaps excluded, computed against all rows) >= threshold, otherwise the IUPAC ambiguity code (DNA) or 'X' (protein); pure gap column → '-'. */
    public static String compute(AlignmentDocument aln, double threshold, boolean ignoreGaps);
    public static char iupac(Set<Character> bases);        // {A,G}→R, {C,T}→Y, {G,T}→K, {A,C}→M, {C,G}→S, {A,T}→W, {C,G,T}→B, {A,G,T}→D, {A,C,T}→H, {A,C,G}→V, all four→N
}
public final class Conservation {
    public static double[] identity(AlignmentDocument aln);          // Fraction of rows with the most frequent non-gap character, 0..1
    public static double[] entropyBits(AlignmentDocument aln);       // Shannon entropy in bits per column (excluding gaps)
}
public final class IdentityMatrix {
    public static double[][] percent(AlignmentDocument aln);         // Pairwise identity in % (identical non-gap columns / columns in which neither has a gap)
}
public final class SequenceLogo {
    public record Column(char[] symbols, double[] bits) {}           // sorted ascending by height
    public static List<Column> compute(AlignmentDocument aln, boolean applySmallSampleCorrection);
}
```

Logo: `R_i = log2(s) − (H_i + e_n)` with `s` = 4 (DNA) or 20 (protein), `H_i` entropy, `e_n` small-sample correction `(s−1)/(2 ln2 · n)`; height of a symbol = frequency × `R_i`
(Schneider & Stephens 1990). Without correction `e_n = 0`.

**Tests:**
- Rows `ACGT`, `ACGT`, `ACGA`: in column 4, `T` has a fraction of 2/3 ≈ 0.667. `Consensus.compute(aln, 0.7, true)` = `ACGW` (T and A → IUPAC `W`), `compute(aln, 0.5, true)` = `ACGT`.
- `Consensus.iupac(Set.of('A','G'))='R'`; `Set.of('A','C','G','T')='N'`.
- Identity: column with `A,A,A` → 1.0; `A,A,C` → 0.6667 (±1e-4); `A,-,-` → 1.0.
- Entropy: `A,C,G,T` (once each) → 2.0 bits; `A,A,A,A` → 0.0.
- Identity matrix: diagonal 100; `ACGT` against `ACGA` → 75; columns with a gap in one row are not counted (`AC-T` vs `ACGT` → 100).
- Logo: four equally frequent bases → total height 0 (without correction); a single base → height 2 bits.

---

## Task 3.5: Alignment tools

**Prerequisite:** 3.3, 3.4.

**Files:** `align/tools/ProfileAlignment.java`, `align/tools/TranslationAlignment.java`, `align/tools/StripColumns.java`,
`align/tools/RealignRegion.java`, `align/tools/ExtractConsensus.java` + operations (`align.profile`, `align.translate`, `align.strip`, `align.realign`, `align.consensus`), tests.

- **ProfileAlignment:** merge two existing alignments into one without changing the gap structure within the alignments.
  The existing private method `MultipleSequenceAlignment.mergeProfiles` becomes `public static AlignmentDocument alignProfiles(AlignmentDocument a, AlignmentDocument b)`
  (same behavior; keep existing tests green). Gap columns arise only as pure insertions in the respective other group.
- **TranslationAlignment:** nucleotide sequences (length a multiple of 3, otherwise exception with the sequence name) → translate (frame 1, `Sequence.translate`) → protein alignment (via the chosen `Aligner`)
  → back-translation: each amino acid is replaced by its original codon, each protein gap column by `---`. Result: DNA alignment (column count = 3 × protein columns).
- **StripColumns:** parameter `gapFractionThreshold` (0..1): remove columns in which the fraction of gaps is ≥ threshold (threshold 1.0 = only pure gap columns; 0.0 = all columns with at least one gap).
- **RealignRegion:** column range `[from,to)` and row selection → realign the ungapped subsequences of these rows within the range using the chosen `Aligner` and write them back; the alignment's column count may change
  (all other rows in the range are padded with gap columns, everything to the left/right remains unchanged).
- **ExtractConsensus:** produces a `SequenceDocument` (name `<Alignment> consensus`) from `Consensus.compute` without gaps.

**Tests:**
- Profile: `A = {ACGT, ACGT}`, `B = {AGT}` → result has 3 rows, `ungapped` of each row unchanged; the first two rows are still identical to each other.
- Translation alignment: `ATGAAATAG`, `ATGAAGTAG` → column count 9, back-translation of each row yields the input; length not a multiple of 3 → `OperationException` with the name.
- Strip: alignment with column 2 containing only gaps → removed at threshold 1.0; for a column with 1 gap out of 4 rows and threshold 0.5 it stays, at 0.25 it is removed.
- RealignRegion with `BuiltinAligner`: rows unchanged outside the range; `ungapped` of each row equals the original.
- ExtractConsensus contains no `-`.

---

## Task 3.6: Newick parser, TreeDocument tools

**Prerequisite:** 3.1.

**Files:** `tree/Newick.java`, `tree/TreeOps.java`, tests. (The existing `Phylogeny.TreeNode` continues to be used; additions there are allowed:
`setSupport`, `getSupport` already exist.)

```java
public final class Newick {
    public static Phylogeny.TreeNode parse(String text);          // one tree, ends with ';'
    public static List<Phylogeny.TreeNode> parseAll(String text); // several
    public static String write(Phylogeny.TreeNode root, int decimals);   // Locale.ROOT
}
```

Grammar: `node := ( '(' node (',' node)* ')' )? label? (':' length)?`; labels may be enclosed in `'…'` (with `''` as escape), underscores are kept, comments `[ … ]`
are ignored; a label on an **internal** node is read as a support value (`setSupport`, if numeric) or as a node name. Malformed parenthesization →
`IllegalArgumentException` with position.

```java
public final class TreeOps {
    public static Phylogeny.TreeNode midpointRoot(Phylogeny.TreeNode unrootedRoot);
    public static Phylogeny.TreeNode rootWithOutgroup(Phylogeny.TreeNode root, Set<String> outgroupLeaves);
    public static Phylogeny.TreeNode ladderize(Phylogeny.TreeNode root, boolean ascending);
    public static Phylogeny.TreeNode rotate(Phylogeny.TreeNode root, Phylogeny.TreeNode node);   // reverse children
    public static Phylogeny.TreeNode collapseBelowSupport(Phylogeny.TreeNode root, double minSupport);
    public static List<Set<String>> clades(Phylogeny.TreeNode root);       // leaf sets of all internal nodes (excluding root, excluding trivial ones)
    public static double robinsonFoulds(Phylogeny.TreeNode a, Phylogeny.TreeNode b);   // number of differing bipartitions
}
```

**Tests:**
- `parse("((A:0.1,B:0.2)90:0.05,C:0.3);")` → root with two children; the first internal node has support 90 and branch length 0.05; `write(...)` returns the same text (choose 2 or 1 decimals as appropriate) — round-trip via `parse(write(x))` is equal in topology and lengths.
- Quoted: `('Homo sapiens':1,B:2);` → leaf name `Homo sapiens`.
- Error: `(A,B` → `IllegalArgumentException`.
- Midpoint root: tree `((A:1,B:1):1,C:5);` (treated as unrooted). The longest distance is A–C = 7, the midpoint lies 3.5 from C, i.e. on the branch to C (1.5 from the branching point). After rooting, root→C and root→A both have distance 3.5 (±1e-9).
- `rootWithOutgroup({"C"})`: C is the sister of the rest.
- `collapseBelowSupport(…, 95)` removes the internal node with support 90 (A and B become direct children of the root), branch lengths add up.
- `robinsonFoulds` of two identical trees = 0; `((A,B),(C,D))` vs `((A,C),(B,D))` = 2.
- Ladderize orders by number of leaves.

---

## Task 3.7: Tree building

**Prerequisite:** 3.1, 3.3, 3.6.

**Files:** `tree/TreeBuilder.java`, `tree/DistanceModel.java`, `tree/Bootstrap.java`, `tree/IqTreeBackend.java`, `tree/FastTreeBackend.java`, operation `tree.build`, tests.

```java
public enum DistanceModel { P_DISTANCE, JUKES_CANTOR, KIMURA_2P, POISSON }      // POISSON for proteins
public enum TreeMethod { NEIGHBOR_JOINING, UPGMA, IQTREE, FASTTREE }

public final class TreeBuilder {
    public TreeBuilder(ToolRegistry tools);
    public TreeDocument build(AlignmentDocument aln, TreeMethod method, DistanceModel model,
                              int bootstrapReplicates, long seed, OperationContext ctx) throws OperationException;
}
public final class Bootstrap {
    /** Felsenstein bootstrap for distance methods: draw columns with replacement, build tree, write bipartition frequency in % onto the reference tree. */
    public static void annotate(Phylogeny.TreeNode referenceTree, AlignmentDocument aln,
                                java.util.function.Function<AlignmentDocument, Phylogeny.TreeNode> builder,
                                int replicates, long seed);
}
```

**Distances** (alignment columns with a gap in either of the two rows are skipped pairwise): `p = differences / compared columns`;
JC69: `d = −3/4 ln(1 − 4/3 p)` (for `p ≥ 0.75` → `Double.POSITIVE_INFINITY`, then capped at 10); K2P: `d = −1/2 ln(1 − 2P − Q) − 1/4 ln(1 − 2Q)` with transitions `P` and transversions `Q`;
Poisson (protein): `d = −ln(1 − p)`. The existing methods in `Phylogeny` (`jukesCantorDistance`, `kimura2ParamDistance`) may be used if their tests confirm the values — otherwise reimplement.

**Backends:**
- IQ-TREE: `iqtree2 -s aln.fa -m MFP -B <n> --prefix out -nt 1 -redo` (without bootstrap, omit `-B`); result `out.treefile` (Newick, support on internal nodes). Search for the program name in this order: `iqtree3`, `iqtree2`, `iqtree` (on the development machine `iqtree3` 3.1.4 is installed via Homebrew; command-line options are compatible).
- FastTree: `FastTree -nt -gtr -quiet aln.fa` (DNA) or `FastTree -quiet aln.fa` (protein); Newick on stdout, support as a value 0..1 → ×100.

**Tests:**
- 4 sequences with clear structure `A1`,`A2` (nearly identical) and `B1`,`B2` (nearly identical, different from A): NJ and UPGMA yield `((A1,A2),(B1,B2))` (compare via `robinsonFoulds == 0` against the expected tree from `Newick.parse`).
- Distance values: `p` for `ACGT` vs `ACGA` = 0.25; JC69 = `−0.75·ln(1 − 4/3·0.25)` = 0.3041 (±1e-4). K2P for one transition in 4 bases (`ACGT` vs `GCGT`): `P=0.25`, `Q=0` → `−0.5·ln(0.5) − 0.25·ln(1)` = 0.3466 (±1e-4).
- Bootstrap with clear separation and 100 replicates: support of the main split ≥ 95; same seed → same values.
- Fake backends (shell scripts that output a fixed Newick file) test the flow without IQ-TREE/FastTree; real programs only with `Assumptions`.

---

## Task 3.8: Tree layout and export

**Prerequisite:** 3.6.

**Files:** `view/TreeLayout.java`, `view/SvgWriter.java`, tests.

```java
public enum TreeStyle { RECTANGULAR_PHYLOGRAM, RECTANGULAR_CLADOGRAM, CIRCULAR }
public record NodeBox(Phylogeny.TreeNode node, double x, double y, double parentX, double parentY, boolean leaf, String label) {}
public final class TreeLayout {
    public TreeLayout(TreeStyle style, double width, double height, double margin);
    public List<NodeBox> layout(Phylogeny.TreeNode root);
}
```

**Rules:** Rectangular: leaves get `y = margin + i · (height − 2·margin)/(n−1)` in leaf order (for `n=1` the middle); internal nodes get `y` = mean of the child `y` values.
`x` phylogram = `margin + (distance from root / largest distance) · (width − 2·margin)`; cladogram: `x` by depth (leaves all the way to the right, `x` = margin + (1 − height/root height) · …, where height = largest number of edges down to a leaf). Circular: leaf angles evenly spaced over `0…2π`, radius = distance (phylogram).
`SvgWriter.write(List<NodeBox>, ...)` produces SVG (lines, labels, scale bar), as a `String` without an external library.

**Tests:** Tree `((A:1,B:1):1,C:2);` phylogram 400×300, margin 10: `x` of A, B, C = 10 + (2/2)·380 = 390 (distance 2 / max 2 = 1); `y` of the three leaves = 10, 150, 290; internal node `y` = 80, `x` = 10 + (1/2)·380 = 200;
root `x` = 10. Cladogram: all leaves `x` = 390. SVG contains exactly 3 `<text` elements for the leaves and is well-formed (parse with `javax.xml.parsers`).

---

## Task 3.9: Operations and user interface

**Prerequisite:** 3.1–3.8, 1.11.

- **Operations:** `align.run`, `align.profile`, `align.translate`, `align.strip`, `align.realign`, `align.consensus`, `tree.build`, `tree.reroot`, `tree.collapse`. One test each for the standard case and the error case.
- **Alignment view (`fx/AlignmentView`, presenter `AlignmentPresenter`):** rows with names on the left, columns on the right; color schemes (nucleotides: A green, C blue, G orange, T red;
  amino acids by group hydrophobic/polar/charged/aromatic/special – self-chosen colors), consensus row, identity bar (`Conservation.identity`), optional sequence logo,
  selection of rows/columns, sorting of rows, double-click on a row name opens the sequence. Drawing is virtualized (only visible rows/columns).
- **Tree view (`fx/TreeView`, presenter `TreePresenter`):** `TreeLayout`, zoom, click selects node/clade, context actions (root here, rotate, collapse, export subtree),
  show/hide support values, export PNG (`WritableImage`), SVG (`SvgWriter`) and Newick.
- **Identity heatmap:** `IdentityMatrix.percent` as a colored area with numbers.
- **Presenter tests:** column selection `[3,6)` → `selectedColumns()`; sorting rows by identity to the consensus yields the expected order; click on a node yields the correct `clades` set.

---

## Overall acceptance of Phase 3 (integration test `AlignTreeIT`)

1. Ten short, self-generated related sequences (mutation of an ancestor sequence with a fixed `Random` seed, some indels) as documents.
2. Alignment with `BuiltinAligner` (and additionally with MAFFT, if installed): all rows of equal length, `ungapped` = original.
3. NJ tree with 100 bootstrap replicates: all leaves present, support values 0–100, Newick round-trip.
4. Root by midpoint, compute layout, write SVG (well-formed).
5. Save everything in a project, reload, identical.
