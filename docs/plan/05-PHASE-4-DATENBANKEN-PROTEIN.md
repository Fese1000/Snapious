# Phase 4: Datenbanken und Protein

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: NCBI-Suche und -Download, BLAST (remote und lokal), UniProt, Proteineigenschaften, RNA-Faltung, 3D-Ansicht.

Neue Pakete: `net/` (HTTP-Abstraktion, Cache), `db/ncbi/`, `db/uniprot/`, `blast/`, `protein/`, `rna/`.

**Regeln für alles mit Netzwerk (verbindlich):**
- Alle Netzwerkzugriffe laufen über das Interface `HttpFetcher` (4.1). Tests benutzen eine **Fake-Implementierung** mit fest hinterlegten Antworten.
  Echte Aufrufe stehen in Tests mit `@Tag("network")` (nicht Teil von `mvn test`).
- Rate-Limits der Dienste einhalten (siehe jeweilige Aufgabe), `User-Agent: Snapious/1.0 (privates Lernprojekt)`.
- Antworten werden im Cache-Verzeichnis (`~/.snapious/cache`) abgelegt, Schlüssel = SHA-256 der URL; Standardgültigkeit 24 h.
- Es werden **keine** Zugangsdaten gespeichert; optionale NCBI-API-Schlüssel nur in den Einstellungen (Phase 6), nie im Code.

---

## Aufgabe 4.1: HTTP-Abstraktion und Cache

**Voraussetzung:** keine.

**Dateien:** `net/HttpFetcher.java`, `net/JdkHttpFetcher.java`, `net/CachingFetcher.java`, `net/RateLimiter.java`, Tests.

```java
public interface HttpFetcher {
    HttpResult get(String url, Map<String,String> headers) throws IOException;
    HttpResult post(String url, Map<String,String> headers, String body) throws IOException;     // application/x-www-form-urlencoded
}
public record HttpResult(int status, String body, Map<String,List<String>> headers) {
    public boolean ok() { return status >= 200 && status < 300; }
}
public final class JdkHttpFetcher implements HttpFetcher { ... }        // java.net.http.HttpClient, Timeout 30 s, Redirects folgen
public final class CachingFetcher implements HttpFetcher {
    public CachingFetcher(HttpFetcher delegate, Path cacheDir, Duration ttl, Clock clock);   // nur GET werden gecacht, nur Status 200
}
public final class RateLimiter {
    public RateLimiter(int maxRequests, Duration perPeriod, Clock clock);
    public void acquire() throws InterruptedException;      // blockiert bis ein Platz frei ist
    public Duration nextWait();                              // Wartezeit ohne zu blockieren (für Tests)
}
```

**Tests:** `CachingFetcher` mit Fake: zweiter GET auf dieselbe URL ruft den Delegate nicht erneut auf; nach Ablauf der TTL (Fake-`Clock` vorspulen) ruft er erneut auf; Fehlerstatus 500 wird nicht
gecacht. `RateLimiter(3, 1 s)`: vier Aufrufe in Folge → `nextWait()` nach dem dritten > 0 und ≤ 1 s (mit Fake-Clock). `JdkHttpFetcher` gegen einen lokalen `com.sun.net.httpserver.HttpServer` auf
freiem Port (200-Antwort und 404): Status und Body korrekt.

---

## Aufgabe 4.2: NCBI Entrez (Suche und Download)

**Voraussetzung:** 4.1, Phase 0.3 (GenBank-Parser).

**Dateien:** `db/ncbi/EntrezClient.java`, `db/ncbi/SearchResult.java`, Tests mit Beispielantworten in `src/test/resources/ncbi/`.

```java
public record SearchResult(int count, List<String> ids) {}
public record SummaryEntry(String id, String accession, String title, String organism, int length, String moleculeType) {}

public final class EntrezClient {
    public EntrezClient(HttpFetcher fetcher, String toolName, String email, String apiKey /* darf null sein */);
    public SearchResult search(String database, String term, int retmax) throws IOException;              // db: "nuccore" | "protein" | "gene"
    public List<SummaryEntry> summaries(String database, List<String> ids) throws IOException;
    public List<SequenceDocument> fetchGenBank(String database, List<String> ids) throws IOException;     // rettype=gb
    public String fetchFasta(String database, String id) throws IOException;
}
```

