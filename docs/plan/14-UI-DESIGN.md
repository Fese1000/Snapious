# Phase 7 – Oberfläche: eigener Look, leichtere Bedienung

Ausgangslage: Die App nutzt Standard-SwiftUI (NavigationSplitView, Form, TabView, Toolbar mit ~9 gleichwertigen Knöpfen,
Systemfarben, 116 verstreute `Color`/`.font`-Stellen, kein gemeinsames Design). Ziel: ein erkennbarer eigener Stil und
weniger Klicks bis zum Ergebnis. Rust-Kern bleibt unberührt; alles passiert in `app/Sources/SnapiousApp`.

## Leitlinien
1. **Ein Design-System statt Streufarben**: alle Farben, Abstände, Schriften, Radien kommen aus einer Datei (`Theme.swift`).
2. **Aufgabe vor Funktion**: Der Nutzer wählt, was er tun will („Klonieren“, „Primer“, „Sequenzieren prüfen“), nicht Menüs/Tabs.
3. **Daten sind die Hauptfigur**: Sequenz, Karte, Gel und Baum bekommen den meisten Platz; Bedienelemente treten zurück.
4. **Ruhig, dicht, dunkel-tauglich**: Hell- und Dunkelmodus gleichwertig, Wissenschaft-Look (wie Figuren in Papers), keine Apple-Standardgrau-Flächen.
5. **Immer ein Weg zurück**: Rückgängig, Verlauf, kein modaler Dialog ohne Abbrechen.

## Schritt 1 – Design-System (`Theme.swift`, `Components.swift`)
- Palette: Basis Tinten-Dunkelblau/Papierweiß, ein kräftiger Akzent (Petrol/Türkis), Zweitakzent Bernstein; feste
  DNA-Basenfarben (A grün, C blau, G bernstein, T rot) und Feature-Farben für Gene/Promotoren/Marker, kontrastgeprüft.
- Schrift: Monospace-Sequenzschrift (SF Mono → optional gebündelte JetBrains Mono), Überschriften mit `.rounded`/Serif-Akzent.
- Bausteine: `Card`, `SectionHeader`, `PillButton`, `IconButton`, `Chip`/`Tag`, `StatTile`, `EmptyState`, `Toast`, `Badge`.
- Alle Views auf diese Bausteine umstellen, keine direkten `Color.accentColor`/`.font(.caption)` mehr (per Skript prüfen).

## Schritt 2 – Gerüst und Navigation
- Eigene Seitenleiste statt Systemliste: Projekt-Kopf, Suchfeld oben, Ordner/Favoriten/zuletzt benutzt, Typ-Symbole mit Farbe,
  Vorschau-Minikarte je Sequenz.
- Toolbar aufräumen: nur *Importieren*, *Suchen (⌘K)*, *Rückgängig*; alles andere wandert in die **Befehlspalette**.
- **Befehlspalette ⌘K**: alle 29 Operationen, Ansichten, Dokumente, Einstellungen per Tippen finden und ausführen.
- Tabs durch **Aufgaben-Leiste** ersetzen: Gruppen *Ansehen* (Sequenz, Karte), *Bauen* (Verdau, Klonieren, Primer, Guides),
  *Prüfen* (Reads, Analyse, Protein, RNA), *Vergleichen* (Alignment, Baum), *Sequenzdaten* (NGS), *Werkzeuge*.
- Rechte **Inspektor-Seitenleiste** (ein-/ausblendbar) für Auswahl, Features, Notizen, Versionen statt eigener Dialoge.

## Schritt 3 – Sequenz- und Kartenansicht (Herzstück)
- Sequenz: Basenfarben, Codonraster, Übersetzungszeilen an/aus, Lineal, sanfte Hover-/Auswahlhervorhebung, Minimap-Übersicht.
- Feature-Spuren als abgerundete Pfeile mit Beschriftung, Überlappungen auf Spuren verteilt, Klick = Inspektor.
- Karte: Plasmid-Ring mit Verlaufsfarben, Enzym-Etiketten ohne Überlappung (Kollisionsvermeidung), Zoom per Pinch,
  Animationen beim Wechsel Ring↔Linear.
- Gel, Baum, Heatmap, Logo, Coverage im selben Stil; einheitlicher **Export als PNG/SVG** direkt am Diagramm.

## Schritt 4 – Geführte Abläufe
- **Startseite** statt leerem Fenster: große Karten „Projekt öffnen“, „Aus NCBI holen“, „Datei hereinziehen“, Beispielprojekt, zuletzt benutzt.
- **Assistenten** für Standardaufgaben (Schritt-Leiste oben, Vorschau live): Klonierung (Vektor→Insert→Enzyme→Ergebnis),
  Primerdesign, Sequenzierung prüfen, Variantensuche aus Reads.
- Leerzustände mit Hinweis und Knopf („Noch keine Primer – Primer entwerfen“), verständliche Fehlermeldungen mit Lösungsvorschlag.
- Erstes-Mal-Rundgang (3–4 Sprechblasen) und Tooltips mit Tastenkürzel.

## Schritt 5 – Rückmeldung und Feinschliff
- Fortschritt/Abbrechen für lange Aufgaben (BLAST, Mapping, Alignment) als Toast/Leiste unten, nicht als Sperrdialog.
- Rückgängig/Wiederholen für alle Änderungen (auf Basis der Versionen).
- Tastaturkürzel-Übersicht (⌘/), Drag-and-drop-Ziele sichtbar, Mehrfachauswahl-Aktionsleiste.
- Animationen kurz (150–250 ms), `Reduce Motion` respektieren; VoiceOver-Beschriftungen, Mindestkontrast 4,5:1, dynamische Schriftgröße.
- Fenster-Größen: Layout von 900 px bis Vollbild getestet (Snapshot-Tests je Breite).

## Reihenfolge und Aufwand
| # | Paket | Ergebnis sichtbar | Aufwand |
|---|-------|-------------------|---------|
| 1 | Theme + Bausteine, ContentView/Seitenleiste umgestellt | App sieht anders aus | mittel |
| 2 | Befehlspalette + aufgeräumte Toolbar + Aufgaben-Leiste | weniger Klicks | mittel |
| 3 | Sequenz-/Kartenansicht neu | größter Qualitätssprung | groß |
| 4 | Startseite + Assistenten + Leerzustände | Einstieg leicht | mittel |
| 5 | Inspektor, Toasts, Undo, Barrierefreiheit | Feinschliff | mittel |

## Prüfen
- Snapshot-Modus (`Snapious --snapshot`) um alle neuen Ansichten in Hell/Dunkel und 3 Breiten erweitern, Ergebnis jeweils ansehen.
- SelfTest bleibt grün; neue reine Logik (Befehlssuche, Kollisionsvermeidung Etiketten, Farbzuordnung) mit Swift-Tests.
- Live-Check per Computer-Use nach jedem Paket; Rust-Tests unverändert grün.

## Arbeitsteilung
Layout- und Designarbeit macht Claude selbst (visuelles Urteil nötig). Qwen bekommt nur reine Logik als Karte:
Befehlssuche (Fuzzy-Match), Etiketten-Kollisionsvermeidung, Spur-Zuteilung überlappender Features.
