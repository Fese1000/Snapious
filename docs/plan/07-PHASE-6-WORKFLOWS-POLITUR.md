# Phase 6: Workflows, Plugins, Dokumentverwaltung und Politur

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: Operationen zu Workflows verketten, Plugins nachladen, Dokumente verwalten (Ordner, Suche, Versionen), Einstellungen, Export, Installationspaket und Qualitätswerkzeuge.

Neue Pakete: `workflow/`, `plugin/`, `project/` (Verwaltung), `settings/`, `export/`.

---

## Aufgabe 6.1: Operationen registrieren (Operation-Registry)

**Voraussetzung:** Phasen 0–5 (soweit umgesetzt).

**Dateien:** `ops/OperationRegistry.java`, Test.

```java
public final class OperationRegistry {
    public static OperationRegistry withBuiltins(ToolRegistry tools, HttpFetcher fetcher);    // registriert alle vorhandenen Operationen
    public void register(Operation op);                          // doppelte ID → IllegalArgumentException
    public Optional<Operation> find(String id);
    public List<Operation> all();                                // nach id sortiert
}
```

`withBuiltins` registriert alle in den Phasen 0–5 entstandenen Operationen (nur die, die bereits existieren; Liste in `OperationRegistry` als Konstante halten und bei jeder neuen Operation ergänzen).

**Tests:** Registrieren zweier Operationen mit gleicher ID → Exception; `find("edit.insert")` vorhanden; `all()` sortiert; `withBuiltins` enthält keine doppelten IDs.

---

## Aufgabe 6.2: Workflows

**Voraussetzung:** 6.1.

**Dateien:** `workflow/Workflow.java`, `workflow/WorkflowStep.java`, `workflow/WorkflowRunner.java`, `workflow/WorkflowJson.java`, Tests.

```java
public record StepInput(String reference) {}     // "$in:0" (Eingabedokument 0), "$step:2:1" (Ausgabe 1 des Schritts 2, 0-basiert)
public record WorkflowStep(String id, String operationId, Map<String,String> parameters, List<StepInput> inputs) {}
public record Workflow(String name, String description, int inputCount, List<WorkflowStep> steps, List<String> outputs /* Verweise wie bei StepInput */) {}

public final class WorkflowRunner {
    public WorkflowRunner(OperationRegistry registry, OperationRunner runner);
    public WorkflowResult run(Workflow wf, List<Document> inputs, OperationContext ctx) throws OperationException;
    public List<WorkflowResult> runBatch(Workflow wf, List<Document> inputs, OperationContext ctx);   // jeweils ein Dokument als Eingabe, Fehler eines Laufs stoppen die anderen nicht
}
public record WorkflowResult(List<Document> outputs, List<HistoryEntry> history, List<String> errors) {}

public final class WorkflowJson {
    public static String write(Workflow wf);
    public static Workflow read(String json);
}
```

**Regeln:** Schritte laufen in Reihenfolge; Verweise nur auf frühere Schritte (Vorwärtsverweis → `IllegalArgumentException` beim Validieren). Vor der Ausführung validieren: alle `operationId` bekannt, Verweise gültig,
`inputCount` stimmt mit der Anzahl übergebener Dokumente überein. Jeder Schritt erzeugt über `OperationRunner` seinen `HistoryEntry`. Fehler in einem Schritt → `OperationException` mit Schrittnummer; `runBatch` sammelt Fehler pro Eingabe.
JSON-Format selbst definieren und dokumentieren (`{"name":…, "steps":[{"id":…,"operation":…,"parameters":{…},"inputs":["$in:0"]}], "outputs":["$step:2:0"]}`), mit der `Json`-Hilfsklasse aus Phase 0/4.

**Tests:**
- Workflow `edit.reverseComplement` → `edit.insert` (position=1, bases=GG) auf `AATT`: Ergebnis `rc(AATT)`=`AATT` → Insert `A` `GG` `ATT` → `AGGATT`.
- Zwei Eingaben mit `runBatch` (`ATGC`, `GGGG`) → zwei Ergebnisse.
- Unbekannte Operation → `IllegalArgumentException` beim Validieren (vor jeder Ausführung; keine Historie).
- JSON round-trip: `read(write(wf)).equals(wf)`.
- Fehler im zweiten Schritt (ungültiger Parameter) → Ausnahme nennt `Schritt 2`; `runBatch` meldet den Fehler für diese Eingabe, die anderen laufen weiter.

---

## Aufgabe 6.3: Plugin-API

> **→ ersetzt:** Plugins sind **externe Programme** (`plugin.json`), siehe `08-RUST-KERN-SWIFT-APP.md`, Abschnitt „Abweichungen pro Phase". `ServiceLoader`/JAR entfällt.

