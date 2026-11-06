# Phase 1: Sequenz- und Kartenansicht

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: Das Hauptfenster mit Sequenzansicht, Annotationsspuren, Karte (kreisförmig/linear), Enzymanzeige, Bearbeiten mit
Undo, automatischer Feature-Erkennung und mehr Dateiformaten.

**Leitprinzip:** Alles Rechnerische (Layout, Geometrie, Statistik) steckt in reinen Java-Klassen ohne JavaFX und wird
mit Zahlen getestet. Die JavaFX-Klassen zeichnen nur, was diese Klassen berechnet haben. Es gibt **keine** UI-Tests mit Fenstern.

Neue Pakete: `view/` (reine Layout-Logik), `fx/` (Zeichnen), `features/` (Feature-Datenbank und -Erkennung),
`io/` (weitere Formate), `edit/` (Bearbeiten mit Undo).

---

## Aufgabe 1.1: Zeilenlayout der Sequenzansicht

**Voraussetzung:** Phase 0.

**Dateien:** `view/SequenceLayout.java`, Test `view/SequenceLayoutTest.java`.

```java
public final class SequenceLayout {
    public SequenceLayout(int sequenceLength, int basesPerLine);          // basesPerLine >= 1
    public int lineCount();                                               // ceil(length / basesPerLine), mind. 1 bei Länge 0 → 0
    public int lineOf(int position);                                      // position / basesPerLine
    public int columnOf(int position);                                    // position % basesPerLine
    public int lineStart(int line);                                       // line * basesPerLine
    public int lineEnd(int line);                                         // min(lineStart + basesPerLine, length), exklusiv
    public int firstVisibleLine(double scrollY, double lineHeight);      // floor(scrollY / lineHeight)
    public int lastVisibleLine(double scrollY, double viewHeight, double lineHeight); // inklusive, begrenzt auf lineCount()-1
    public static int basesPerLineFor(double viewWidth, double charWidth, double marginWidth, int groupSize);
}
```

`basesPerLineFor`: verfügbare Breite `viewWidth - marginWidth`, ein Zeichen `charWidth`, nach je `groupSize` Zeichen
ein zusätzlicher Abstand von `charWidth`. Ergebnis ist ein **Vielfaches von `groupSize`**, mindestens `groupSize`.
Beispiel: `viewWidth=800, charWidth=10, marginWidth=80, groupSize=10` → Gruppenbreite 110 px (10 Zeichen + 1 Lücke) →
`floor(720/110) = 6` Gruppen → 60 Basen pro Zeile.

**Tests:**
- Länge 125, 60 pro Zeile: `lineCount()=3`, `lineOf(59)=0`, `lineOf(60)=1`, `columnOf(61)=1`, `lineEnd(2)=125`.
- Länge 0: `lineCount()=0`.
- `firstVisibleLine(250, 20)=12`; `lastVisibleLine(250, 100, 20)` bei 100 Zeilen = 17 (Pixel 250..350 → Zeilen 12..17).
- `basesPerLineFor(800,10,80,10)=60` und `basesPerLineFor(50,10,0,10)=10` (Minimum).

---

## Aufgabe 1.2: Auswahlmodell

**Voraussetzung:** keine.

**Dateien:** `view/Selection.java`, Test `view/SelectionTest.java`.

```java
public record Selection(int anchor, int caret) {            // beide 0-basiert, Positionen ZWISCHEN den Basen (0..length)
    public static Selection none();                         // anchor=caret=-1
    public static Selection caretAt(int position);
    public static Selection range(int start, int end);
    public boolean isEmpty();                               // anchor == caret oder none
    public int start();  public int end();                  // min / max
    public int length();
    public Selection extendTo(int position);                // anchor bleibt
    public Selection clamp(int sequenceLength);
}
```

**Tests:** `range(5,2).start()==2`, `.end()==5`, `.length()==3`; `extendTo(9)` auf `caretAt(3)` → `start 3, end 9`;
`clamp(4)` auf `range(2,10)` → `end 4`; `none().isEmpty()` wahr.

---

## Aufgabe 1.3: Annotationsspuren (Lane-Zuordnung)

**Voraussetzung:** 0.1.