URLs (Basis `https://eutils.ncbi.nlm.nih.gov/entrez/eutils/`):
- Suche: `esearch.fcgi?db=<db>&term=<urlencoded term>&retmax=<n>&retmode=json&tool=<tool>&email=<email>[&api_key=<key>]`. Antwort: JSON `{"esearchresult":{"count":"…","idlist":["…"]}}`.
- Zusammenfassung: `esummary.fcgi?db=<db>&id=<id1,id2>&retmode=json` → JSON `result.<id>` mit Feldern `accessionversion`, `title`, `organism`, `slen`, `moltype`.
- Download: `efetch.fcgi?db=<db>&id=<ids>&rettype=gb&retmode=text` → GenBank-Text, mit `GenBankFormat.parse` auslesen. FASTA: `rettype=fasta`.
- Rate-Limit: ohne API-Schlüssel 3 Anfragen/s, mit Schlüssel 10/s (`RateLimiter`). Fehlerstatus 429 → `IOException("NCBI: zu viele Anfragen")`.
- JSON mit der kleinen `Json`-Hilfsklasse aus Phase 0.5 parsen (Klasse bei Bedarf in ein gemeinsames Paket `util/` verschieben, Tests behalten).

**Tests (Fake-Fetcher mit Dateien aus `src/test/resources/ncbi/`; Beispielantworten selbst schreiben im echten Format):**
- `search("nuccore", "pUC19[All Fields]", 5)` → `count`, `ids` korrekt aus der Beispielantwort; URL des Aufrufs enthält `db=nuccore`, `retmax=5`, `tool=`, kodierten Suchbegriff (`%5B`).
- `summaries(...)` → `accession`, `title`, `length` aus Beispiel-JSON.
- `fetchGenBank` mit einer kleinen GenBank-Datei (die `mini.gb` aus Phase 0) → 1 Dokument; Fehler 404 → `IOException`.
- Mit API-Schlüssel enthält die URL `api_key=`; ohne nicht.
- Netzwerktest (`@Tag("network")`): `search("nuccore","L09137",1)` liefert mindestens eine ID.

---

## Aufgabe 4.3: BLAST (remote über NCBI, lokal mit BLAST+)

**Voraussetzung:** 4.1, 0.6.

**Dateien:** `blast/BlastHit.java`, `blast/BlastParser.java`, `blast/LocalBlast.java`, `blast/RemoteBlast.java`, Tests.

```java
public record BlastHit(String queryId, String subjectId, double percentIdentity, int alignmentLength, int mismatches, int gapOpens,
                       int queryStart, int queryEnd, int subjectStart, int subjectEnd, double evalue, double bitScore) {}

public final class BlastParser {
    /** Tabellenformat -outfmt 6 mit den Standardspalten (12 Spalten, tabgetrennt). Zeilen mit '#' ignorieren. */
    public static List<BlastHit> parseTabular(String text);
}
public final class LocalBlast {
    public LocalBlast(ToolRegistry tools);
    public Path makeDatabase(List<Sequence> subjects, boolean nucleotide, Path directory) throws IOException;      // schreibt FASTA und ruft makeblastdb
    public List<BlastHit> search(String program, Sequence query, Path database, double evalue, int maxHits) throws IOException;
}
public final class RemoteBlast {
    public RemoteBlast(HttpFetcher fetcher, Sleeper sleeper);
    public String submit(String program, String database, String query) throws IOException;          // gibt RID zurück
    public BlastStatus status(String rid) throws IOException;                                        // WAITING | READY | FAILED | UNKNOWN
    public List<BlastHit> results(String rid) throws IOException;
    public List<BlastHit> run(String program, String database, String query, OperationContext ctx) throws IOException, InterruptedException;
}
```

**Lokal:** `makeblastdb -in subjects.fa -dbtype nucl|prot -out <dir>/db`; Suche: `blastn|blastp|blastx|tblastn -query q.fa -db <dir>/db -evalue <e> -max_target_seqs <n> -outfmt "6 qseqid sseqid pident length mismatch gapopen qstart qend sstart send evalue bitscore"`.

