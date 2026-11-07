# Phase 2: Cloning and Primers

> **Implementation note:** The signatures in this document are written in Java notation. The implementation is in **Rust** (mapping: `08-RUST-KERN-SWIFT-APP.md`, table "Abbildung Java-Anleitung → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-ARBEITSANWEISUNG.md` first.


Goal: build constructs virtually (restriction, Gibson/In-Fusion/SLIC, Golden Gate, Gateway, TOPO/TA), simulate PCR and gels,
design primers, optimize codons, find gRNAs, check sequencing results.

New packages: `cloning/sim/` (simulations), `primer/` (supplementary), `gel/`, `crispr/`, `codon/`, `seqcheck/`.
The existing classes `RestrictionEnzymeDB`, `CloningMethod` (text protocols) and `PrimerDesign` remain and
are used. All simulations are **pure functions** on `SequenceDocument` and are wrapped as an `Operation` (Phase 0.4).

**Basic concepts (for all tasks):**
- All positions 0-based, end exclusive. Sequences in uppercase, alphabet `ACGT` (+`N`).
- "Top strand" = the stored sequence 5'→3'. "Bottom strand" = reverse complement.
- A restriction cut has two positions in the top coordinate system: `cutTop` (cut in the top strand) and
  `cutBottom` (cut in the bottom strand, expressed in top coordinates). If `cutBottom > cutTop`, a
  **5' overhang** of length `cutBottom − cutTop` results; if `cutBottom < cutTop`, a 3' overhang; if both are equal, **blunt**.
- The **overhang in top notation** is the subsequence `top[min(cutTop,cutBottom), max(cutTop,cutBottom))`.
  Example EcoRI `G^AATTC` (cutTop=1, cutBottom=5 relative to the recognition start): overhang `AATT`, 5' type.
  PstI `CTGCA^G` (cutTop=5, cutBottom=1): overhang `TGCA`, 3' type.

---

## Task 2.0: Preparatory work on existing classes

**Prerequisite:** Phase 1.

1. `SequenceDocument.rotate(int newOrigin)` (circular documents only, otherwise `IllegalStateException`; `0 <= newOrigin < length`):
   the new sequence is `bases[newOrigin..] + bases[..newOrigin]`. Features are shifted along: position `p` becomes `(p − newOrigin) mod L`.
   An interval `[a,b)` that spans the new origin is split into two intervals `[a',L)` and `[0,b')` (join).
   Test: `ACGTAC` circular, `rotate(2)` → `GTACAC`. Feature `[2,4)` (GT) becomes `[0,2)`. Feature `[1,4)` (CGT) then spans the new
   origin and becomes a join of `[5,6)` and `[0,2)`; its total length stays 3.
2. `Alignment.semiGlobal(String read, String reference)` (in `alignment/Alignment.java`): global alignment in which
   end gaps in `reference` are **not penalized** (first row/column of the matrix = 0 for the reference dimension; result =
   maximum of the last row of the read). Returns `AlignResult`; additionally `int referenceStart()` and `referenceEnd()`.
   Test: `semiGlobal("ACGT", "TTTACGTTTT")` → score 8, `referenceStart()=3`, `referenceEnd()=7`.
3. `RestrictionEnzymeDB.RestrictionEnzyme` gets `int getCutTopOffset()` (= `getCutPositionWithinRecognitionSite()`) and
   `int getCutBottomOffset()` (= `getBottomStrandCutPosition()`), alias methods only; a test suffices: EcoRI → 1 and 5.

---

## Task 2.1: Virtual digest

**Prerequisite:** 2.0.

**Files:** `cloning/sim/EndType.java`, `cloning/sim/End.java`, `cloning/sim/Fragment.java`, `cloning/sim/VirtualDigest.java`, tests.

```java
public enum EndType { BLUNT, FIVE_PRIME, THREE_PRIME }

/** One end of a DNA fragment. overhang is in top notation (see basic concepts), empty for BLUNT. */
public record End(EndType type, String overhang) {
    public static final End BLUNT_END = new End(EndType.BLUNT, "");
    public End reverseComplement();     // type stays, overhang is reverse-complemented
    public boolean pairsWith(End other);   // see rule below
}

/**
 * Double-stranded fragment. top contains the top strand from its 5' end to its 3' end.
 * left/right are the ends. features are in fragment coordinates.
 */
public record Fragment(String top, End left, End right, List<Feature> features, String sourceName,
                       int sourceStart, int sourceEnd) {
    public int length();                       // top.length()
    public Fragment reverseComplement();       // top rc, left/right swapped and each rc, features mirrored
    public boolean canJoin(Fragment next);     // this.right.pairsWith(next.left)
}

public final class VirtualDigest {
    public VirtualDigest(RestrictionEnzymeDB db);
    public List<Fragment> digest(SequenceDocument doc, List<RestrictionEnzyme> enzymes);
    public List<CutSite> cuts(SequenceDocument doc, List<RestrictionEnzyme> enzymes);   // sorted by cutTop
}
```

**Rule `pairsWith`:** `a.pairsWith(b)` is true if (1) both are `BLUNT` or (2) both have the same type (`FIVE_PRIME` or `THREE_PRIME`)
and `a.overhang().equals(b.overhang())`. (This holds because the top notation of the overhang is compared in each case;
rationale: after joining, Top(left) + Top(right) yields the original sequence, see 2.2.)

**Computing `digest`:**
1. Determine all cuts of the chosen enzymes. For this, add a public method to `RestrictionEnzymeDB`:
   `List<int[]> cutPositions(SequenceDocument doc, RestrictionEnzyme enzyme)`, which returns `{cutTop, cutBottom}` in top coordinates for every
   hit (both strands, for circular sequences also across the origin). Internally these are exactly the values
   `Match.cutTop()` and `Match.cutBottom()` of the class `RestrictionEnzyme` (already computed there, mirrored for both orientations).
   For circular sequences, run the search on `bases + bases.substring(0, maxRecognitionLength−1)` and map positions modulo `L`.
2. Merge duplicate `cutTop` values (different enzymes cutting at the same point).
3. **Linear:** sort cuts; fragments `[0,c1)`, `[c1,c2)`, …, `[ck,L)`. The left end of the first and the right end of the last
   fragment are `BLUNT_END` (document ends). A cut at position 0 or `L` does not produce an empty fragment.
4. **Circular:** no cuts → one fragment (the whole sequence, both ends `BLUNT_END`, no additional marker needed).
   With `k ≥ 1` cuts, `k` fragments result; the last one runs across the origin (rotate the document beforehand with `rotate(c1)`).
5. End types from `cutTop`/`cutBottom`: the right end of the fragment left of the cut and the left end of the fragment right of the cut
   **both** get `End(type, overhang)` with `type` = FIVE_PRIME if `cutBottom > cutTop`, THREE_PRIME if smaller, BLUNT if equal;
   `overhang = top.substring(min, max)` (from the source sequence, modulo length for circular ones).
6. Features: every feature whose range falls completely into the fragment is kept with shifted coordinates;
   features that span a cut are dropped.
7. `sourceStart/End` = cut positions in the source (on wrap: `sourceEnd < sourceStart` allowed).

**Tests (reference values, all verifiable by hand):**
- Linear `AAGGAATTCGATTAGCTAGC` (20 bp), EcoRI (GAATTC at 3, cutTop=4, cutBottom=8): fragments `AAGG` (left `BLUNT`, right `FIVE_PRIME "AATT"`)
  and `AATTCGATTAGCTAGC` (left `FIVE_PRIME "AATT"`, right `BLUNT`). `fragment[0].canJoin(fragment[1])` is true.
- Circular 1000 bp, EcoRI site at 100, BamHI site at 400, otherwise `A` filler: digest with both → 2 fragments with lengths 300 and 700.
  (Assemble the test sequence in the test: `"A"*100 + "GAATTC" + …` and set lengths exactly, total 1000.)
- Circular with one cut site → 1 fragment of the same length, left edge `FIVE_PRIME` (EcoRI), right edge likewise (same overhang).
- PstI (3' overhang): fragment `…CTGCA` | `G…`; left end of the right fragment is `THREE_PRIME "TGCA"`.
- Blunt cutter SmaI (`CCC^GGG`): both ends `BLUNT`, fragments `…CCC` and `GGG…`.
- `reverseComplement()` of a fragment does not change the length, swaps the ends, and `rc.rc.equals(original)` (fragment equality).
- No matching enzyme → one fragment (linear: the whole sequence).
- An enzyme hitting a strand in the opposite orientation (BsaI reverse site) cuts at the mirrored position (test with the sequence from 2.4).

---

## Task 2.2: Restriction cloning (ligation)

**Prerequisite:** 2.1.

**Files:** `cloning/sim/Ligation.java`, `cloning/sim/RestrictionCloning.java`, `cloning/sim/CloningCandidate.java`, operation
`cloning/sim/RestrictionCloningOperation.java` (`id = "clone.restriction"`), tests.

```java
public record CloningCandidate(Fragment vector, Fragment insert, boolean insertReversed,
                               boolean directional, SequenceDocument construct, List<String> warnings) {}

public final class Ligation {
    /** Joins fragments in order into one top strand. Throws IllegalArgumentException if a pair does not match. */
    public static String join(List<Fragment> fragments, boolean circular);
}

public final class RestrictionCloning {
    public RestrictionCloning(VirtualDigest digest);
    /** All possible circular products from the vector and insert digests. */
    public List<CloningCandidate> candidates(SequenceDocument vector, List<RestrictionEnzyme> vectorEnzymes,
                                             SequenceDocument insert, List<RestrictionEnzyme> insertEnzymes);
}
```

**Ligation rule:** fragments `f1, f2, …, fn` in this order; for every pair `fi.canJoin(fi+1)` must hold;
for `circular` additionally `fn.canJoin(f1)`. Result top strand = `f1.top() + f2.top() + … + fn.top()` (without additions). Example
EcoRI/BamHI reconstitution: `…G` + `AATTC…` → `GAATTC`. Example BamHI (overhang `GATC`) with BglII (`GATC`) → `…G` + `GATCT…` → `GGATCT` (hybrid, valid).

**`candidates`:**
1. Digest vector fragments and insert fragments. The **vector backbone candidate** is every vector fragment that does not have both ends
   as `BLUNT_END` document ends, or (circular vector with one cut) the only fragment. By default only the **largest**
   vector fragment is considered as backbone; parameter `allVectorFragments=true` (operation parameter `vectorFragment=index`) permits others.
2. Insert candidates: all insert fragments that have a cut end at at least one end (inner fragments preferred).
3. For every pair (V, I) and both orientations (I or I.reverseComplement()) check: `V.canJoin(I)` and `I.canJoin(V)` (circular).
   Match → `CloningCandidate`. `directional = true` if the two ends of the vector are **different**
   (`!V.left().equals(V.right())`) — then only one orientation is possible.
4. Construct: `top = V.top() + I.top()`; name `<vector.name> + <insert.name>`; topology CIRCULAR;
   keep features of both fragments (shift insert features by `V.length()`); metadata `source = "clone.restriction"`.
5. Warnings: "Insert orientation not directional (both orientations possible)" if two candidates arise for the same (V,I);
   "Insert contains internal site of <enzyme>" if the `digest` of the insert yields more fragments than expected;
   "Vector re-ligation possible" if the two vector ends match each other (`V.right.pairsWith(V.left)`).
6. Sorting: directional candidates first, then by warning count, then by length.

**Tests:**
- Vector (circular, 40 bp) `…GGATCC…GAATTC…` and insert (linear, 20 bp) with an EcoRI site 5' and a BamHI site 3', both enzymes: exactly one
  directional candidate; construct length = vector backbone + insert inner fragment (summed by hand); both recognition sites are present in the construct
  again (`indexOf("GAATTC")` and `indexOf("GGATCC")` ≥ 0).
- EcoRI only (one enzyme): two candidates (both orientations), warning "not directional", warning "re-ligation".
- BamHI vector, BglII insert: candidate present, hybrid `GGATCT` or `AGATCC` in the construct (depending on orientation).
- Incompatible ends (EcoRI vector, BamHI insert only): no candidates, empty list.
- Construct has length `V.length() + I.length()` (sum of the fragments).
- Features are kept: insert feature `[2,6)` appears in the construct at `V.length()+2 .. +6` (forward orientation).

---

## Task 2.3: Gibson / In-Fusion / SLIC

**Prerequisite:** 2.0.

**Files:** `cloning/sim/OverlapAssembly.java`, `cloning/sim/OverlapAssemblyOperation.java` (`id = "clone.overlap"`), tests.

```java
public record Overlap(int fromIndex, int toIndex, int length, double tm) {}

public final class OverlapAssembly {
    public OverlapAssembly(int minOverlap, int maxOverlap);       // default Gibson 15..40; In-Fusion 15..20; SLIC 25..80
    /** fragments in desired order; circular = the last one joins back to the first. */
    public AssemblyResult assemble(List<SequenceDocument> fragments, boolean circular);
}
public record AssemblyResult(SequenceDocument construct, List<Overlap> overlaps, List<String> warnings) {}
```

**Algorithm:**
1. For every pair `(Ai, Ai+1)` (for `circular` also `(An, A1)`) find the **longest** overlap `k` with `minOverlap ≤ k ≤ maxOverlap` for which
   the last `k` bases of `Ai` are identical to the first `k` bases of `Ai+1`. No hit → `IllegalArgumentException`
   naming the fragments.
2. Construct (linear): `A1 + A2[k1..] + A3[k2..] + …`. Circular: the same, then at the end the `An` end overlaps with the start: the overlap
   between `An` and `A1` is cut off from the **end** (`construct = A1 + A2[k1..] + … + An[kn-1..]` without the last `kn` bases).
3. `tm` = melting temperature of the overlap using the nearest-neighbor formula from task 2.8 (`NearestNeighbor.tm`). Until that
   class exists, use the simple Wallace formula; replace it later.
4. Warnings: overlap Tm < 48 °C; overlap Tm > 65 °C; overlap occurs twice in one fragment (not unique); overlap with GC > 80 % or < 20 %;
   ambiguous overlap (the overlap sequence occurs at another fragment end).
5. Keep fragment features (shifted, overlap region deduplicated: a feature that lay entirely in the trimmed overlap part
   of `Ai+1` is dropped if it is present identically in `Ai`).

**Design mode (primers with overhangs):**

```java
public record GibsonPrimers(String forward, String reverse, int overlapLength) {}
public static GibsonPrimers designPrimers(SequenceDocument insert, int insertStart, int insertEnd,
                                          SequenceDocument vector, int vectorCutPosition, int overlapLength);
```

Forward primer = last `overlapLength` bases left of the vector cut position (vector top, `[cut−n,cut)`) + binding part of the insert
(first bases from `insertStart`, length chosen so that Tm (task 2.8) is in the range 58–62 °C; length 18–30). Reverse primer = reverse complement of
(last bases of the insert from `insertEnd` backwards, suitable length) with the reverse complement of the first `overlapLength` bases right of the
cut position **at the 5' end**. Default `overlapLength = 25`. Note in the comment: example with numbers.

**Tests:**
- Fragments `A = X + O1`, `B = O1 + Y + O2`, `C = O2 + Z` with `O1 = "ACGTACGTACGTACGTACGT"` (20), `O2 = "TTGGCCAATTGGCCAATTGG"` (20), `X,Y,Z` each
  30 bases from `ATATAT…`/`CCGG…`: linear → length `30 + 20 + 30 + 20 + 30 = 130`; overlaps `[A→B 20, B→C 20]`.
- Circular with an additional overlap `O3` from `C` to `A`: length = sum of fragment lengths − 3×20.
- Fragments without overlap → `IllegalArgumentException`.
- Overlap of 10 bp at `minOverlap=15` is not detected.
- `designPrimers`: forward starts with the `overlapLength` bases left of the vector cut and ends with the first bases of the insert;
  length of the binding part 18–30; reverse primer is reverse-complementary to the insert end (check: `rc(reverse)` ends with
  the vector overlap sequence on the right).

---

## Task 2.4: Golden Gate

**Prerequisite:** 2.1.

**Files:** `cloning/sim/GoldenGate.java`, `cloning/sim/GoldenGateOperation.java` (`id = "clone.goldengate"`), tests.

```java
public record GoldenGateResult(SequenceDocument construct, List<Fragment> parts, List<String> warnings) {}

public final class GoldenGate {
    public GoldenGate(RestrictionEnzymeDB db, String enzymeName);   // BsaI, BsmBI, Esp3I, BbsI, BpiI, SapI, BspQI
    public GoldenGateResult assemble(List<SequenceDocument> parts, boolean circular);
}
```

**Algorithm:**
1. Digest each part with the type IIS enzyme (`VirtualDigest`). **Inner fragment** = the fragment bounded by a cut site
   on each side (not the first/last outer one). Parts with fewer than two cut sites → warning
   "Part <name> has fewer than 2 cut sites" and the part is used unchanged as an inner fragment with blunt ends.
   If a **circular vector** has two cut sites: the fragment that does **not** contain the recognition sites is the backbone.
2. Form a chain from the inner fragments via `canJoin`: start at a fragment whose left overhang is not needed by any right
   overhang of another fragment (linear case) or at an arbitrary one (circular); follow the unique pairing.
3. Ambiguities → warning: same overhang at several ends; **palindromic** overhang (e.g. `GATC`, `AATT`) that pairs with itself;
   overhang that matches the reverse complement of another (wrong orientation).
4. Internal cut sites in the part (more than two cuts) → warning "Internal recognition site in <name>".
5. Construct with `Ligation.join`. After assembly, **no** recognition sites of the enzyme may remain in the construct
   (recognition sites lie in the cut-off edge regions); otherwise warning.
6. Unconnected fragments → `IllegalArgumentException("Parts cannot be joined into a chain: …")`.

**Test sequences (BsaI: `GGTCTC N^NNNN`, recognition forward; reverse: `GAGACC` with the cut before it):**
Part with edges: `"TT" + "GGTCTC" + "A" + OVH_L + BODY + OVH_R + "T" + "GAGACC" + "TT"` with e.g.
`OVH_L="AATG"`, `OVH_R="GCTT"`, `BODY = "CCCCCCCCCCCC"` (12 C). Expectation: inner fragment `top = "AATG" + BODY` (length 16), `left = FIVE_PRIME "AATG"`,
`right = FIVE_PRIME "GCTT"` (overhang in top notation: the bottom overhang of the right side is `GCTT`).
A second part with left overhang `GCTT` and right overhang `CGAT` (BODY `GGGGGGGGGGGG`) connects; a third part `CGAT`→`AATG` closes
circularly.
- Circular result: length = `3 × (4 + 12)` = 48; contains neither `GGTCTC` nor `GAGACC`.
- Two parts with identical right overhang `GCTT` → warning "duplicate overhang".
- Part with internal `GGTCTC` → warning "Internal recognition site".
- Palindromic overhang `GATC` → warning.
- Unjoinable parts → `IllegalArgumentException`.
- `Esp3I` (same recognition sequence as `BsmBI`, `CGTCTC`) yields the same fragments as `BsmBI`.

---

## Task 2.5: Gateway, TOPO and TA/GC

**Prerequisite:** 2.0.

**Files:** `cloning/sim/Gateway.java`, `cloning/sim/Topo.java`, `src/main/resources/cloning/gateway-sites.json`, tests.

**Gateway (model):** Each `att` site consists of *left arm + core + right arm*: `attB = Bl + core + Br`, `attP = Pl + core + Pr`,
`attL = Bl + core + Pr`, `attR = Pl + core + Br`.
- **BP reaction:** `attB × attP → attL + attR`. Input: product (PCR product with `attB1…attB2` at the ends, linear) + donor vector (circular,
  `attP1…attP2` with a selection gene in between). Result: entry clone (circular) = vector backbone + insert, flanked by `attL1` and `attL2`;
  by-product = selection gene with `attR1/attR2` (warning/second document optional).
- **LR reaction:** `attL × attR → attB + attP`. Entry clone (`attL1…attL2`) + destination vector (`attR1…attR2`) → expression clone with `attB1…attB2`.
- Recombination takes place **within the core**; `att1` recombines only with `att1`, `att2` only with `att2`.
- Implementation: load the four sequences per `att1`/`att2` (`Bl, core, Br, Pl, Pr`) from `gateway-sites.json`. **The real sequences are in the
  public Invitrogen documentation and are not entered here**: in the repository the file initially contains only a placeholder structure,
  and the test data is defined in the test itself (`GatewaySites.of(...)`). Note in `OFFENE_FRAGEN.md` that the real sequences still have to be entered.

```java
public record GatewaySite(String name, String leftArmB, String core, String rightArmB, String leftArmP, String rightArmP) {
    public String attB(); public String attP(); public String attL(); public String attR();
}
public final class Gateway {
    public Gateway(GatewaySite att1, GatewaySite att2);
    public SequenceDocument bpReaction(SequenceDocument attBProduct, SequenceDocument donorVector);
    public SequenceDocument lrReaction(SequenceDocument entryClone, SequenceDocument destinationVector);
}
```

**TOPO / TA / GC:**

```java
public final class Topo {
    /** TA cloning: insert (with 3' A overhangs from PCR) is placed into the linear T vector, both orientations possible. */
    public static List<SequenceDocument> taCloning(SequenceDocument tVector, SequenceDocument pcrProduct);
    /** Directional TOPO (CACC): insert must start with CACC; does not remove CACC; only one orientation. */
    public static SequenceDocument directionalTopo(SequenceDocument topoVector, SequenceDocument insertWithCacc);
}
```

`taCloning`: construct = `tVector.bases() + pcrProduct.bases()` (circular) or `tVector.bases() + rc(pcrProduct.bases())`; return two documents,
named with "(orientation +)" / "(orientation −)". `directionalTopo`: `insert` must start with `CACC`, otherwise `IllegalArgumentException`;
construct = vector + insert.

**Tests:** With self-defined arms (e.g. `Bl="AAAA"`, `core="TTGT"`, `Br="CCCC"`, `Pl="GGGG"`, `Pr="ATAT"`): `attB="AAAATTGTCCCC"`,
`attP="GGGGTTGTATAT"`, `attL="AAAATTGTATAT"`, `attR="GGGGTTGTCCCC"`. BP: product `X + attB1 + INS + attB2 + Y` + donor `D1 + attP1 + SEL + attP2 + D2` → entry clone
contains `attL1 + INS + attL2` and **not** the `SEL`. LR yields `attB1 + INS + attB2` again. Missing att site → `IllegalArgumentException`.
TOPO: assembly lengths = sum; `directionalTopo` without CACC → exception.

---

## Task 2.6: Gel simulation

**Prerequisite:** none.

**Files:** `gel/GelSimulator.java`, `gel/Ladder.java`, `gel/Lane.java`, `gel/Band.java`, tests.

```java
public record Band(int sizeBp, double mobility, double intensity) {}          // mobility 0 (top) … 1 (bottom)
public record Lane(String label, List<Band> bands) {}
public record Ladder(String name, List<Integer> sizes) {
    public static Ladder generic1kb();    // 250, 500, 750, 1000, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 8000, 10000 (generic ladder, no manufacturer specification)
    public static Ladder generic100bp();  // 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1200, 1500
}
public final class GelSimulator {
    public GelSimulator(double agarosePercent);           // 0.5 … 3.0
    public double mobility(int sizeBp);                   // see formula
    public Lane lane(String label, List<Integer> fragmentSizes, double massNg);   // intensity proportional to size/sum
    public Lane ladderLane(Ladder ladder);
}
```

**Formula:** resolution range `[Smin, Smax]` from the table (interpolated log-linearly between the support points):

| Agarose % | Smin (bp) | Smax (bp) |
|---|---|---|
| 0.7 | 500 | 15000 |
| 1.0 | 300 | 10000 |
| 1.5 | 150 | 5000 |
| 2.0 | 100 | 3000 |

`mobility(size) = clamp((log10(Smax) − log10(size)) / (log10(Smax) − log10(Smin)), 0, 1)` (larger fragments migrate less).
Intensity of a fragment = `massNg * size / sum(sizes)`, normalized so that the maximum intensity of a lane is 1.0.

**Tests:** At 1.0 %: `mobility(10000)=0.0`, `mobility(300)=1.0`, `mobility(1732)≈0.5` (±0.01; `sqrt(300·10000)=1732.05`).
Monotonicity: `mobility(500) > mobility(1000) > mobility(5000)` (smaller fragments migrate further). At 1.5 % the range is narrower: `mobility(1000)` at 1.5 % is greater than at 1.0 %.
Sizes outside the range are clamped (`mobility(50)=1.0`
at 1.0 %). Lane with two identical fragments: one band with double intensity (bands of the same size are merged).
Intermediate values of the table (e.g. 1.25 %) lie between the neighboring values.

---

## Task 2.7: PCR simulation

**Prerequisite:** 2.0.

**Files:** `cloning/sim/PcrSimulator.java`, `cloning/sim/PcrOperation.java` (`id = "pcr.simulate"`), tests.

```java
public record PrimerBinding(String primerName, int start, int end, boolean forward, int mismatches, int tailLength) {}
public record PcrProduct(String sequence, int templateStart, int templateEnd, PrimerBinding forward, PrimerBinding reverse) {}
public record Primer(String name, String sequence) {}

public final class PcrSimulator {
    public PcrSimulator(int minThreePrimeMatch, int maxMismatches, int maxProductLength);   // default 12, 2, 10000
    public List<PrimerBinding> findBindings(SequenceDocument template, Primer primer);
    public List<PcrProduct> simulate(SequenceDocument template, List<Primer> primers);
}
```

**Rules:**
- A primer binds at a site if the **last `minThreePrimeMatch` bases at the 3' end match exactly** and the remaining (5'-side) part of the
  primer, as far as it reaches the template, has at most `maxMismatches` deviations. The 5' part that **no longer** matches the template
  (primer extends beyond the template or deviates beyond `maxMismatches`) counts as a tail (`tailLength`) only if it lies **entirely at the 5' end**:
  determine the longest run from the 3' end backwards in which deviations are ≤ `maxMismatches`; everything before it is tail.
- A forward primer binds to the top strand (primer sequence corresponds to the top sequence), a reverse primer binds to the bottom strand (the reverse complement of the primer sequence occurs in the top).
- Product: forward binding on the left, reverse binding to its right (for a circular template also across the origin). Product sequence =
  `forward primer (complete, incl. tail)` + template region between the primer ends + `rc(reverse primer complete)`.
- Discard products longer than `maxProductLength`. Primer pairs in which both primers have the same orientation yield no product.
- Result sorted by length.

**Tests (template as `ATGCCGTAGCTAGCATCGATCGGATCCGATCGATCGTAGCTAGCTAGGCTAGC`, length 53 — verify with `bases.len()` in the test):**
- Forward `ATGCCGTAGCTAGC` (position 0), reverse = reverse complement of the last 14 bases → product has the length of the template (53), sequence equal to the template.
- Forward with 5' tail `GAATTC` + 14 binding bases → product length 53 + 6 = 59, starts with `GAATTC`; `tailLength == 6`.
- Primer with a mismatch at the 3' end (last base wrong) → no binding.
- Primer with one mismatch in the 5' region, `maxMismatches=1` → binding with `mismatches == 1`.
- Two forward primers → no product.
- Circular template: primers on both sides of the origin → product runs across the origin, length correct.

---

## Task 2.8: Melting temperature (nearest-neighbor) and primer scoring

**Prerequisite:** none.

**Files:** `primer/NearestNeighbor.java`, `primer/PrimerChecks.java`, tests. (Alongside the existing `PrimerDesign` class.)

```java
public final class NearestNeighbor {
    /** Tm in °C. Concentrations: oligoNanoMolar (e.g. 250), naMilliMolar (e.g. 50), ignore Mg. */
    public static double tm(String sequence, double oligoNanoMolar, double naMilliMolar);
    public static double deltaGKcal(String sequence, double temperatureCelsius, double naMilliMolar);   // optional, for dimers
}
```

**Parameters (SantaLucia 1998, unified; cite the source in the Javadoc: SantaLucia J Jr., PNAS 95:1460–1465, 1998):**

| Dinucleotide (5'→3') | ΔH (kcal/mol) | ΔS (cal/mol·K) |
|---|---|---|
| AA = TT | −7.9 | −22.2 |
| AT | −7.2 | −20.4 |
| TA | −7.2 | −21.3 |
| CA = TG | −8.5 | −22.7 |
| GT = AC | −8.4 | −22.4 |
| CT = AG | −7.8 | −21.0 |
| GA = TC | −8.2 | −22.2 |
| CG | −10.6 | −27.2 |
| GC | −9.8 | −24.4 |
| GG = CC | −8.0 | −19.9 |

Initiation: per end with a G·C pair ΔH +0.1, ΔS −2.8; per end with an A·T pair ΔH +2.3, ΔS +4.1. Symmetry correction for self-complementary
sequences: ΔS −1.4. Salt: `ΔS_Na = ΔS + 0.368 · (N − 1) · ln([Na+] in mol/l)` with `N` = number of phosphates = length. Result:

`Tm = ΔH · 1000 / (ΔS_Na + R · ln(CT / x)) − 273.15` with `R = 1.987`, `CT` = oligo concentration in mol/l, `x = 4` (not self-complementary) or `1`
(self-complementary). Mind the units: ΔH in kcal → ×1000 for cal.

**Tests (properties, no external reference numbers):**
- Monotonicity: lengthening a sequence increases `tm` (e.g. 15-mer < 20-mer of the same GC content).
- More GC at the same length → higher Tm (`GCGCGCGCGCGCGCGCGCGC` > `ATATATATATATATATATAT`, 20 nt).
- A 20-mer with 50 % GC (e.g. `ATGCGTACCTAGGATCCTAG`) lies between 50 and 62 °C at 250 nM / 50 mM Na⁺.
- Higher salt concentration → higher Tm.
- Symmetry: `tm(s) == tm(reverseComplement(s))` (±0.01).
- Sequence too short (< 2 bases) → `IllegalArgumentException`.

`PrimerChecks`:
- `int maxHomopolymerRun(String)`, `boolean hasGcClamp(String)` (last 2 bases contain G/C, at most 3 G/C in the last 5),
  `double hairpinScore(String)` (largest stem ≥ 4 bp, loop ≥ 3: stem length as score), `double selfDimerScore(String, String)`
  (longest complementary stretch between sequence A and the reverse complement of B, 3' end weighted double).
- Tests: `maxHomopolymerRun("AAAAGC")=4`; `hasGcClamp("ATATATGC")` true, `hasGcClamp("ATATATAT")` false;
  `hairpinScore("GGATCCAAAAGGATCC")` ≥ 6; `selfDimerScore("GGATCC","GGATCC")` ≥ 6 (palindromic).

---

## Task 2.9: Primer design

**Prerequisite:** 2.8.

**Files:** `primer/PrimerPicker.java`, `primer/PrimerConstraints.java`, `primer/PrimerPair.java` (new, **different name** than the existing
inner class: `PickedPair`), tests.

```java
public record PrimerConstraints(int minLength, int maxLength, double minTm, double maxTm, double optTm,
                                double minGc, double maxGc, double maxTmDifference,
                                int minProduct, int maxProduct, boolean requireGcClamp,
                                double oligoNanoMolar, double naMilliMolar) {
    public static PrimerConstraints standard();   // 18, 27, 57, 63, 60, 0.40, 0.60, 3.0, 100, 3000, true, 250, 50
}
public record PickedPrimer(String sequence, int start, int end, boolean forward, double tm, double gc, double penalty) {}
public record PickedPair(PickedPrimer forward, PickedPrimer reverse, int productSize, double penalty) {}

public final class PrimerPicker {
    public PrimerPicker(PrimerConstraints constraints);
    /** Best pair that encloses the target range [targetStart, targetEnd). */
    public List<PickedPair> pickPairs(String template, int targetStart, int targetEnd, int maxResults);
    /** Primer with a fixed 5' end (e.g. cloning): forward from a position, free length. */
    public Optional<PickedPrimer> pickForwardAt(String template, int start);
    public Optional<PickedPrimer> pickReverseEndingAt(String template, int endExclusive);
}
```

**Algorithm:**
- Forward primer candidates: start positions in `[max(0, targetStart − maxProduct+…) , targetStart]` (at least such that the target lies in the product) with
  lengths `minLength…maxLength`; reverse primer symmetrically to the right of `targetEnd`.
- A candidate is valid if Tm, GC, GC clamp (if required), homopolymer ≤ 4, hairpin score < 6, self-dimer score < 7 are satisfied.
- Penalty (smaller is better): `|tm − optTm| + 5·|gc − 0.5| + 2·hairpinScore/10 + 2·selfDimerScore/10`.
- Pairs: `|Tm_f − Tm_r| ≤ maxTmDifference`, product size in `[minProduct, maxProduct]`, cross-dimer score `< 7`. Pair penalty = sum of the primer penalties + `|Tm_f − Tm_r|`.
- Return sorted by penalty, at most `maxResults`.

**Tests:** template of 400 bases, generated deterministically in the test (`new Random(42)`, uniformly distributed `ACGT`) with target range `[150,250)`:
- There is ≥ 1 pair; both primers satisfy the constraints (length, Tm range, GC); product size ≥ 100; target lies within the product;
  forward primer sequence is **exactly** `template.substring(start,end)`, reverse primer is the reverse complement of the template subsequence.
- `pickForwardAt(template, 100)` returns a primer that starts at position 100.
- Extremely GC-rich template (only `G`/`C`) → no pair (empty list), no error.
- Template shorter than `minLength` → empty list.

---

## Task 2.10: Cloning primers (extensions)

**Prerequisite:** 2.9, 2.3.

**Files:** `primer/CloningPrimers.java`, test.

```java
public final class CloningPrimers {
    /** Restriction cloning: tail = clamp + recognition site (+ optional linker) before the binding part. */
    public static PickedPair withRestrictionSites(String template, int start, int end, PrimerPicker picker,
                                                  RestrictionEnzyme forwardEnzyme, RestrictionEnzyme reverseEnzyme,
                                                  String clamp);     // clamp e.g. "GCGC" (4-6 bases)
}
```

Forward: `clamp + forwardEnzyme.getRecognitionSequence() + bindingBases(from start)`; reverse: `clamp + reverseEnzyme.getRecognitionSequence() + rc(bindingBases)`.
Enzymes with an ambiguous recognition sequence (characters other than ACGT) → `IllegalArgumentException`. As an exception, the warning "enzyme cuts internally" is returned in the result object:
`record CloningPrimerResult(PickedPair pair, List<String> warnings)`; add the warning if the insert fragment (template from `start` to `end`) contains the recognition sequence.

**Tests:** EcoRI/BamHI primers for a 200 bp fragment without these sites: forward primer starts with `GCGCGAATTC`, reverse primer with `GCGCGGATCC`; fragment with
an internal EcoRI site → warning. Digesting the hypothetical PCR product (`PcrSimulator` with these primers) yields an inner fragment with EcoRI/BamHI ends (integration of 2.7, 2.1).

---

## Task 2.11: Codon optimization

**Prerequisite:** none (uses `Sequence.aminoAcidFor` from Phase 1.4).

**Files:** `codon/CodonTable.java`, `codon/CodonOptimizer.java`, resource `src/main/resources/codon/<organism>.json`, tests.

```java
public final class CodonTable {
    public static CodonTable fromJson(String json);                 // { "organism": "...", "codons": { "ATG": 1.0, "GCT": 0.18, ... } } (relative frequency per amino acid or per thousand)
    public static CodonTable load(String resourceName);              // e.g. "ecoli-k12"
    public double frequency(String codon);
    public List<String> codonsFor(char aminoAcid);                   // by descending frequency
}
public enum Strategy { MOST_FREQUENT, HARMONIZED }
public record OptimizationOptions(Strategy strategy, Set<String> avoidRecognitionSites, int maxHomopolymer,
                                  double minGc, double maxGc, long seed) {
    public static OptimizationOptions standard();    // MOST_FREQUENT, empty, 6, 0.35, 0.65, 1
}
public final class CodonOptimizer {
    public CodonOptimizer(CodonTable table);
    public String optimize(String proteinSequence, OptimizationOptions options);     // returns DNA (without stop; stop '*' → best stop codon)
    public String optimizeDna(String codingDna, OptimizationOptions options);        // keep the protein, replace codons
}
```

**Algorithm:**
1. `MOST_FREQUENT`: for each amino acid the most frequent codon. `HARMONIZED`: choose the codon randomly (deterministic with `seed`) according to the frequency distribution.
2. Then correction steps in fixed order: (a) If a forbidden recognition site (IUPAC-free, ACGT sequences only) occurs in the result, replace one of the involved codons with
   the next most frequent synonymous codon that removes the site without creating a new one. (b) Break homopolymer runs longer than `maxHomopolymer`:
   replace one codon in the run. (c) Global GC content outside `[minGc, maxGc]`: repeatedly replace the codon with the smallest frequency change
   that moves the GC content in the desired direction, until it is within the window or no change is possible anymore.
3. The result must **always encode the same protein** (test!), regardless of the options.

**Resources:** Create `src/main/resources/codon/test-table.json` with a **small synthetic** table (for tests). Real tables
(E. coli K-12, Homo sapiens, S. cerevisiae) come from the public Kazusa database (`https://www.kazusa.or.jp/codon/`); please note in `OFFENE_FRAGEN.md` that these files have to be generated from
Kazusa, and write a small helper program `tools/ImportKazusa.java` that converts the text representation of the page into the JSON format (test with a
self-written mini example in the Kazusa text format).

**Tests (synthetic table: per amino acid one "best" codon with frequency 0.6 and a second with 0.4):**
- `optimize("MKL*", MOST_FREQUENT)` yields the best codon in each case; back-translation (`Sequence.translate`) yields `MKL*`.
- With `avoidRecognitionSites={"GAATTC"}`: the result does not contain `GAATTC`, protein unchanged (choose the input protein so that the best choice would create the site).
- Homopolymer: input `KKKKKK` (AAA best) with `maxHomopolymer=6` → result contains no run `AAAAAAA` (7).
- Deterministic: twice with the same `seed` gives the same result; `HARMONIZED` gives roughly a 60/40 distribution for `L` over 1000 amino acids (±8 %).
- Empty protein → empty string; invalid character → `IllegalArgumentException`.

---

## Task 2.12: CRISPR gRNA search

**Prerequisite:** none.

**Files:** `crispr/GuideFinder.java`, `crispr/Guide.java`, `crispr/OffTargetScanner.java`, tests.

```java
public record Guide(String protospacer, String pam, int start, int end, boolean forward, double gc, double score, List<String> warnings) {}
public record PamSpec(String name, String pattern, int guideLength, boolean pamThreePrime) {
    public static PamSpec spCas9();   // "NGG", 20 nt, PAM at the 3' end
    public static PamSpec saCas9();   // "NNGRRT", 21 nt, PAM at the 3' end
    public static PamSpec cas12a();   // "TTTV", 23 nt, PAM at the 5' end
}
public final class GuideFinder {
    public GuideFinder(PamSpec spec);
    public List<Guide> find(String sequence, int from, int to, boolean circular);   // both strands in the range
}
public final class OffTargetScanner {
    public static int countSites(String reference, Guide guide, PamSpec spec, int maxMismatches);   // hits other than the guide itself
}
```

**Rules:** For every position where the pattern matches (IUPAC codes: `N` all, `R` A/G, `V` A/C/G), the adjacent guide sequence is read
(SpCas9: 20 bases directly 5' of the PAM). On the opposite strand the same with the reverse complement; `start/end` always refer to top coordinates,
`forward=false` marks the opposite strand. Circular sequences: search across the origin. Guides at the edge (too short) are dropped.
**Score** (0–100, deliberately simple, document in the Javadoc): start 100; −20 if GC content < 40 % or > 70 %; −30 if `TTTT` occurs in the
protospacer (termination signal of U6 polymerase); −10 if the protospacer contains a homopolymer run ≥ 5. For every deducted rule, a plain-text warning.
Do not invent any further scoring model.

**Tests:** `find("AAAAAAAAAAAAAAAAAAAATGGCCC…")`: a sequence that contains exactly one known NGG PAM (`TGG`) with the preceding 20 bases (defined in the test) →
guide with `protospacer` = these 20 bases, `pam="TGG"`, `forward=true`. The same sequence reverse-complemented → `forward=false`, `start/end` correspond to the top coordinates.
Guide with `TTTT` → score ≤ 70 and a warning. `countSites` in a reference that contains the guide twice (once exact, once with 2 mismatches) and `maxMismatches=3` →
returns 2 (both); with `maxMismatches=1` → 1.

---

## Task 2.13: Sequencing alignment ("Align to Reference")

**Prerequisite:** 2.0, Phase 1.10 (AB1), 2.1.

**Files:** `seqcheck/SequencingCheck.java`, `seqcheck/Difference.java`, tests.

```java
public enum DifferenceType { MISMATCH, INSERTION, DELETION, LOW_QUALITY }
public record Difference(DifferenceType type, int referencePosition, int readPosition, String expected, String observed, int quality) {}
public record CheckResult(String readName, boolean reversed, int referenceStart, int referenceEnd,
                          double identity, List<Difference> differences, String alignedRead, String alignedReference) {}

public final class SequencingCheck {
    public SequencingCheck(int minQuality);                                              // default 20
    public CheckResult check(SequenceDocument reference, String readName, String readBases, int[] quality);
    public CheckResult check(SequenceDocument reference, Chromatogram read);
}
```

**Procedure:**
1. Align the read forward and reverse-complemented against the reference with `Alignment.semiGlobal`; the variant with the higher score wins (`reversed`).
   For a circular reference: use the reference doubled with itself (`ref + ref`) and map positions back modulo length.
2. Determine the differences from the alignment; bases with `quality < minQuality` are additionally reported as `LOW_QUALITY` (even if they match); in the reverse direction the quality is reversed along with the read.
3. `identity` = matches / aligned columns (excluding edge gaps) in percent.

**Tests:** reference `ACGTACGTACGTACGTACGTACGTACGT` (28), read = reference segment from 4 with length 16 with one substitution at read position 8 (`C`→`T`): `MISMATCH` at
`referencePosition 12`, `identity` = 15/16 · 100 = 93.75. Read reverse-complemented → `reversed=true`, same difference. Insertion of one base → `INSERTION`. Deletion of one base → `DELETION`.
Quality 10 at position 3 → `LOW_QUALITY` there. Circular reference, read across the origin → aligned correctly.

---

## Task 2.14: Operations and user interface (cloning)

**Prerequisite:** 2.1–2.13, 1.11.

**Operations** (`Operation` classes with IDs, registered in an `OperationRegistry`): `clone.restriction`, `clone.overlap`, `clone.goldengate`,
`clone.gateway.bp`, `clone.gateway.lr`, `pcr.simulate`, `primer.pick`, `crispr.find`, `codon.optimize`, `seq.check`, `digest.virtual`. Each operation
parses its `Map<String,String>` parameters robustly (default values, errors → `OperationException` with a German message) — with one test each for the default case and an
invalid parameter.

**User interface (`fx/`, with presenter classes and tests for the presenters):**
- **Digest view:** enzyme selection (search), fragment list (length, ends), gel preview (`GelSimulator`) next to it on the right, ladder choice.
- **Cloning wizard:** steps choose vector → choose insert → choose method → parameters → candidate list with warnings → save construct (as a new document in the project, history entry).
- **PCR/primer dialog:** template + target range → `PrimerPicker` suggestions; draw primer pairs on the map as arrows (`primer_bind` features); "Simulate PCR" shows product and gel.
- **Sequencing:** choose read files (AB1/FASTA) → `SequencingCheck`; differences as a list and highlighted in the sequence view.
- Every generated construct is a `SequenceDocument` with history (`HistoryEntry`).

**Presenter tests:** For each presenter, test the state transitions without a window (e.g. cloning wizard: after selecting vector, insert and enzymes, the candidate list contains the
expected number).

---

## Overall acceptance Phase 2

Scenario (automate as the integration test `ClonePipelineIT`, with `puc19.gb` from the test resources):
1. Load pUC19 (circular, 2686 bp), digest with EcoRI + HindIII: two fragments (small MCS fragment, large backbone); sum of lengths = 2686.
2. Create a synthetic 300 bp insert with EcoRI/HindIII ends (in the test), clone it into the backbone via `RestrictionCloning`: exactly one directional candidate,
   construct length = backbone + insert inner fragment.
3. Design primers for a control PCR across the insert with `PrimerPicker`; `PcrSimulator` yields exactly one product whose size is ≥ the insert length.
4. Simulate a gel for the digest and the PCR product: band positions strictly monotonic in size.
5. Save the construct as GenBank, reload, identical.