**Voraussetzung:** 6.1.

**Dateien:** `plugin/SnapiousPlugin.java`, `plugin/PluginLoader.java`, `plugin/Importer.java`, `plugin/Exporter.java`, Beispiel-Plugin unter `examples/hello-plugin/` (eigenes kleines Maven-Projekt), Tests.

```java
public interface SnapiousPlugin {
    String name();
    String version();
    default List<Operation> operations() { return List.of(); }
    default List<Importer> importers() { return List.of(); }
    default List<Exporter> exporters() { return List.of(); }
}
public interface Importer { String name(); List<String> extensions(); List<Document> read(Path file) throws IOException; }
public interface Exporter { String name(); String extension(); boolean accepts(Document d); void write(Document d, Path file) throws IOException; }

public final class PluginLoader {
    public static List<SnapiousPlugin> load(Path directory);             // alle *.jar im Ordner per URLClassLoader + ServiceLoader<SnapiousPlugin>
    public static void register(List<SnapiousPlugin> plugins, OperationRegistry ops, FormatRegistry formats);
}
```

Fehlerhafte JARs werden übersprungen (Warnung im Log-Interface `PluginLog`), nie ein Absturz. Plugin-Ordner: `~/.snapious/plugins`. Plugins laufen **ohne Sandbox** — Hinweis in der README und im Einstellungsdialog.

**Tests:** Im Test ein Mini-JAR bauen (`javax.tools.JavaCompiler` kompiliert eine Plugin-Klasse im `@TempDir`; `META-INF/services/com.snapious.plugin.SnapiousPlugin` mit dem Klassennamen; mit `JarOutputStream` verpacken) →
`load` findet das Plugin mit `name()`, dessen Operation ist nach `register` in der Registry. Defektes JAR (Zufallsbytes) → übersprungen, kein Fehler. Doppelte Operations-ID aus Plugin → wird übersprungen, Warnung.

---

## Aufgabe 6.4: Dokumentverwaltung (Ordner, Suche, Stapelaktionen)

**Voraussetzung:** Phase 0.5.

**Dateien:** Erweiterung `store/ProjectStore` (Schema-Version 2: Spalte `folder_id`, Tabelle `folders`, Tabelle `notes`, Tabelle `favorites`), `project/ProjectService.java`, `project/DocumentSearch.java`, `project/BatchRename.java`, `project/Duplicates.java`, Tests.

```java
public record Folder(String id, String name, String parentId) {}
public final class ProjectService {
    public ProjectService(ProjectStore store);
    public Folder createFolder(String name, String parentId);
    public void moveDocument(String documentId, String folderId);          // null = Wurzel
    public void renameFolder(String id, String name); public void deleteFolder(String id, boolean deleteContents);
    public void setNote(String documentId, String text); public Optional<String> note(String documentId);
    public void setFavorite(String documentId, boolean favorite); public List<DocumentInfo> favorites();
    public List<DocumentInfo> list(String folderId);
}
public final class DocumentSearch {
    public static List<DocumentInfo> byName(ProjectStore store, String text);                 // enthält, case-insensitiv
    public static List<DocumentInfo> byType(ProjectStore store, String type);
    public static List<DocumentInfo> byFeatureText(ProjectStore store, String text);          // in Feature-Name/-Typ/-Qualifiern
    public static List<DocumentInfo> byMetadata(ProjectStore store, String key, String value);
    public static List<DocumentInfo> bySequenceMotif(ProjectStore store, String motifIupac);   // Motiv (IUPAC) in Sequenzen, beide Stränge
}
public final class BatchRename {
    /** Muster: {name} Ursprungsname, {n} Zähler ab 1, {n:03} Zähler mit führenden Nullen, {type}. */
    public static Map<String,String> plan(List<DocumentInfo> docs, String pattern);           // id → neuer Name
}
public final class Duplicates {
    public static List<List<DocumentInfo>> findSequenceDuplicates(ProjectStore store);        // gleiche Basen (SHA-256 der normalisierten Großbuchstaben-Sequenz), gleiche Typ
}
```

Migration von Schema 1 auf 2 beim Öffnen: `ALTER TABLE documents ADD COLUMN folder_id TEXT`, neue Tabellen anlegen, `PRAGMA user_version = 2`. Schema 1-Dateien müssen sich öffnen lassen (Test mit einer im Test erzeugten Version-1-Datei).