**Dateien:** `view/FeatureLayout.java`, Test `view/FeatureLayoutTest.java`.

```java
public record PlacedFeature(Feature feature, Interval segment, int lane) {}

public final class FeatureLayout {
    /** Verteilt die Segmente aller Features auf Spuren, sodass sich keine zwei überlappen. */
    public static List<PlacedFeature> assignLanes(List<Feature> features);
    public static int laneCount(List<PlacedFeature> placed);
}
```

Algorithmus (Intervall-Färbung, greedy): alle Segmente (jedes Intervall jedes Features) nach `start`, dann nach Länge absteigend
sortieren; jedes Segment in die erste Spur legen, deren letztes Ende `<= segment.start` ist; sonst neue Spur anlegen.
Alle Segmente **eines** Features landen in derselben Spur (Join): wähle für das Feature die kleinste Spur, in der **alle**
seine Segmente passen. (Dann Features statt Segmente sortieren: nach `location().start()`, dann Länge absteigend.)

**Tests:**
- Drei Features `[0,10)`, `[5,15)`, `[10,20)` → Spuren 0, 1, 0 (das dritte passt hinter das erste).
- Zwei disjunkte Features → beide Spur 0, `laneCount=1`.
- Join-Feature `[0,5)`+`[20,25)` und Feature `[10,15)`: Kein Segment überlappt ein anderes, das Join-Feature belegt Spur 0, `[10,15)` passt ebenfalls in Spur 0. Erwartet beide Spur 0.
- Leere Liste → leere Ausgabe, `laneCount=0`.

---

## Aufgabe 1.4: Translations- und Rahmenspur

**Voraussetzung:** 0.2.

**Dateien:** `view/TranslationTrack.java`, Test `view/TranslationTrackTest.java`.

```java
public record AminoAcidCell(int basePosition, char aminoAcid, boolean isStart, boolean isStop) {}   // basePosition = 1. Base des Codons

public final class TranslationTrack {
    /** Translation im Rahmen 0, 1, 2 (Vorwärtsstrang) über [from, to). Unvollständige Codons am Ende entfallen. */
    public static List<AminoAcidCell> frame(String bases, int frame, int from, int to);
    /** Translation entlang eines Features (CDS): folgt den Intervallen (join) und dem Strang, Rahmen aus Qualifier /codon_start (Standard 1). */
    public static List<AminoAcidCell> forFeature(String bases, Feature cds);
}
```

Verwendet die Codon-Tabelle aus `Sequence` — dafür eine **öffentliche statische** Methode
`Sequence.aminoAcidFor(String codon)` (Großbuchstaben, T statt U) aus der bestehenden privaten Methode `codonToAminoAcid`
herausziehen (Verhalten unverändert; vorhandene Tests müssen grün bleiben).

**Tests:**
- `frame("ATGAAATAG", 0, 0, 9)` → `M`(start), `K`, `*`(stop); `basePosition` 0, 3, 6.
- `frame("AATGAAATAG", 1, 0, 10)` → gleiche drei Zellen mit `basePosition` 1, 4, 7.
- `forFeature` mit CDS `[0,9)` REVERSE auf `CTATTTCAT` → Reverse-Komplement `ATGAAATAG` → `M K *`.
- CDS mit Join `[0,3)`+`[6,9)` auf `ATGCCCTAG` → Codons `ATG`, `TAG` → `M *`.

---

## Aufgabe 1.5: Statistik und Zwischenablage für die Auswahl

**Voraussetzung:** 1.2.

**Dateien:** `view/SelectionStats.java`, `view/SequenceClipboard.java`, Tests dazu.

```java
public record SelectionStats(int length, double gcPercent, double tmCelsius) {
    public static SelectionStats of(String bases);        // Tm: bis 13 Basen Wallace (2*(A+T)+4*(G+C)),
                                                          // darüber 64.9 + 41*(G+C-16.4)/N   (Marmur/Doty, Quelle angeben)
}
public final class SequenceClipboard {
    public static String asText(String bases);                    // so wie sie ist
    public static String asFasta(String name, String bases);      // via Sequence.toFASTA-Format, 60 pro Zeile
    public static String asReverseComplement(String bases);
}
```

