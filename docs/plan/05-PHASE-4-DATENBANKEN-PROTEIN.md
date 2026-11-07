# Phase 4: Databases and Protein

> **Implementation note:** The signatures in this document are written in Java notation. The implementation is in **Rust** (mapping: `08-RUST-KERN-SWIFT-APP.md`, table "Mapping Java instructions → Rust").
> Tasks that mention JavaFX/Maven/JDBC/`jpackage` are **replaced** (Swift tasks S.x or `rusqlite`, `cargo`); they are marked "→ replaced" below. Read `00-ARBEITSANWEISUNG.md` first.


Goal: NCBI search and download, BLAST (remote and local), UniProt, protein properties, RNA folding, 3D view.

New packages: `net/` (HTTP abstraction, cache), `db/ncbi/`, `db/uniprot/`, `blast/`, `protein/`, `rna/`.

**Rules for everything involving the network (binding):**
- All network access goes through the interface `HttpFetcher` (4.1). Tests use a **fake implementation** with fixed canned responses.
  Real calls appear in tests tagged `@Tag("network")` (not part of `mvn test`).
- Observe the services' rate limits (see the respective task), `User-Agent: Snapious/1.0 (privates Lernprojekt)`.
- Responses are stored in the cache directory (`~/.snapious/cache`), key = SHA-256 of the URL; default validity 24 h.
- **No** credentials are stored; optional NCBI API keys only in the settings (Phase 6), never in code.

---

## Task 4.1: HTTP abstraction and cache

**Prerequisite:** none.

**Files:** `net/HttpFetcher.java`, `net/JdkHttpFetcher.java`, `net/CachingFetcher.java`, `net/RateLimiter.java`, tests.

```java
public interface HttpFetcher {
    HttpResult get(String url, Map<String,String> headers) throws IOException;
    HttpResult post(String url, Map<String,String> headers, String body) throws IOException;     // application/x-www-form-urlencoded
}
public record HttpResult(int status, String body, Map<String,List<String>> headers) {
    public boolean ok() { return status >= 200 && status < 300; }
}
public final class JdkHttpFetcher implements HttpFetcher { ... }        // java.net.http.HttpClient, timeout 30 s, follow redirects
public final class CachingFetcher implements HttpFetcher {
    public CachingFetcher(HttpFetcher delegate, Path cacheDir, Duration ttl, Clock clock);   // only GETs are cached, only status 200
}
public final class RateLimiter {
    public RateLimiter(int maxRequests, Duration perPeriod, Clock clock);
    public void acquire() throws InterruptedException;      // blocks until a slot is free
    public Duration nextWait();                              // wait time without blocking (for tests)
}
```

**Tests:** `CachingFetcher` with a fake: a second GET on the same URL does not call the delegate again; after the TTL expires (fast-forward the fake `Clock`) it calls again; error status 500 is not
cached. `RateLimiter(3, 1 s)`: four calls in a row → `nextWait()` after the third is > 0 and ≤ 1 s (with fake clock). `JdkHttpFetcher` against a local `com.sun.net.httpserver.HttpServer` on a
free port (200 response and 404): status and body correct.

---

## Task 4.2: NCBI Entrez (search and download)

**Prerequisite:** 4.1, Phase 0.3 (GenBank parser).

**Files:** `db/ncbi/EntrezClient.java`, `db/ncbi/SearchResult.java`, tests with sample responses in `src/test/resources/ncbi/`.

```java
public record SearchResult(int count, List<String> ids) {}
public record SummaryEntry(String id, String accession, String title, String organism, int length, String moleculeType) {}

public final class EntrezClient {
    public EntrezClient(HttpFetcher fetcher, String toolName, String email, String apiKey /* may be null */);
    public SearchResult search(String database, String term, int retmax) throws IOException;              // db: "nuccore" | "protein" | "gene"
    public List<SummaryEntry> summaries(String database, List<String> ids) throws IOException;
    public List<SequenceDocument> fetchGenBank(String database, List<String> ids) throws IOException;     // rettype=gb
    public String fetchFasta(String database, String id) throws IOException;
}
```

URLs (base `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/`):
- Search: `esearch.fcgi?db=<db>&term=<urlencoded term>&retmax=<n>&retmode=json&tool=<tool>&email=<email>[&api_key=<key>]`. Response: JSON `{"esearchresult":{"count":"…","idlist":["…"]}}`.
- Summary: `esummary.fcgi?db=<db>&id=<id1,id2>&retmode=json` → JSON `result.<id>` with fields `accessionversion`, `title`, `organism`, `slen`, `moltype`.
- Download: `efetch.fcgi?db=<db>&id=<ids>&rettype=gb&retmode=text` → GenBank text, parse with `GenBankFormat.parse`. FASTA: `rettype=fasta`.
- Rate limit: without an API key 3 requests/s, with a key 10/s (`RateLimiter`). Error status 429 → `IOException("NCBI: zu viele Anfragen")`.
- Parse JSON with the small `Json` helper class from Phase 0.5 (move the class into a shared package `util/` if needed, keep the tests).