**Tests:** Ordner anlegen, Dokument verschieben, `list(folderId)` zeigt es; Löschen eines Ordners mit `deleteContents=false` verschiebt Inhalte in die Wurzel. `byFeatureText("lacZ")` findet das Dokument mit Feature `lacZα`. `bySequenceMotif` findet `GAATTC` auch im Reverse-Komplement
(Dokument enthält `GAATTC`, Suche nach `GAATTC` findet es; Dokument enthält nur `GGATTC`-Reverse-Komplement `GAATCC`, Suche nach `GGATTC` findet es ebenfalls). Motiv `RGATCY` findet ein Dokument mit `GGATCC` und eines mit `AGATCT`.
`BatchRename.plan` mit Muster `Konstrukt_{n:03}` für 3 Dokumente → `Konstrukt_001` … `Konstrukt_003`. `findSequenceDuplicates` gruppiert zwei Dokumente mit gleicher Sequenz (unterschiedliche Namen), nicht aber DNA mit identischem Text als RNA.

---

## Aufgabe 6.5: Dokumentversionen

**Voraussetzung:** 6.4.

**Dateien:** Erweiterung `store/ProjectStore` (Tabelle `versions(doc_id, version, time, payload, summary)`), `project/Versions.java`, Tests.

- Bei jedem `save(doc)`, dessen `payload` sich vom zuletzt gespeicherten unterscheidet, wird die **vorherige** Fassung in `versions` abgelegt (höchstens 50 pro Dokument, älteste fallen weg).
- `List<VersionInfo> versions(String docId)`; `Document loadVersion(String docId, int version)`; `void restore(String docId, int version)` (aktuelle Fassung wird dabei selbst zur Version).

**Tests:** Dokument speichern, ändern, speichern → eine Version (die erste Fassung); `restore` bringt den Ursprungszustand zurück und legt die zwischenzeitliche Fassung als weitere Version ab; 60 Änderungen → 50 Versionen.
Gleiche Fassung erneut speichern erzeugt keine neue Version.

---

## Aufgabe 6.6: Einstellungen

**Voraussetzung:** 0.6.

**Dateien:** `settings/Settings.java`, `settings/SettingsStore.java`, Test.

```java
public record Settings(Map<String,String> toolPaths, String ncbiEmail, String ncbiApiKey, String defaultOrganism, boolean darkMode,
                       int undoLimit, Path pluginDirectory, Path cacheDirectory) {
    public static Settings defaults();
}
public final class SettingsStore {
    public SettingsStore(Path file);                 // JSON-Datei ~/.snapious/settings.json
    public Settings load(); public void save(Settings s);
}
```

Datei mit Berechtigungen nur für den Nutzer (`PosixFilePermissions rw-------`), falls Dateisystem es erlaubt (sonst ignorieren). API-Schlüssel nie im Log ausgeben (`toString` maskiert ihn).
Einstellungsdialog in JavaFX (Presenter testbar): Werkzeugpfade mit Knopf „Prüfen" (`ToolRegistry.run(tool, ["--version"])`) und Statusanzeige.

**Tests:** Round-trip; fehlende Datei → Standardwerte; beschädigte Datei → Standardwerte und die defekte Datei wird als `.bak` gesichert; `toString()` enthält den Schlüssel nicht.

---

## Aufgabe 6.7: Export (Bild, SVG, Text)

**Voraussetzung:** Phase 1, 3.8.

**Dateien:** `export/MapSvg.java`, `export/SequenceTextExport.java`, `export/ReportBuilder.java`, Tests.

- `MapSvg.circular(SequenceDocument, CircularMapLayout, EnzymeMapModel, int size)` und `MapSvg.linear(...)`: SVG mit Features (Pfeile/Bögen), Enzymbeschriftungen, Titel (Name, Länge). Ausgabe wohlgeformtes XML.
- `SequenceTextExport.format(doc, int basesPerLine, boolean numbers, boolean features)`: Textdarstellung mit Positionszahlen, optional Translation und Feature-Liste.
- `ReportBuilder`: erzeugt aus einem `SequenceDocument` einen Textbericht (Länge, GC-Gehalt, Features-Tabelle, Enzym-Einzelschneider) als Markdown; für Klonierungsergebnisse zusätzlich Verlauf (`HistoryEntry`) mit Parametern.
- PDF: bewusst **nicht** mit einer neuen Bibliothek; Anleitung in der README: SVG im Browser oder Vorschau als PDF drucken.

**Tests:** SVG der zirkulären Karte parsen (`DocumentBuilder`), Anzahl `<path`/`<text` plausibel (ein Feature → mindestens ein `path`); `SequenceTextExport` bei 10 Basen pro Zeile und Positionszahlen: Zeile 1 beginnt mit `1`, Zeile 2 mit `11`.
Markdown-Bericht enthält die Überschriften `# <Name>` und `## Features`.