**Remote (NCBI-URL-API, `https://blast.ncbi.nlm.nih.gov/Blast.cgi`):**
- Absenden (POST, form): `CMD=Put&PROGRAM=<blastn|blastp|…>&DATABASE=<nt|nr|…>&QUERY=<sequenz>`; Antwort enthält Zeilen `RID = <wert>` und `RTOE = <sekunden>` (im HTML/Text per Regex `RID\s*=\s*(\S+)`).
- Status (GET): `CMD=Get&FORMAT_OBJECT=SearchInfo&RID=<rid>` → Text enthält `Status=WAITING`, `Status=READY`, `Status=FAILED` oder `Status=UNKNOWN`; `ThereAreHits=yes` bei Treffern.
- Ergebnis (GET): `CMD=Get&FORMAT_TYPE=Tabular&RID=<rid>` — Tabellenformat (kann zusätzliche Kommentarzeilen enthalten).
- Wartezeiten nach NCBI-Regeln: nach dem Absenden mindestens `RTOE` Sekunden (mindestens 10) warten, dann **höchstens einmal pro Minute** den Status abfragen (`Sleeper`-Interface für testbares Warten); keine parallelen Massenabfragen. Timeout nach 15 Minuten → `IOException`.

**Tests:**
- `parseTabular` mit 2 Zeilen: `q1\ts1\t98.5\t100\t1\t0\t1\t100\t11\t110\t1e-45\t180`  → `percentIdentity 98.5`, `alignmentLength 100`, `evalue 1e-45`, `bitScore 180`; Kommentarzeilen ignoriert; Zeile mit falscher Spaltenzahl → `IllegalArgumentException`.
- `RemoteBlast.run` mit Fake-Fetcher: erste Statusantwort `WAITING`, zweite `READY`, dann Ergebnis; der Fake-`Sleeper` zeichnet die Wartezeiten auf: erste Wartezeit ≥ RTOE, danach 60 s.
- `LocalBlast` mit Fake-Tools (Shell-Skripte, die `makeblastdb` nachahmen und eine feste Tabelle ausgeben) prüft Kommandozeile und Parsing; mit echtem BLAST+ nur unter `Assumptions`: Suche der Teilsequenz einer Datenbanksequenz findet 100 % Identität.

---

## Aufgabe 4.4: UniProt

**Voraussetzung:** 4.1.

**Dateien:** `db/uniprot/UniProtClient.java`, `db/uniprot/UniProtEntry.java`, Tests mit Beispiel-JSON.

```java
public record ProteinFeature(String type, String description, int start, int end) {}     // 1-basiert, inklusiv (UniProt) → intern auf 0-basiert/exklusiv umrechnen für Feature
public record UniProtEntry(String accession, String name, String organism, String sequence, List<ProteinFeature> features) {
    public SequenceDocument toDocument();       // Protein-SequenceDocument; Features Typen "Domain", "Region", "Active site", … nach Feature-Modell
}
public final class UniProtClient {
    public UniProtClient(HttpFetcher fetcher);
    public List<String> search(String query, int size) throws IOException;           // Akzessionen
    public UniProtEntry fetch(String accession) throws IOException;
}
```

URLs: Suche `https://rest.uniprot.org/uniprotkb/search?query=<q>&size=<n>&format=json&fields=accession`; Eintrag `https://rest.uniprot.org/uniprotkb/<acc>.json`.
JSON-Felder: `primaryAccession`, `uniProtkbId`, `organism.scientificName`, `sequence.value`, `features[]` mit `type`, `description`, `location.start.value`, `location.end.value`.

**Tests:** Beispiel-JSON (selbst geschrieben, kleiner Eintrag mit 2 Features): `toDocument()` hat Typ PROTEIN; ein Feature `Domain` 10–20 (UniProt, 1-basiert inklusiv) wird zum Interval `[9,20)`;
Eintrag ohne Features → leere Liste; 404 → `IOException`.

---

## Aufgabe 4.5: Proteineigenschaften

**Voraussetzung:** keine.

**Dateien:** `protein/ProteinProperties.java`, `protein/Hydropathy.java`, Tests.