**Tests (fake fetcher with files from `src/test/resources/ncbi/`; write sample responses yourself in the real format):**
- `search("nuccore", "pUC19[All Fields]", 5)` → `count`, `ids` correct from the sample response; the call's URL contains `db=nuccore`, `retmax=5`, `tool=`, encoded search term (`%5B`).
- `summaries(...)` → `accession`, `title`, `length` from sample JSON.
- `fetchGenBank` with a small GenBank file (the `mini.gb` from Phase 0) → 1 document; error 404 → `IOException`.
- With an API key the URL contains `api_key=`; without it, not.
- Network test (`@Tag("network")`): `search("nuccore","L09137",1)` returns at least one ID.

---

## Task 4.3: BLAST (remote via NCBI, local with BLAST+)

**Prerequisite:** 4.1, 0.6.

**Files:** `blast/BlastHit.java`, `blast/BlastParser.java`, `blast/LocalBlast.java`, `blast/RemoteBlast.java`, tests.

```java
public record BlastHit(String queryId, String subjectId, double percentIdentity, int alignmentLength, int mismatches, int gapOpens,
                       int queryStart, int queryEnd, int subjectStart, int subjectEnd, double evalue, double bitScore) {}

public final class BlastParser {
    /** Tabular format -outfmt 6 with the default columns (12 columns, tab-separated). Ignore lines with '#'. */
    public static List<BlastHit> parseTabular(String text);
}
public final class LocalBlast {
    public LocalBlast(ToolRegistry tools);
    public Path makeDatabase(List<Sequence> subjects, boolean nucleotide, Path directory) throws IOException;      // writes FASTA and calls makeblastdb
    public List<BlastHit> search(String program, Sequence query, Path database, double evalue, int maxHits) throws IOException;
}
public final class RemoteBlast {
    public RemoteBlast(HttpFetcher fetcher, Sleeper sleeper);
    public String submit(String program, String database, String query) throws IOException;          // returns RID
    public BlastStatus status(String rid) throws IOException;                                        // WAITING | READY | FAILED | UNKNOWN
    public List<BlastHit> results(String rid) throws IOException;
    public List<BlastHit> run(String program, String database, String query, OperationContext ctx) throws IOException, InterruptedException;
}
```

**Local:** `makeblastdb -in subjects.fa -dbtype nucl|prot -out <dir>/db`; search: `blastn|blastp|blastx|tblastn -query q.fa -db <dir>/db -evalue <e> -max_target_seqs <n> -outfmt "6 qseqid sseqid pident length mismatch gapopen qstart qend sstart send evalue bitscore"`.

**Remote (NCBI URL API, `https://blast.ncbi.nlm.nih.gov/Blast.cgi`):**
- Submit (POST, form): `CMD=Put&PROGRAM=<blastn|blastp|…>&DATABASE=<nt|nr|…>&QUERY=<sequence>`; the response contains lines `RID = <value>` and `RTOE = <seconds>` (in the HTML/text via regex `RID\s*=\s*(\S+)`).
- Status (GET): `CMD=Get&FORMAT_OBJECT=SearchInfo&RID=<rid>` → text contains `Status=WAITING`, `Status=READY`, `Status=FAILED` or `Status=UNKNOWN`; `ThereAreHits=yes` when there are hits.
- Result (GET): `CMD=Get&FORMAT_TYPE=Tabular&RID=<rid>` — tabular format (may contain additional comment lines).
- Waiting times according to NCBI rules: after submitting, wait at least `RTOE` seconds (at least 10), then query the status **at most once per minute** (`Sleeper` interface for testable waiting); no parallel bulk queries. Timeout after 15 minutes → `IOException`.

**Tests:**
- `parseTabular` with 2 lines: `q1\ts1\t98.5\t100\t1\t0\t1\t100\t11\t110\t1e-45\t180`  → `percentIdentity 98.5`, `alignmentLength 100`, `evalue 1e-45`, `bitScore 180`; comment lines ignored; line with the wrong number of columns → `IllegalArgumentException`.
- `RemoteBlast.run` with a fake fetcher: first status response `WAITING`, second `READY`, then the result; the fake `Sleeper` records the wait times: first wait ≥ RTOE, then 60 s.
- `LocalBlast` with fake tools (shell scripts that imitate `makeblastdb` and output a fixed table) checks the command line and parsing; with real BLAST+ only under `Assumptions`: searching for a subsequence of a database sequence finds 100 % identity.

