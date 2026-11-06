GENEIOUS-FUNKTIONEN:
- Suchen von Proteindomänen über verschiedene Pfam-Datenbanken (A, B, Clan)
- Abrufen von Domäneninformationen für gegebene Proteinsequenzen
- Extrahieren der Sequenzstruktur (Domain-Architektur) aus Proteinen
- Abrufen vollständiger oder Seed-Alignments für Pfam-A-Domänen
- Retrieven kompletter UniProt-Sequenzen aus Domänenalignments
- Identifizieren ähnlicher Proteine mit gleicher Domain-Reihenfolge
- Anzeigen von Referenzen und Metadaten zu Domänen oder Clans
- Generieren von Dokumenten für Pfam-A, B und Clan-Domänen
- Verwalten lokaler Pfam-Datenbankdateien und deren Speicherort

ABGLEICH:
- Suchen von Proteindomänen über verschiedene Pfam-Datenbanken (A, B, Clan) | Kern: nein | App: nein | Keine Pfam-Datenbankintegration vorhanden; nur allgemeine `db/uniprot.rs` und `bio/protein.rs`
- Abrufen von Domäneninformationen für gegebene Proteinsequenzen | Kern: nein | App: nein | Keine spezifische Domänenabfrage; `bio/protein.rs` bietet nur physikochemische Eigenschaften
- Extrahieren der Sequenzstruktur (Domain-Architektur) aus Proteinen | Kern: nein | App: nein | Keine Analyse von Protein-Domänenstrukturen vorhanden
- Abrufen vollständiger oder Seed-Alignments für Pfam-A-Domänen | Kern: nein | App: nein | Keine Integration von Pfam-spezifischen Alignments; nur allgemeine `bio/align.rs` und `io/fasta.rs`
- Retrieven kompletter UniProt-Sequenzen aus Domänenalignments | Kern: nein | App: ja | `db/uniprot.rs` ermöglicht UniProt-Import, aber keine Verknüpfung über Domänenalignments
- Identifizieren ähnlicher Proteine mit gleicher Domain-Reihenfolge | Kern: nein | App: nein | Keine Funktion zum Vergleich von Domänenarchitekturen; nur `bio/distance.rs` für Sequenzdistanzen
- Anzeigen von Referenzen und Metadaten zu Domänen oder Clans | Kern: nein | App: nein | Keine Pfam-spezifischen Metadaten; nur allgemeine `model/document.rs` und `db/uniprot_entry.rs`
- Generieren von Dokumenten für Pfam-A, B und Clan-Domänen | Kern: nein | App: ja | `model/document.rs` und `export/text.rs` existieren, aber keine pfam-spezifische Dokumentgenerierung
- Verwalten lokaler Pfam-Datenbankdateien und deren Speicherort | Kern: nein | App: ja | `store/project_store.rs` und `settings.rs` existieren, aber keine spezifische Pfam-Dateiverwaltung
