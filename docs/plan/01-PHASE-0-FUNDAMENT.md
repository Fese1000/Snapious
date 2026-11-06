# Phase 0: Fundament

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: Dokumentmodell, Annotationen, GenBank mit Features, Operationen mit Verlauf, Projektspeicher (SQLite),
Anbindung externer Programme, JavaFX-Grundgerüst. Danach können alle späteren Phasen darauf aufbauen.

Neue Pakete (alle unter `src/main/java/com/snapious/`):

```
model/   Strand, Location, Feature, Topology, Document, SequenceDocument
io/      GenBankFormat (neu, neben dem bestehenden FileIO)
ops/     Operation, OperationContext, OperationResult, HistoryEntry, OperationRunner
store/   ProjectStore
tools/   ToolRegistry, ToolResult
fx/      SnapiousFxApp (JavaFX-Einstieg, noch minimal)
```

---

## Aufgabe 0.1: Feature-Modell

**Voraussetzung:** keine.

**Dateien:** `model/Strand.java`, `model/Location.java`, `model/Feature.java`, Tests `model/FeatureTest.java`.

```java
public enum Strand { FORWARD, REVERSE, NONE }      // + / - / unbekannt

/** Ein zusammenhängender Bereich, 0-basiert, Ende exklusiv. */
public record Interval(int start, int end) {
    public Interval { if (start < 0 || end < start) throw new IllegalArgumentException(...); }
    public int length() { return end - start; }
    public boolean overlaps(Interval o) { return start < o.end && o.start < end; }
}

/** Position eines Features: ein oder mehrere Intervalle (join) auf einem Strang. */
public record Location(List<Interval> intervals, Strand strand, boolean partialStart, boolean partialEnd) {
    public Location { intervals = List.copyOf(intervals); if (intervals.isEmpty()) throw ...; }
    public static Location of(int start, int end, Strand strand);
    public int start();          // kleinster Start aller Intervalle
    public int end();            // größtes Ende
    public int length();         // Summe der Intervall-Längen
    public boolean contains(int position);
    public Location shifted(int delta);    // alle Intervalle um delta verschieben
}

public final class Feature {
    public Feature(String type, String name, Location location, Map<String, List<String>> qualifiers);
    // Getter: type() z.B. "CDS", "gene", "promoter", name(), location(), qualifiers()
    public String qualifier(String key);                // erster Wert oder null
    public Feature withLocation(Location l);            // Kopie mit neuer Position
    // equals/hashCode über type, name, location, qualifiers
}
```

Die Qualifier-Map ist unveränderlich kopiert (`Map.copyOf` mit `List.copyOf` der Werte), Reihenfolge der Einfügung
bleibt per `LinkedHashMap` beim Erzeugen erhalten.

**Tests (Referenzwerte):**
- `Location.of(10, 20, FORWARD).length() == 10`, `.contains(10)` ist wahr, `.contains(20)` falsch.
- Join aus `[0,10)` und `[20,25)`: `length() == 15`, `start() == 0`, `end() == 25`, `contains(15)` falsch.
- `shifted(5)` auf `[10,20)` ergibt `[15,25)`.
- Ungültiges Intervall `new Interval(5, 3)` wirft `IllegalArgumentException`.
- `Feature.qualifier("gene")` liefert den ersten Wert; fehlender Schlüssel liefert `null`.

**Fertig, wenn:** Tests grün.

---

## Aufgabe 0.2: SequenceDocument

**Voraussetzung:** 0.1.

**Dateien:** `model/Topology.java`, `model/Document.java`, `model/SequenceDocument.java`, Test `model/SequenceDocumentTest.java`.