---

## Task 4.4: UniProt

**Prerequisite:** 4.1.

**Files:** `db/uniprot/UniProtClient.java`, `db/uniprot/UniProtEntry.java`, tests with sample JSON.

```java
public record ProteinFeature(String type, String description, int start, int end) {}     // 1-based, inclusive (UniProt) → convert internally to 0-based/exclusive for Feature
public record UniProtEntry(String accession, String name, String organism, String sequence, List<ProteinFeature> features) {
    public SequenceDocument toDocument();       // protein SequenceDocument; feature types "Domain", "Region", "Active site", … mapped to the feature model
}
public final class UniProtClient {
    public UniProtClient(HttpFetcher fetcher);
    public List<String> search(String query, int size) throws IOException;           // accessions
    public UniProtEntry fetch(String accession) throws IOException;
}
```

URLs: search `https://rest.uniprot.org/uniprotkb/search?query=<q>&size=<n>&format=json&fields=accession`; entry `https://rest.uniprot.org/uniprotkb/<acc>.json`.
JSON fields: `primaryAccession`, `uniProtkbId`, `organism.scientificName`, `sequence.value`, `features[]` with `type`, `description`, `location.start.value`, `location.end.value`.

**Tests:** Sample JSON (self-written, small entry with 2 features): `toDocument()` has type PROTEIN; a `Domain` feature 10–20 (UniProt, 1-based inclusive) becomes the interval `[9,20)`;
entry without features → empty list; 404 → `IOException`.

---

## Task 4.5: Protein properties

**Prerequisite:** none.

**Files:** `protein/ProteinProperties.java`, `protein/Hydropathy.java`, tests.

```java
public final class ProteinProperties {
    public static double molecularWeight(String protein);                     // average mass in Da, + water
    public static double isoelectricPoint(String protein);                    // bisection pH 0..14
    public static double extinctionCoefficient280(String protein, boolean reducedCysteines);   // M^-1 cm^-1
    public static Map<Character,Integer> composition(String protein);
    public static double chargeAtPh(String protein, double ph);
    public static double aromaticity(String protein);
    public static double instabilityIndex(String protein);                    // optional: only if the dipeptide table (Guruprasad 1990) is entered yourself, otherwise omit
}
public final class Hydropathy {
    public static double[] kyteDoolittle(String protein, int window);         // mean per position over a centered window (odd), edges: shorten the window
}
```

**Constants:**
- Average residue masses (Da): G 57.0519, A 71.0788, S 87.0782, P 97.1167, V 99.1326, T 101.1051, C 103.1388, L 113.1594, I 113.1594, N 114.1038,
  D 115.0886, Q 128.1307, K 128.1741, E 129.1155, M 131.1926, H 137.1411, F 147.1766, R 156.1875, Y 163.1760, W 186.2132; water 18.0153.
- Kyte-Doolittle: A 1.8, R −4.5, N −3.5, D −3.5, C 2.5, Q −3.5, E −3.5, G −0.4, H −3.2, I 4.5, L 3.8, K −3.9, M 1.9, F 2.8, P −1.6, S −0.8, T −0.7, W −0.9, Y −1.3, V 4.2.
- pKa values (EMBOSS set): N-terminus 8.6, C-terminus 3.6, D 3.9, E 4.1, C 8.5, Y 10.1, H 6.5, K 10.8, R 12.5.
  Charge = Σ positive `1/(1+10^(pH−pKa))` − Σ negative `1/(1+10^(pKa−pH))`. pI = pH at which the charge is 0 (bisection to 0.001 accuracy).
- Extinction (Pace et al. 1995): `ε280 = nW · 5500 + nY · 1490 + nC/2 · 125` (cystine term only when `reducedCysteines=false`; otherwise the cysteine term is 0).
- Unknown characters (`X`, `*`) are ignored for mass (no warning needed), a stop `*` at the end is cut off.

