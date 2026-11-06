GENEIOUS-FUNKTIONEN:
- Exportiert Tabellenwerte im CSV-Format (kommagetrennt)
- Exportiert Tabellenwerte im TSV-Format (tabulatorgetrennt)
- Bietet Option zum Komprimieren der exportierten Tabellen als ZIP
- Exportiert Felder aus Sequenzlisten-Dokumenten (eine Zeile pro Liste)
- Exportiert Felder aus einzelnen Sequenzen (mehrere Zeilen pro Liste)
- Kann zugehörige Sequenzen in JSON-Daten exportieren
- Fügt Spalte „geneious_row_index“ zur CSV-Datei hinzu
- Erlaubt Auswahl spezifischer zu exportierender Felder
- Exportiert vollständige Dokumenttabellen für nicht-Sequenz-Dokumente
- Begrenzt die Anzahl exportierter zugehöriger Sequenzen pro Zeile

ABGLEICH:
- Exportiert Tabellenwerte im CSV-Format (kommagetrennt) | Kern: nein | App: ja | export_document
- Exportiert Tabellenwerte im TSV-Format (tabulatorgetrennt) | Kern: nein | App: ja | export_document
- Bietet Option zum Komprimieren der exportierten Tabellen als ZIP | Kern: nein | App: nein | <fehlt>
- Exportiert Felder aus Sequenzlisten-Dokumenten (eine Zeile pro Liste) | Kern: ja | App: ja | export_document
- Exportiert Felder aus einzelnen Sequenzen (mehrere Zeilen pro Liste) | Kern: ja | App: ja | export_document
- Kann zugehörige Sequenzen in JSON-Daten exportieren | Kern: ja | App: ja | export_document
- Fügt Spalte „geneious_row_index“ zur CSV-Datei hinzu | Kern: nein | App: ja | export_document (Index kann durch `documents` oder `selection_stats` abgeleitet werden, aber keine explizite Spaltenbenennung im Export-Modul genannt)
- Erlaubt Auswahl spezifischer zu exportierender Felder | Kern: ja | App: ja | export_document
- Exportiert vollständige Dokumenttabellen für nicht-Sequenz-Dokumente | Kern: ja | App: ja | export_document
- Begrenzt die Anzahl exportierter zugehöriger Sequenzen pro Zeile | Kern: nein | App: ja | export_document (Logik liegt in der Implementierung von `export_document`, aber keine explizite Begrenzungsfunktion im Code-Listing genannt)
