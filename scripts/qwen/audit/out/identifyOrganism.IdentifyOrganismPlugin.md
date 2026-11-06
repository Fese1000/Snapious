GENEIOUS-FUNKTIONEN:
- Klassifiziert Sequenzen durch globalen Alignment-Vergleich mit Datenbanksequenzen
- Berechnet Überlappungsidentität und Query-Identität für taxonomische Einordnung
- Unterstützt die Analyse mehrerer Loci durch gewichtete Durchschnittsberechnung
- Gruppiert Sequenzen basierend auf Namenspräfixen für Organismenzuordnung
- Generiert Best-Ergebnis-Tabellen mit Identitätswerten zu nächsten Treffern
- Erstellt Zusammenfassungstabellen zur Verteilung von Klassifizierungsergebnissen
- Zeigt Paarweise-Alignments und Bäume für einzelne Abfrageergebnisse an
- Führt taxonomische Hierarchieanalyse durch Aufspaltung bei widersprüchlichen Daten
- Filtert Ergebnisse basierend auf minimaler Überlappung und Identitätsschwellenwerten
- Markiert Ergebnisse farblich (grün/orange/rot) nach Übereinstimmungsgrad
- Baut Bäume aus konkatinierten Alignments oder einzelnen Loci
- Importiert Sequenzmetadaten und verarbeitet Datenbankordnerstrukturen

ABGLEICH:
- Klassifiziert Sequenzen durch globalen Alignment-Vergleich mit Datenbanksequken | Kern: ja | App: ja | bio/align.rs (Needleman-Wunsch global), db/ncbi.rs, db/uniprot.rs
- Berechnet Überlappungsidentität und Query-Identität für taxonomische Einordnung | Kern: ja | App: ja | bio/align_stats.rs (Identitätsmatrix), bio/align.rs
- Unterstützt die Analyse mehrerer Loci durch gewichtete Durchschnittsberechnung | Kern: nein | App: nein | Keine integrierte gewichtete Multi-Locus-Analyse vorhanden; nur `bio/align.rs` und `bio/distance.rs` für einzelne Alignments/Distanzen.
- Gruppiert Sequenzen basierend auf Namenspräfixen für Organismenzuordnung | Kern: ja | App: nein | `project/search.rs` bietet Stapelumbenennung und Dublettensuche, aber keine automatische Gruppierung nach Präfix für taxonomische Zuordnung.
- Generiert Best-Ergebnis-Tabellen mit Identitätswerten zu nächsten Treffern | Kern: ja | App: teilweise | `bio/align_stats.rs` und `bio/blast.rs` (Tabellenformat 6) liefern Identitätswerte, aber keine automatische "Best Hit"-Sortierung für Taxonomie-Reports.
- Erstellt Zusammenfassungstabellen zur Verteilung von Klassifizierungsergebnissen | Kern: ja | App: nein | Keine Funktion zur aggregierten statistischen Zusammenfassung von Klassifizierungsergebnissen vorhanden.
- Zeigt Paarweise-Alignments und Bäume für einzelne Abfrageergebnisse | Kern: ja | App: ja | bio/align.rs (Alignments), bio/tree.rs (Bäume), view/tree_layout.rs
- Führt taxonomische Hierarchieanalyse durch Aufspaltung bei widersprüchlichen Daten | Kern: ja | App: nein | Keine logische Analyse von taxonomischen Konflikten oder Hierarchie-Aufspaltung implementiert.
- Filtert Ergebnisse basierend auf minimaler Überlappung und Identitätsschwellenwerten | Kern: ja | App: teilweise | `bio/align_stats.rs` und `bio/blast.rs` berechnen Identität, aber es gibt keine integrierte Filterfunktion für taxonomische Schwellenwerte.
- Markiert Ergebnisse farblich (grün/orange/rot) nach Übereinstimmungsgrad | Kern: ja | App: nein | Die App berechnet Werte, markiert aber keine Ergebnisse automatisch nach taxonomischen Schwellenwerten farbcodiert.
- Baut Bäume aus konkatinierten Alignments oder einzelnen Loci | Kern: ja | App: teilweise | `bio/msa.rs` und `bio/tree.rs` können Bäume bauen, aber keine automatische Konkatenierung mehrerer Loci für einen gemeinsamen Baum.
- Importiert Sequenzmetadaten und verarbeitet Datenbankordnerstrukturen | Kern: ja | App: ja | db/ncbi.rs, db/uniprot.rs, store/folders.rs