```java
public enum Topology { LINEAR, CIRCULAR }

public interface Document {
    String id();                         // eindeutig im Projekt (UUID-String), bei neuen Dokumenten zufällig
    String name();
    Instant created();
    Map<String, String> metadata();      // frei, z.B. "organism", "source", "accession"
    String type();                       // "sequence", "alignment", "tree", "table"
}

public final class SequenceDocument implements Document {
    public SequenceDocument(String id, String name, Sequence sequence, Topology topology,
                            List<Feature> features, Map<String, String> metadata, Instant created);
    public static SequenceDocument of(String name, Sequence sequence);       // linear, keine Features, neue UUID
    public Sequence sequence();
    public Topology topology();
    public List<Feature> features();                                        // unveränderlich
    public int length();
    public String bases();

    // Alle Änderungen liefern ein NEUES Dokument (gleiche id!), das Original bleibt unverändert:
    public SequenceDocument withName(String name);
    public SequenceDocument withTopology(Topology t);
    public SequenceDocument withFeatures(List<Feature> features);
    public SequenceDocument insert(int position, String bases);     // verschiebt Features
    public SequenceDocument delete(int start, int end);             // entfernt/kürzt Features
    public SequenceDocument replace(int start, int end, String bases);
    public SequenceDocument subDocument(int start, int end);        // Ausschnitt mit enthaltenen Features, Positionen -start
    public SequenceDocument reverseComplement();                    // Features gespiegelt, Strang gedreht
}
```

**Regeln für Features bei Änderungen** (verbindlich):
- `insert(pos, bases)` mit Länge `n`: Intervalle mit `start >= pos` werden um `n` verschoben; ein Intervall mit
  `start < pos < end` wird um `n` verlängert (`end += n`); Intervalle mit `end <= pos` bleiben.
  Ausnahme: `end == pos` bleibt unverändert (Einfügen direkt hinter dem Feature verlängert es nicht).
- `delete(start, end)` mit `d = end - start`: Intervalle komplett im gelöschten Bereich verschwinden; teilweise
  überlappende werden gekürzt; Intervalle dahinter werden um `d` verschoben. Verschwinden alle Intervalle eines
  Features, entfällt das Feature.
- `replace` = `delete` gefolgt von `insert` an `start`.
- `subDocument(s, e)`: nur Features, die **vollständig** in `[s, e)` liegen, übernehmen (Intervalle um `-s`).
- `reverseComplement()`: Intervall `[a, b)` bei Länge `L` wird zu `[L-b, L-a)`, Strang `FORWARD`↔`REVERSE`,
  Reihenfolge der Join-Intervalle umkehren.
- Ungültige Bereiche: `IndexOutOfBoundsException` (wie `Sequence.subsequence`).
- `Sequence` ist veränderlich (hat Setter) → im Konstruktor defensiv kopieren
  (`new Sequence(id, name, bases, type)`).

**Tests (Referenzwerte):**
- Dokument `ACGTACGTAC` (Länge 10), Feature `gene` `[2,6)` FORWARD:
  - `insert(0, "TT")` → bases `TTACGTACGTAC`, Feature `[4,8)`.
  - `insert(4, "TT")` → Feature `[2,8)` (Einfügen mittendrin verlängert).
  - `insert(6, "TT")` → Feature unverändert `[2,6)`.
  - `delete(0, 3)` → bases `TACGTAC`, Feature `[0,3)`.
  - `delete(2, 6)` → Feature entfällt.
  - `delete(4, 8)` → Feature `[2,4)`.
- `reverseComplement()` von `AACCGGTT` (Feature `[0,2)` FORWARD) → bases `AACCGGTT`, Feature `[6,8)` REVERSE.
- `subDocument(2, 8)` des ersten Beispiels enthält das Feature als `[0,4)`.
- Original bleibt nach jeder Änderung unverändert (`bases()` und `features()` vergleichen).

**Fertig, wenn:** Tests grün.

---

## Aufgabe 0.3: GenBank mit Features (Lesen und Schreiben)

**Voraussetzung:** 0.2.

**Dateien:** `io/GenBankFormat.java`, Test `io/GenBankFormatTest.java`, Testressource `src/test/resources/mini.gb`.

```java
public final class GenBankFormat {
    public static List<SequenceDocument> parse(String text) throws IOException;     // mehrere Einträge möglich ("//")
    public static List<SequenceDocument> read(Path file) throws IOException;
    public static String format(SequenceDocument doc);
    public static void write(SequenceDocument doc, Path file) throws IOException;
}
```