```java
public final class ProteinProperties {
    public static double molecularWeight(String protein);                     // durchschnittliche Masse in Da, + Wasser
    public static double isoelectricPoint(String protein);                    // Bisektion pH 0..14
    public static double extinctionCoefficient280(String protein, boolean reducedCysteines);   // M^-1 cm^-1
    public static Map<Character,Integer> composition(String protein);
    public static double chargeAtPh(String protein, double ph);
    public static double aromaticity(String protein);
    public static double instabilityIndex(String protein);                    // optional: nur wenn Dipeptid-Tabelle (Guruprasad 1990) selbst eingetragen wird, sonst weglassen
}
public final class Hydropathy {
    public static double[] kyteDoolittle(String protein, int window);         // Mittelwert pro Position über ein zentriertes Fenster (ungerade), Ränder: Fenster kürzen
}
```

**Konstanten:**
- Durchschnittliche Restmassen (Da): G 57.0519, A 71.0788, S 87.0782, P 97.1167, V 99.1326, T 101.1051, C 103.1388, L 113.1594, I 113.1594, N 114.1038,
  D 115.0886, Q 128.1307, K 128.1741, E 129.1155, M 131.1926, H 137.1411, F 147.1766, R 156.1875, Y 163.1760, W 186.2132; Wasser 18.0153.
- Kyte-Doolittle: A 1.8, R −4.5, N −3.5, D −3.5, C 2.5, Q −3.5, E −3.5, G −0.4, H −3.2, I 4.5, L 3.8, K −3.9, M 1.9, F 2.8, P −1.6, S −0.8, T −0.7, W −0.9, Y −1.3, V 4.2.
- pKa-Werte (EMBOSS-Satz): N-Terminus 8.6, C-Terminus 3.6, D 3.9, E 4.1, C 8.5, Y 10.1, H 6.5, K 10.8, R 12.5.
  Ladung = Σ positive `1/(1+10^(pH−pKa))` − Σ negative `1/(1+10^(pKa−pH))`. pI = pH, bei dem die Ladung 0 ist (Bisektion auf 0,001 genau).
- Extinktion (Pace et al. 1995): `ε280 = nW · 5500 + nY · 1490 + nC/2 · 125` (nur bei `reducedCysteines=false` mit Cystinen; sonst Cystein-Term 0).
- Unbekannte Zeichen (`X`, `*`) werden bei Masse ignoriert (Warnung nicht nötig), Stopp `*` am Ende wird abgeschnitten.

**Tests:**
- `molecularWeight("GG")` = 2·57.0519 + 18.0153 = 132.1191 (±0.001). `molecularWeight("")` = 0.
- `composition("AAB")` ignoriert unbekannte Zeichen nicht, sondern zählt nur A; Test auf `A=2`.
- `isoelectricPoint("GGGG")` = (8,6 + 3,6) / 2 = 6,1 (±0,05; nur die beiden Termini tragen Ladung). Ein Peptid aus 10×K hat pI > 10, ein Peptid aus 10×D hat pI < 4.
- `extinctionCoefficient280("W")` = 5500; `("Y")` = 1490; `("CC", false)` = 125.
- `kyteDoolittle("AAAA", 3)`: überall 1.8 (Ränder mit gekürztem Fenster gleich); `kyteDoolittle("AR", 1)` = [1.8, −4.5].
- `chargeAtPh("K", 7.0)` = 0,9755 (N-Terminus) + 0,9998 (Lys) − 0,9996 (C-Terminus) = 0,9757 (±0,005).

---

## Aufgabe 4.6: RNA-Faltung (ViennaRNA)

**Voraussetzung:** 0.6.

**Dateien:** `rna/RnaFold.java`, `rna/RnaStructure.java`, Tests.

```java
public record RnaStructure(String sequence, String dotBracket, double energy) {
    public List<int[]> basePairs();            // aus Punkt-Klammer-Notation, 0-basierte Paare (i,j), i<j
}
public final class RnaFold {
    public RnaFold(ToolRegistry tools);
    public RnaStructure fold(String rna) throws IOException;     // RNAfold --noPS (stdin), Ausgabe: Zeile 1 Sequenz, Zeile 2 "((...)) (-1.20)"
    static RnaStructure parseOutput(String output);               // paketprivat, testbar
}
```

`basePairs()`: Stapel für `(`/`)`; unausgewogene Klammern → `IllegalArgumentException`. Energie aus den Klammern am Zeilenende (`( -1.20)` oder `(-1.20)`).

