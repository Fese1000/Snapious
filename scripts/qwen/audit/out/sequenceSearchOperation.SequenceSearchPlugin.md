GENEIOUS-FUNKTIONEN:
- Erstellen lokaler BLAST-Datenbanken aus Sequenzlisten
- Ausführen von BLAST-Suchen gegen lokale oder öffentliche Datenbanken
- Annotation von Sequenzen durch Abgleich mit BLAST-Ergebnissen
- Suche nach ähnlichen Sequenzen in lokalen oder öffentlichen Datenbanken
- Verwaltung und Löschung lokaler BLAST-Datenbanken
- Einrichtung von Suchdiensten für lokale oder öffentliche Datenbanken
- Abfrage von DNA-Sequenzen gegen Protein-Datenbanken (blastx/tblastn)
- Abfrage von Protein-Sequenzen gegen Protein-Datenbanken (blastp)
- Abfrage von Nukleotid-Sequenzen gegen Nukleotid-Datenbanken (blastn)
- Suche über die NCBI-Website oder lokale Custom-BLAST-Dienste
- Extrahieren und Übersetzen von Annotationen für BLAST-Suchen
- Filtern von Sequenzen basierend auf Treffer oder Nicht-Treffer

ABGLEICH:
- Erstellen lokaler BLAST-Datenbanken aus Sequenzlisten | Kern: nein | App: nein | Keine Funktion zum Erstellen lokaler BLAST-Datenbanken vorhanden (nur Suchen gegen Projektsequenzen oder Remote)
- Ausführen von BLAST-Suchen gegen lokale oder öffentliche Datenbanken | Kern: ja | App: ja | `bio/blast.rs` (lokales BLAST+), `bio/blast_remote.rs` (NCBI-URL-API)
- Annotation von Sequenzen durch Abgleich mit BLAST-Ergebnissen | Kern: ja | App: nein | Keine automatische Annotation basierend auf BLAST-Treffern implementiert (Features müssen manuell oder via `import_annotations` hinzugefügt werden)
- Suche nach ähnlichen Sequenzen in lokalen oder öffentlichen Datenbanken | Kern: ja | App: ja | `bio/blast.rs`, `bio/blast_remote.rs`, `search.rs` (lokale BLAST-ähnliche Suche)
- Verwaltung und Löschung lokaler BLAST-Datenbanken | Kern: ja | App: nein | Keine Funktionen zur Erstellung, Verwaltung oder Löschung externer BLAST-Datenbank-Verzeichnisse
- Einrichtung von Suchdiensten für lokale oder öffentliche Datenbanken | Kern: ja | App: teilweise | `bio/blast.rs` und `bio/blast_remote.rs` bieten die Schnittstellen, aber keine explizite "Dienstverwaltung"-UI/Funktion wie in Geneious Plugins üblich; eher direkte Aufrufe
- Abfrage von DNA-Sequenzen gegen Protein-Datenbanken (blastx/tblastn) | Kern: ja | App: nein | `bio/blast.rs` listet nur `blastn`, `blastp`; keine Translation-against-Protein-Datenbank-Funktion (blastx/tblastn) erwähnt
- Abfrage von Protein-Sequenzen gegen Protein-Datenbanken (blastp) | Kern: ja | App: ja | `bio/blast.rs` (blastp)
- Abfrage von Nukleotid-Sequenzen gegen Nukleotid-Datenbanken (blastn) | Kern: ja | App: ja | `bio/blast.rs` (blastn)
- Suche über die NCBI-Website oder lokale Custom-BLAST-Dienste | Kern: ja | App: ja | `bio/blast_remote.rs` (NCBI), `bio/blast.rs` (lokales BLAST+)
- Extrahieren und Übersetzen von Annotationen für BLAST-Suchen | Kern: ja | App: nein | Keine spezifische Funktion zum Extrahieren/Übersetzen von Annotationen *für den Zweck* der BLAST-Suche; Translation ist über `bio/genetic_code.rs` und `ops/phase2.rs` allgemein verfügbar, aber nicht als BLAST-Vorbereitungsschritt integriert
- Filtern von Sequenzen basierend auf Treffer oder Nicht-Treffer | Kern: ja | App: nein | Keine Funktion zum Filtern von Dokumenten/Sequenzen basierend auf BLAST-Ergebnisstatus (Treffer/Nicht-Treffer)
