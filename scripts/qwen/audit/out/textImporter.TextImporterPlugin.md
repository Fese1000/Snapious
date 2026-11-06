GENEIOUS-FUNKTIONEN:
- Erstellen leerer Textdokumente für Notizen
- Importieren einfacher Textdateien
- Exportieren von Textdokumenten
- Erstellen neuer Textdokumente über Operationen
- Importieren von Textdokumenten via Plugin
- Verarbeiten von HTML-Dokumenten als Text
- Generieren von plain text Dateien
- Anzeige von Notiz-Inhalten im Editor
- Integration in Biomatters Geneious Prime

ABGLEICH:
- Erstellen leerer Textdokumente für Notizen | Kern: ja | App: ja | model/document.rs (Dokument-Erstellung), view/edit_session.rs
- Importieren einfacher Textdateien | Kern: ja | App: ja | io/fasta.rs (als roher Text interpretierbar), io/paste.rs
- Exportieren von Textdokumenten | Kern: ja | App: ja | export/text.rs, view/edit_session.rs
- Erstellen neuer Textdokumente über Operationen | Kern: ja | App: ja | ops/builtin.rs, ops/registry.rs (create_sequence)
- Importieren von Textdokumenten via Plugin | Kern: nein | App: nein | plugin.rs (definiert Struktur, aber kein spezifischer Text-Import-Plugin implementiert)
- Verarbeiten von HTML-Dokumenten als Text | Kern: nein | App: nein | Keine HTML-Parsing-Funktionalität vorhanden
- Generieren von plain text Dateien | Kern: ja | App: ja | export/text.rs, io/fasta.rs
- Anzeige von Notiz-Inhalten im Editor | Kern: ja | App: ja | view/edit_session.rs, model/document.rs (note field)
- Integration in Biomatters Geneious Prime | Kern: nein | App: nein | Snapious ist eine eigenständige Anwendung, keine Plugin-Architektur für Geneious Prime
