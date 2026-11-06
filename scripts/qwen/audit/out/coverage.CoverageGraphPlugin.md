GENEIOUS-FUNKTIONEN:
- Erzeugt Coverage-Annotationen auf Referenzsequenzen
- Findet Regionen mit hoher oder niedriger Abdeckung
- Zeigt Graphen der Sequenzabdeckung in Contigs/Alignments
- Berechnet durchschnittliche Abdeckung über Annotationen
- Kopiert Annotationen auf Konsensussequenz mit Coverage-Qualifikator
- Ermittelt minimale, mittlere und maximale Abdeckung pro Annotation
- Findet Regionen basierend auf Schwellenwert der Sequenzanzahl
- Findet Regionen basierend auf Standardabweichungen vom Mittelwert
- Analysiert Abdeckung in ausgewählten Regionen von Alignments
- Generiert Warnung bei fehlendem Speicher für Berechnungen
- Prüft Verfügbarkeit von Referenzsequenz in Alignment
- KEINE

ABGLEICH:
- Erzeugt Coverage-Annotationen auf Referenzsequenzen | Kern: ja | App: ja | coverage.rs (Coverage-Analyse liefert Depth/Regionen; Annotation-Erzeugung über `add_features` möglich, aber keine automatische "Coverage-Annotation" als Feature-Typ spezifiziert)
- Findet Regionen mit hoher oder niedriger Abdeckung | Kern: ja | App: ja | coverage.rs (abgedeckte Regionen und Lücken)
- Zeigt Graphen der Sequenzabdeckung in Contigs/Alignments | Kern: ja | App: nein | coverage.rs (berechnet Depth/Histogramm, aber keine visuelle Graphen-Anzeige in der Schnittstelle genannt)
- Berechnet durchschnittliche Abdeckung über Annotationen | Kern: ja | App: nein | coverage.rs (berechnet Depth je Position, aber keine Aggregation über benutzerdefinierte Annotationen spezifiziert)
- Kopiert Annotationen auf Konsensussequenz mit Coverage-Qualifikator | Kern: ja | App: nein | coverage.rs (Konsens über `alignment_consensus` möglich, aber keine Kopie von Coverage-Annotationen mit Qualifikator spezifiziert)
- Ermittelt minimale, mittlere und maximale Abdeckung pro Annotation | Kern: ja | App: nein | coverage.rs (berechnet Depth, aber keine statistische Auswertung pro Annotation-Intervall spezifiziert)
- Findet Regionen basierend auf Schwellenwert der Sequenzanzahl | Kern: ja | App: ja | coverage.rs (abgedeckte Regionen; Schwellenwert-Logik implizit in "Regionen" enthalten, aber keine explizite Filterfunktion nach absoluter Read-Anzahl genannt)
- Findet Regionen basierend auf Standardabweichungen vom Mittelwert | Kern: ja | App: nein | coverage.rs (Statistiken genannt, aber keine spezifische Berechnung von Standardabweichungen zur Regionsfindung)
- Analysiert Abdeckung in ausgewählten Regionen von Alignments | Kern: ja | App: teilweise | coverage.rs, alignment_variants (Alignment-Daten verfügbar, aber keine spezifische "Coverage-Analyse im Alignment"-Operation in der Schnittstelle)
- Generiert Warnung bei fehlendem Speicher für Berechnungen | Kern: ja | App: nein | (Keine spezifische Funktion, allgemeine Fehlerbehandlung via `view/error_hint.rs`)
- Prüft Verfügbarkeit von Referenzsequenz in Alignment | Kern: ja | App: ja | alignment (Alignment-Dokument enthält Referenz; Prüfung ist Teil der Datenstruktur, aber keine separate API-Funktion)
- KEINE | Kern: nein | App: nein | (Keine passenden Snapious-Funktionen)