**Format (Lesen):**
- `LOCUS`-Zeile: Name, Länge, `bp`, Molekültyp (`DNA`, `RNA`, `mRNA` → type DNA/RNA), Topologie (`circular` oder `linear`).
  Beispiel: `LOCUS       mini                      60 bp    DNA     circular   SYN 01-JAN-2024`.
- `DEFINITION`, `ACCESSION`, `ORGANISM` → `metadata` (`definition`, `accession`, `organism`).
- Abschnitt `FEATURES`: Zeile beginnt mit 5 Leerzeichen + Typ (Spalte 6-20) + Position (ab Spalte 22). Qualifier-Zeilen
  beginnen mit 21 Leerzeichen und `/key="value"` (Wert kann über mehrere Zeilen laufen, Anführungszeichen beenden),
  oder `/key=value` (ohne Anführungszeichen) oder `/key` (ohne Wert → Wert `""`).
- Positionsgrammatik (GenBank ist 1-basiert, inklusiv → intern 0-basiert, Ende exklusiv):
  - `5` → `[4,5)`; `5..10` → `[4,10)`; `<5..10` → partialStart; `5..>10` → partialEnd;
  - `complement(5..10)` → REVERSE; `join(1..3,7..9)` → zwei Intervalle; `complement(join(1..3,7..9))` → REVERSE mit
    zwei Intervallen (Reihenfolge in aufsteigender Position speichern);
  - `join(complement(1..3),complement(7..9))` darf mit `IOException("nicht unterstützt: …")` abgelehnt werden.
- Name des Features = erster vorhandener Qualifier aus der Reihenfolge `label`, `gene`, `product`, `note`, sonst der Typ.
- Abschnitt `ORIGIN`: Zeilen `        1 acgtacgtac gtacgtacgt ...` → Ziffern und Leerzeichen entfernen, Großschreibung.
- Der Sequenztyp aus dem LOCUS bestimmt `SequenceType` (DNA wenn unklar). Nicht erlaubte Zeichen → `IOException` mit Zeilennummer.

**Format (Schreiben):**
- LOCUS-Zeile mit korrekter Länge und `circular`/`linear`; `FEATURES` in Eingabereihenfolge; Positionen zurück nach 1-basiert,
  `complement(...)` und `join(...)` korrekt verschachtelt; Qualifier-Werte in Anführungszeichen, auf 58 Zeichen umbrechen
  (21 Leerzeichen Einzug); `ORIGIN` mit 60 Basen pro Zeile in 6 Blöcken à 10, Zeilenbeginn rechtsbündig 9 Stellen;
  Abschluss `//`.

**Testressource `mini.gb`** (60 bp zirkulär, 2 Features; die Sequenz exakt so verwenden):

```
LOCUS       mini                      60 bp    DNA     circular   SYN 01-JAN-2024
DEFINITION  Testplasmid.
ACCESSION   TEST0001
FEATURES             Location/Qualifiers
     gene            5..20
                     /gene="abc"
     CDS             complement(30..50)
                     /gene="xyz"
                     /note="zwei Zeilen
                     Fortsetzung"
ORIGIN
        1 atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc
//
```

**Tests:**
- `parse(...)` liefert ein Dokument mit `length() == 60`, `topology() == CIRCULAR`, `metadata().get("accession").equals("TEST0001")`.
- Feature 1: Typ `gene`, Name `abc`, Position `[4,20)` FORWARD. Feature 2: Typ `CDS`, Name `xyz`, `[29,50)` REVERSE,
  Qualifier `note` = `zwei Zeilen Fortsetzung` (Zeilenumbruch + Einzug → ein Leerzeichen).
- Round-trip: `parse(format(doc))` ergibt gleiche Bases, Features (inkl. Qualifier) und Topologie.
- `join(1..3,7..9)` wird zu zwei Intervallen `[0,3)` und `[6,9)`.
- Ungültiges Zeichen in `ORIGIN` (z. B. `x`) → `IOException`.
- Zwei Einträge in einem Text → Liste der Länge 2.

