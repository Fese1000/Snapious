GENEIOUS-FUNKTIONEN:
- Übertrift Annotationen von Sequenzen auf eine Referenzsequenz
- Berechnet prozentuale Ähnlichkeit basierend auf Score-Matrizen (z.B. Blosum62)
- Fügt Annotationen gleichen Typs zusammen, wenn sie gleiche Regionen abdecken
- Speichert Herkunft (Transferred From) und Ähnlichkeit (Transferred Similarity) als Qualifier
- Erweitet oder kürzt Annotationen bei End-Gaps basierend auf Übereinstimmung
- Überträgt Annotationen auf die Konsensussequenz anstelle der Referenz
- Filtert übertragene Annotationen nach Typ, Mindestlänge und Ähnlichkeitsschwelle
- Schließt mehrfache gleiche Treffer aus, außer Primer-Matches

ABGLEICH:
- Übertrift Annotationen von Sequenzen auf eine Referenzsequanz | Kern: ja | App: ja | model/feature.rs (Features haben Positionen und Strang), ops/phase2.rs (Klonierungs-Operationen enthalten Bearbeitung), view/edit_session.rs (Bearbeitungssitzung)
- Berechnet prozentuale Ähnlichkeit basierend auf Score-Matrizen (z.B. Blosum62) | Kern: ja | App: ja | bio/align_stats.rs (Identitätsmatrix, Konsens), bio/protein.rs (Eigenschaften von Proteinsequenzen)
- Fügt Annotationen gleichen Typs zusammen, wenn sie gleiche Regionen abdecken | Kern: teilweise | App: nein | model/feature.rs (Features haben Intervalle), aber keine explizite "Merge"-Funktion in der App-Schnittstelle genannt; view/feature_layout.rs (Verteilung, keine Zusammenführung)
- Speichert Herkunft (Transferred From) und Ähnlichkeit (Transferred Similarity) als Qualifier | Kern: ja | App: nein | model/feature.rs (Features haben Qualifier), aber keine spezifische Funktion zum Setzen dieser speziellen Metadaten-Flags in der App-Schnittstelle genannt
- Erweitet oder kürzt Annotationen bei End-Gaps basierend auf Übereinstimmung | Kern: ja | App: nein | bio/align_stats.rs (Konsens), view/edit_session.rs (Bearbeiten/Einfügen/Löschen), aber keine automatische "Gap-basierte Anpassung" von Annotationen in der App-Schnittstelle genannt
- Überträgt Annotationen auf die Konsensussequenz anstelle der Referenz | Kern: ja | App: nein | bio/align_stats.rs (Konsens), model/alignment.rs (Alignment-Dokument), aber keine spezifische Funktion zum Kopieren von Features auf den Konsens in der App-Schnittstelle genannt
- Filtert übertragene Annotationen nach Typ, Mindestlänge und Ähnlichkeitsschwelle | Kern: ja | App: nein | bio/align_stats.rs (Konservierung, Identität), model/feature.rs (Features haben Typ/Länge), aber keine spezifische Filterfunktion für diesen Workflow in der App-Schnittstelle genannt
- Schließt mehrfache gleiche Treffer aus, außer Primer-Matches | Kern: ja | App: nein | primer/picker.rs (Primer-Auswahl), bio/motif.rs (Motivsuche), aber keine spezifische Deduplizierungslogik für Annotationen in der App-Schnittstelle genannt
