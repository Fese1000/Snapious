GENEIOUS-FUNKTIONEN:
- Realignt einen Bereich einer bestehenden Ausrichtung
- Führt Regionenausrichtung durch, wenn Plugins verfügbar sind
- Prüft, ob Sequenzen nur Lücken in der Region enthalten
- Erkennt Fälle, bei denen alle Sequenzen nur Lücken haben
- Erkennt Fälle, bei denen alle außer einer Sequenz nur Lücken haben
- Verhindert die Neuanordnung von Sequenzen während der Ausrichtung
- Verhindert die teilweise Umkehrung von Sequenzen
- Fordert zur Auswahl einer oder mehrerer Alignments auf
- Bietet Optionen für die Regionenausrichtung an
- Zeigt Status „teilweise neu ausgerichtet“ an

ABGLEICH:
- Realignt einen Bereich einer bestehenden Ausrichtung | Kern: ja | App: ja | bio/align.rs (Neuanfrage der Alignment-Funktionen auf einem Subset/Substring)
- Führt Regionenausrichtung durch, wenn Plugins verfügbar sind | Kern: ja | App: ja | bio/backends.rs (Aufruf externer Programme wie MAFFT/MUSCLE auf Teilsequenzen)
- Prüft, ob Sequenzen nur Lücken in der Region enthalten | Kern: ja | App: nein | bio/align_stats.rs (Konsenz/Kennzahlen berechenbar, aber keine spezifische "nur-Lücken"-Prüflogik als UI-Feature genannt)
- Erkennt Fälle, bei denen alle Sequenzen nur Lücken haben | Kern: ja | App: nein | bio/align_stats.rs (Statistik verfügbar, aber keine explizite Fehlerbehandlung für "leere Region" im Interface)
- Erkennt Fälle, bei denen alle außer einer Sequenz nur Lücken haben | Kern: ja | App: nein | bio/align_stats.rs (Keine spezifische Logik für diesen Edge-Case im Interface)
- Verhindert die Neuanordnung von Sequenzen während der Ausrichtung | Kern: ja | App: nein | bio/align.rs (Standard-Alignments erlauben Umordnung; keine Option für "fixed order" im Interface genannt)
- Verhindert die teilweise Umkehrung von Sequenzen | Kern: ja | App: nein | bio/align.rs (Keine Option für "no reverse complement" im Interface genannt)
- Fordert zur Auswahl einer oder mehrerer Alignments auf | Kern: ja | App: ja | ops/phase3.rs (Alignment-Operationen; Auswahllogik implizit in Workflow/Dokument-Auswahl)
- Bietet Optionen für die Regionenausrichtung an | Kern: ja | App: teilweise | bio/align.rs (Parameter für NW/SW/Semi-global; keine spezifische "Region"-UI, aber Substring-Alignment möglich)
- Zeigt Status „teilweise neu ausgerichtet“ an | Kern: ja | App: nein | view/progress_format.rs (Fortschrittsanzeige vorhanden, aber kein spezifischer Status "teilweise neu ausgerichtet")
