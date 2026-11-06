GENEIOUS-FUNKTIONEN:
- Zeigt Chromatogramme an
- Zeigt Sequenzlogos für Chromatogrammdokumente an
- Skaliert die Darstellung nach Konfidenz

ABGLEICH:
- Zeigt Chromatogramme an | Kern: ja | App: ja | io/abif.rs (liest Basen, Qualitäten, Peakpositionen und Spuren aus `.ab1`), view/seq_layout.rs (Darstellung der Sequenzansicht, jedoch keine explizite visuelle Chromatogramm-Darstellung wie Balkendiagramme der Peaks in der Schnittliste genannt, aber Daten sind vorhanden)
- Zeigt Sequenzlogos für Chromatogrammdokumente an | Kern: ja | App: ja | bio/align_stats.rs (Sequenzlogo), io/abif.rs (Chromatogrammdaten als Quelle)
- Skaliert die Darstellung nach Konfidenz | Kern: ja | App: nein | Keine explizite Funktion zur dynamischen Skalierung der visuellen Darstellung basierend auf Qualitätskonfidenz in der Schnittstelle genannt; view/seq_layout.rs und io/abif.rs liefern Daten, aber keine spezifische "Konfidenz-Skalierung" der Anzeige ist in der Funktionsliste enthalten.
