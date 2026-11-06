GENEIOUS-FUNKTIONEN:
- Rückübersetzung von Proteinsequenzen in DNA-Sequenzen unter Nutzung genetischer Codes
- Import und Verwaltung von Codon-Nutzungs-Tabellen für verschiedene Organismen
- Generierung von Dotplots zur visuellen Vergleichsanalyse zweier Sequenzen
- Suche nach PROSITE-Motiven in Proteinsequenzen mittels fuzznuc/fuzzpro
- Berechnung der Ladungsverteilung entlang einer Aminosäuresequenz
- Anzeige von Minimap-Ansichten für schnelle Übersicht über Sequenzähnlichkeiten
- Generierung von Annotationen basierend auf gefundenen Motiven oder Mustern
- Darstellung und Analyse von DNA-Dotplots mit Zoom- und Pan-Funktionen
- Validierung und Korrektur von Codon-Nutzungsdaten und Sequenzlängen
- Unterstützung verschiedener genetischer Codes (z.B. mitochondriale, bakterielle)

ABGLEICH:
- Rückübersetzung von Proteinsequenzen in DNA-Sequenzen unter Nutzung genetischer Codes | Kern: nein | App: nein | (fehlt; `bio/genetic_code.rs` bietet nur Translation Protein <- DNA, keine Rückübersetzung/Retro-Translation)
- Import und Verwaltung von Codon-Nutzungs-Tabellen für verschiedene Organismen | Kern: ja | App: ja | `bio/codon_tables.rs`, `bio/kazusa.rs` (Laden der eingebetteten JSON-Tabellen)
- Generierung von Dotplots zur visuellen Vergleichsanalyse zweier Sequenzen | Kern: ja | App: ja | `bio/dotplot.rs`, `app: dotplot`
- Suche nach PROSITE-Motiven in Proteinsequenzen mittels fuzznuc/fuzzpro | Kern: ja | App: teilweise | `bio/motif.rs` (Suche nach IUPAC-Motiven, aber keine spezifische PROSITE-Datenbank-Integration oder fuzznuc/fuzzpro-Semantik)
- Berechnung der Ladungsverteilung entlang einer Aminosäuresequenz | Kern: ja | App: ja | `bio/protein.rs` (enthält Ladungsberechnung, jedoch keine explizite "Verteilung" über die Sequenzlängenachse als separater Output, nur Gesamtladung/pI/Extinktion)
- Anzeige von Minimap-Ansichten für schnelle Übersicht über Sequenzähnlichkeiten | Kern: ja | App: ja | `view/minimap.rs` (Dichte-Streifen für Übersicht), `app: map_reads_to_reference` (für Mapping-Übersicht)
- Generierung von Annotationen basierend auf gefundenen Motiven oder Mustern | Kern: ja | App: teilweise | `primer/annotate.rs` (Primerstellen als Features), `view/feature_detect.rs` (automatische ORF-Erkennung), aber keine allgemeine "Motiv-basierte Annotation" wie bei PROSITE-Suchergebnissen)
- Darstellung und Analyse von DNA-Dotplots mit Zoom- und Pan-Funktionen | Kern: ja | App: teilweise | `bio/dotplot.rs` (Logik vorhanden), `app: dotplot` (Schnittstelle vorhanden); Zoom/Pan ist eine UI-Implementierungsdetails, die Logik `bio/dotplot.rs` existiert)
- Validierung und Korrektur von Codon-Nutzungsdaten und Sequenzlängen | Kern: ja | App: nein | (fehlt; `bio/codon_usage.rs` berechnet CAI/RSCU, aber es gibt keine explizite "Validierung und Korrektur"-Funktion für die Tabellendaten selbst)
- Unterstützung verschiedener genetischer Codes (z.B. mitochondriale, bakterielle) | Kern: ja | App: ja | `bio/genetic_code.rs` (Tabellen 1, 2, 4, 5 und 11 sind implementiert)
