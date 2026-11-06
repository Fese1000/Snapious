GENEIOUS-FUNKTIONEN:
- Zeigt phylogenetische Bäume grafisch an
- Berechnet und zeigt Distanzmatrizen für Taxa
- Zeigt prozentuale Identität von Basen/Residuen an
- Zeigt absolute Anzahl identischer Positionen (Identities)
- Zeigt absolute Anzahl unterschiedlicher Positionen (Differences)
- Berechnet prozentuale Ähnlichkeit bei Proteinalignments
- Zeigt Konsensunterstützung für Konsensbäume an
- Berechnet patristische Distanzen durch Astlängensumme
- Zeigt Heatmap für Distanzvisualisierung an
- Ermöglicht das Umbenennen von Taxa bei Duplikaten
- Zeigt Warnung bei korrupten oder fehlerhaften Bäumen
- Speichert Baumansichten als hochqualitative Bilddateien

ABGLEICH:
- Zeigt phylogenetische Bäume grafisch an | Kern: ja | App: nein | view/tree_layout.rs (Layout/SVG), aber keine interaktive grafische Anzeige im UI vorhanden
- Berechnet und zeigt Distanzmatrizen für Taxa | Kern: ja | App: nein | bio/distance.rs (berechnet Distanzen), aber keine Matrix-Anzeige/Funktion in der App-Schnittstelle
- Zeigt prozentuale Identität von Basen/Residuen an | Kern: ja | App: ja | bio/align_stats.rs (Identitätsmatrix), alignment_identity_matrix
- Zeigt absolute Anzahl identischer Positionen (Identities) | Kern: ja | App: ja | bio/align_stats.rs (Konsens/Kennzahlen), alignment_identity_matrix enthält diese Info
- Zeigt absolute Anzahl unterschiedlicher Positionen (Differences) | Kern: ja | App: ja | bio/align_stats.rs (Konsens/Kennzahlen), alignment_identity_matrix enthält diese Info
- Berechnet prozentuale Ähnlichkeit bei Proteinalignments | Kern: ja | App: nein | bio/align_stats.rs und bio/protein.rs vorhanden, aber keine spezifische Funktion für "prozentuale Ähnlichkeit" in der App-Schnittstelle (nur Identität/Konsens)
- Zeigt Konsensunterstützung für Konsensbäume an | Kern: ja | App: nein | bio/align_stats.rs (Konsens), aber keine "Unterstützung" oder Bootstrap-Werte für Bäume in der App-Schnittstelle
- Berechnet patristische Distanzen durch Astlängensumme | Kern: ja | App: nein | bio/tree.rs (Baumdatenstruktur), aber keine Funktion zur Berechnung der patristischen Distanz in der App-Schnittstelle
- Zeigt Heatmap für Distanzvisualisierung an | Kern: ja | App: nein | bio/distance.rs vorhanden, aber keine Heatmap-Visualisierung in der App-Schnittstelle
- Ermöglicht das Umbenennen von Taxa bei Duplikaten | Kern: ja | App: ja | project/search.rs (Stapelumbenennung), rename_document
- Zeigt Warnung bei korrupten oder fehlerhaften Bäumen | Kern: ja | App: ja | view/error_hint.rs (Fehlermeldungen), tree_newick (Newick lesen/schreiben)
- Speichert Baumansichten als hochqualitative Bilddateien | Kern: ja | App: ja | view/tree_layout.rs (SVG-Ausgabe), tree_newick (Export), export_document
