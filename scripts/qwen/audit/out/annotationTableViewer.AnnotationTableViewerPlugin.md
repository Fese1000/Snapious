GENEIOUS-FUNKTIONEN:
- Zeigt alle Annotationen ausgewählter Sequenzen in einer Tabelle an.
- Synchronisiert die Auswahl zwischen Tabellenzeilen und Sequenzansicht.
- Filtert Zeilen nach Annotationstypen über den Typen-Button.
- Zeigt oder verbirgt Spalten, die Annotationseigenschaften entsprechen.
- Erlaubt das Neuanordnen und Ändern der Spaltenbreite.
- Exportiert die aktuelle Tabelle als CSV-Datei.
- Extrahiert ausgewählte Annotationen aus den Dokumenten.
- Übersetzt extrahierte Annotationen (z.B. DNA zu Protein).
- Filtert Zeilen basierend auf einzelnen Spaltenwerten.
- Speichert und stellt Sortier- und Filtereinstellungen wieder her.

ABGLEICH:
- Zeigt alle Annotationen ausgewählter Sequenzen in einer Tabelle an | Kern: teilweise | App: nein | `model/feature.rs` (Datenstruktur vorhanden), aber keine tabellarische UI-Ansicht implementiert
- Synchronisiert die Auswahl zwischen Tabellenzeilen und Sequenzansicht | Kern: nein | App: nein | Keine Synchronisationslogik für eine externe Tabelle gefunden
- Filtert Zeilen nach Annotationstypen über den Typen-Button | Kern: teilweise | App: nein | `model/feature.rs` enthält Feature-Typen, aber kein UI-Filter für eine Tabelle
- Zeigt oder verbirgt Spalten, die Annotationseigenschaften entsprechen | Kern: nein | App: nein | Keine dynamische Spaltenauswahl für Annotationen implementiert
- Erlaubt das Neuanordnen und Ändern der Spaltenbreite | Kern: nein | App: nein | Keine Tabellen-UI mit resizebaren Spalten vorhanden
- Exportiert die aktuelle Tabelle als CSV-Datei | Kern: teilweise | App: nein | `export/text.rs` existiert, aber kein spezifischer CSV-Export für Annotationstabellen
- Extrahiert ausgewählte Annotationen aus den Dokumenten | Kern: ja | App: ja | `model/feature.rs` (Features), `ops/more.rs` (Operationen), `cloning/enzymes.rs` etc. für Feature-Daten
- Übersetzt extrahierte Annotationen (z.B. DNA zu Protein) | Kern: ja | App: ja | `bio/genetic_code.rs` (Translation), `ops/more.rs` (Translation-Operationen)
- Filtert Zeilen basierend auf einzelnen Spaltenwerten | Kern: teilweise | App: nein | `project/search.rs` (Suche in Dokumenten), aber keine interaktive Tabellenfilterung
- Speichert und stellt Sortier- und Filtereinstellungen wieder her | Kern: nein | App: nein | Keine Persistenz von Tabellenansichts-Einstellungen (Sortierung/Filter) gefunden
