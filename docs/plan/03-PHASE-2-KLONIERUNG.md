# Phase 2: Klonierung und Primer

> **Hinweis zur Umsetzung:** Die Signaturen in diesem Dokument sind in Java-Schreibweise. Umgesetzt wird in **Rust** (Abbildung: `08-RUST-KERN-SWIFT-APP.md`, Tabelle „Abbildung Java-Anleitung → Rust").
> Aufgaben, die JavaFX/Maven/JDBC/`jpackage` erwähnen, sind **ersetzt** (Swift-Aufgaben S.x bzw. `rusqlite`, `cargo`); sie sind unten mit „→ ersetzt" markiert. Zuerst `00-ARBEITSANWEISUNG.md` lesen.


Ziel: Konstrukte virtuell bauen (Restriktion, Gibson/In-Fusion/SLIC, Golden Gate, Gateway, TOPO/TA), PCR und Gel
simulieren, Primer entwerfen, Codons optimieren, gRNAs finden, Sequenzierungsergebnisse prüfen.

Neue Pakete: `cloning/sim/` (Simulationen), `primer/` (ergänzend), `gel/`, `crispr/`, `codon/`, `seqcheck/`.
Die vorhandenen Klassen `RestrictionEnzymeDB`, `CloningMethod` (Textprotokolle) und `PrimerDesign` bleiben bestehen und
werden verwendet. Alle Simulationen sind **reine Funktionen** auf `SequenceDocument` und als `Operation` (Phase 0.4) verpackt.

**Grundbegriffe (für alle Aufgaben):**
- Alle Positionen 0-basiert, Ende exklusiv. Sequenzen in Großbuchstaben, Alphabet `ACGT` (+`N`).
- „Top-Strang" = die gespeicherte Sequenz 5'→3'. „Bottom-Strang" = Reverse-Komplement.
- Ein Restriktionsschnitt hat zwei Positionen im Top-Koordinatensystem: `cutTop` (Schnitt im Top-Strang) und
  `cutBottom` (Schnitt im Bottom-Strang, in Top-Koordinaten ausgedrückt). Ist `cutBottom > cutTop`, entsteht ein
  **5'-Überhang** der Länge `cutBottom − cutTop`; ist `cutBottom < cutTop`, ein **3'-Überhang**; sind beide gleich, **stumpf**.
- Der **Überhang in Top-Schreibweise** ist die Teilsequenz `top[min(cutTop,cutBottom), max(cutTop,cutBottom))`.
  Beispiel EcoRI `G^AATTC` (cutTop=1, cutBottom=5 relativ zum Erkennungsstart): Überhang `AATT`, 5'-Typ.
  PstI `CTGCA^G` (cutTop=5, cutBottom=1): Überhang `TGCA`, 3'-Typ.

---

## Aufgabe 2.0: Vorarbeiten an bestehenden Klassen

**Voraussetzung:** Phase 1.

1. `SequenceDocument.rotate(int newOrigin)` (nur zirkuläre Dokumente, sonst `IllegalStateException`; `0 <= newOrigin < length`):
   die neue Sequenz ist `bases[newOrigin..] + bases[..newOrigin]`. Features werden mitverschoben: Position `p` wird zu `(p − newOrigin) mod L`.
   Ein Intervall `[a,b)`, das dabei den neuen Nullpunkt überspannt, wird in zwei Intervalle `[a',L)` und `[0,b')` aufgeteilt (Join).
   Test: `ACGTAC` zirkulär, `rotate(2)` → `GTACAC`. Feature `[2,4)` (GT) wird `[0,2)`. Feature `[1,4)` (CGT) überspannt danach den neuen
   Nullpunkt und wird ein Join aus `[5,6)` und `[0,2)`; seine Gesamtlänge bleibt 3.
2. `Alignment.semiGlobal(String read, String reference)` (in `alignment/Alignment.java`): globales Alignment, bei dem
   Endlücken in `reference` **nicht bestraft** werden (erste Zeile/Spalte der Matrix = 0 für die Referenz-Dimension; Ergebnis =
   Maximum der letzten Zeile des Reads). Gibt `AlignResult` zurück; ergänzend `int referenceStart()` und `referenceEnd()`.
   Test: `semiGlobal("ACGT", "TTTACGTTTT")` → Score 8, `referenceStart()=3`, `referenceEnd()=7`.
3. `RestrictionEnzymeDB.RestrictionEnzyme` bekommt `int getCutTopOffset()` (= `getCutPositionWithinRecognitionSite()`) und
   `int getCutBottomOffset()` (= `getBottomStrandCutPosition()`), nur Alias-Methoden; Test genügt: EcoRI → 1 und 5.

---

## Aufgabe 2.1: Virtueller Verdau

**Voraussetzung:** 2.0.

**Dateien:** `cloning/sim/EndType.java`, `cloning/sim/End.java`, `cloning/sim/Fragment.java`, `cloning/sim/VirtualDigest.java`, Tests.

```java
public enum EndType { BLUNT, FIVE_PRIME, THREE_PRIME }

/** Ein Ende eines DNA-Fragments. overhang ist in Top-Schreibweise (siehe Grundbegriffe), bei BLUNT leer. */
public record End(EndType type, String overhang) {
    public static final End BLUNT_END = new End(EndType.BLUNT, "");
    public End reverseComplement();     // Typ bleibt, overhang wird reverse-komplementiert
    public boolean pairsWith(End other);   // siehe Regel unten
}

/**
 * Doppelsträngiges Fragment. top enthält den Oberstrang von seinem 5'-Ende bis zum 3'-Ende.
 * left/right sind die Enden. features sind in Fragment-Koordinaten.
 */
public record Fragment(String top, End left, End right, List<Feature> features, String sourceName,
                       int sourceStart, int sourceEnd) {
    public int length();                       // top.length()
    public Fragment reverseComplement();       // top rc, left/right vertauscht und je rc, Features gespiegelt
    public boolean canJoin(Fragment next);     // this.right.pairsWith(next.left)
}

public final class VirtualDigest {
    public VirtualDigest(RestrictionEnzymeDB db);
    public List<Fragment> digest(SequenceDocument doc, List<RestrictionEnzyme> enzymes);
    public List<CutSite> cuts(SequenceDocument doc, List<RestrictionEnzyme> enzymes);   // sortiert nach cutTop
}
```

**Regel `pairsWith`:** `a.pairsWith(b)` ist wahr, wenn (1) beide `BLUNT` oder (2) beide denselben Typ (`FIVE_PRIME` oder `THREE_PRIME`)
haben und `a.overhang().equals(b.overhang())`. (Das gilt, weil jeweils die Top-Schreibweise des Überhangs verglichen wird;
Begründung: nach dem Verbinden ergibt Top(links) + Top(rechts) die ursprüngliche Folge, siehe 2.2.)

**Berechnung `digest`:**
1. Alle Schnitte der gewählten Enzyme bestimmen. Dafür in `RestrictionEnzymeDB` eine öffentliche Methode
   `List<int[]> cutPositions(SequenceDocument doc, RestrictionEnzyme enzyme)` ergänzen, die für jeden Treffer (beide Stränge, bei
   zirkulärer Sequenz auch über den Nullpunkt) `{cutTop, cutBottom}` in Top-Koordinaten liefert. Intern sind das genau die Werte
   `Match.cutTop()` und `Match.cutBottom()` der Klasse `RestrictionEnzyme` (dort bereits für beide Orientierungen gespiegelt berechnet).
   Bei zirkulären Sequenzen die Suche auf `bases + bases.substring(0, maxRecognitionLength−1)` ausführen und Positionen modulo `L` abbilden.
2. Doppelte `cutTop` (verschiedene Enzyme schneiden am selben Punkt) zusammenfassen.
3. **Linear:** Schnitte sortieren; Fragmente `[0,c1)`, `[c1,c2)`, …, `[ck,L)`. Das linke Ende des ersten und das rechte des letzten
   Fragments sind `BLUNT_END` (Dokumentenden). Ein Schnitt bei Position 0 oder `L` erzeugt kein leeres Fragment.
4. **Zirkulär:** keine Schnitte → ein Fragment (die ganze Sequenz, beide Enden `BLUNT_END`, zusätzlich Markierung nicht nötig).
   Mit `k ≥ 1` Schnitten entstehen `k` Fragmente; das letzte läuft über den Nullpunkt (Dokument vorher mit `rotate(c1)` drehen).
5. Ende-Typen aus `cutTop`/`cutBottom`: rechtes Ende des Fragments links des Schnitts und linkes Ende des Fragments rechts vom Schnitt
   bekommen **beide** `End(type, overhang)` mit `type` = FIVE_PRIME wenn `cutBottom > cutTop`, THREE_PRIME wenn kleiner, BLUNT wenn gleich;
   `overhang = top.substring(min, max)` (aus der Quellsequenz, bei Zirkularität modulo Länge).
6. Features: jedes Feature, dessen Bereich komplett in das Fragment fällt, wird mit verschobenen Koordinaten übernommen;
   Features, die einen Schnitt überspannen, entfallen.
7. `sourceStart/End` = Schnittpositionen in der Quelle (bei Wrap: `sourceEnd < sourceStart` erlaubt).

**Tests (Referenzwerte, alles von Hand nachrechenbar):**
- Linear `AAGGAATTCGATTAGCTAGC` (20 bp), EcoRI (GAATTC bei 3, cutTop=4, cutBottom=8): Fragmente `AAGG` (links `BLUNT`, rechts `FIVE_PRIME "AATT"`)
  und `AATTCGATTAGCTAGC` (links `FIVE_PRIME "AATT"`, rechts `BLUNT`). `fragment[0].canJoin(fragment[1])` wahr.
- Zirkulär 1000 bp, EcoRI-Stelle bei 100, BamHI-Stelle bei 400, sonst `A`-Füllung: Verdau mit beiden → 2 Fragmente mit Längen 300 und 700.
  (Test-Sequenz im Test zusammenbauen: `"A"*100 + "GAATTC" + …` und Längen exakt einstellen, Summe 1000.)
- Zirkulär mit einer Schnittstelle → 1 Fragment gleicher Länge, linke Kante `FIVE_PRIME` (EcoRI), rechte Kante ebenso (gleicher Überhang).
- PstI (3'-Überhang): Fragment `…CTGCA` | `G…`; linkes Ende des rechten Fragments `THREE_PRIME "TGCA"`.
- Stumpfer Schneider SmaI (`CCC^GGG`): beide Enden `BLUNT`, Fragmente `…CCC` und `GGG…`.
- `reverseComplement()` eines Fragments ändert die Länge nicht, vertauscht die Enden und `rc.rc.equals(original)` (Fragmentgleichheit).
- Kein passendes Enzym → ein Fragment (linear: die ganze Sequenz).
- Enzym, das einen Strang im Gegensinn trifft (BsaI-Rückwärtsstelle), schneidet an der gespiegelten Position (Test mit der Sequenz aus 2.4).

---

## Aufgabe 2.2: Restriktionsklonierung (Ligation)

**Voraussetzung:** 2.1.

**Dateien:** `cloning/sim/Ligation.java`, `cloning/sim/RestrictionCloning.java`, `cloning/sim/CloningCandidate.java`, Operation
`cloning/sim/RestrictionCloningOperation.java` (`id = "clone.restriction"`), Tests.

```java
public record CloningCandidate(Fragment vector, Fragment insert, boolean insertReversed,
                               boolean directional, SequenceDocument construct, List<String> warnings) {}

public final class Ligation {
    /** Verbindet Fragmente in Reihenfolge zu einem Top-Strang. Wirft IllegalArgumentException, wenn ein Paar nicht passt. */
    public static String join(List<Fragment> fragments, boolean circular);
}

public final class RestrictionCloning {
    public RestrictionCloning(VirtualDigest digest);
    /** Alle möglichen zirkulären Produkte aus Vektor- und Insert-Verdau. */
    public List<CloningCandidate> candidates(SequenceDocument vector, List<RestrictionEnzyme> vectorEnzymes,
                                             SequenceDocument insert, List<RestrictionEnzyme> insertEnzymes);
}
```

**Regel Ligation:** Fragmente `f1, f2, …, fn` in dieser Reihenfolge; für jedes Paar muss `fi.canJoin(fi+1)` gelten;
bei `circular` zusätzlich `fn.canJoin(f1)`. Ergebnis-Top-Strang = `f1.top() + f2.top() + … + fn.top()` (ohne Zusatz). Beispiel
EcoRI/BamHI-Rekonstitution: `…G` + `AATTC…` → `GAATTC`. Beispiel BamHI (Überhang `GATC`) mit BglII (`GATC`) → `…G` + `GATCT…` → `GGATCT` (Hybrid, gültig).

**`candidates`:**
1. Vektorfragmente und Insert-Fragmente verdauen. Der **Vektor-Rückgrat-Kandidat** ist jedes Vektorfragment, das beide Ende nicht
   `BLUNT_END`-Dokumentenden hat bzw. (zirkulärer Vektor mit einem Schnitt) das einzige Fragment. Standardmäßig wird nur das **größte**
   Vektorfragment als Rückgrat betrachtet; Parameter `allVectorFragments=true` (Operation-Parameter `vectorFragment=index`) erlaubt andere.
2. Insert-Kandidaten: alle Insert-Fragmente, die mindestens an einem Ende ein Schnittende haben (innere Fragmente bevorzugt).
3. Für jedes Paar (V, I) und beide Orientierungen (I oder I.reverseComplement()) prüfen: `V.canJoin(I)` und `I.canJoin(V)` (zirkulär).
   Treffer → `CloningCandidate`. `directional = true`, wenn die beiden Enden des Vektors **unterschiedlich** sind
   (`!V.left().equals(V.right())`) — dann ist nur eine Orientierung möglich.
4. Konstrukt: `top = V.top() + I.top()`; Name `<vector.name> + <insert.name>`; Topologie CIRCULAR;
   Features beider Fragmente übernehmen (Insert-Features um `V.length()` verschieben); Metadaten `source = "clone.restriction"`.
5. Warnungen: „Insert-Orientierung nicht gerichtet (beide Orientierungen möglich)", wenn zwei Kandidaten für dasselbe (V,I)
   entstehen; „Insert enthält interne Schnittstelle von <Enzym>", wenn `digest` des Inserts mehr Fragmente liefert als erwartet;
   „Vektor-Rückligation möglich", wenn die beiden Vektor-Enden zueinander passen (`V.right.pairsWith(V.left)`).
6. Sortierung: gerichtete Kandidaten zuerst, dann nach Warnanzahl, dann nach Länge.

**Tests:**
- Vektor (zirkulär, 40 bp) `…GGATCC…GAATTC…` und Insert (linear, 20 bp) mit EcoRI-Stelle 5' und BamHI-Stelle 3', beide Enzyme: genau ein
  gerichteter Kandidat; Konstruktlänge = Vektorrückgrat + Insert-Innenfragment (von Hand zusammengezählt); beide Erkennungsstellen im Konstrukt
  wieder vorhanden (`indexOf("GAATTC")` und `indexOf("GGATCC")` ≥ 0).
- Nur EcoRI (ein Enzym): zwei Kandidaten (beide Orientierungen), Warnung „nicht gerichtet", Warnung „Rückligation".
- BamHI-Vektor, BglII-Insert: Kandidat vorhanden, Hybrid `GGATCT` bzw. `AGATCC` im Konstrukt (je nach Orientierung).
- Unverträgliche Enden (EcoRI-Vektor, BamHI-Insert nur): keine Kandidaten, leere Liste.
- Konstrukt hat Länge `V.length() + I.length()` (Summe der Fragmente).
- Features werden übernommen: Insert-Feature `[2,6)` erscheint im Konstrukt bei `V.length()+2 .. +6` (Vorwärtsorientierung).

---

## Aufgabe 2.3: Gibson / In-Fusion / SLIC

**Voraussetzung:** 2.0.

**Dateien:** `cloning/sim/OverlapAssembly.java`, `cloning/sim/OverlapAssemblyOperation.java` (`id = "clone.overlap"`), Tests.

```java
public record Overlap(int fromIndex, int toIndex, int length, double tm) {}

public final class OverlapAssembly {
    public OverlapAssembly(int minOverlap, int maxOverlap);       // Standard Gibson 15..40; In-Fusion 15..20; SLIC 25..80
    /** fragments in gewünschter Reihenfolge; circular = letztes verbindet sich wieder mit dem ersten. */
    public AssemblyResult assemble(List<SequenceDocument> fragments, boolean circular);
}
public record AssemblyResult(SequenceDocument construct, List<Overlap> overlaps, List<String> warnings) {}
```

**Algorithmus:**
1. Für jedes Paar `(Ai, Ai+1)` (bei `circular` auch `(An, A1)`) den **längsten** Overlap `k` mit `minOverlap ≤ k ≤ maxOverlap` suchen, für den
   gilt: die letzten `k` Basen von `Ai` sind identisch zu den ersten `k` Basen von `Ai+1`. Kein Treffer → `IllegalArgumentException`
   mit Angabe der Fragmentnamen.
2. Konstrukt (linear): `A1 + A2[k1..] + A3[k2..] + …`. Zirkulär: dasselbe, danach am Ende `An`-Ende mit Anfang überlappend: Der Overlap
   zwischen `An` und `A1` wird vom **Ende** abgeschnitten (`construct = A1 + A2[k1..] + … + An[kn-1..]` ohne die letzten `kn` Basen).
3. `tm` = Schmelztemperatur des Overlaps nach der Nearest-Neighbor-Formel aus Aufgabe 2.8 (`NearestNeighbor.tm`). Vorläufig bei fehlender
   Klasse einfache Wallace-Formel; später ersetzen.
4. Warnungen: Overlap-Tm < 48 °C; Overlap-Tm > 65 °C; Overlap zweimal in einem Fragment (nicht eindeutig); Overlap mit GC > 80 % oder < 20 %;
   mehrdeutiger Overlap (die Overlap-Sequenz kommt in einem anderen Fragment-Ende vor).
5. Features der Fragmente übernehmen (verschoben, Overlap-Bereich dedupliziert: Feature, das vollständig im abgeschnittenen Overlap-Teil
   von `Ai+1` lag, entfällt, wenn es identisch in `Ai` vorhanden ist).

**Design-Modus (Primer mit Überhängen):**

```java
public record GibsonPrimers(String forward, String reverse, int overlapLength) {}
public static GibsonPrimers designPrimers(SequenceDocument insert, int insertStart, int insertEnd,
                                          SequenceDocument vector, int vectorCutPosition, int overlapLength);
```

Forward-Primer = letzte `overlapLength` Basen links der Vektor-Schnittposition (Vektor-Top, `[cut−n,cut)`) + bindender Teil des Inserts
(erste Basen ab `insertStart`, Länge so gewählt, dass Tm (Aufgabe 2.8) im Bereich 58–62 °C; Länge 18–30). Reverse-Primer = Reverse-Komplement von
(letzte Basen des Inserts ab `insertEnd` rückwärts, passende Länge) mit Reverse-Komplement der ersten `overlapLength` Basen rechts der
Schnittposition **am 5'-Ende**. Standard `overlapLength = 25`. Hinweis im Kommentar: Beispiel mit Zahlen.

**Tests:**
- Fragmente `A = X + O1`, `B = O1 + Y + O2`, `C = O2 + Z` mit `O1 = "ACGTACGTACGTACGTACGT"` (20), `O2 = "TTGGCCAATTGGCCAATTGG"` (20), `X,Y,Z` je
  30 Basen aus `ATATAT…`/`CCGG…`: linear → Länge `30 + 20 + 30 + 20 + 30 = 130`; Overlaps `[A→B 20, B→C 20]`.
- Zirkulär mit zusätzlichem Overlap `O3` von `C` nach `A`: Länge = Summe der Fragmentlängen − 3×20.
- Fragmente ohne Overlap → `IllegalArgumentException`.
- Overlap 10 bp bei `minOverlap=15` wird nicht erkannt.
- `designPrimers`: Forward beginnt mit den `overlapLength` Basen links vom Vektorschnitt und endet mit den ersten Basen des Inserts;
  Länge des bindenden Teils 18–30; Reverse-Primer ist reverse-komplementär zum Insert-Ende (Prüfung: `rc(reverse)` endet mit
  Vektor-Overlap-Sequenz rechts).

---

## Aufgabe 2.4: Golden Gate

**Voraussetzung:** 2.1.

**Dateien:** `cloning/sim/GoldenGate.java`, `cloning/sim/GoldenGateOperation.java` (`id = "clone.goldengate"`), Tests.

```java
public record GoldenGateResult(SequenceDocument construct, List<Fragment> parts, List<String> warnings) {}

public final class GoldenGate {
    public GoldenGate(RestrictionEnzymeDB db, String enzymeName);   // BsaI, BsmBI, Esp3I, BbsI, BpiI, SapI, BspQI
    public GoldenGateResult assemble(List<SequenceDocument> parts, boolean circular);
}
```

**Algorithmus:**
1. Jedes Teil mit dem Typ-IIS-Enzym verdauen (`VirtualDigest`). **Innenfragment** = das Fragment, das von je einer Schnittstelle
   an beiden Seiten begrenzt ist (nicht das erste/letzte außen liegende). Teile mit weniger als zwei Schnittstellen → Warnung
   „Teil <Name> hat weniger als 2 Schnittstellen" und das Teil wird unverändert als Innenfragment mit stumpfen Enden verwendet.
   Hat ein **zirkulärer Vektor** zwei Schnittstellen: das Fragment, das die Erkennungsstellen **nicht** enthält, ist das Rückgrat.
2. Aus den Innenfragmenten über `canJoin` eine Kette bilden: starte bei einem Fragment, dessen linker Überhang von keinem rechten
   Überhang eines anderen Fragments gebraucht wird (linearer Fall) bzw. bei einem beliebigen (zirkulär); folge der eindeutigen Paarung.
3. Mehrdeutigkeiten → Warnung: gleicher Überhang an mehreren Enden; **palindromischer** Überhang (z. B. `GATC`, `AATT`), der sich selbst
   paart; Überhang, der zum Reverse-Komplement eines anderen passt (falsche Orientierung).
4. Innere Schnittstellen im Teil (mehr als zwei Schnitte) → Warnung „Interne Erkennungsstelle in <Name>".
5. Konstrukt mit `Ligation.join`. Nach dem Zusammenbau dürfen **keine** Erkennungsstellen des Enzyms mehr im Konstrukt vorhanden sein
   (Erkennungsstellen liegen in den ausgeschnittenen Randbereichen); sonst Warnung.
6. Nicht verbundene Fragmente → `IllegalArgumentException("Teile lassen sich nicht zu einer Kette verbinden: …")`.

**Test-Sequenzen (BsaI: `GGTCTC N^NNNN`, Erkennung vorwärts; Rückwärts: `GAGACC` mit Schnitt davor):**
Teil mit Rand: `"TT" + "GGTCTC" + "A" + OVH_L + BODY + OVH_R + "T" + "GAGACC" + "TT"` mit z. B.
`OVH_L="AATG"`, `OVH_R="GCTT"`, `BODY = "CCCCCCCCCCCC"` (12 C). Erwartung: Innenfragment `top = "AATG" + BODY` (Länge 16), `left = FIVE_PRIME "AATG"`,
`right = FIVE_PRIME "GCTT"` (Überhang in Top-Schreibweise: Bottom-Überhang der rechten Seite ist `GCTT`).
Ein zweites Teil mit linkem Überhang `GCTT` und rechtem Überhang `CGAT` (BODY `GGGGGGGGGGGG`) schließt an; ein drittes Teil `CGAT`→`AATG` schließt
zirkulär.
- Zirkuläres Ergebnis: Länge = `3 × (4 + 12)` = 48; enthält weder `GGTCTC` noch `GAGACC`.
- Zwei Teile mit identischem rechten Überhang `GCTT` → Warnung „doppelter Überhang".
- Teil mit internem `GGTCTC` → Warnung „Interne Erkennungsstelle".
- Palindromischer Überhang `GATC` → Warnung.
- Nicht verbindbare Teile → `IllegalArgumentException`.
- `Esp3I` (gleiche Erkennungssequenz wie `BsmBI`, `CGTCTC`) liefert dieselben Fragmente wie `BsmBI`.

---

## Aufgabe 2.5: Gateway, TOPO und TA/GC

**Voraussetzung:** 2.0.

**Dateien:** `cloning/sim/Gateway.java`, `cloning/sim/Topo.java`, `src/main/resources/cloning/gateway-sites.json`, Tests.

**Gateway (Modell):** Jede `att`-Stelle besteht aus *linkem Arm + Kern + rechtem Arm*: `attB = Bl + core + Br`, `attP = Pl + core + Pr`,
`attL = Bl + core + Pr`, `attR = Pl + core + Br`.
- **BP-Reaktion:** `attB × attP → attL + attR`. Eingabe: Produkt (PCR-Produkt mit `attB1…attB2` an den Enden, linear) + Donorvektor (zirkulär,
  `attP1…attP2` mit Selektionsgen dazwischen). Ergebnis: Entry-Klon (zirkulär) = Vektor-Rückgrat + Insert, begrenzt von `attL1` und `attL2`;
  Beiprodukt = Selektionsgen mit `attR1/attR2` (Warnung/zweites Dokument optional).
- **LR-Reaktion:** `attL × attR → attB + attP`. Entry-Klon (`attL1…attL2`) + Destination-Vektor (`attR1…attR2`) → Expressionsklon mit `attB1…attB2`.
- Die Rekombination erfolgt **innerhalb des Kerns**; `att1` rekombiniert nur mit `att1`, `att2` nur mit `att2`.
- Implementierung: die vier Sequenzen je `att1`/`att2` (`Bl, core, Br, Pl, Pr`) aus `gateway-sites.json` laden. **Die echten Sequenzen stehen in der
  öffentlichen Invitrogen-Dokumentation und sind hier nicht eingetragen**: die Datei enthält im Repository zunächst nur Platzhalter-Struktur
  und die Testdaten werden im Test selbst definiert (`GatewaySites.of(...)`). In `OFFENE_FRAGEN.md` vermerken, dass die echten Sequenzen einzutragen sind.

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
    /** TA-Klonierung: Insert (mit 3'-A-Überhängen vom PCR) wird in den linearen T-Vektor gesetzt, beide Orientierungen möglich. */
    public static List<SequenceDocument> taCloning(SequenceDocument tVector, SequenceDocument pcrProduct);
    /** Gerichtetes TOPO (CACC): Insert muss mit CACC beginnen; entfernt kein CACC; nur eine Orientierung. */
    public static SequenceDocument directionalTopo(SequenceDocument topoVector, SequenceDocument insertWithCacc);
}
```

`taCloning`: Konstrukt = `tVector.bases() + pcrProduct.bases()` (zirkulär) bzw. `tVector.bases() + rc(pcrProduct.bases())`; zwei Dokumente zurückgeben,
Namen mit „(Orientierung +)" / „(Orientierung −)". `directionalTopo`: `insert` muss mit `CACC` beginnen, sonst `IllegalArgumentException`;
Konstrukt = Vektor + Insert.

**Tests:** Mit selbst definierten Armen (z. B. `Bl="AAAA"`, `core="TTGT"`, `Br="CCCC"`, `Pl="GGGG"`, `Pr="ATAT"`): `attB="AAAATTGTCCCC"`,
`attP="GGGGTTGTATAT"`, `attL="AAAATTGTATAT"`, `attR="GGGGTTGTCCCC"`. BP: Produkt `X + attB1 + INS + attB2 + Y` + Donor `D1 + attP1 + SEL + attP2 + D2` → Entry-Klon
enthält `attL1 + INS + attL2` und **nicht** das `SEL`. LR liefert wieder `attB1 + INS + attB2`. Nicht vorhandene att-Stelle → `IllegalArgumentException`.
TOPO: Zusammenbau-Längen = Summe; `directionalTopo` ohne CACC → Exception.

---

## Aufgabe 2.6: Gel-Simulation

**Voraussetzung:** keine.

**Dateien:** `gel/GelSimulator.java`, `gel/Ladder.java`, `gel/Lane.java`, `gel/Band.java`, Tests.

```java
public record Band(int sizeBp, double mobility, double intensity) {}          // mobility 0 (oben) … 1 (unten)
public record Lane(String label, List<Band> bands) {}
public record Ladder(String name, List<Integer> sizes) {
    public static Ladder generic1kb();    // 250, 500, 750, 1000, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 8000, 10000 (generische Leiter, keine Herstellerangabe)
    public static Ladder generic100bp();  // 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1200, 1500
}
public final class GelSimulator {
    public GelSimulator(double agarosePercent);           // 0.5 … 3.0
    public double mobility(int sizeBp);                   // siehe Formel
    public Lane lane(String label, List<Integer> fragmentSizes, double massNg);   // Intensität proportional zu Größe/Summe
    public Lane ladderLane(Ladder ladder);
}
```

**Formel:** Auflösungsbereich `[Smin, Smax]` aus Tabelle (log-linear zwischen den Stützstellen interpoliert):

| Agarose % | Smin (bp) | Smax (bp) |
|---|---|---|
| 0.7 | 500 | 15000 |
| 1.0 | 300 | 10000 |
| 1.5 | 150 | 5000 |
| 2.0 | 100 | 3000 |

`mobility(size) = clamp((log10(Smax) − log10(size)) / (log10(Smax) − log10(Smin)), 0, 1)` (größere Fragmente wandern weniger).
Intensität eines Fragments = `massNg * size / sum(sizes)`, normiert so, dass die maximale Intensität einer Spur 1.0 ist.

**Tests:** Bei 1,0 %: `mobility(10000)=0.0`, `mobility(300)=1.0`, `mobility(1732)≈0.5` (±0.01; `sqrt(300·10000)=1732.05`).
Monotonie: `mobility(500) > mobility(1000) > mobility(5000)` (kleinere Fragmente wandern weiter). Bei 1,5 % ist der Bereich enger: `mobility(1000)` bei 1,5 % ist größer als bei 1,0 %.
Größen außerhalb des Bereichs werden geklemmt (`mobility(50)=1.0`
bei 1,0 %). Spur mit zwei gleichen Fragmenten: eine Bande mit doppelter Intensität (Banden gleicher Größe werden zusammengefasst).
Zwischenwerte der Tabelle (z. B. 1,25 %) liegen zwischen den Nachbarwerten.

---

## Aufgabe 2.7: PCR-Simulation

**Voraussetzung:** 2.0.

**Dateien:** `cloning/sim/PcrSimulator.java`, `cloning/sim/PcrOperation.java` (`id = "pcr.simulate"`), Tests.

```java
public record PrimerBinding(String primerName, int start, int end, boolean forward, int mismatches, int tailLength) {}
public record PcrProduct(String sequence, int templateStart, int templateEnd, PrimerBinding forward, PrimerBinding reverse) {}
public record Primer(String name, String sequence) {}

public final class PcrSimulator {
    public PcrSimulator(int minThreePrimeMatch, int maxMismatches, int maxProductLength);   // Standard 12, 2, 10000
    public List<PrimerBinding> findBindings(SequenceDocument template, Primer primer);
    public List<PcrProduct> simulate(SequenceDocument template, List<Primer> primers);
}
```

**Regeln:**
- Ein Primer bindet an einer Stelle, wenn die **letzten `minThreePrimeMatch` Basen am 3'-Ende exakt** passen und der restliche (5'-seitige) Teil des
  Primers, soweit er an die Vorlage reicht, höchstens `maxMismatches` Abweichungen hat. Der 5'-Teil, der **nicht** mehr an die Vorlage passt
  (Primer ragt über das Template hinaus oder weicht über `maxMismatches`), zählt als Schwanz (`tailLength`) nur dann, wenn er **vollständig am 5'-Ende** liegt:
  bestimme die längste Folge ab dem 3'-Ende rückwärts, in der Abweichungen ≤ `maxMismatches` sind; alles davor ist Schwanz.
- Vorwärtsprimer bindet an Top-Strang (Primersequenz entspricht Top-Folge), Rückwärtsprimer bindet an Bottom-Strang (Reverse-Komplement der Primersequenz kommt in Top vor).
- Produkt: Vorwärtsbindung links, Rückwärtsbindung rechts davon (bei zirkulärer Vorlage auch über den Nullpunkt). Produktsequenz =
  `forwardPrimer (komplett, inkl. Schwanz)` + Template-Bereich zwischen den Primer-Enden + `rc(reversePrimer komplett)`.
- Produkte länger als `maxProductLength` verwerfen. Primerpaare, bei denen beide Primer dieselbe Orientierung haben, ergeben kein Produkt.
- Ergebnis sortiert nach Länge.

**Tests (Template als `ATGCCGTAGCTAGCATCGATCGGATCCGATCGATCGTAGCTAGCTAGGCTAGC`, Länge 53 — im Test per `bases.len()` prüfen):**
- Forward `ATGCCGTAGCTAGC` (Position 0), Reverse = Reverse-Komplement der letzten 14 Basen → Produkt hat die Länge des Templates (53), Sequenz gleich dem Template.
- Forward mit 5'-Schwanz `GAATTC` + 14 bindende Basen → Produktlänge 53 + 6 = 59, beginnt mit `GAATTC`; `tailLength == 6`.
- Primer mit Abweichung im 3'-Ende (letzte Base falsch) → keine Bindung.
- Primer mit einer Abweichung im 5'-Bereich, `maxMismatches=1` → Bindung mit `mismatches == 1`.
- Zwei Vorwärtsprimer → kein Produkt.
- Zirkuläre Vorlage: Primer auf beiden Seiten des Nullpunkts → Produkt läuft über den Nullpunkt, Länge korrekt.

---

## Aufgabe 2.8: Schmelztemperatur (Nearest-Neighbor) und Primer-Bewertung

**Voraussetzung:** keine.

**Dateien:** `primer/NearestNeighbor.java`, `primer/PrimerChecks.java`, Tests. (Neben der vorhandenen `PrimerDesign`-Klasse.)

```java
public final class NearestNeighbor {
    /** Tm in °C. Konzentrationen: oligoNanoMolar (z. B. 250), naMilliMolar (z. B. 50), Mg ignorieren. */
    public static double tm(String sequence, double oligoNanoMolar, double naMilliMolar);
    public static double deltaGKcal(String sequence, double temperatureCelsius, double naMilliMolar);   // optional, für Dimer
}
```

**Parameter (SantaLucia 1998, Unified; Quelle in Javadoc nennen: SantaLucia J Jr., PNAS 95:1460–1465, 1998):**

| Dinukleotid (5'→3') | ΔH (kcal/mol) | ΔS (cal/mol·K) |
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

Initiation: pro Ende mit G·C-Paar ΔH +0.1, ΔS −2.8; pro Ende mit A·T-Paar ΔH +2.3, ΔS +4.1. Symmetriekorrektur bei selbstkomplementären
Sequenzen: ΔS −1.4. Salz: `ΔS_Na = ΔS + 0.368 · (N − 1) · ln([Na+] in mol/l)` mit `N` = Anzahl der Phosphate = Länge. Ergebnis:

`Tm = ΔH · 1000 / (ΔS_Na + R · ln(CT / x)) − 273.15` mit `R = 1.987`, `CT` = Oligokonzentration in mol/l, `x = 4` (nicht selbstkomplementär) bzw. `1`
(selbstkomplementär). Einheiten beachten: ΔH in kcal → ×1000 für cal.

**Tests (Eigenschaften, keine fremden Referenzzahlen):**
- Monotonie: Verlängern einer Sequenz erhöht `tm` (z. B. 15-mer < 20-mer des gleichen GC-Anteils).
- Mehr GC bei gleicher Länge → höhere Tm (`GCGCGCGCGCGCGCGCGCGC` > `ATATATATATATATATATAT`, 20 nt).
- Eine 20-mer-Sequenz mit 50 % GC (z. B. `ATGCGTACCTAGGATCCTAG`) liegt bei 250 nM / 50 mM Na⁺ zwischen 50 und 62 °C.
- Höhere Salzkonzentration → höhere Tm.
- Symmetrie: `tm(s) == tm(reverseComplement(s))` (±0.01).
- Zu kurze Sequenz (< 2 Basen) → `IllegalArgumentException`.

`PrimerChecks`:
- `int maxHomopolymerRun(String)`, `boolean hasGcClamp(String)` (letzte 2 Basen enthalten G/C, höchstens 3 G/C in den letzten 5),
  `double hairpinScore(String)` (größter Stamm ≥ 4 bp, Schleife ≥ 3: Länge des Stamms als Score), `double selfDimerScore(String, String)`
  (längster komplementärer Abschnitt zwischen Sequenz A und Reverse-Komplement von B, 3'-Ende doppelt gewichtet).
- Tests: `maxHomopolymerRun("AAAAGC")=4`; `hasGcClamp("ATATATGC")` wahr, `hasGcClamp("ATATATAT")` falsch;
  `hairpinScore("GGATCCAAAAGGATCC")` ≥ 6; `selfDimerScore("GGATCC","GGATCC")` ≥ 6 (palindromisch).

---

## Aufgabe 2.9: Primer-Design

**Voraussetzung:** 2.8.

**Dateien:** `primer/PrimerPicker.java`, `primer/PrimerConstraints.java`, `primer/PrimerPair.java` (neu, **anderer Name** als die vorhandene
innere Klasse: `PickedPair`), Tests.

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
    /** Bestes Paar, das den Zielbereich [targetStart, targetEnd) einschließt. */
    public List<PickedPair> pickPairs(String template, int targetStart, int targetEnd, int maxResults);
    /** Primer mit festem 5'-Ende (z. B. Klonierung): Vorwärts ab Position, Länge frei. */
    public Optional<PickedPrimer> pickForwardAt(String template, int start);
    public Optional<PickedPrimer> pickReverseEndingAt(String template, int endExclusive);
}
```

**Algorithmus:**
- Kandidaten Vorwärtsprimer: Startpositionen in `[max(0, targetStart − maxProduct+…) , targetStart]` (mindestens so, dass das Ziel im Produkt liegt) mit
  Längen `minLength…maxLength`; Rückwärtsprimer symmetrisch rechts von `targetEnd`.
- Ein Kandidat ist gültig, wenn Tm, GC, GC-Klammer (falls verlangt), Homopolymer ≤ 4, Hairpin-Score < 6, Selbstdimer-Score < 7 erfüllt sind.
- Strafe (kleiner ist besser): `|tm − optTm| + 5·|gc − 0.5| + 2·hairpinScore/10 + 2·selfDimerScore/10`.
- Paare: `|Tm_f − Tm_r| ≤ maxTmDifference`, Produktgröße in `[minProduct, maxProduct]`, Kreuz-Dimer-Score `< 7`. Paar-Strafe = Summe der Primerstrafen + `|Tm_f − Tm_r|`.
- Rückgabe sortiert nach Strafe, höchstens `maxResults`.

**Tests:** Template aus 400 Basen, im Test deterministisch erzeugt (`new Random(42)`, gleichverteilt `ACGT`) mit Zielbereich `[150,250)`:
- Es gibt ≥ 1 Paar; beide Primer erfüllen die Randbedingungen (Länge, Tm-Bereich, GC); Produktgröße ≥ 100; Ziel liegt im Produkt;
  Vorwärtsprimersequenz ist **exakt** `template.substring(start,end)`, Rückwärtsprimer ist das Reverse-Komplement der Template-Unterfolge.
- `pickForwardAt(template, 100)` liefert einen Primer, der bei Position 100 beginnt.
- Extrem GC-reiche Vorlage (nur `G`/`C`) → kein Paar (leere Liste), kein Fehler.
- Template kürzer als `minLength` → leere Liste.

---

## Aufgabe 2.10: Klonierungsprimer (Erweiterungen)

**Voraussetzung:** 2.9, 2.3.

**Dateien:** `primer/CloningPrimers.java`, Test.

```java
public final class CloningPrimers {
    /** Restriktionsklonierung: Schwanz = Klammer + Erkennungsstelle (+ optional Linker) vor dem bindenden Teil. */
    public static PickedPair withRestrictionSites(String template, int start, int end, PrimerPicker picker,
                                                  RestrictionEnzyme forwardEnzyme, RestrictionEnzyme reverseEnzyme,
                                                  String clamp);     // clamp z. B. "GCGC" (4-6 Basen)
}
```

Vorwärts: `clamp + forwardEnzyme.getRecognitionSequence() + bindendeBasen(ab start)`; Rückwärts: `clamp + reverseEnzyme.getRecognitionSequence() + rc(bindendeBasen)`.
Enzyme mit mehrdeutiger Erkennungssequenz (Zeichen außer ACGT) → `IllegalArgumentException`. Warnung als Ausnahme „Enzym schneidet intern" lieferst du im Rückgabeobjekt:
`record CloningPrimerResult(PickedPair pair, List<String> warnings)`; ergänze die Warnung, wenn das Insert-Fragment (Template ab `start` bis `end`) die Erkennungssequenz enthält.

**Tests:** EcoRI/BamHI-Primer für ein 200-bp-Fragment ohne diese Stellen: Vorwärtsprimer beginnt mit `GCGCGAATTC`, Rückwärtsprimer mit `GCGCGGATCC`; Fragment mit
innerer EcoRI-Stelle → Warnung. Verdauen des gedachten PCR-Produkts (`PcrSimulator` mit diesen Primern) liefert ein Innenfragment mit EcoRI-/BamHI-Enden (Integration von 2.7, 2.1).

---

## Aufgabe 2.11: Codon-Optimierung

**Voraussetzung:** keine (nutzt `Sequence.aminoAcidFor` aus Phase 1.4).

**Dateien:** `codon/CodonTable.java`, `codon/CodonOptimizer.java`, Ressource `src/main/resources/codon/<organismus>.json`, Tests.

```java
public final class CodonTable {
    public static CodonTable fromJson(String json);                 // { "organism": "...", "codons": { "ATG": 1.0, "GCT": 0.18, ... } } (relative Häufigkeit je Aminosäure oder Pro-Tausend)
    public static CodonTable load(String resourceName);              // z. B. "ecoli-k12"
    public double frequency(String codon);
    public List<String> codonsFor(char aminoAcid);                   // nach Häufigkeit absteigend
}
public enum Strategy { MOST_FREQUENT, HARMONIZED }
public record OptimizationOptions(Strategy strategy, Set<String> avoidRecognitionSites, int maxHomopolymer,
                                  double minGc, double maxGc, long seed) {
    public static OptimizationOptions standard();    // MOST_FREQUENT, leer, 6, 0.35, 0.65, 1
}
public final class CodonOptimizer {
    public CodonOptimizer(CodonTable table);
    public String optimize(String proteinSequence, OptimizationOptions options);     // liefert DNA (ohne Stop; Stop '*' → bestes Stopcodon)
    public String optimizeDna(String codingDna, OptimizationOptions options);        // Protein behalten, Codons ersetzen
}
```

**Algorithmus:**
1. `MOST_FREQUENT`: für jede Aminosäure das häufigste Codon. `HARMONIZED`: Codon zufällig (deterministisch mit `seed`) nach Häufigkeitsverteilung wählen.
2. Danach Korrekturschritte in fester Reihenfolge: (a) Wenn eine verbotene Erkennungsstelle (IUPAC-frei, nur ACGT-Sequenzen) im Ergebnis vorkommt, eines der beteiligten Codons durch
   das nächsthäufigste synonyme Codon ersetzen, das die Stelle entfernt, ohne eine neue zu erzeugen. (b) Homopolymer-Läufe länger als `maxHomopolymer`
   brechen: ein Codon im Lauf ersetzen. (c) Globaler GC-Anteil außerhalb von `[minGc, maxGc]`: wiederholt das Codon mit der geringsten Häufigkeitsänderung ersetzen,
   das den GC-Anteil in die gewünschte Richtung bewegt, bis er im Fenster liegt oder keine Änderung mehr möglich ist.
3. Das Ergebnis muss **immer dasselbe Protein** codieren (Test!), unabhängig von den Optionen.

**Ressourcen:** Lege `src/main/resources/codon/test-table.json` mit einer **kleinen synthetischen** Tabelle an (für Tests). Echte Tabellen
(E. coli K-12, Homo sapiens, S. cerevisiae) kommen aus der öffentlichen Kazusa-Datenbank (`https://www.kazusa.or.jp/codon/`); bitte in `OFFENE_FRAGEN.md` notieren, dass diese Dateien aus
Kazusa zu erzeugen sind, und ein kleines Hilfsprogramm `tools/ImportKazusa.java` schreiben, das die Textdarstellung der Seite in das JSON-Format umwandelt (Test mit einem
selbst geschriebenen Mini-Beispiel im Kazusa-Textformat).

**Tests (synthetische Tabelle: pro Aminosäure ein „bestes" Codon mit Häufigkeit 0.6 und ein zweites mit 0.4):**
- `optimize("MKL*", MOST_FREQUENT)` ergibt das jeweils beste Codon; Rückübersetzung (`Sequence.translate`) ergibt `MKL*`.
- Mit `avoidRecognitionSites={"GAATTC"}`: Das Ergebnis enthält `GAATTC` nicht, Protein unverändert (Eingabeprotein so wählen, dass die beste Wahl die Stelle erzeugen würde).
- Homopolymer: Eingabe `KKKKKK` (AAA bester) mit `maxHomopolymer=6` → Ergebnis enthält keinen Lauf `AAAAAAA` (7).
- Deterministisch: zweimal mit gleichem `seed` gleiches Ergebnis; `HARMONIZED` hat bei 1000 Aminosäuren `L` ungefähr 60/40-Verteilung (±8 %).
- Leeres Protein → leerer String; Ungültiges Zeichen → `IllegalArgumentException`.

---

## Aufgabe 2.12: CRISPR-gRNA-Suche

**Voraussetzung:** keine.

**Dateien:** `crispr/GuideFinder.java`, `crispr/Guide.java`, `crispr/OffTargetScanner.java`, Tests.

```java
public record Guide(String protospacer, String pam, int start, int end, boolean forward, double gc, double score, List<String> warnings) {}
public record PamSpec(String name, String pattern, int guideLength, boolean pamThreePrime) {
    public static PamSpec spCas9();   // "NGG", 20 nt, PAM am 3'-Ende
    public static PamSpec saCas9();   // "NNGRRT", 21 nt, PAM am 3'-Ende
    public static PamSpec cas12a();   // "TTTV", 23 nt, PAM am 5'-Ende
}
public final class GuideFinder {
    public GuideFinder(PamSpec spec);
    public List<Guide> find(String sequence, int from, int to, boolean circular);   // beide Stränge im Bereich
}
public final class OffTargetScanner {
    public static int countSites(String reference, Guide guide, PamSpec spec, int maxMismatches);   // Treffer außer dem Guide selbst
}
```

**Regeln:** Für jede Position, an der das Muster (IUPAC-Codes: `N` alle, `R` A/G, `V` A/C/G) passt, wird die Guide-Sequenz benachbart gelesen
(SpCas9: 20 Basen direkt 5' der PAM). Auf dem Gegenstrang dasselbe mit Reverse-Komplement; `start/end` beziehen sich immer auf Top-Koordinaten,
`forward=false` markiert den Gegenstrang. Zirkuläre Sequenzen: Suche über den Nullpunkt. Guides am Rand (zu kurz) entfallen.
**Score** (0–100, bewusst einfach, im Javadoc dokumentieren): Start 100; −20 wenn GC-Anteil < 40 % oder > 70 %; −30 wenn `TTTT` im Protospacer
vorkommt (Terminationssignal der U6-Polymerase); −10 wenn der Protospacer einen Homopolymer-Lauf ≥ 5 enthält. Zu jeder abgezogenen Regel eine Warnung in Klartext.
Kein weiteres Bewertungsmodell erfinden.

**Tests:** `find("AAAAAAAAAAAAAAAAAAAATGGCCC…")`: eine Sequenz, die genau eine bekannte NGG-PAM (`TGG`) mit vorangehenden 20 Basen enthält (im Test definiert) →
Guide mit `protospacer` = diese 20 Basen, `pam="TGG"`, `forward=true`. Dieselbe Sequenz reverse-komplementiert → `forward=false`, `start/end` entsprechen den Top-Koordinaten.
Guide mit `TTTT` → Score ≤ 70 und Warnung. `countSites` in einer Referenz, die den Guide zweimal enthält (einmal exakt, einmal mit 2 Mismatches) und `maxMismatches=3` →
Rückgabe 2 (beide); mit `maxMismatches=1` → 1.

---

## Aufgabe 2.13: Sequenzierungsabgleich („Align to Reference")

**Voraussetzung:** 2.0, Phase 1.10 (AB1), 2.1.

**Dateien:** `seqcheck/SequencingCheck.java`, `seqcheck/Difference.java`, Tests.

```java
public enum DifferenceType { MISMATCH, INSERTION, DELETION, LOW_QUALITY }
public record Difference(DifferenceType type, int referencePosition, int readPosition, String expected, String observed, int quality) {}
public record CheckResult(String readName, boolean reversed, int referenceStart, int referenceEnd,
                          double identity, List<Difference> differences, String alignedRead, String alignedReference) {}

public final class SequencingCheck {
    public SequencingCheck(int minQuality);                                              // Standard 20
    public CheckResult check(SequenceDocument reference, String readName, String readBases, int[] quality);
    public CheckResult check(SequenceDocument reference, Chromatogram read);
}
```

**Ablauf:**
1. Read vorwärts und reverse-komplementär mit `Alignment.semiGlobal` gegen die Referenz ausrichten; die Variante mit dem höheren Score gewinnt (`reversed`).
   Bei zirkulärer Referenz: Referenz mit sich selbst verdoppelt (`ref + ref`) verwenden und Positionen modulo Länge zurückrechnen.
2. Aus dem Alignment die Differenzen bestimmen; Basen mit `quality < minQuality` werden zusätzlich als `LOW_QUALITY` gemeldet (auch wenn sie passen), die Qualität in der Rückwärtsrichtung wird mitgedreht.
3. `identity` = Matches / ausgerichtete Spalten (ohne Randlücken) in Prozent.

**Tests:** Referenz `ACGTACGTACGTACGTACGTACGTACGT` (28), Read = Referenz-Teilstück ab 4 Länge 16 mit einer Substitution an Read-Position 8 (`C`→`T`): `MISMATCH` an
`referencePosition 12`, `identity` = 15/16 · 100 = 93.75. Read reverse-komplementiert → `reversed=true`, gleiche Differenz. Insertion einer Base → `INSERTION`. Löschung einer Base → `DELETION`.
Qualität 10 an Position 3 → `LOW_QUALITY` dort. Referenz zirkulär, Read über den Nullpunkt → korrekt ausgerichtet.

---

## Aufgabe 2.14: Operationen und Oberfläche (Klonierung)

**Voraussetzung:** 2.1–2.13, 1.11.

**Operationen** (`Operation`-Klassen mit IDs, registriert in einer `OperationRegistry`): `clone.restriction`, `clone.overlap`, `clone.goldengate`,
`clone.gateway.bp`, `clone.gateway.lr`, `pcr.simulate`, `primer.pick`, `crispr.find`, `codon.optimize`, `seq.check`, `digest.virtual`. Jede Operation
parst ihre `Map<String,String>`-Parameter robust (Standardwerte, Fehler → `OperationException` mit deutscher Meldung) — mit je einem Test für Standardfall und
fehlerhaften Parameter.

**Oberfläche (`fx/`, mit Presenter-Klassen und Tests für die Presenter):**
- **Verdau-Ansicht:** Enzym-Auswahl (Suche), Fragmentliste (Länge, Enden), Gel-Vorschau (`GelSimulator`) rechts daneben, Leiterwahl.
- **Klonierungs-Assistent:** Schritte Vektor wählen → Insert wählen → Methode wählen → Parameter → Kandidaten-Liste mit Warnungen → Konstrukt speichern (als neues Dokument im Projekt, Verlaufseintrag).
- **PCR/Primer-Dialog:** Template + Zielbereich → `PrimerPicker`-Vorschläge; Primer-Paare auf der Karte als Pfeile (`primer_bind`-Features) einzeichnen; „PCR simulieren" zeigt Produkt und Gel.
- **Sequenzierung:** Read-Dateien (AB1/FASTA) wählen → `SequencingCheck`; Differenzen als Liste und in der Sequenzansicht markiert.
- Jedes erzeugte Konstrukt ist ein `SequenceDocument` mit Verlauf (`HistoryEntry`).

**Presenter-Tests:** Für jeden Presenter die Zustandsübergänge ohne Fenster prüfen (z. B. Klonierungs-Assistent: nach Auswahl von Vektor und Insert und Enzymen enthält die Kandidatenliste die
erwartete Anzahl).

---

## Gesamtabnahme Phase 2

Szenario (als Integrationstest `ClonePipelineIT` automatisieren, mit `puc19.gb` aus den Testressourcen):
1. pUC19 laden (zirkulär, 2686 bp), mit EcoRI + HindIII verdauen: zwei Fragmente (kleines MCS-Fragment, großes Rückgrat); Summe der Längen = 2686.
2. Ein synthetisches 300-bp-Insert mit EcoRI-/HindIII-Enden erzeugen (im Test), per `RestrictionCloning` in das Rückgrat klonieren: genau ein gerichteter Kandidat,
   Konstruktlänge = Rückgrat + Insert-Innenfragment.
3. Primer für eine Kontroll-PCR über das Insert mit `PrimerPicker` entwerfen, `PcrSimulator` liefert genau ein Produkt, dessen Größe ≥ Insertlänge ist.
4. Gel für Verdau und PCR-Produkt simulieren: Bandenpositionen streng monoton in der Größe.
5. Konstrukt als GenBank speichern, neu laden, identisch.
