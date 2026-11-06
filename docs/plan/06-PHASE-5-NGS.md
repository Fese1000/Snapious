# Phase 5: NGS-Grundlagen (optional)

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: Qualitätsbericht, Trimming, Mapping über externe Programme, BAM/SAM lesen, Coverage-Ansicht, einfache Varianten, Assembly-Aufruf, große Dateien streamen.
Diese Phase ist unabhängig von Phase 2 und kann übersprungen werden, ohne dass andere Phasen leiden.

Neue Pakete: `ngs/fastq/`, `ngs/qc/`, `ngs/trim/`, `ngs/map/`, `ngs/bam/`, `ngs/variant/`, `ngs/assembly/`, `ngs/index/`.

**Grundregeln:** Dateien werden **streamend** verarbeitet (nie ganze FASTQ/BAM in den Speicher laden); alle Iteratoren sind `AutoCloseable`. Testdaten sind klein und im Test erzeugt
(`new Random(seed)`); keine echten Sequenzierdaten ins Repository.

---

## Aufgabe 5.1: FASTQ-Streaming und Index

**Voraussetzung:** keine.

**Dateien:** `ngs/fastq/FastqReader.java`, `ngs/fastq/FastqWriter.java`, `ngs/fastq/FastqRecord.java`, `ngs/index/FaiIndex.java`, Tests.

```java
public record FastqRecord(String name, String comment, String bases, String quality) {
    public int length(); public int[] phred(QualityScoreConverter.EncodingType enc);
}
public final class FastqReader implements Iterator<FastqRecord>, AutoCloseable {
    public static FastqReader open(Path file) throws IOException;      // .gz automatisch über GZIPInputStream
    public FastqReader(Reader reader);
}
public final class FastqWriter implements AutoCloseable { public void write(FastqRecord r) throws IOException; ... }
public final class FaiIndex {
    public record Entry(String name, long length, long offset, int basesPerLine, int bytesPerLine) {}
    public static FaiIndex build(Path fasta) throws IOException;
    public static FaiIndex read(Path fai) throws IOException;
    public void write(Path fai) throws IOException;
    public String fetch(Path fasta, String name, long start, long end) throws IOException;       // 0-basiert, Ende exklusiv; RandomAccessFile
}
```

FASTQ: 4 Zeilen je Eintrag (`@name comment`, Bases, `+`, Qualität). Prüfen: Länge Bases = Länge Qualität, sonst `IOException("Zeile n: …")`.
FAI (samtools-Format, tabgetrennt): `name \t length \t offset \t basesPerLine \t bytesPerLine`; `offset` = Byteposition der ersten Base; `bytesPerLine = basesPerLine + 1` (LF) bzw. `+2` (CRLF).

**Tests:** 3 Beispiel-Einträge → richtige Namen/Kommentare; Länge ungleich → `IOException` mit Zeilennummer; gzip-Datei im Test erzeugen und lesen; 100.000 Einträge streamen (im Test erzeugt) mit geringem Speicher
(kein `List` aller Einträge; Zähler genügt). FAI: FASTA mit zwei Sequenzen (60 pro Zeile) → Einträge mit `basesPerLine=60`, `bytesPerLine=61`; `fetch(fasta,"seq2",58,64)` liefert die 6 Basen über den Zeilenumbruch hinweg.

---

## Aufgabe 5.2: Qualitätsbericht (FastQC-artig)

**Voraussetzung:** 5.1.

**Dateien:** `ngs/qc/QcAnalyzer.java`, `ngs/qc/QcReport.java`, Tests.

```java
public record QcReport(long reads, int minLength, int maxLength, double meanLength, double gcPercent,
                       double[] meanQualityPerPosition, double[] q25PerPosition, double[] medianPerPosition, double[] q75PerPosition,
                       Map<Integer,Long> meanQualityPerReadHistogram, int[] gcHistogram100, Map<Integer,Long> lengthDistribution,
                       double[][] baseContentPerPosition /* [pos][A,C,G,T,N] in % */,
                       double duplicationPercent, Map<String,Long> overrepresented, double adapterPercent) {}

public final class QcAnalyzer {
    public QcAnalyzer(QualityScoreConverter.EncodingType encoding, String adapter);   // Standard SANGER, "AGATCGGAAGAGC"
    public QcReport analyze(FastqReader reader, int maxReadsForDuplication);          // Standard 100000
}
```

