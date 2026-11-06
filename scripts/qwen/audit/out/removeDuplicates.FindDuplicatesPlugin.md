GENEIOUS-FUNKTIONEN:
- Finden von Duplikaten in lokalen Dokumenten oder Datenbanken
- Identifizieren von Sequenzen mit identischen Resten
- Erkennen von Dokumenten mit demselben Datenbank-ID (z.B. Accession)
- Erkennen von Dokumenten mit demselben Namen
- Auswählen älterer Duplikate in der Liste
- Auswählen am wenigsten kürzlich geänderter Duplikate
- Extrahieren eindeutiger Sequenzen aus den Ergebnissen
- Suchen nach Duplikaten innerhalb von Ausrichtungen oder Listen
- Ignorieren gepaarter Leseinformationen bei der Suche (Warnung)

ABGLEICH:
- Finden von Duplikaten in lokalen Dokumenten oder Datenbanken | Kern: ja | App: ja | `project/search.rs` (`find_duplicates`)
- Identifizieren von Sequenzen mit identischen Resten | Kern: ja | App: nein | Keine direkte Funktion für "identische Reste" (Suffix/Prefix) in der Schnittstelle; `find_duplicates` prüft wahrscheinlich ganze Sequenzen oder Metadaten, nicht spezifisch "Reste" im Sinne von Suffix-Automaten ohne weitere Spezifikation. Da `find_duplicates` existiert, aber nicht spezifiziert ist *wie* (ganze Seq vs Rest), und "Reste" oft Teilstrings bedeutet: `nein`. (Streng: keine Funktion "find_identical_subsequences" oder ähnlich).
- Erkennen von Dokumenten mit demselben Datenbank-ID (z.B. Accession) | Kern: ja | App: ja | `project/search.rs` (`find_duplicates`) – Duplikatssuche umfasst typischerweise ID-Vergleiche; `features` oder `documents` könnten Accessionen enthalten, aber eine spezifische "find_by_accession" fehlt. Da `find_duplicates` existiert und Accessionen oft Teil des Namens/ID sind, ist es "teilweise" abgedeckt durch die Duplikatslogik, aber keine explizite Funktion. Ich setze "teilweise" auf `project/search.rs` (`find_duplicates`), da Accession-Übereinstimmung eine Form von Duplikat ist.
- Erkennen von Dokumenten mit demselben Namen | Kern: ja | App: ja | `project/search.rs` (`find_duplicates`)
- Auswählen älterer Duplikate in der Liste | Kern: ja | App: nein | Keine Funktion zur Auswahl basierend auf "Alter" oder "Erstellungsdatum" von Duplikaten. `versions` existiert, aber keine Funktion "select_oldest_duplicate".
- Auswählen am wenigsten kürzlich geänderter Duplikate | Kern: ja | App: nein | Keine Funktion zur Auswahl basierend auf "Zuletzt geändert"-Zeitstempel von Duplikaten.
- Extrahieren eindeutiger Sequenzen aus den Ergebnissen | Kern: ja | App: nein | Keine Funktion "extract_unique_sequences". `find_duplicates` findet sie, extrahiert aber nicht automatisch eine neue Liste/Dokumente ohne weitere manuelle Schritte.
- Suchen nach Duplikaten innerhalb von Ausrichtungen oder Listen | Kern: ja | App: teilweise | `project/search.rs` (`find_duplicates`) kann auf Listen angewendet werden, aber keine spezifische Funktion "find_duplicates_in_alignment". `alignment_variants` zeigt Unterschiede, nicht Duplikate. Ich setze "teilweise" auf `project/search.rs` (`find_duplicates`), da Listen als Dokumente behandelt werden können.
- Ignorieren gepaarter Leseinformationen bei der Suche (Warnung) | Kern: ja | App: nein | Keine Funktion oder Option, um "paired read info" bei der Duplikatsuche zu ignorieren. `fastq_stream` liest Paare, aber keine spezifische Logik im Duplikatsfinder sichtbar.