**Fertig, wenn:** Tests grün; `FileIO.readGenBank` bleibt unverändert bestehen.

---

## Aufgabe 0.4: Operationen und Verlauf

**Voraussetzung:** 0.2.

**Dateien:** `ops/Operation.java`, `ops/OperationContext.java`, `ops/OperationResult.java`, `ops/HistoryEntry.java`,
`ops/OperationRunner.java`, Tests `ops/OperationRunnerTest.java`.

```java
public interface Operation {
    String id();                         // stabiler Name, z.B. "edit.insert", "clone.restriction"
    String label();                      // Anzeigename (deutsch)
    OperationResult run(List<Document> inputs, Map<String, String> parameters, OperationContext ctx)
        throws OperationException;
}

public interface OperationContext {      // Fortschritt und Abbruch
    void progress(double fraction, String message);   // 0.0 .. 1.0
    boolean isCancelled();
}

public record OperationResult(List<Document> outputs, String summary) {}

public class OperationException extends Exception { ... }   // Checked, mit Nachricht für die Oberfläche

public record HistoryEntry(String id, String operationId, Map<String,String> parameters,
                           List<String> inputIds, List<String> outputIds,
                           Instant time, String summary) {}

public final class OperationRunner {
    public OperationRunner(HistoryListener listener);                       // listener.add(HistoryEntry)
    public OperationResult execute(Operation op, List<Document> inputs,
                                   Map<String,String> parameters, OperationContext ctx) throws OperationException;
}
```

Verhalten von `execute`: führt aus, erzeugt bei Erfolg einen `HistoryEntry` (neue UUID, aktuelle Zeit,
IDs der Ein- und Ausgabedokumente) und meldet ihn dem Listener. Bei `OperationException` **kein** Eintrag.
Wird `ctx.isCancelled()` nach dem Lauf wahr, werden die Ergebnisse verworfen und `OperationException("abgebrochen")` geworfen.

Zusätzlich zwei einfache Operationen als Beispiel und für Tests:
`ops/builtin/ReverseComplementOperation` (`id = "edit.reverseComplement"`, ein `SequenceDocument` rein,
reverse-komplementiertes raus mit Name `"<alt> (rev-kom)"`) und `ops/builtin/InsertOperation`
(`edit.insert`, Parameter `position`, `bases`).

**Tests:**
- Erfolgreicher Lauf von `edit.reverseComplement` mit `AACCGGTT` → Ergebnis `AACCGGTT`; Test mit `ATGC` → `GCAT`.
- Ein `HistoryEntry` wurde gemeldet: `operationId == "edit.reverseComplement"`, `inputIds` enthält die Eingabe-ID.
- `InsertOperation` mit `position=2`, `bases=GG` auf `AATT` → `AAGGTT`.
- Fehlerhafter Parameter (`position=abc`) → `OperationException`, kein History-Eintrag.
- Abbruch: Operation, die `ctx.isCancelled()` nach Start auf `true` setzt → `OperationException`, kein Eintrag.

**Fertig, wenn:** Tests grün.

---

## Aufgabe 0.5: Projektspeicher (SQLite)

> **→ ersetzt:** In Rust mit `rusqlite` (Feature `bundled`) und `serde_json` statt JDBC und eigener Json-Klasse; Schema und Tests bleiben gleich. Maven-Zeile ignorieren.

**Voraussetzung:** 0.2, 0.3, 0.4.

**Maven:** Abhängigkeit `org.xerial:sqlite-jdbc:3.45.3.0` (Scope compile) in `pom.xml` ergänzen.

**Dateien:** `store/ProjectStore.java`, Test `store/ProjectStoreTest.java`.

```java
public final class ProjectStore implements AutoCloseable {
    public static ProjectStore open(Path file) throws IOException;      // legt Datei und Schema an, falls neu
    public void save(Document doc) throws IOException;                   // Insert oder Update nach id
    public Optional<Document> load(String id) throws IOException;
    public List<DocumentInfo> list() throws IOException;                 // id, name, type, created
    public void delete(String id) throws IOException;
    public void addHistory(HistoryEntry e) throws IOException;
    public List<HistoryEntry> history() throws IOException;              // nach Zeit aufsteigend
    public List<HistoryEntry> historyFor(String documentId) throws IOException;   // Eintrag, in dem die ID als Ein- oder Ausgabe vorkommt
    @Override public void close() throws IOException;
}
public record DocumentInfo(String id, String name, String type, Instant created) {}
```

