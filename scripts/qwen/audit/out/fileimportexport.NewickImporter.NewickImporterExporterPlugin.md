GENEIOUS-FUNKTIONEN:
- Importiert Newick-Bäume aus Dateien
- Exportiert Bäume im Newick-Format
- Zeigt Newick-Bäume mit Stützwerten an
- Prüft, ob Datei Bäume enthält
- Zeigt Dateianfang bei leeren Dateien an

ABGLEICH:
- Importiert Newick-Bäume aus Dateien | Kern: ja | App: ja | bio/newick.rs, open
- Exportiert Bäume im Newick-Format | Kern: ja | App: ja | bio/newick.rs, export_document
- Zeigt Newick-Bäume mit Stützwerten an | Kern: nein | App: ja | tree_layout (Layout/Anzeige vorhanden, aber explizite Stützwert-Anzeige im Text nicht spezifiziert, jedoch "tree_layout" und "tree_newick" vorhanden; da "Stützwerte" spezifisch sind und nicht explizit genannt, streng genommen nein für die *genaue* Funktion, aber tree_layout deckt die Anzeige ab. Da die Anforderung "streng: im Zweifel nein" lautet und Stützwerte nicht explizit in der Funktionsliste von bio/newick.rs oder view/tree_layout.rs als *Feature* genannt werden, sondern nur das Layout allgemein: **nein** für die spezifische Anzeige von Stützwerten, ja für das Anzeigen des Baumes. Ich setze "teilweise" oder "nein". Da Newick Stützwerte enthalten kann und tree_layout sie rendern *könnte*, aber der Text sagt nur "Baumlayout... und SVG-Ausgabe", fehlt die explizite Nennung der Stützwert-Visualisierung. Ich entscheide mich für **teilweise**, da die Datenstruktur (Newick) und das Layout existieren, aber die spezifische UI-Logik für Stützwerte nicht explizit als Funktion benannt ist. Alternativ: **nein**, da "Stützwerte anzeigen" eine spezifische UI-Forderung ist, die nicht im Text steht. Ich bleibe streng: **nein** für die spezifische Funktion "Anzeigen von Stützwerten", da nur "Baumlayout" genannt wird.
- Prüft, ob Datei Bäume enthält | Kern: ja | App: ja | bio/newick.rs (Lesen/Prüfen impliziert), open
- Zeigt Dateianfang bei leeren Dateien an | Kern: nein | App: ja | open (Fehlerbehandlung/Info bei leerer Datei ist Teil des Öffnungsprozesses, aber keine explizite Funktion "show_start_of_empty_file". Ich setze **nein**, da es nicht explizit als Funktion aufgeführt ist, sondern Teil der allgemeinen Fehlerbehandlung sein könnte).

Korrektur/Verfeinerung basierend auf strikter Auslegung:
1. Import Newick: `bio/newick.rs` (lesen), App: `open`. -> ja | ja
2. Export Newick: `bio/newick.rs` (schreiben), App: `export_document`. -> ja | ja
3. Zeigt Stützwerte an: Der Text sagt zu `bio/newick.rs`: "Bäume lesen und schreiben." Zu `view/tree_layout.rs`: "Baumlayout... und SVG-Ausgabe." Es wird nicht explizit gesagt, dass Stützwerte (Support Values) *angezeigt* werden. Daher: **nein** für die spezifische UI-Funktion, auch wenn die Datenstruktur es erlaubt.
4. Prüft ob Datei Bäume enthält: `bio/newick.rs` "lesen" impliziert Parsing/Prüfung. App: `open`. -> ja | ja
5. Zeigt Dateianfang bei leeren Dateien: Nicht explizit als Funktion genannt. `open` ist der Aufruf. -> nein | ja (da `open` existiert, aber die spezifische Logik "Zeige Anfang" nicht als Funktion benannt ist).

Ergebnis:
- Importiert Newick-Bäume aus Dateien | Kern: ja | App: ja | bio/newick.rs, open
- Exportiert Bäume im Newick-Format | Kern: ja | App: ja | bio/newick.rs, export_document
- Zeigt Newick-Bäume mit Stützwerten an | Kern: nein | App: ja | view/tree_layout (Layout vorhanden, aber spezifische Stützwert-Anzeige nicht explizit genannt)
- Prüft, ob Datei Bäume enthält | Kern: ja | App: ja | bio/newick.rs, open
- Zeigt Dateianfang bei leeren Dateien an | Kern: nein | App: ja | open (allgemeine Fehlerbehandlung, keine spezifische Funktion)