**Tests:** `of("ATGC")` → `length 4`, `gcPercent 50.0`, `tm 12.0`. `of("GGGGGGGGGGGGGGGGGGGG")` (20 G) → `gc 100`,
`tm = 64.9 + 41*(20-16.4)/20 = 72.28`. `asReverseComplement("ATGC")="GCAT"`. `asFasta("x", 70 Zeichen)` → 2 Sequenzzeilen (60 + 10).

---

## Aufgabe 1.6: Geometrie der Plasmidkarte

**Voraussetzung:** 0.1.

**Dateien:** `view/CircularMapLayout.java`, `view/LinearMapLayout.java`, Tests.

```java
public final class CircularMapLayout {
    public CircularMapLayout(int sequenceLength, double centerX, double centerY, double radius);
    /** Winkel in Grad, 0° = oben (12 Uhr), im Uhrzeigersinn; Position 0 liegt bei 0°. */
    public double angleOf(double position);
    public double[] pointAt(double position, double radiusOffset);       // {x, y}
    public double sweepOf(int start, int end);                           // (end-start)/length*360
    /** Beschriftungspositionen ohne Überlappung: gibt für jede Ankerposition einen Winkel zurück, Mindestabstand minAngleDeg. */
    public double[] spreadLabels(double[] anglesDeg, double minAngleDeg);
}
public final class LinearMapLayout {
    public LinearMapLayout(int sequenceLength, double x0, double x1);
    public double xOf(double position);                                  // x0 + position/length*(x1-x0)
    public int positionAt(double x);                                     // Umkehrung, gerundet, begrenzt 0..length
}
```

`pointAt`: `x = cx + (r+off)*sin(a)`, `y = cy - (r+off)*cos(a)` (a im Bogenmaß).
`spreadLabels`: Winkel sortieren, von links nach rechts jeweils auf `max(angle, previous + minAngle)` schieben,
am Ende zyklisch prüfen (Rest-Überlappung zum Anfang +360° durch gleichmäßiges Zurückschieben auflösen); Reihenfolge des Eingabe-Arrays im Ergebnis erhalten.

**Tests:** Länge 1000, Zentrum (0,0), r=100: `angleOf(0)=0`, `angleOf(250)=90`, `angleOf(500)=180`;
`pointAt(0,0)` = (0,−100) (±1e-9), `pointAt(250,0)` = (100, 0); `sweepOf(0,250)=90`.
`LinearMapLayout(1000, 50, 550)`: `xOf(500)=300`, `positionAt(300)=500`, `positionAt(0)=0`.
`spreadLabels([10,11,12], 5)` → `[10,15,20]` (±1e-9). Zwei Labels weit auseinander bleiben unverändert.

---

## Aufgabe 1.7: Enzymanzeige-Logik

**Voraussetzung:** Phase 0 (`RestrictionEnzymeDB` ist vorhanden).

**Dateien:** `view/EnzymeMapModel.java`, Test.

```java
public enum EnzymeFilter { ALL, UNIQUE_CUTTERS, DOUBLE_CUTTERS, NON_CUTTERS }

public final class EnzymeMapModel {
    public EnzymeMapModel(RestrictionEnzymeDB db, SequenceDocument doc);
    public Map<String, List<CutSite>> sitesByEnzyme();                           // alle Enzyme, die schneiden
    public List<CutSite> sites(EnzymeFilter filter, Set<String> onlyNames);     // onlyNames leer = alle
    public List<String> nonCutters();
    public boolean isCircular();                                                  // von doc.topology()
}
```

Hinweis: Für zirkuläre Sequenzen können Erkennungsstellen über den Nullpunkt hinweg liegen (Ende+Anfang). Dafür
`RestrictionEnzymeDB.findAllCutSites` **nicht** ändern, sondern in `EnzymeMapModel` die Sequenz um `maxRecognitionLength-1`
Basen vom Anfang verlängert suchen und Treffer mit `position >= length` auf `position - length` abbilden (Duplikate entfernen).