**Schema** (beim Öffnen mit `CREATE TABLE IF NOT EXISTS`, plus `PRAGMA user_version = 1`):

```sql
CREATE TABLE documents(id TEXT PRIMARY KEY, type TEXT NOT NULL, name TEXT NOT NULL,
                       created INTEGER NOT NULL, payload TEXT NOT NULL);
CREATE TABLE history(id TEXT PRIMARY KEY, time INTEGER NOT NULL, operation TEXT NOT NULL,
                     parameters TEXT NOT NULL, inputs TEXT NOT NULL, outputs TEXT NOT NULL, summary TEXT);
```

`payload` ist ein selbst erzeugtes JSON (kein neues Jackson hinzufügen; kleine private Hilfsklasse `Json` in `store/`
mit `String write(Object)` / `Object parse(String)` für Map/List/String/Number/Boolean/null reicht).
Für `SequenceDocument` enthält das JSON: `name`, `type: "DNA|RNA|PROTEIN"`, `bases`, `topology`, `metadata`,
`features` (type, name, strand, partialStart, partialEnd, intervals als `[[s,e],...]`, qualifiers).
Unbekannte Dokumenttypen beim Laden: `IOException("Unbekannter Dokumenttyp: …")`.

**Tests (temporäres Verzeichnis `@TempDir`):**
- Dokument speichern, Store schließen, neu öffnen, laden → gleiche Bases, Features, Topologie, Metadaten.
- `save` zweimal mit derselben ID und geändertem Namen → `list()` hat 1 Eintrag mit neuem Namen.
- `delete` entfernt das Dokument; `load` liefert `Optional.empty()`.
- Verlauf: zwei Einträge speichern → `history()` in zeitlicher Reihenfolge; `historyFor(id)` findet Eintrag als Eingabe und als Ausgabe.
- Datei mit anderer `user_version` (z. B. 99) öffnen → `IOException`.

**Fertig, wenn:** Tests grün.

---

## Aufgabe 0.6: Anbindung externer Programme

**Voraussetzung:** keine.

**Dateien:** `tools/ToolRegistry.java`, `tools/ToolResult.java`, Test `tools/ToolRegistryTest.java`.

```java
public record ToolResult(int exitCode, String stdout, String stderr, Duration duration) {
    public boolean ok() { return exitCode == 0; }
}

public final class ToolRegistry {
    public static ToolRegistry defaults();                       // sucht in PATH und gängigen Pfaden (/opt/homebrew/bin, /usr/local/bin)
    public ToolRegistry withExecutable(String tool, Path executable);   // Pfad manuell setzen (unveränderlich: neue Instanz)
    public boolean isAvailable(String tool);
    public Optional<Path> executable(String tool);
    public ToolResult run(String tool, List<String> args, Path workingDir, String stdin, Duration timeout)
        throws IOException;                                      // IOException bei fehlendem Tool, Timeout, Start-Fehlern
    public static boolean isAvailableStatic(String tool);        // Kurzform für Assumptions: defaults().isAvailable(tool)
}
```

Bekannte Tool-Namen (nur Suchnamen, nichts installieren): `mafft`, `muscle`, `clustalo`, `blastn`, `blastp`, `makeblastdb`,
`primer3_core`, `iqtree3`, `iqtree2`, `iqtree`, `FastTree`, `minimap2`, `bwa`, `samtools`, `bcftools`, `RNAfold`, `spades.py`.

Umsetzung mit `ProcessBuilder`: stdout und stderr in zwei Threads lesen (sonst Blockade bei großer Ausgabe),
Timeout mit `Process.waitFor(timeout)`, bei Überschreitung `destroyForcibly()` und `IOException("Zeitüberschreitung")`.