**Tests:**
- `molecularWeight("GG")` = 2·57.0519 + 18.0153 = 132.1191 (±0.001). `molecularWeight("")` = 0.
- `composition("AAB")` does not ignore unknown characters but counts only A; test for `A=2`.
- `isoelectricPoint("GGGG")` = (8.6 + 3.6) / 2 = 6.1 (±0.05; only the two termini carry charge). A peptide of 10×K has pI > 10, a peptide of 10×D has pI < 4.
- `extinctionCoefficient280("W")` = 5500; `("Y")` = 1490; `("CC", false)` = 125.
- `kyteDoolittle("AAAA", 3)`: 1.8 everywhere (edges with shortened window equal); `kyteDoolittle("AR", 1)` = [1.8, −4.5].
- `chargeAtPh("K", 7.0)` = 0.9755 (N-terminus) + 0.9998 (Lys) − 0.9996 (C-terminus) = 0.9757 (±0.005).

---

## Task 4.6: RNA folding (ViennaRNA)

**Prerequisite:** 0.6.

**Files:** `rna/RnaFold.java`, `rna/RnaStructure.java`, tests.

```java
public record RnaStructure(String sequence, String dotBracket, double energy) {
    public List<int[]> basePairs();            // from dot-bracket notation, 0-based pairs (i,j), i<j
}
public final class RnaFold {
    public RnaFold(ToolRegistry tools);
    public RnaStructure fold(String rna) throws IOException;     // RNAfold --noPS (stdin), output: line 1 sequence, line 2 "((...)) (-1.20)"
    static RnaStructure parseOutput(String output);               // package-private, testable
}
```

`basePairs()`: stack for `(`/`)`; unbalanced brackets → `IllegalArgumentException`. Energy from the parentheses at the end of the line (`( -1.20)` or `(-1.20)`).

**Tests:** `parseOutput("GGGAAACCC\n(((...))) ( -3.20)\n")` → `dotBracket "(((...)))"`, energy −3.2, pairs `(0,8),(1,7),(2,6)`. Wrong brackets → exception.
With `RNAfold` installed: `GGGGAAAACCCC` folds with at least 3 pairs (`Assumptions`).
Display in the UI: dot-bracket notation as text below the sequence plus a simple arc diagram (arcs above the sequence); drawing via the layout class `ArcDiagramLayout` (arc center = (i+j)/2, radius = (j−i)/2), with a numeric test.

---

## Task 4.7: Display 3D structure

**Prerequisite:** Phase 0.7.

**Files:** `structure/PdbFile.java`, `fx/StructureView.java`, resource `src/main/resources/structure/viewer.html`, test `structure/PdbFileTest.java`.

- `PdbFile.parse(String)` reads `ATOM`/`HETATM` lines: atom name (columns 13–16), residue name (18–20), chain (22), residue number (23–26), x/y/z (31–38, 39–46, 47–54), element (77–78). Returns `List<Atom>`;
  `chains()`, `sequence(char chain)` (three-letter code → one-letter code, standard amino acids; unknown `X`).
- `StructureView` embeds a `WebView` that loads `viewer.html`; the PDB file is passed as text via `executeScript`. The HTML page uses the freely available
  library **3Dmol.js**; the file is **not** downloaded automatically at build time — please note in `OFFENE_FRAGEN.md` that `3Dmol-min.js` has to be placed manually in `src/main/resources/structure/`
  if the network is interrupted.
- Tests only for `PdbFile` (no window).

**Tests:** Small PDB file (3 amino acids G-A-S, self-written, correctly formatted columns) → `sequence('A')="GAS"`, 3 residues, coordinates match.
Line that is too short → skipped, no crash.

---

## Task 4.8: User interface and operations

**Prerequisite:** 4.1–4.7, 1.11, 3.9.

- **NCBI search window:** input field, database selection, hit list (summaries), multi-selection, "Download" saves as documents in the project (with history entry `db.ncbi.fetch`).
- **BLAST dialog:** query from selection, program (blastn/blastp/blastx), database (remote: `nt`, `nr`, `refseq_rna`; local: path), E-value, sortable hit table, double-click loads the subject from NCBI. Progress and cancel via `OperationContext`.
- **UniProt dialog:** search, load entry (document with domains as features).
- **Protein view:** property list, hydropathy plot (line, window 9), composition as bars.
- **RNA view:** folding, structure as text and arc diagram.
- Operations: `db.ncbi.search`, `db.ncbi.fetch`, `blast.local`, `blast.remote`, `db.uniprot.fetch`, `protein.properties`, `rna.fold`.
- Presenter tests: NCBI dialog with fake fetcher (search → list with expected count); BLAST presenter sorts by E-value/bit score.

---

## Overall acceptance of Phase 4

Scenario as a test with fake fetchers: NCBI search → load GenBank document → find ORF → translation → properties (MW, pI) → hydropathy array has length = protein length.
In addition a manual network test (not automated): real search for `L09137`, download, display of the map.
