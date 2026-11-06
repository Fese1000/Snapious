# Offene Fragen und Annahmen

Dieses Dokument sammelt alles, was beim Planen unklar war. Die Fragen wurden **nicht** einzeln gestellt, sondern hier gebündelt.
Die gewählten Annahmen gelten, bis sie beantwortet sind. Wer die Pläne umsetzt, trägt neue Unklarheiten hier ein (Aufgabe, Frage, gewählte Annahme).

## Beantwortete Fragen (Felix, 25.10.2026)

1. **Qwen:** Qwen3.6-35B-A3B (MoE, ca. 3 Mrd. aktive Parameter), lokal über Ollama/LM Studio, Kontextfenster **64k oder mehr**; Arbeitsweise in `09-QWEN-ARBEITSWEISE.md`. → Eine Aufgabe samt `00-ARBEITSANWEISUNG.md` und den betroffenen Quelldateien passt in den Kontext;
   pro Sitzung höchstens eine Aufgabe (bzw. zwei kleine) bearbeiten.
2. **UI/Architektur:** zuerst JavaFX, dann Hybrid (Java + Rust-Prozess), **endgültig: Rust-Kern + native Swift-App (AppKit/SwiftUI)** für macOS. Eine zweite Oberfläche für Windows/Linux (Tauri oder egui) ist später möglich und ändert den Kern nicht. Siehe `08-RUST-KERN-SWIFT-APP.md`.
3. **Phase 5 (NGS):** **zurückgestellt**, wird erst später entschieden. Die Spezifikation `06-PHASE-5-NGS.md` bleibt als Vorrat erhalten; Phase 6 darf **nicht** davon abhängen
   (Operationen aus Phase 5 werden in `OperationRegistry.withBuiltins` nur registriert, wenn sie existieren).
4. **Plattform:** nur macOS (Apple Silicon).
5. **Downloads:** erlaubt (pUC19, Codon-Tabellen, Gateway-att-Sequenzen, 3Dmol.js); Herkunft dokumentieren.
6. **Phase 0:** setzt Claude um; Qwen beginnt mit Phase 1.
7. **Sprache:** Deutsch (Annahme, nicht widersprochen).
8. **Externe Programme:** alle installiert (Homebrew): MAFFT 7.526, BLAST+ 2.17.0, Primer3 2.6.1, IQ-TREE 3.1.4 (`iqtree3`), MUSCLE 5.3, Clustal Omega 1.2.4, FastTree 2.1.11, ViennaRNA 2.7.0 (die letzten vier über den Tap `brewsci/bio`). Rust 1.99.1 (Homebrew) ist ebenfalls installiert.

## Offene Punkte nach der Swift-Entscheidung

- **Xcode:** wird von Felix gerade installiert (nützlich für Vorschau, Instruments, Signieren). Gebaut werden kann auch mit den Command Line Tools (Swift 6.4, AppKit, SwiftUI, Metal vorhanden).
- **Wer macht was:** Kern-Aufgaben (Rust) Qwen, außer Phase 0 und 0.9 (Claude); alle Swift-Aufgaben (S.x) Claude. *Annahme:* so.
- **Java-Projekt:** bleibt als Referenz im Ordner `src/`; wird nach Abschluss von 0.9 nicht mehr gepflegt. *Annahme:* nicht löschen.
- **Zweite Oberfläche (Windows/Linux):** Tauri oder egui, später zu entscheiden.

## Offen nach der Prüfung der Anleitungen

- **Pilotlauf:** Karte 1.1 (siehe `09-QWEN-ARBEITSWEISE.md`) einmal mit Qwen durchspielen, bevor weitere Karten vergeben werden. Danach Kartengröße anpassen.
- **Anleitungen `01`–`07` sind noch in Java-Schreibweise** und werden **kartenweise** in Rust-Karten mit Gerüst und Tests umgesetzt (Claude, eine Karte im Voraus). Das ist bewusst nicht vorab geschehen, weil Gerüste und Tests geprüft kompilieren müssen.
- **Qwen-Verhalten ungeprüft:** Aussagen zur Eignung des Modells sind Einschätzungen, keine Messungen.

