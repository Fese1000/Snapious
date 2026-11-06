GENEIOUS-FUNKTIONEN:
- Lokale Datenbankverwaltung für Dokumente
- Volltextsuche über lokale Ordner erstellen
- Suchindex automatisch bei Start neu aufbauen
- Index-Reparatur bei Korruption oder Fehlern
- Verwaltung von gelöschten Elementen (Deleted Items)
- Ordner erstellen, umbenennen und verschieben
- Suchergebnisse in lokalen Ordnern speichern
- Suche nach Teilwörtern mit Wildcards (*)
- Filterung nach ganzen Wörnern (Whole Words)
- Anzeige von Dokumenten aus öffentlichen Datenbanken
- Unterstützung für importierte und erstellte Dokumente
- KEINE

ABGLEICH:
- Lokale Datenbankverwaltung für Dokumente | Kern: ja | App: ja | store/project_store.rs (SQLite-basierte Speicherung von Dokumenten und Verlauf)
- Volltextsuche über lokale Ordner erstellen | Kern: ja | App: ja | project/search.rs (Suche in Dokumenten eines Projekts)
- Suchindex automatisch bei Start neu aufbauen | Kern: nein | App: nein | Keine automatische Index-Reparatur oder Neuaufbau-Funktion im Code sichtbar; Suchoperationen sind on-demand (`project/search.rs`)
- Index-Reparatur bei Korruption oder Fehlern | Kern: nein | App: nein | Keine Funktion zur Index-Reparatur vorhanden
- Verwaltung von gelöschten Elementen (Deleted Items) | Kern: ja | App: nein | Snapious hat `delete_document` und `versions`, aber keine explizite "Papierkorb"- oder "Gelöschte Elemente"-Verwaltung wie Geneious; gelöschte Dokumente sind nur über Versionshistorie (`store/versions.rs`) wiederherstellbar, aber nicht als separate Kategorie verwaltet
- Ordner erstellen, umbenennen und verschieben | Kern: ja | App: ja | store/folders.rs (Tabellen `folders`), view/command_search.rs/ops für Umbenennung, move_document in App-Schnittstelle
- Suchergebnisse in lokalen Ordnern speichern | Kern: ja | App: teilweise | `project/search.rs` liefert Ergebnisse, aber keine explizite Funktion "Ergebnisse in Ordner speichern" als Batch-Aktion; manuelle Zuordnung über `create_folder` und `move_document` möglich, aber keine direkte Integration
- Suche nach Teilwörtern mit Wildcards (*) | Kern: ja | App: teilweise | `project/search.rs` erwähnt "Stapelumbenennung und Dublettensuche", aber keine explizite Wildcard-Syntax (`*`) in der Beschreibung; `view/command_search.rs` nutzt "unscharfen Vergleich", was keine exakte Wildcard-Suche ist
- Filterung nach ganzen Wörnern (Whole Words) | Kern: ja | App: nein | Keine explizite Option für "Ganzes Wort" in `project/search.rs` oder `view/command_search.rs` beschrieben
- Anzeige von Dokumenten aus öffentlichen Datenbanken | Kern: ja | App: ja | db/ncbi.rs, db/uniprot.rs (Abruf und Anzeige von NCBI/UniProt-Daten), import_file, ncbi_import, uniprot_import
- Unterstützung für importierte und erstellte Dokumente | Kern: ja | App: ja | create_sequence, import_file, store/project_store.rs
