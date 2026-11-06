GENEIOUS-FUNKTIONEN:
- Importiert Sequenzen aus CSV/TSV-Spreadsheets
- Importiert Primer-Daten aus Tabellenkalkulationen
- Verknüpft Metadaten-Spalten mit Dokumentfeldern
- Erstellt Annotationen via 'anno:'-Präfix-Kopfzeilen
- Konvertiert Daten zu DNA-, RNA- oder Proteinsequenzen
- Parst Datumsformate für Dokument-Eigenschaften (z.B. Created)
- Validiert Zellwerte gegen definierte Constraints und Typen
- Zeigt Vorschau der importierten Daten zur Überprüfung an
- Erkennt automatisch Dateierweiterungen für Importtypen
- Übernimmt Sequenzlänge und Typ (linear/circular) aus Daten

ABGLEICH:
- Importiert Sequenzen aus CSV/TSV-Spreadsheets | Kern: nein | App: nein | <fehlt>
- Importiert Primer-Daten aus Tabellenkalkulationen | Kern: nein | App: nein | <fehlt>
- Verknüpft Metadaten-Spalten mit Dokumentfeldern | Kern: nein | App: ja (via `add_features`/`import_annotations`) | io/genbank.rs, io/fasta.rs
- Erstellt Annotationen via 'anno:'-Präfix-Kopfzeilen | Kern: ja | App: ja (via `import_annotations`/`add_features`) | io/gff3.rs, model/feature.rs
- Konvertiert Daten zu DNA-, RNA- oder Proteinsequenzen | Kern: ja | App: ja (via `create_sequence`/`paste`) | model/sequence.rs, io/paste.rs
- Parst Datumsformate für Dokument-Eigenschaften (z.B. Created) | Kern: nein | App: ja (via `import_file`/`open`) | io/genbank.rs, model/document.rs
- Validiert Zellwerte gegen definierte Constraints und Typen | Kern: nein | App: nein | <fehlt>
- Zeigt Vorschau der importierten Daten zur Überprüfung an | Kern: nein | App: ja (via `import_file`/`check_read_file`) | io/fastq.rs, io/fasta.rs
- Erkennt automatisch Dateierweiterungen für Importtypen | Kern: ja | App: ja (via `import_file`/`open`) | io/fasta.rs, io/fastq.rs
- Übernimmt Sequenzlänge und Typ (linear/circular) aus Daten | Kern: ja | App: ja (via `create_sequence`/`open`) | model/document.rs, model/sequence.rs
