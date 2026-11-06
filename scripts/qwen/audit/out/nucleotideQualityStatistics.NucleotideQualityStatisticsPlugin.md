GENEIOUS-FUNKTIONEN:
- Berechnet statistische Metriken basierend auf Nukleotid-Qualitätsdaten
- Zeigt Qualitätsschätzungen direkt im Sequenzansichts-Fenster an
- Berechnet die mittlere Konfidenz (Qualitätswert) der Nukleotide
- Berechnet die erwartete Anzahl fehlerhafter Nukleotide in der Sequenz
- Bestimmt die Wahrscheinlichkeit, dass keine Sequenzierfehler vorliegen
- Berechnet Odds für fehlerfreie Sequenzen unter Einbeziehung von Trimming
- Schließt Lücken (Gaps) bei der Berechnung der Qualitätswerte aus
- Berücksichtigt getrimmte Regionen in den statistischen Auswertungen
- Stellt prozentuale Anteile von Nukleotiden ab einem bestimmten Schwellenwert dar
- Analysiert Rohdaten-Qualität zur Bewertung der Sequenziergenauigkeit

ABGLEICH:
- Berechnet statistische Metriken basierend auf Nukleotid-Qualitätsdaten | Kern: ja | App: ja | io/quality.rs
- Zeigt Qualitätsschätzungen direkt im Sequenzansichts-Fenster an | Kern: ja | App: ja | view/seq_layout.rs (berechnet Layout, Anzeige erfolgt durch View-Logik), io/quality.rs
- Berechnet die mittlere Konfidenz (Qualitätswert) der Nukleotide | Kern: ja | App: nein | Keine spezifische Funktion für "mittlere Konfidenz" in der Schnittstelle; `io/quality.rs` bietet nur Kodierung, keine Aggregation dieser spezifischen Metrik als App-Funktion.
- Berechnet die erwartete Anzahl fehlerhafter Nukleotide in der Sequenz | Kern: ja | App: nein | Keine Funktion zur Berechnung der erwarteten Fehleranzahl aus Qualitätsdaten in der Schnittstelle.
- Bestimmt die Wahrscheinlichkeit, dass keine Sequenzierfehler vorliegen | Kern: ja | App: nein | Keine Funktion zur Berechnung dieser spezifischen Wahrscheinlichkeit in der Schnittstelle.
- Berechnet Odds für fehlerfreie Sequenzen unter Einbeziehung von Trimming | Kern: ja | App: nein | Keine Funktion zur Berechnung von "Odds" für fehlerfreie Sequenzen in der Schnittstelle.
- Schließt Lücken (Gaps) bei der Berechnung der Qualitätswerte aus | Kern: ja | App: teilweise | io/quality.rs (Logik vorhanden, aber keine spezifische App-Funktion "quality_with_gaps_excluded"), ngs/trim.rs (Trimming vorhanden, aber keine Integration in Qualitätsberechnung als separate App-Funktion).
- Berücksichtigt getrimmte Regionen in den statistischen Auswertungen | Kern: ja | App: teilweise | ngs/trim.rs (Trimming vorhanden), aber keine integrierte Funktion, die Trimming-Ergebnisse direkt in Qualitätsstatistiken einfließen lässt.
- Stellt prozentuale Anteile von Nukleotiden ab einem bestimmten Schwellenwert dar | Kern: ja | App: nein | Keine Funktion, die prozentuale Anteile basierend auf Qualitätsschwellenwerten ausgibt.
- Analysiert Rohdaten-Qualität zur Bewertung der Sequenziergenauigkeit | Kern: ja | App: ja | ngs/qc.rs (Qualitätsbericht für FASTQ-Reads), io/quality.rs