---

## Aufgabe 6.8: Oberfläche (Politur)

> **→ ersetzt:** Oberfläche in Swift (**S.9**); Presenter-Logik im Kern.

**Voraussetzung:** alle bisherigen.

- **Hauptfenster:** linke Projektleiste mit Ordnerbaum, Suchfeld, Favoriten; Tabs für geöffnete Dokumente; Menü „Operationen" (aus `OperationRegistry`, nach Kategorien gruppiert über das Präfix der ID); Menü „Workflow" (aufzeichnen aus Verlauf, ausführen, importieren/exportieren); Menü „Plugins".
- **Verlaufsansicht:** Tabelle der `HistoryEntry` pro Dokument; „Wiederherstellen" ruft `Versions.restore`.
- **Tastenkürzel:** Strg/Cmd+O öffnen, S speichern, Z/Y Undo/Redo, F Suche, +/− Zoom, E Enzym-Seitenleiste, L Feature-Seitenleiste.
- **Dunkelmodus:** zwei CSS-Dateien (`light.css`, `dark.css`) in `src/main/resources/fx/`; Umschalten in den Einstellungen; Zeichenfarben (Canvas) aus einer `Theme`-Klasse beziehen, nicht fest codiert.
- **Fehleranzeige:** `OperationException` → Dialog mit deutscher Meldung; unerwartete Fehler → Dialog mit Kurzmeldung + Logdatei `~/.snapious/log.txt` (einfacher Datei-Logger, kein neues Framework).

**Presenter-Tests:** Menüaufbau aus Registry (Anzahl der Einträge pro Kategorie), Workflow-Aufzeichnung (aus 3 History-Einträgen entsteht ein Workflow mit 3 Schritten und passenden Verweisen).

---

## Aufgabe 6.9: Qualität und Verpackung

> **→ ersetzt:** JaCoCo → `cargo llvm-cov`; `jpackage` → `scripts/bundle-app.sh` (`.app`); TestFX entfällt.

- **Testabdeckung:** JaCoCo (`org.jacoco:jacoco-maven-plugin`, Report `target/site/jacoco`). Ziel: ≥ 80 % Zeilenabdeckung in `model`, `ops`, `store`, `cloning`, `primer`, `align`, `tree`; Lücken in einem Bericht `docs/plan/ABDECKUNG.md` festhalten.
- **Leistung:** Tests mit Zeitgrenze (großzügig, z. B. 5 s): `SequenceDocument.insert` auf 1-Mb-Sequenz, `findAllCutSites` auf 1 Mb, NJ-Baum mit 100 Sequenzen à 500 bp, Gel für 1000 Fragmente.
- **Sprache der Meldungen:** Einheitlich Deutsch; ein Test prüft, dass `OperationException`-Nachrichten nicht leer sind.
- **UI-Smoketests** (optional): TestFX nur lokal, nicht im Standardlauf (`-Pui`). Mindestens ein Test startet `SnapiousFxApp`, öffnet `mini.gb` und prüft, dass die Dokumentliste einen Eintrag zeigt.
- **Installationspaket:** `jpackage --type dmg --name Snapious --input target --main-jar snapious-biotools-1.0.0.jar --main-class com.snapious.fx.SnapiousFxApp --module-path <javafx-jmods> --add-modules javafx.controls`
  (macOS); als Skript `scripts/package-mac.sh`. Das Skript prüft zuerst `jpackage -version` und gibt eine verständliche Meldung, wenn JavaFX-JMODs fehlen. **Kein** Signieren/Notarisieren (privates Projekt) — Hinweis in der README.
- **README:** Funktionsliste pro Phase mit Screenshots (optional), Abschnitt „Externe Programme" (welche, wofür, wo eintragen), Abschnitt „Grenzen" (z. B. kein Ersatz für Geneious/SnapGene).

---

## Gesamtabnahme Phase 6

Szenario (manuell, mit Protokoll in `docs/plan/ABNAHME.md`):
1. Neues Projekt, Ordner „Test" anlegen, `puc19.gb` importieren.
2. Workflow aus dem Verlauf aufzeichnen: Verdau → Klonierung → Export; auf drei Insert-Dateien im Stapel ausführen.
3. Version eines Dokuments wiederherstellen.
4. Beispiel-Plugin laden; seine Operation erscheint im Menü und läuft.
5. Dunkelmodus an/aus, Einstellungen speichern und neu starten.
6. `mvn clean package` grün, `scripts/package-mac.sh` erzeugt (falls JavaFX-JMODs vorhanden) ein DMG.
