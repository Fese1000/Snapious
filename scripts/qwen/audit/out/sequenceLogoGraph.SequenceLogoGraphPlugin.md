GENEIOUS-FUNKTIONEN:
- Zeigt Sequenzlogo für Alignments an
- Stellt das Sequence Logo Graph Plugin bereit
- Generiert visuelle Sequenzlogo-Darstellungen

ABGLEICH:
- Zeigt Sequenzlogo für Alignments an | Kern: ja | App: nein | bio/align_stats.rs (Funktion `alignment_logo` fehlt in der Schnittstellenliste)
- Stellt das Sequence Logo Graph Plugin bereit | Kern: nein | App: nein | Keine Plugin-Architektur für visuelle Plugins implementiert (nur externe Programme via `plugin.rs`)
- Generiert visuelle Sequenzlogo-Darstellungen | Kern: ja | App: nein | bio/align_stats.rs (Berechnung vorhanden, aber keine visuelle Darstellung/Schnittstellenfunktion `alignment_logo` fehlt)
