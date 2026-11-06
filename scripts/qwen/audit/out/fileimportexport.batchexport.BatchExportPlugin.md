GENEIOUS-FUNKTIONEN:
- Exportiert mehrere Dokumente gleichzeitig in separate Dateien
- Exportiert alle Dokumente in eine einzige Ausgabedatei
- Überwacht und ersetzt illegale Zeichen in Dateinamen durch Unterstriche
- Prüft auf doppelte Dateinamen und überschreibt bestehende Dateien
- Validiert den Exportpfad auf Existenz und Schreibberechtigung
- Prüft die Einzigartigkeit der Dokumentennamen vor dem Export
- Unterstützt das Geneious-eigene Dateiformat für den Export

ABGLEICH:
- Exportiert mehrere Dokumente gleichzeitig in separate Dateien | Kern: ja | App: ja | export_document (muss im Workflow manuell wiederholt werden)
- Exportiert alle Dokumente in eine einzige Ausgabedatei | Kern: ja | App: nein | Keine Funktion zum Exportieren aller Dokumente in eine einzige Datei vorhanden
- Überwacht und ersetzt illegale Zeichen in Dateinamen durch Unterstriche | Kern: ja | App: nein | Keine automatische Bereinigung von Dateinamen beim Export implementiert
- Prüft auf doppelte Dateinamen und überschreibt bestehende Dateien | Kern: ja | App: nein | Keine explizite Prüfung auf doppelte Dateinamen oder Überschreiblogik im Export-Modus vorhanden
- Validiert den Exportpfad auf Existenz und Schreibberechtigung | Kern: ja | App: nein | Keine explizite Pfadvalidierung vor dem Export sichtbar
- Prüft die Einzigartigkeit der Dokumentennamen vor dem Export | Kern: ja | App: nein | Keine Prüfung auf Einzigartigkeit der Ziel-Dateinamen vor dem Export implementiert
- Unterstützt das Geneious-eigene Dateiformat für den Export | Kern: nein | App: nein | Snapious unterstützt keine Geneious-eigenen Dateiformate (`.dna` wird nur gelesen, nicht als Exportziel für Geneious-Format unterstützt)