**Tests:** Sequenz `GAATTC` + 20×`A` + `GGATCC` (linear): EcoRI und BamHI sind Einzelschneider; `UNIQUE_CUTTERS`
enthält beide, `DOUBLE_CUTTERS` keinen. Zirkulär `TTCAAAAAAGAA` (Länge 12; die Erkennungsstelle `GAATTC` liegt über den Nullpunkt hinweg: `GAA` am Ende + `TTC` am Anfang)
→ EcoRI gefunden bei Position 9; linear betrachtet wird sie nicht gefunden. Sequenz ohne EcoRI-Stelle → `nonCutters()` enthält `EcoRI`.

---

## Aufgabe 1.8: Bearbeiten mit Undo/Redo

**Voraussetzung:** 0.2.

**Dateien:** `edit/EditSession.java`, Test `edit/EditSessionTest.java`.

```java
public final class EditSession {
    public EditSession(SequenceDocument initial);
    public SequenceDocument current();
    public void insert(int position, String bases);
    public void delete(int start, int end);
    public void replace(int start, int end, String bases);
    public void addFeature(Feature f);
    public void removeFeature(Feature f);
    public void setTopology(Topology t);
    public boolean canUndo(); public boolean canRedo();
    public void undo(); public void redo();
    public boolean isModified();               // current != zuletzt gespeicherter Stand
    public void markSaved();
    public int historyLimit();                 // 200 Schritte, älteste fallen weg
}
```

Jede Änderung legt den vorherigen `SequenceDocument` (unveränderlich, kein Kopieren nötig) auf den Undo-Stapel und leert
den Redo-Stapel. Basen werden vor dem Einfügen mit der Validierung von `Sequence` geprüft (ungültig → `IllegalArgumentException`,
Zustand unverändert).

**Tests:** Insert → Undo → Original; Redo → wieder geändert; nach Undo und neuer Änderung ist `canRedo()` falsch;
`isModified()` nach `markSaved()` falsch, nach Änderung wahr, nach Undo wieder falsch;
ungültiges Einfügen (`"X"`) wirft und ändert nichts; 250 Änderungen → mehr als 200 Undo-Schritte sind nicht möglich.

---

## Aufgabe 1.9: Feature-Datenbank und -Erkennung

**Voraussetzung:** 0.1.

**Dateien:** `features/FeatureDefinition.java`, `features/FeatureDatabase.java`, `features/FeatureDetector.java`,
`src/main/resources/features/standard-features.json`, Tests.

```java
public record FeatureDefinition(String name, String type, String sequence, String color, boolean isProtein) {}
public final class FeatureDatabase {
    public static FeatureDatabase standard();                  // lädt standard-features.json aus dem Klassenpfad
    public static FeatureDatabase fromJson(String json);
    public List<FeatureDefinition> all();
}
public final class FeatureDetector {
    public FeatureDetector(FeatureDatabase db, int maxMismatches);          // 0 = exakt
    public List<Feature> detect(SequenceDocument doc, boolean circular);     // beide Stränge, keine Duplikate
}
```

**Inhalt von `standard-features.json`:** Nur Features, deren Sequenz **öffentlich und kurz** ist; schreibe sie selbst aus
frei zugänglichen Quellen (z. B. GenBank-Einträge von pUC19 `L09137`, Lehrbuch-Konsensen). Mindestens (je Name, Typ, Sequenz):

| Name | Typ | Hinweis |
|---|---|---|
| `lac promoter` / `lacZα` Fragment | promoter | aus pUC19-Eintrag |
| `AmpR` | CDS | Sequenz des bla-Gens (861 bp) aus L09137 |
| `ori` (pMB1/ColE1) | rep_origin | aus L09137 |
| `T7 promoter` | promoter | `TAATACGACTCACTATAG` |
| `T7 terminator` | terminator | `CTAGCATAACCCCTTGGGGCCTCTAAACGGGTCTTGAGGGGTTTTTTG` |
| `SP6 promoter` | promoter | `ATTTAGGTGACACTATAG` |
| `M13 fwd` / `M13 rev` | primer_bind | `GTAAAACGACGGCCAGT` / `CAGGAAACAGCTATGAC` |
| `6xHis` | misc_feature | `CATCATCATCATCATCAC` |
| `FLAG` | misc_feature | `GACTACAAGGACGACGATGACAAG` |
| `HA` | misc_feature | `TACCCATACGATGTTCCAGATTACGCT` |
| `loxP` | misc_recomb | `ATAACTTCGTATAGCATACATTATACGAAGTTAT` |
| `attB1` / `attB2` | misc_recomb | siehe `CloningMethod.ATT_B1/ATT_B2` |