**Tests:** `parseOutput("GGGAAACCC\n(((...))) ( -3.20)\n")` → `dotBracket "(((...)))"`, Energie −3.2, Paare `(0,8),(1,7),(2,6)`. Falsche Klammern → Exception.
Mit installiertem `RNAfold`: `GGGGAAAACCCC` faltet mit mindestens 3 Paaren (`Assumptions`).
Darstellung in der Oberfläche: Punkt-Klammer-Notation als Text unter der Sequenz sowie ein einfaches Bogendiagramm (Bögen über der Sequenz); Zeichnen per Layout-Klasse `ArcDiagramLayout` (Bogenmitte = (i+j)/2, Radius = (j−i)/2), mit Zahlentest.

---

## Aufgabe 4.7: 3D-Struktur anzeigen

**Voraussetzung:** Phase 0.7.

**Dateien:** `structure/PdbFile.java`, `fx/StructureView.java`, Ressource `src/main/resources/structure/viewer.html`, Test `structure/PdbFileTest.java`.

- `PdbFile.parse(String)` liest `ATOM`-/`HETATM`-Zeilen: Atomname (Spalten 13–16), Residuenname (18–20), Kette (22), Residuennummer (23–26), x/y/z (31–38, 39–46, 47–54), Element (77–78). Liefert `List<Atom>`;
  `chains()`, `sequence(char chain)` (Dreibuchstabencode → Einbuchstabencode, Standard-Aminosäuren; unbekannte `X`).
- `StructureView` bettet eine `WebView` ein, die `viewer.html` lädt; die PDB-Datei wird als Text per `executeScript` übergeben. Die HTML-Seite verwendet die frei verfügbare
  Bibliothek **3Dmol.js**; die Datei wird beim Build **nicht** automatisch heruntergeladen — bitte in `OFFENE_FRAGEN.md` vermerken, dass `3Dmol-min.js` manuell nach `src/main/resources/structure/`
  gelegt werden muss, falls das Netzwerk unterbrochen ist.
- Tests nur für `PdbFile` (kein Fenster).

**Tests:** Kleine PDB-Datei (3 Aminosäuren G-A-S, selbst geschrieben, korrekt formatierte Spalten) → `sequence('A')="GAS"`, 3 Residuen, Koordinaten stimmen.
Zeile mit zu kurzer Länge → wird übersprungen, kein Absturz.

---

## Aufgabe 4.8: Oberfläche und Operationen

**Voraussetzung:** 4.1–4.7, 1.11, 3.9.

- **NCBI-Suchfenster:** Eingabefeld, Datenbankauswahl, Trefferliste (Zusammenfassungen), Mehrfachauswahl, „Herunterladen" speichert als Dokumente im Projekt (mit Verlaufseintrag `db.ncbi.fetch`).
- **BLAST-Dialog:** Query aus Auswahl, Programm (blastn/blastp/blastx), Datenbank (remote: `nt`, `nr`, `refseq_rna`; lokal: Pfad), E-Wert, Treffer-Tabelle sortierbar, Doppelklick lädt Subject aus NCBI. Fortschritt und Abbrechen über `OperationContext`.
- **UniProt-Dialog:** Suche, Eintrag laden (Dokument mit Domänen als Features).
- **Protein-Ansicht:** Eigenschaftenliste, Hydropathie-Diagramm (Linie, Fenster 9), Zusammensetzung als Balken.
- **RNA-Ansicht:** Faltung, Struktur im Text und Bogendiagramm.
- Operationen: `db.ncbi.search`, `db.ncbi.fetch`, `blast.local`, `blast.remote`, `db.uniprot.fetch`, `protein.properties`, `rna.fold`.
- Presenter-Tests: NCBI-Dialog mit Fake-Fetcher (Suche → Liste mit erwarteter Anzahl); BLAST-Presenter sortiert nach E-Wert/Bitscore.

---

## Gesamtabnahme Phase 4

Szenario als Test mit Fake-Fetchern: NCBI-Suche → GenBank-Dokument laden → ORF finden → Translation → Eigenschaften (MW, pI) → Hydropathie-Array hat Länge = Proteinlänge.
Zusätzlich ein manueller Netzwerktest (nicht automatisiert): echte Suche nach `L09137`, Download, Anzeige der Karte.