**Berechnung:** Qualitäten pro Position als Histogramm (0–93) → Mittelwert/Median/Quartile ohne Reads zu speichern. Duplikation: Hash der **ersten 50 Basen** der ersten
`maxReadsForDuplication` Reads; `duplicationPercent = 100 · (1 − eindeutig/gesamt)`. Überrepräsentiert: Sequenzen (erste 50 Basen), die > 0,1 % aller Reads ausmachen, absteigend (max. 10).
Adapter: Anteil der Reads, die die ersten 12 Basen des Adapters (`AGATCGGAAGAG`) irgendwo enthalten (exakt).

**Tests (Reads im Test erzeugt):**
- 4 Reads `ACGT` mit Qualität `IIII` (Phred 40) → `meanQualityPerPosition = [40,40,40,40]`, `gcPercent = 50`, `reads = 4`, `minLength = maxLength = 4`.
- Vier Reads, davon zwei mit Qualität `!` (0) und zwei mit `I` (40) an derselben Position → Mittelwert 20; Konvention **unterer Median** (Element mit Index `(n−1)/2` der sortierten Werte `[0,0,40,40]`) = 0.
- Duplikation: 10 identische Reads → `duplicationPercent = 90`.
- Adapter: 2 von 10 Reads enthalten `AGATCGGAAGAG` → `adapterPercent = 20`.
- Längenverteilung `{4:3, 5:1}` bei 3 Reads Länge 4 und 1 Read Länge 5.
- Leere Eingabe → `reads = 0`, keine Division durch 0.

---

## Aufgabe 5.3: Trimming

**Voraussetzung:** 5.1.

**Dateien:** `ngs/trim/Trimmer.java`, `ngs/trim/TrimOptions.java`, Tests.

```java
public record TrimOptions(int windowSize, double windowMeanQuality, int minLength, String adapter, int adapterMinOverlap, int maxAdapterMismatches,
                          QualityScoreConverter.EncodingType encoding) {
    public static TrimOptions standard();    // 4, 15, 36, "AGATCGGAAGAGC", 8, 1, SANGER
}
public final class Trimmer {
    public Trimmer(TrimOptions options);
    public Optional<FastqRecord> trim(FastqRecord r);        // leer, wenn kürzer als minLength
    public TrimStats run(FastqReader in, FastqWriter out) throws IOException;
}
public record TrimStats(long inputReads, long outputReads, long basesRemoved, long adaptersFound) {}
```