Jeder Eintrag bekommt eine Farbe (Hex). Einträge, deren Sequenz du nicht sicher aus einer freien Quelle belegen kannst,
**weglassen** und in `OFFENE_FRAGEN.md` notieren.

**Erkennung:** Sliding-Window über beide Stränge, Vergleich mit höchstens `maxMismatches` Abweichungen (Hamming),
IUPAC nicht nötig. Zirkulär: Sequenz um `maxLength-1` verlängern und Treffer über den Nullpunkt als Join-Location
(`[start,length)`+`[0,rest)`) ausgeben. Doppelte Treffer (gleiche Definition, gleiche Position) entfernen; ein Treffer mit weniger
Mismatches verdrängt überlappende der gleichen Definition. Das erzeugte `Feature` hat `type`, `name`, Strang und den Qualifier
`note="automatisch erkannt"`.

**Tests:** Sequenz mit eingebauter `T7 promoter`-Sequenz an Position 10 → ein Feature `[10,28)` FORWARD. Dieselbe Sequenz
reverse-komplementiert eingebaut → REVERSE. Mit einer Abweichung und `maxMismatches=0` nicht gefunden, mit `1` gefunden.
Feature über den Nullpunkt bei zirkulär: Join mit zwei Intervallen. Leere Datenbank → leere Liste.

---

## Aufgabe 1.10: Weitere Dateiformate

**Voraussetzung:** 0.2, 0.3.

**Dateien (jeweils mit Lesen und, wo sinnvoll, Schreiben):**
`io/EmblFormat.java`, `io/Gff3Format.java`, `io/BedFormat.java`, `io/NexusFormat.java`, `io/PhylipFormat.java`,
`io/ClustalFormat.java`, `io/Ab1Reader.java`, Tests mit kleinen Beispieldateien unter `src/test/resources/`.

Alle Formate geben Dokumente aus Phase 0 zurück. Alignment-Formate (Nexus, Phylip, Clustal) liefern zunächst nur
`List<Sequence>` mit Gaps (`-`) **erhalten** (Klasse `AlignedSequences(List<String> names, List<String> rows)`); ein
`AlignmentDocument` kommt in Phase 3.

- **EMBL:** wie GenBank, aber Zeilenpräfixe `ID`, `FT` (Features), `SQ` (Sequenz mit Zahlen am Zeilenende), `//`. Positionssyntax identisch.
- **GFF3:** Spalten `seqid, source, type, start, end, score, strand, phase, attributes` (tabulatorgetrennt, 1-basiert, inklusiv).
  `Name=`/`ID=` → Featurename, restliche Attribute → Qualifier. Zeilen `##FASTA` leiten Sequenzen ein. Features gehören zu der
  Sequenz mit passender `seqid`. Schreiben: `format(SequenceDocument)`.
- **BED:** `chrom start end name score strand` (0-basiert, Ende exklusiv!) → Features an ein vorhandenes Dokument hängen:
  `List<Feature> read(Path, String sequenceName)`.
- **Phylip (sequenziell und interleaved):** erste Zeile `n  Länge`, Namen 10 Zeichen (oder durch Leerzeichen getrennt, relaxed).
- **Clustal:** Kopfzeile `CLUSTAL`, Blöcke `Name  Sequenz`, Konservierungszeilen (`*`, `:`, `.`) ignorieren.
- **Nexus:** `#NEXUS`, `BEGIN DATA;`/`MATRIX`…`;`, Kommentare in `[ ]` entfernen, `FORMAT … GAP=- MISSING=?`.
- **AB1 (ABIF, big-endian):**
  - Dateikopf: Bytes 0..3 = `ABIF`, Version `short` bei 4.
  - Verzeichniseintrag (je 28 Bytes: Name 4 Zeichen, Nummer `int`, Elementtyp `short`, Elementgröße `short`,
    Elementanzahl `int`, Datengröße `int`, Datenoffset `int`, Handle `int`): erster Eintrag bei Offset 6 beschreibt das
    Verzeichnis selbst – `Elementanzahl` bei Offset 18, `Datenoffset` bei Offset 26.
  - Ist die Datengröße ≤ 4, stehen die Daten **im Feld Datenoffset selbst**.
  - Benötigte Tags: `PBAS`(Nr. 1; Basen als ASCII), `PCON`(1; Qualitätswerte als Bytes), `PLOC`(1; Peakpositionen als `short`),
    `FWO_`(1; Basenreihenfolge, 4 Zeichen, z. B. `GATC`), `DATA`(Nr. 9–12; Spurdaten `short` für die vier Farben in der `FWO_`-Reihenfolge),
    `SMPL`(1; Probenname als Pascal-String: 1. Byte = Länge).
  - Ergebnis: `record Chromatogram(String sampleName, String bases, int[] quality, int[] peakLocations, Map<Character,int[]> traces)`.