## Bekannte Lücken in den Spezifikationen (bewusst offen gelassen)

- **Gateway-Sequenzen** (`gateway-sites.json`): echte Arm-/Kernsequenzen müssen aus der Herstellerdokumentation eingetragen werden (2.5).
- **Codon-Tabellen** für E. coli, Mensch, Hefe: aus Kazusa erzeugen (2.11).
- **Feature-Datenbank** (`standard-features.json`): nur Einträge, deren Sequenz sich aus freien Quellen belegen lässt (1.9).
- **3Dmol.js** manuell in die Ressourcen legen (4.7).
- **PDF-Export** nicht eingeplant (6.7).
- **Instabilitätsindex** (4.5) nur, wenn die Dipeptid-Tabelle aus der Originalquelle übernommen wird.
- **Echte Sanger-Dateien (.ab1)** werden nicht ins Repository gelegt; Tests erzeugen synthetische Dateien.

## Fragen aus der Umsetzung

- **Nachtlauf 25./26.10.2026 (Claude):**
  - Die Swift-Ansichten (Sequenz, Karte, Verdau mit Gel, CRISPR, Klonieren, Primer) bauen fehlerfrei, wurden aber **nicht von Hand angeschaut** (kein Bildschirmzugriff während der Nacht). Bitte einmal durchklicken und Auffälligkeiten melden.
  - Codon-Tabellen (E. coli, Mensch, Hefe) fehlen weiterhin: `CodonTable::from_counts` kann sie aus Zählungen bauen, die Kazusa-Daten müssen noch beschafft werden.
  - Gibson-/Überlappungs-Zusammenbau, Golden Gate, Ligation, PCR, Gel, CRISPR und Codon-Optimierung sind im Kern fertig; die Oberfläche kann bisher Verdau, Ligation (Restriktionsklonierung), CRISPR und Primerwahl.
  - Wallace-/NN-Tm: Überlapp-Tm nutzt `nearest_neighbor(250 nM, 50 mM)`.
  - Abweichung vom Plan 2.2: Die Warnung „Insert enthält interne Schnittstelle“ ist nicht umgesetzt.
  - Plan 2.6 behauptet, ein dichteres Gel lasse Fragmente weiter laufen; die Formel liefert das Gegenteil (richtig für Agarose). Test und Kern folgen der Formel.
  - Qwen-Warteschlange gestoppt (Datei `scripts/qwen/STOP`); zum Weiterarbeiten `rm scripts/qwen/STOP` und `nohup scripts/qwen/worker.sh &`, Aufgaben als Zeilen in `scripts/qwen/queue.txt`.
  - Oberfläche geprüft über Bildschirmfotos ohne Fenster (`Snapious --snapshot`): Sequenz-, Karten-, Alignment- und Baumansicht sehen richtig aus; die Ansichten mit Tabellen, Eingabefeldern und Knöpfen (Verdau, CRISPR, Klonieren, Primer, Analyse, Protein, Sequenzierung, RNA) konnten so nicht gerendert und müssen einmal von Hand angesehen werden.
  - macOS fragte beim Testen nach Bildschirmaufnahme-Rechten für Claude; das wurde nicht bestätigt (Sicherheitseinstellung).
  - BLAST nutzt `-task blastn` (bzw. `blastn-short` unter 30 Basen), damit kurze Sequenzen Treffer finden.
  - SnapGene-Import (`.dna`) wurde nur mit selbst gebauten Dateien nach der bekannten Blockstruktur getestet. Bitte mit einer echten `.dna`-Datei ausprobieren und Abweichungen melden (Flags, Feature-XML).
  - Workflows: Der Dialog zeigt das JSON direkt; ein grafischer Editor fehlt.
