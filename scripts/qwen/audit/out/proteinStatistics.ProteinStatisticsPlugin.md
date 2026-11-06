GENEIOUS-FUNKTIONEN:
- Berechnet das Molekulargewicht in kDa (einzelne Stränge bei DNA)
- Berechnet den isoelektrischen Punkt (nur für Proteine verfügbar)
- Bestimmt die Ladung des Proteins bei pH 7
- Berechnet den Extinktionskoeffizienten nach Gill und von Hippel
- Berechnet die prozentuale positive Ladung (BLSM62)
- Zeigt Proteinstatistiken im Sequenzansichtsviewer an
- Stellt Molekulargewicht, isoelektrischen Punkt und Extinktionskoeffizient bereit

ABGLEICH:
- Berechnet das Molekulargewicht in kDa (einzelne Stränge bei DNA) | Kern: ja | App: ja | bio/protein.rs (Masse), model/sequence.rs
- Berechnet den isoelektrischen Punkt (nur für Proteine verfügbar) | Kern: ja | App: ja | bio/protein.rs (isoelektrischer Punkt)
- Bestimmt die Ladung des Proteins bei pH 7 | Kern: ja | App: ja | bio/protein.rs (Ladung)
- Berechnet den Extinktionskoeffizienten nach Gill und von Hippel | Kern: ja | App: ja | bio/protein.rs (Extinktion)
- Berechnet die prozentuale positive Ladung (BLSM62) | Kern: nein | App: nein | bio/protein.rs enthält nur "Ladung", keine spezifische prozentuale Berechnung nach BLSM62 genannt
- Zeigt Proteinstatistiken im Sequenzansichtsviewer an | Kern: ja | App: ja | bio/protein.rs (Eigenschaften), view/seq_layout.rs (Anzeige)
- Stellt Molekulargewicht, isoelektrischen Punkt und Extinktionskoeffizient bereit | Kern: ja | App: ja | bio/protein.rs