**Algorithmus:**
1. **Adapter (3'-Ende):** Suche die früheste Position `p`, ab der `bases[p..]` zum Präfix des Adapters passt (Überlappung ≥ `adapterMinOverlap` bzw. bis Read-Ende, höchstens `maxAdapterMismatches` Abweichungen); schneide ab `p`.
2. **Qualität (Sliding Window, wie Trimmomatic-Prinzip):** Von 5' nach 3' ein Fenster von `windowSize`; sobald der Mittelwert der Phred-Werte `< windowMeanQuality`, schneide ab Fensterbeginn.
3. `minLength` prüfen.

**Tests:** Read `ACGTACGTACGT` + Adapter-Anfang `AGATCGGAAGAG` mit voller Qualität → Ergebnis `ACGTACGTACGT`, `adaptersFound=1`.
Read `ACGTACGTACGT` mit Qualitäten `IIIIIIII!!!!` (8 × Phred 40, danach 4 × Phred 0), Fenster 4, Schwelle 15: Fenstermittel ab Index 0–4 = 40, ab Index 5 = 30, ab Index 6 = 20, ab Index 7 = 10 (< 15) → Schnitt am Fensterbeginn Index 7 → Ergebnis `ACGTACG` (7 Basen).
Read kürzer als `minLength` nach dem Trimmen → leer. Adapter mit 1 Mismatch wird erkannt, mit 2 nicht (bei `maxAdapterMismatches=1`). `run` zählt korrekt.

---

## Aufgabe 5.4: SAM/BAM lesen

**Voraussetzung:** 5.1.

**Dateien:** `ngs/bam/AlignmentRecord.java`, `ngs/bam/Cigar.java`, `ngs/bam/SamReader.java`, `ngs/bam/BamReader.java`, Tests.

```java
public record CigarOp(char op, int length) {}      // M I D N S H P = X
public record AlignmentRecord(String readName, int flag, String referenceName, int position /*0-basiert*/, int mapq,
                              List<CigarOp> cigar, String bases, String quality /*Phred+33*/, int mateRef, int matePos, int templateLength) {
    public boolean isUnmapped(); public boolean isReverse(); public boolean isSecondary(); public boolean isSupplementary(); public boolean isDuplicate();
    public int referenceEnd();               // position + Summe der Längen von M,D,N,=,X
}
public interface AlignmentReader extends Iterator<AlignmentRecord>, AutoCloseable { List<String> referenceNames(); Map<String,Integer> referenceLengths(); }
public final class SamReader implements AlignmentReader { public static SamReader open(Path sam) throws IOException; }
public final class BamReader implements AlignmentReader { public static BamReader open(Path bam) throws IOException; }
```

**SAM:** Kopfzeilen `@SQ SN:<name> LN:<len>` → Referenzen; Datenzeilen mit 11 Pflichtspalten (`QNAME FLAG RNAME POS MAPQ CIGAR RNEXT PNEXT TLEN SEQ QUAL`), `POS` 1-basiert → intern 0-basiert, `*` → leer.
**BAM (binär, little-endian, BGZF = aneinandergehängte gzip-Blöcke → `GZIPInputStream` liest sie hintereinander):**
- Kopf: Magic `BAM\1`, `l_text` (int32) + Text, `n_ref` (int32), je Referenz: `l_name` (int32), Name (inkl. NUL), `l_ref` (int32).
- Datensatz: `block_size` (int32), `refID` (int32), `pos` (int32, 0-basiert), `l_read_name` (uint8), `mapq` (uint8), `bin` (uint16), `n_cigar_op` (uint16), `flag` (uint16), `l_seq` (int32),
  `next_refID` (int32), `next_pos` (int32), `tlen` (int32), `read_name` (NUL-terminiert), `cigar` (`n_cigar_op` × uint32: Länge = `x >> 4`, Operation = `"MIDNSHP=X".charAt(x & 0xf)`),
  `seq` (`(l_seq+1)/2` Bytes, 4 Bit pro Base, Tabelle `"=ACMGRSVTWYHKDBN"`), `qual` (`l_seq` Bytes, Phred ohne Offset; `0xFF` = fehlend), danach optionale Felder (überspringen).
- `refID = -1` → unmapped / `"*"`.

**Tests:** SAM-Zeile `r1 0 chr1 5 60 4M 0 0 ACGT IIII` → `position 4`, Cigar `[4M]`, `referenceEnd 8`. `3S4M2D5M`: `referenceEnd = pos + 4 + 2 + 5`.
Flag 16 → `isReverse`; Flag 4 → `isUnmapped`. BAM: **im Test selbst eine Mini-BAM-Datei erzeugen** (Kopf + 2 Datensätze, mit `GZIPOutputStream` in einem Block gepackt), lesen und mit den
Werten der Eingabe vergleichen. Beschädigte Datei (falsches Magic) → `IOException`.

---

## Aufgabe 5.5: Mapping-Backends

**Voraussetzung:** 0.6, 5.4.

**Dateien:** `ngs/map/Mapper.java`, `ngs/map/Minimap2Mapper.java`, `ngs/map/BwaMapper.java`, Tests.

```java
public interface Mapper {
    String name(); boolean isAvailable();
    Path map(Path reference, List<Path> reads, MapOptions options, Path outputSam, OperationContext ctx) throws OperationException;
}
public record MapOptions(String preset /* "sr" (short reads), "map-ont", "map-pb" */, int threads) {}
```

Aufrufe: minimap2 `minimap2 -ax <preset> -t <threads> <ref.fa> <reads1> [reads2] > out.sam`; BWA: erst `bwa index ref.fa` (falls Indexdateien fehlen), dann `bwa mem -t <threads> ref.fa reads1 [reads2] > out.sam`.
Ausgabe immer in die angegebene SAM-Datei (stdout umleiten über `ToolRegistry` mit Datei-Weiterleitung; erweitere `ToolRegistry.run` um eine Variante `runToFile(...)`).

**Tests:** Fake-Programme prüfen Kommandozeile (Preset, Threads, Pfade); mit echtem minimap2 unter `Assumptions`: Referenz 5 kb (zufällig, Seed), 200 Reads à 100 bp (aus der Referenz geschnitten, 1 % Fehler) → mindestens 95 % mit `!isUnmapped` und
Position innerhalb ±5 Basen der wahren Herkunft.

---

## Aufgabe 5.6: Coverage aus Alignments

**Voraussetzung:** 5.4, vorhandener `CoverageAnalyzer`.

**Dateien:** `ngs/map/CoverageFromAlignments.java`, Test.

```java
public final class CoverageFromAlignments {
    public static int[] depth(AlignmentReader reader, String referenceName, int referenceLength, boolean skipDuplicates, int minMapq);
    public static CoverageAnalyzer.CoverageResult analyze(AlignmentReader reader, String referenceName, int referenceLength, int threshold);
}
```

Berechnung CIGAR-bewusst: nur `M`, `=`, `X` erhöhen die Tiefe; `D` und `N` nicht (optional Parameter `countDeletions`); `I`, `S`, `H`, `P` verbrauchen keine Referenz. Unmapped, sekundäre (Flag 256) und – je nach Parameter – Duplikate
(Flag 1024) werden übersprungen. `analyze` baut daraus die bestehende `CoverageResult`-Struktur.

**Tests:** Referenzlänge 20; Read A `pos 0, 10M` und Read B `pos 5, 5M2D5M` → Tiefe (vollständig): `[1,1,1,1,1,2,2,2,2,2,0,0,1,1,1,1,1,0,0,0]`.
Duplikat-Flag mit `skipDuplicates=true` zählt nicht; `minMapq=20` ignoriert Reads mit MAPQ 10; Soft-Clip (`3S5M`) verschiebt nicht.

---

## Aufgabe 5.7: Einfache Variantenerkennung (Pileup)

**Voraussetzung:** 5.4, 5.6.

**Dateien:** `ngs/variant/Pileup.java`, `ngs/variant/VariantCaller.java`, `ngs/variant/Variant.java`, `io/VcfFormat.java` (Schreiben), Tests.

```java
public record Variant(String reference, int position /*0-basiert*/, char refBase, char altBase, int depth, int altCount, double altFraction, double quality) {}
public final class VariantCaller {
    public VariantCaller(int minDepth, double minAltFraction, int minBaseQuality, int minMapq);   // Standard 10, 0.2, 20, 20
    public List<Variant> call(AlignmentReader reader, SequenceDocument reference);
}
```

**Algorithmus:** Pro Referenzposition Basenzählung aus allen Reads, **CIGAR-bewusst** (Base `i` des Reads gehört zur Referenzposition gemäß Cigar-Walk), Basen mit Qualität < `minBaseQuality` ignorieren.
Alt-Base = häufigste von der Referenz abweichende Base; Variante, wenn `depth ≥ minDepth` und `altCount/depth ≥ minAltFraction`. `quality` = `−10·log10(P)` mit `P` als einseitiger Binomial-Wahrscheinlichkeit
`P(X ≥ altCount | n=depth, p=0.01)` (Fehlerrate 1 %); Binomial über logarithmierte Summe exakt für `n ≤ 1000`, darüber Normalapproximation. Keine Indels (nur SNVs); Indels werden im Pileup ignoriert.
`VcfFormat.write(List<Variant>, reference)`: Kopf `##fileformat=VCFv4.2`, `#CHROM POS ID REF ALT QUAL FILTER INFO`; `POS` 1-basiert; INFO `DP=<depth>;AF=<fraction>`.

**Tests:** Referenz `AAAAAAAAAA`; 20 Reads über alle Positionen, davon 10 mit `G` an Position 4 (Qualität hoch) → genau eine Variante: `position 4`, `refBase A`, `altBase G`, `altFraction 0.5`, `depth 20`.
Nur 2 von 20 Reads mit `G` (0,1) → keine Variante bei `minAltFraction 0.2`. Basen mit Qualität 10 werden nicht gezählt. Tiefe 5 < `minDepth` → keine Variante.
VCF-Ausgabe: Zeile `ref\t5\t.\tA\tG\t…` (Position 1-basiert = 5).

---

## Aufgabe 5.8: De-novo-Assembly (nur Aufruf)

**Voraussetzung:** 0.6.

**Dateien:** `ngs/assembly/SpadesAssembler.java`, Test.

```java
public final class SpadesAssembler {
    public SpadesAssembler(ToolRegistry tools);
    public List<SequenceDocument> assemble(Path reads1, Path reads2 /*darf null*/, Path workDir, int threads, OperationContext ctx) throws OperationException;
}
```

Aufruf: `spades.py -1 reads1 [-2 reads2 | -s reads1] -o <workDir> -t <threads>`. Ergebnis: `contigs.fasta` → Dokumente (Name = FASTA-Name, Metadaten `length`, `coverage` aus dem SPAdes-Namen `NODE_1_length_1234_cov_56.7`).
Fehler: Exitcode ≠ 0 → `OperationException` mit letzten 20 Zeilen der Ausgabe.

**Tests:** Fake `spades.py` (Skript schreibt `contigs.fasta` mit zwei Contigs `NODE_1_length_100_cov_5.5`, `NODE_2_length_50_cov_2.0`) → 2 Dokumente, Metadaten `coverage` = `5.5`. Fehlerfall: Skript mit Exitcode 1.

---

## Aufgabe 5.9: Oberfläche (NGS)

**Voraussetzung:** 5.1–5.8, 1.11.

- **QC-Ansicht:** Diagramme (Linien/Balken) aus `QcReport`: Qualität pro Position (Boxplot-artig mit Quartilen), Qualitätsverteilung, GC-Verteilung, Längenverteilung, Basenzusammensetzung; Zusammenfassung mit Ampel (grün/gelb/rot nach einfachen Schwellen, selbst festlegen und im Code kommentieren).
- **Trimming-Dialog:** Eingabedateien, Optionen, Fortschritt (`OperationContext`), Statistik.
- **Mapping-Dialog:** Referenz auswählen (Dokument), Reads, Programm/Preset, Ergebnis wird als Alignment-Datei (Pfad) im Projekt referenziert (SAM/BAM **nicht** in die Projektdatei kopieren; Tabelle `external_files(id, path, kind)` im Projektspeicher).
- **Coverage-Spur:** unter der Sequenzansicht (Tiefe als Fläche), Variantenmarkierungen als Features (`variation`).
- **Presenter-Tests:** Ampel-Logik, Skalierung der Diagramme (Werte → Pixel) als reine Funktionen.

---

## Gesamtabnahme Phase 5

Integrationstest `NgsIT`: Zufallsreferenz 5 kb (Seed) → Reads simulieren (200 × 100 bp, eine Substitution an Position 2500 in 50 % der Reads) → QC → Trimming → (mit minimap2, sonst übersprungen) Mapping →
Coverage-Array plausibel (mittlere Tiefe ≈ 4) → Variantenaufruf findet genau die Substitution an Position 2500. Ohne minimap2: Alignments direkt als SAM aus den simulierten Positionen schreiben und denselben Aufrufpfad testen.
