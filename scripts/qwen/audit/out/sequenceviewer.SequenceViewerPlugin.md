GENEIOUS-FUNKTIONEN:
- Sequenzen linear und zirkular visualisieren
- DNA-, RNA- und Proteinsequenzen anzeigen
- Mehrfachalignments und Paarlesungen darstellen
- Annotationen (Features) an Sequenzen erstellen und bearbeiten
- Farbcodierung nach Aminosäureeigenschaften oder Konsens
- Farbgebung basierend auf Qualitätswerten oder Übereinstimmung
- Konsensus- und Referenzsequenz-Vergleiche durchführen
- Sequenzen konvertieren (DNA zu RNA)
- Annotationen exportieren und in Bilder speichern
- Zu spezifischen Restnummern springen (GoToResidue)
- Filtereinstellungen für Sequenzansichten anwenden
- Keine expliziten biologischen Analysefunktionen erkennbar

ABGLEICH:
- Sequenzen linear und zirkular visualisieren | Kern: ja | App: nein | view/map_layout.rs (Logik vorhanden, keine UI-Implementierung in der Schnittstellenliste)
- DNA-, RNA- und Proteinsequenzen anzeigen | Kern: ja | App: nein | model/sequence.rs (Typen vorhanden, keine explizite "Anzeigen"-Funktion in der Schnittstelle)
- Mehrfachalignments und Paarlesungen darstellen | Kern: ja | App: nein | bio/alignment.rs (Berechnung vorhanden, keine Darstellungs-Funktion in der Schnittstelle)
- Annotationen (Features) an Sequenzen erstellen und bearbeiten | Kern: ja | App: ja | add_features, remove_feature (Dateien: model/feature.rs)
- Farbcodierung nach Aminosäureeigenschaften oder Konsens | Kern: ja | App: nein | bio/align_stats.rs (Konsenzberechnung vorhanden, keine Farbcodierungs-Funktion in der Schnittstelle)
- Farbgebung basierend auf Qualitätswerten oder Übereinstimmung | Kern: ja | App: nein | io/quality.rs (Qualitätsdaten vorhanden, keine Farbcodierungs-Funktion in der Schnittstelle)
- Konsensus- und Referenzsequenz-Vergleiche durchführen | Kern: ja | App: ja | alignment_consensus, alignment_variants (Dateien: bio/align_stats.rs, bio/variants.rs)
- Sequenzen konvertieren (DNA zu RNA) | Kern: ja | App: nein | bio/genetic_code.rs (Translation vorhanden, keine explizite DNA-zu-RNA-Konvertierungsfunktion in der Schnittstelle)
- Annotationen exportieren und in Bilder speichern | Kern: ja | App: teilweise | export_document, map_svg.rs (Export Logik vorhanden, aber keine explizite "in Bilder speichern"-Funktion in der Schnittstellenliste, nur SVG-Generierung Logik)
- Zu spezifischen Restnummern springen (GoToResidue) | Kern: ja | App: nein | view/selection.rs (Positionierung Logik vorhanden, keine "Springen"-Funktion in der Schnittstelle)
- Filtereinstellungen für Sequenzansichten anwenden | Kern: ja | App: nein | view/feature_layout.rs (Layout Logik vorhanden, keine Filter-Funktion in der Schnittstelle)
- Keine expliziten biologischen Analysefunktionen erkennbar | Kern: nein | App: ja | Viele Funktionen vorhanden (z.B. blast_search, pick_primers)
