GENEIOUS-FUNKTIONEN:
- Berechnet Sekundärstrukturen für RNA-Sequenzen
- Berechnet Sekundärstrukturen für DNA-Sequenzen
- Berechnet Sekundärstrukturen für Primer-Sequenzen
- Bestimmt freie Gesamtenergie des Ensembles
- Berechnet Häufigkeit optimaler Strukturen im Ensemble
- Ermittelt Ensemble-Diversität der Strukturpopulation
- Berechnet Partitionsfunktion und Basenpaarungswahrscheinlichkeiten
- Visualisiert Strukturen nach Wahrscheinlichkeit (Farbcodierung)
- Zeigt Sequenzauswahl im Faltviewer hervor
- Bereinigt Strukturen durch Vermeidung isolierter Paare
- Berücksichtigt koaxiales Stapeln benachbarter Helices
- Integriert Dangling-End-Energien an Helixenden

ABGLEICH:
- Berechnet Sekundärstrukturen für RNA-Sequenzen | Kern: ja | App: ja | bio/rna_fold.rs (RNAfold)
- Berechnet Sekundärstrukturen für DNA-Sequenzen | Kern: nein | App: nein | Keine Funktion vorhanden
- Berechnet Sekundärstrukturen für Primer-Sequenzen | Kern: nein | App: nein | Keine Funktion vorhanden
- Bestimmt freie Gesamtenergie des Ensembles | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare/Punkt-Klammer-Schreibweise; Energieberechnung fehlt
- Berechnet Häufigkeit optimaler Strukturen im Ensemble | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare; Ensemble-Häufigkeit fehlt
- Ermittelt Ensemble-Diversität der Strukturpopulation | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare; Diversität fehlt
- Berechnet Partitionsfunktion und Basenpaarungswahrscheinlichkeiten | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare; Partitionfunktion/Wahrscheinlichkeiten fehlen
- Visualisiert Strukturen nach Wahrscheinlichkeit (Farbcodierung) | Kern: ja | App: nein | view/tree_layout.rs und view/map_svg.rs existieren, aber keine RNA-Strukturvisualisierung mit Wahrscheinlichkeitsfarbcodierung
- Zeigt Sequenzauswahl im Faltviewer hervor | Kern: ja | App: nein | view/seq_layout.rs existiert, aber keine Integration mit einem interaktiven Faltviewer
- Bereinigt Strukturen durch Vermeidung isolierter Paare | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare; Bereinigungsalgorithmus fehlt
- Berücksichtigt koaxiales Stapeln benachbarter Helices | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare; koaxiales Stapeln fehlt
- Integriert Dangling-End-Energien an Helixenden | Kern: ja | App: nein | bio/rna_fold.rs liest nur Basenpaare; Dangling-End-Energien fehlen
