# Phase 5: NGS Basics (optional)

> **Implementation note:** The signatures in this document are written in Java notation. Implementation is in **Rust** (mapping: `08-RUST-KERN-SWIFT-APP.md`, table "Mapping Java guide → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-ARBEITSANWEISUNG.md` first.


Goal: quality report, trimming, mapping via external programs, reading BAM/SAM, coverage view, simple variants, assembly invocation, streaming large files.
This phase is independent of Phase 2 and can be skipped without affecting other phases.

New packages: `ngs/fastq/`, `ngs/qc/`, `ngs/trim/`, `ngs/map/`, `ngs/bam/`, `ngs/variant/`, `ngs/assembly/`, `ngs/index/`.

**Ground rules:** Files are processed **in a streaming fashion** (never load a whole FASTQ/BAM into memory); all iterators are `AutoCloseable`. Test data are small and generated in the test
(`new Random(seed)`); no real sequencing data in the repository.

---

## Task 5.1: FASTQ Streaming and Index

**Prerequisite:** none.

**Files:** `ngs/fastq/FastqReader.java`, `ngs/fastq/FastqWriter.java`, `ngs/fastq/FastqRecord.java`, `ngs/index/FaiIndex.java`, tests.

```java
public record FastqRecord(String name, String comment, String bases, String quality) {
    public int length(); public int[] phred(QualityScoreConverter.EncodingType enc);
}
public final class FastqReader implements Iterator<FastqRecord>, AutoCloseable {
    public static FastqReader open(Path file) throws IOException;      // .gz automatically via GZIPInputStream
    public FastqReader(Reader reader);
}
public final class FastqWriter implements AutoCloseable { public void write(FastqRecord r) throws IOException; ... }
public final class FaiIndex {
    public record Entry(String name, long length, long offset, int basesPerLine, int bytesPerLine) {}
    public static FaiIndex build(Path fasta) throws IOException;
    public static FaiIndex read(Path fai) throws IOException;
    public void write(Path fai) throws IOException;
    public String fetch(Path fasta, String name, long start, long end) throws IOException;       // 0-based, end exclusive; RandomAccessFile
}
```

FASTQ: 4 lines per entry (`@name comment`, bases, `+`, quality). Check: length of bases = length of quality, otherwise `IOException("Line n: …")`.
FAI (samtools format, tab-separated): `name \t length \t offset \t basesPerLine \t bytesPerLine`; `offset` = byte position of the first base; `bytesPerLine = basesPerLine + 1` (LF) or `+2` (CRLF).

**Tests:** 3 example entries → correct names/comments; unequal length → `IOException` with line number; create and read a gzip file in the test; stream 100,000 entries (generated in the test) with low memory
(no `List` of all entries; a counter is enough). FAI: FASTA with two sequences (60 per line) → entries with `basesPerLine=60`, `bytesPerLine=61`; `fetch(fasta,"seq2",58,64)` returns the 6 bases across the line break.

---

## Task 5.2: Quality Report (FastQC-style)

**Prerequisite:** 5.1.

**Files:** `ngs/qc/QcAnalyzer.java`, `ngs/qc/QcReport.java`, tests.

```java
public record QcReport(long reads, int minLength, int maxLength, double meanLength, double gcPercent,
                       double[] meanQualityPerPosition, double[] q25PerPosition, double[] medianPerPosition, double[] q75PerPosition,
                       Map<Integer,Long> meanQualityPerReadHistogram, int[] gcHistogram100, Map<Integer,Long> lengthDistribution,
                       double[][] baseContentPerPosition /* [pos][A,C,G,T,N] in % */,
                       double duplicationPercent, Map<String,Long> overrepresented, double adapterPercent) {}

public final class QcAnalyzer {
    public QcAnalyzer(QualityScoreConverter.EncodingType encoding, String adapter);   // default SANGER, "AGATCGGAAGAGC"
    public QcReport analyze(FastqReader reader, int maxReadsForDuplication);          // default 100000
}
```

**Computation:** Qualities per position as a histogram (0–93) → mean/median/quartiles without storing reads. Duplication: hash of the **first 50 bases** of the first
`maxReadsForDuplication` reads; `duplicationPercent = 100 · (1 − unique/total)`. Overrepresented: sequences (first 50 bases) that make up > 0.1 % of all reads, descending (max. 10).
Adapter: fraction of reads that contain the first 12 bases of the adapter (`AGATCGGAAGAG`) anywhere (exact).

**Tests (reads generated in the test):**
- 4 reads `ACGT` with quality `IIII` (Phred 40) → `meanQualityPerPosition = [40,40,40,40]`, `gcPercent = 50`, `reads = 4`, `minLength = maxLength = 4`.
- Four reads, two of them with quality `!` (0) and two with `I` (40) at the same position → mean 20; convention **lower median** (element with index `(n−1)/2` of the sorted values `[0,0,40,40]`) = 0.
- Duplication: 10 identical reads → `duplicationPercent = 90`.
- Adapter: 2 of 10 reads contain `AGATCGGAAGAG` → `adapterPercent = 20`.
- Length distribution `{4:3, 5:1}` for 3 reads of length 4 and 1 read of length 5.
- Empty input → `reads = 0`, no division by 0.

---

## Task 5.3: Trimming

**Prerequisite:** 5.1.

**Files:** `ngs/trim/Trimmer.java`, `ngs/trim/TrimOptions.java`, tests.

```java
public record TrimOptions(int windowSize, double windowMeanQuality, int minLength, String adapter, int adapterMinOverlap, int maxAdapterMismatches,
                          QualityScoreConverter.EncodingType encoding) {
    public static TrimOptions standard();    // 4, 15, 36, "AGATCGGAAGAGC", 8, 1, SANGER
}
public final class Trimmer {
    public Trimmer(TrimOptions options);
    public Optional<FastqRecord> trim(FastqRecord r);        // empty if shorter than minLength
    public TrimStats run(FastqReader in, FastqWriter out) throws IOException;
}
public record TrimStats(long inputReads, long outputReads, long basesRemoved, long adaptersFound) {}
```

**Algorithm:**
1. **Adapter (3' end):** Find the earliest position `p` from which `bases[p..]` matches the prefix of the adapter (overlap ≥ `adapterMinOverlap` or up to the read end, at most `maxAdapterMismatches` deviations); cut from `p`.
2. **Quality (sliding window, as in the Trimmomatic principle):** A window of `windowSize` from 5' to 3'; as soon as the mean of the Phred values is `< windowMeanQuality`, cut from the window start.
3. Check `minLength`.

**Tests:** Read `ACGTACGTACGT` + adapter start `AGATCGGAAGAG` with full quality → result `ACGTACGTACGT`, `adaptersFound=1`.
Read `ACGTACGTACGT` with qualities `IIIIIIII!!!!` (8 × Phred 40, then 4 × Phred 0), window 4, threshold 15: window mean from index 0–4 = 40, from index 5 = 30, from index 6 = 20, from index 7 = 10 (< 15) → cut at window start index 7 → result `ACGTACG` (7 bases).
Read shorter than `minLength` after trimming → empty. Adapter with 1 mismatch is detected, with 2 it is not (with `maxAdapterMismatches=1`). `run` counts correctly.

---

## Task 5.4: Reading SAM/BAM

**Prerequisite:** 5.1.

**Files:** `ngs/bam/AlignmentRecord.java`, `ngs/bam/Cigar.java`, `ngs/bam/SamReader.java`, `ngs/bam/BamReader.java`, tests.

```java
public record CigarOp(char op, int length) {}      // M I D N S H P = X
public record AlignmentRecord(String readName, int flag, String referenceName, int position /*0-based*/, int mapq,
                              List<CigarOp> cigar, String bases, String quality /*Phred+33*/, int mateRef, int matePos, int templateLength) {
    public boolean isUnmapped(); public boolean isReverse(); public boolean isSecondary(); public boolean isSupplementary(); public boolean isDuplicate();
    public int referenceEnd();               // position + sum of the lengths of M,D,N,=,X
}
public interface AlignmentReader extends Iterator<AlignmentRecord>, AutoCloseable { List<String> referenceNames(); Map<String,Integer> referenceLengths(); }
public final class SamReader implements AlignmentReader { public static SamReader open(Path sam) throws IOException; }
public final class BamReader implements AlignmentReader { public static BamReader open(Path bam) throws IOException; }
```

**SAM:** Header lines `@SQ SN:<name> LN:<len>` → references; data lines with 11 mandatory columns (`QNAME FLAG RNAME POS MAPQ CIGAR RNEXT PNEXT TLEN SEQ QUAL`), `POS` 1-based → internally 0-based, `*` → empty.
**BAM (binary, little-endian, BGZF = concatenated gzip blocks → `GZIPInputStream` reads them back to back):**
- Header: magic `BAM\1`, `l_text` (int32) + text, `n_ref` (int32), per reference: `l_name` (int32), name (incl. NUL), `l_ref` (int32).
- Record: `block_size` (int32), `refID` (int32), `pos` (int32, 0-based), `l_read_name` (uint8), `mapq` (uint8), `bin` (uint16), `n_cigar_op` (uint16), `flag` (uint16), `l_seq` (int32),
  `next_refID` (int32), `next_pos` (int32), `tlen` (int32), `read_name` (NUL-terminated), `cigar` (`n_cigar_op` × uint32: length = `x >> 4`, operation = `"MIDNSHP=X".charAt(x & 0xf)`),
  `seq` (`(l_seq+1)/2` bytes, 4 bits per base, table `"=ACMGRSVTWYHKDBN"`), `qual` (`l_seq` bytes, Phred without offset; `0xFF` = missing), followed by optional fields (skip).
- `refID = -1` → unmapped / `"*"`.

**Tests:** SAM line `r1 0 chr1 5 60 4M 0 0 ACGT IIII` → `position 4`, Cigar `[4M]`, `referenceEnd 8`. `3S4M2D5M`: `referenceEnd = pos + 4 + 2 + 5`.
Flag 16 → `isReverse`; flag 4 → `isUnmapped`. BAM: **create a mini BAM file in the test itself** (header + 2 records, packed into one block with `GZIPOutputStream`), read it, and compare with the
input values. Corrupt file (wrong magic) → `IOException`.

---

## Task 5.5: Mapping Backends

**Prerequisite:** 0.6, 5.4.

**Files:** `ngs/map/Mapper.java`, `ngs/map/Minimap2Mapper.java`, `ngs/map/BwaMapper.java`, tests.

```java
public interface Mapper {
    String name(); boolean isAvailable();
    Path map(Path reference, List<Path> reads, MapOptions options, Path outputSam, OperationContext ctx) throws OperationException;
}
public record MapOptions(String preset /* "sr" (short reads), "map-ont", "map-pb" */, int threads) {}
```

Invocations: minimap2 `minimap2 -ax <preset> -t <threads> <ref.fa> <reads1> [reads2] > out.sam`; BWA: first `bwa index ref.fa` (if index files are missing), then `bwa mem -t <threads> ref.fa reads1 [reads2] > out.sam`.
Output always goes to the given SAM file (redirect stdout via `ToolRegistry` with file redirection; extend `ToolRegistry.run` with a variant `runToFile(...)`).

**Tests:** Fake programs check the command line (preset, threads, paths); with real minimap2 under `Assumptions`: 5 kb reference (random, seed), 200 reads of 100 bp each (cut from the reference, 1 % errors) → at least 95 % with `!isUnmapped` and
position within ±5 bases of the true origin.

---

## Task 5.6: Coverage from Alignments

**Prerequisite:** 5.4, existing `CoverageAnalyzer`.

**Files:** `ngs/map/CoverageFromAlignments.java`, test.

```java
public final class CoverageFromAlignments {
    public static int[] depth(AlignmentReader reader, String referenceName, int referenceLength, boolean skipDuplicates, int minMapq);
    public static CoverageAnalyzer.CoverageResult analyze(AlignmentReader reader, String referenceName, int referenceLength, int threshold);
}
```

CIGAR-aware computation: only `M`, `=`, `X` increase the depth; `D` and `N` do not (optional parameter `countDeletions`); `I`, `S`, `H`, `P` consume no reference. Unmapped, secondary (flag 256) and – depending on the parameter – duplicates
(flag 1024) are skipped. `analyze` builds the existing `CoverageResult` structure from this.

**Tests:** Reference length 20; read A `pos 0, 10M` and read B `pos 5, 5M2D5M` → depth (full): `[1,1,1,1,1,2,2,2,2,2,0,0,1,1,1,1,1,0,0,0]`.
Duplicate flag with `skipDuplicates=true` does not count; `minMapq=20` ignores reads with MAPQ 10; soft clip (`3S5M`) does not shift.

---

## Task 5.7: Simple Variant Calling (Pileup)

**Prerequisite:** 5.4, 5.6.

**Files:** `ngs/variant/Pileup.java`, `ngs/variant/VariantCaller.java`, `ngs/variant/Variant.java`, `io/VcfFormat.java` (writing), tests.

```java
public record Variant(String reference, int position /*0-based*/, char refBase, char altBase, int depth, int altCount, double altFraction, double quality) {}
public final class VariantCaller {
    public VariantCaller(int minDepth, double minAltFraction, int minBaseQuality, int minMapq);   // default 10, 0.2, 20, 20
    public List<Variant> call(AlignmentReader reader, SequenceDocument reference);
}
```

**Algorithm:** Per reference position, count bases from all reads, **CIGAR-aware** (base `i` of the read belongs to the reference position according to the CIGAR walk), ignore bases with quality < `minBaseQuality`.
Alt base = most frequent base differing from the reference; a variant if `depth ≥ minDepth` and `altCount/depth ≥ minAltFraction`. `quality` = `−10·log10(P)` with `P` as the one-sided binomial probability
`P(X ≥ altCount | n=depth, p=0.01)` (error rate 1 %); binomial via logarithmic sum, exact for `n ≤ 1000`, normal approximation above that. No indels (SNVs only); indels are ignored in the pileup.
`VcfFormat.write(List<Variant>, reference)`: header `##fileformat=VCFv4.2`, `#CHROM POS ID REF ALT QUAL FILTER INFO`; `POS` 1-based; INFO `DP=<depth>;AF=<fraction>`.

**Tests:** Reference `AAAAAAAAAA`; 20 reads across all positions, 10 of them with `G` at position 4 (high quality) → exactly one variant: `position 4`, `refBase A`, `altBase G`, `altFraction 0.5`, `depth 20`.
Only 2 of 20 reads with `G` (0.1) → no variant at `minAltFraction 0.2`. Bases with quality 10 are not counted. Depth 5 < `minDepth` → no variant.
VCF output: line `ref\t5\t.\tA\tG\t…` (1-based position = 5).

---

## Task 5.8: De Novo Assembly (invocation only)

**Prerequisite:** 0.6.

**Files:** `ngs/assembly/SpadesAssembler.java`, test.

```java
public final class SpadesAssembler {
    public SpadesAssembler(ToolRegistry tools);
    public List<SequenceDocument> assemble(Path reads1, Path reads2 /*may be null*/, Path workDir, int threads, OperationContext ctx) throws OperationException;
}
```

Invocation: `spades.py -1 reads1 [-2 reads2 | -s reads1] -o <workDir> -t <threads>`. Result: `contigs.fasta` → documents (name = FASTA name, metadata `length`, `coverage` from the SPAdes name `NODE_1_length_1234_cov_56.7`).
Error: exit code ≠ 0 → `OperationException` with the last 20 lines of the output.

**Tests:** Fake `spades.py` (script writes `contigs.fasta` with two contigs `NODE_1_length_100_cov_5.5`, `NODE_2_length_50_cov_2.0`) → 2 documents, metadata `coverage` = `5.5`. Error case: script with exit code 1.

---

## Task 5.9: User Interface (NGS)

**Prerequisite:** 5.1–5.8, 1.11.

- **QC view:** Charts (lines/bars) from `QcReport`: quality per position (box-plot-like with quartiles), quality distribution, GC distribution, length distribution, base composition; summary with traffic light (green/yellow/red by simple thresholds, define them yourself and comment them in the code).
- **Trimming dialog:** Input files, options, progress (`OperationContext`), statistics.
- **Mapping dialog:** Choose reference (document), reads, program/preset; the result is referenced in the project as an alignment file (path) (do **not** copy SAM/BAM into the project file; table `external_files(id, path, kind)` in the project store).
- **Coverage track:** Below the sequence view (depth as an area), variant markers as features (`variation`).
- **Presenter tests:** Traffic-light logic, scaling of the charts (values → pixels) as pure functions.

---

## Overall Acceptance of Phase 5

Integration test `NgsIT`: random 5 kb reference (seed) → simulate reads (200 × 100 bp, one substitution at position 2500 in 50 % of the reads) → QC → trimming → (with minimap2, otherwise skipped) mapping →
coverage array plausible (mean depth ≈ 4) → variant calling finds exactly the substitution at position 2500. Without minimap2: write alignments directly as SAM from the simulated positions and test the same call path.
