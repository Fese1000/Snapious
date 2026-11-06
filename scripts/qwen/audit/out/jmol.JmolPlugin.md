GENEIOUS-FUNKTIONEN:
- 3D-Proteinstrukturen visuell darstellen und betrachten
- Molekülansichten speichern (z. B. Ball-and-Stick, Drahtgitter)
- Farbschemata für Biomoleküle anpassen und ändern
- Strukturzustände (State) sichern und wiederherstellen
- Befehlszeile zur Steuerung der 3D-Anzeige nutzen
- Strukturbaum (StrukturTree) zur Navigation durch Modelle
- Halos und Markierungen für Selektionen anzeigen/verbergen
- 3D-Strukturen mit weißem Hintergrund drucken
- Bilder (JPG/PNG) in hoher Auflösung exportieren
- Modellzustände inline speichern und laden
- Ribbon-Border-Einstellungen für visuelle Darstellung anpassen
- KEINE

ABGLEICH:
- 3D-Proteinstrukturen visuell darstellen und betrachten | Kern: nein | App: nein | Keine 3D-Visualisierung vorhanden (nur PDB-Lesefunktion in `io/pdb.rs`)
- Molekülansichten speichern (z. B. Ball-and-Stick, Drahtgitter) | Kern: nein | App: nein | Keine Rendering-Engine für molekulare Darstellungen vorhanden
- Farbschemata für Biomoleküle anpassen und ändern | Kern: nein | App: nein | Keine interaktive Farbanpassung für 3D-Modelle vorhanden
- Strukturzustände (State) sichern und wiederherstellen | Kern: nein | App: nein | Keine Verwaltung von 3D-Render-Zuständen vorhanden
- Befehlszeile zur Steuerung der 3D-Anzeige nutzen | Kern: nein | App: nein | Keine Befehlskonsole für 3D-Viewer vorhanden
- Strukturbaum (StrukturTree) zur Navigation durch Modelle | Kern: nein | App: nein | Keine Baumstruktur für PDB-Modelle vorhanden (nur phylogenetische Bäume in `bio/tree.rs`)
- Halos und Markierungen für Selektionen anzeigen/verbergen | Kern: nein | App: nein | Keine visuellen Halos für 3D-Selektionen vorhanden
- 3D-Strukturen mit weißem Hintergrund drucken | Kern: nein | App: nein | Keine Druckfunktion für 3D-Ansichten vorhanden
- Bilder (JPG/PNG) in hoher Auflösung exportieren | Kern: nein | App: nein | Nur SVG-Export für Plasmid-/Baumkarten (`view/map_svg.rs`, `view/tree_layout.rs`), keine Rasterbild-Exportfunktion für 3D
- Modellzustände inline speichern und laden | Kern: nein | App: nein | Keine Speicherung von 3D-Modellzuständen vorhanden
- Ribbon-Border-Einstellungen für visuelle Darstellung anpassen | Kern: nein | App: nein | Keine Einstellungsmöglichkeiten für Ribbon-Darstellung vorhanden