**Tests:** Pro Format eine kleine Beispieldatei (5–10 Zeilen, selbst geschrieben) mit erwarteten Werten, Round-trip wo
Schreiben möglich. Für AB1: Eine **synthetisch im Test erzeugte** Mini-ABIF-Datei (Bytes im Test zusammenbauen: Kopf,
zwei Einträge `PBAS1` mit `ACGT` und `PCON1` mit `{30,40,50,60}`) → Basen `ACGT`, Qualität `[30,40,50,60]`. Keine echten
.ab1-Dateien ins Repository legen (Datenschutz).

---

## Aufgabe 1.11: Hauptansicht zusammensetzen (JavaFX)

> **→ ersetzt:** durch **S.2–S.5** (Swift). Der Presenter-Teil bleibt im Rust-Kern (`view/presenters`); Zeichnen macht Swift. Nicht als JavaFX umsetzen.

**Voraussetzung:** 1.1–1.10 und 0.7.

**Dateien:** `fx/SequenceCanvas.java`, `fx/MapCanvas.java`, `fx/EditorView.java` (SplitPane: Karte links, Sequenz rechts),
`fx/EditorPresenter.java`. Der **Presenter** verbindet `EditSession`, `Selection`, `EnzymeMapModel`, `FeatureLayout`;
er ist testbar ohne Fenster.

Verhalten:
- Klick/Ziehen in der Sequenz setzt die `Selection`; Klick auf ein Feature wählt dessen Bereich; Auswahl wird in der Karte
  als Bogen hervorgehoben und umgekehrt (Klick in der Karte → `LinearMapLayout.positionAt` bzw. Winkel → Position).
- Tastatur: Pfeile bewegen den Cursor, Shift erweitert, Strg/Cmd+C kopiert (`SequenceClipboard.asText`), Strg/Cmd+Z/Y Undo/Redo,
  Entf/Backspace löscht die Auswahl, Buchstaben `A C G T` fügen Basen ein (nur im Bearbeiten-Modus, Schalter oben).
- Statusleiste: `SelectionStats` der Auswahl und Position des Cursors.
- Seitenleiste „Enzyme": `EnzymeFilter` Auswahl, Liste mit Anzahl der Schnittstellen.
- Seitenleiste „Features": Liste, Doppelklick springt zur Position, Hinzufügen/Löschen über `EditSession`.
- Schaltfläche „Features erkennen" ruft `FeatureDetector` auf und zeigt Vorschläge zum Übernehmen.
- Zeichnen nur sichtbarer Zeilen (`firstVisibleLine`…`lastVisibleLine`).

**Tests:** `EditorPresenter`: Auswahl setzen → `SelectionStats` korrekt; Entfernen der Auswahl über den Presenter ändert das
Dokument und die Auswahl wird auf die Löschposition zusammengezogen; Karten-Klick bei Winkel 90° auf Länge 1000 → Position 250.

**Fertig, wenn (manuell prüfen):** `puc19.gb` öffnen → Karte und Sequenz sichtbar, EcoRI-Einzelschneider markiert,
Auswahl synchron, Bearbeiten und Undo funktionieren, 1-Mb-Zufallssequenz (im Test erzeugen und per `Sequence.toFASTA`
speichern) scrollt flüssig. Alle `view/`-Tests grün.