**Tests (ohne echte Bio-Tools, plattformunabhängig mit `sh`, nur auf Unix; sonst `Assumptions.assumeTrue`):**
- `withExecutable("echo", Path.of("/bin/echo")).run("echo", List.of("hallo"), ...)` → `stdout` ist `hallo\n`, `ok()`.
- `run("cat", ..., stdin="abc")` → `stdout == "abc"`.
- `run("sh", List.of("-c", "exit 3"), ...)` → `exitCode == 3`.
- `run("sleep", List.of("5"), ..., Duration.ofMillis(200))` → `IOException`.
- Unbekanntes Tool `"gibtsnicht"` → `isAvailable` false, `run` wirft `IOException`.
- Große Ausgabe (`sh -c 'yes | head -n 200000'`) blockiert nicht.

**Fertig, wenn:** Tests grün.

---

## Aufgabe 0.7: JavaFX-Grundgerüst

> **→ ersetzt:** durch Aufgabe **S.0** (Swift-Fenster) in `08-RUST-KERN-SWIFT-APP.md`. Nicht umsetzen.

**Voraussetzung:** 0.5 (für „Projekt öffnen"), sonst keine.

**Maven (`pom.xml`):**
- Properties: `<javafx.version>21.0.5</javafx.version>`.
- Abhängigkeiten `org.openjfx:javafx-controls` und `org.openjfx:javafx-graphics` (Version `${javafx.version}`).
  Die Plattform-Klassifizierer löst Maven über das Profil des OpenJFX-Artefakts automatisch auf.
- Plugin `org.openjfx:javafx-maven-plugin:0.0.8` mit `<mainClass>com.snapious.fx.SnapiousFxApp</mainClass>`.
  Starten: `mvn javafx:run`.
- **Kein** `module-info.java` (läuft auf dem Klassenpfad). Das bestehende Swing-`SnapiousApp` bleibt Hauptklasse im JAR.

**Dateien:** `fx/SnapiousFxApp.java` (extends `javafx.application.Application`), `fx/ProjectModel.java`, Test `fx/ProjectModelTest.java`.

Fenster 1280×800, Titel „Snapious", links eine `ListView<DocumentInfo>` (Dokumentliste), Mitte ein leerer
Platzhalter („Dokument auswählen"), oben Menü „Datei": „Projekt öffnen…", „Projekt neu…", „Sequenz importieren…"
(FASTA oder GenBank über `FileChooser`), „Beenden". Importierte Dokumente werden im `ProjectStore` gespeichert und
erscheinen in der Liste.

**Wichtig:** Die Logik steckt in `ProjectModel` (ohne JavaFX-Typen außer `ObservableList`, oder ganz ohne), damit sie
ohne Fenster getestet werden kann:

```java
public final class ProjectModel {
    public void open(Path file) throws IOException;
    public void importFile(Path file) throws IOException;     // .fa/.fasta/.gb/.gbk → Dokumente speichern
    public List<DocumentInfo> documents();
    public Optional<Document> load(String id) throws IOException;
}
```

**Tests:** `ProjectModel` mit `@TempDir`: Projekt öffnen, `mini.gb` importieren → `documents()` hat 1 Eintrag namens `mini`.
FASTA mit zwei Einträgen importieren → 2 Dokumente. Unbekannte Dateiendung → `IOException`. **Kein** Test, der ein Fenster öffnet.

**Fertig, wenn:** `mvn test` grün und `mvn javafx:run` öffnet das Fenster; Import von `mini.gb` zeigt den Eintrag in der Liste.

---

## Aufgabe 0.8: Abschluss Phase 0

- Alle neuen Pakete mit kurzer Beschreibung ins README (Abschnitt „Architektur").
- Abschnitt „Fortschritt" in `00-ARBEITSANWEISUNG.md`: Phase 0 abhaken.
- `mvn clean package` läuft komplett durch, `java -jar target/snapious-biotools-1.0.0.jar` startet wie vorher die Swing-Oberfläche.
- Mindestens 40 neue Tests in dieser Phase (Summe aller Aufgaben).
