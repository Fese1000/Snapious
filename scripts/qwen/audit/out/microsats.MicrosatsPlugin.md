GENEIOUS-FUNKTIONEN:
- Importiert und zeigt Microsat-Rohdaten an (nur lesbar)
- Ermöglicht das Laden von Microsat-Daten in Geneious Prime
- Unterstützt die Zuordnung von Laddern (Ladder Matching)
- Führt Peak-Calling durch, um Signale zu identifizieren
- Bietet Allel-Binning zur Gruppierung von Allelen an
- Zeigt Microsatellite Raw Trace als schreibgeschützte Ansicht

ABGLEICH:
- Importiert und zeigt Microsat-Rohdaten an (nur lesbar) | Kern: teilweise | App: ja | io/abif.rs (liest .ab1 Rohdaten, aber keine spezifische Microsat-Visualisierung oder "Raw Trace"-Ansicht wie im Plugin beschrieben, nur Basen/Qualitäten)
- Ermöglicht das Laden von Microsat-Daten in Geneious Prime | Kern: ja | App: ja | io/abif.rs (Import von .ab1 Dateien ist implementiert)
- Unterstützt die Zuordnung von Laddern (Ladder Matching) | Kern: nein | App: nein | Keine Funktion zum automatischen Ladder-Matching oder Kalibrierung von Fragmentgrößen basierend auf internen Standards vorhanden
- Führt Peak-Calling durch, um Signale zu identifizieren | Kern: teilweise | App: ja | io/abif.rs (liest Peakpositionen, aber kein dediziertes "Peak-Calling"-Algorithmus zur Allelerkennung aus Rohsignalen, nur Basis-Calling)
- Bietet Allel-Binning zur Gruppierung von Allelen an | Kern: nein | App: nein | Keine Funktion zum automatischen Binning von Allelgrößen oder Gruppierung nach Größe/Toleranzfenstern vorhanden
- Zeigt Microsatellite Raw Trace als schreibgeschützte Ansicht | Kern: nein | App: ja | io/abif.rs (Rohdaten werden gelesen, aber es gibt keine spezifische "Raw Trace"-Ansicht/Simulation wie in der Originalbeschreibung des Plugins genannt; view/seq_layout zeigt nur Sequenztext)
