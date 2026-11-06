GENEIOUS-FUNKTIONEN:
- Stellt Sequenzähnlichkeitsgraphen für Alignments bereit
- Berechnet mittlere paarweise Identität über Spaltenpaare
- Visualisiert 100% Identität durch grüne Farbe
- Zeigt mindestens 30%, unter 100% Identität grün-braun
- Darstellt unter 30% Identität durch rote Farbe

ABGLEICH:
- Stellt Sequenzähnlichkeitsgraphen für Alignments bereit | Kern: nein | App: nein | Keine Graph-Datenstruktur oder -Visualisierung für Ähnlichkeitsnetzwerke vorhanden (nur Baumstrukturen in `bio/tree.rs`)
- Berechnet mittlere paarweise Identität über Spaltenpaare | Kern: teilweise | App: ja | `bio/align_stats.rs` (Identitätsmatrix/Kennzahlen), jedoch keine explizite Aggregation "mittlere Identität über Spaltenpaare" als eigene Funktion, aber `alignment_identity_matrix` existiert
- Visualisiert 100% Identität durch grüne Farbe | Kern: ja | App: nein | Snapious bietet `alignment_logo` und Textausgabe, aber keine spezifische farbcodierte Visualisierung (Heatmap/Graph) basierend auf Identitätsschwellenwerten
- Zeigt mindestens 30%, unter 100% Identität grün-braun | Kern: ja | App: nein | Keine farbcodierte Visualisierung von Identitätsschwellen in Snapious vorhanden
- Darstellt unter 30% Identität durch rote Farbe | Kern: ja | App: nein | Keine farbcodierte Visualisierung von Identitätsschwellen in Snapious vorhanden
