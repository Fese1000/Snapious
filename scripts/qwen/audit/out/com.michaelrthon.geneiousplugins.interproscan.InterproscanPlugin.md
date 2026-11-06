GENEIOUS-FUNKTIONEN:
- Sucht nach Protein-Domänen mit InterProScan
- Fügt 'goterms'-Option zur Datenbanksuche hinzu
- Fügt 'pathways'-Option zur Datenbsuchsuche hinzu
- Wählt Darstellung der InterPro-Features aus
- Erkennt funktionale Domänen in Proteinen
- Zeigt InterPro-Begriffe als Qualifikatoren an
- Fügt Features ohne Ergebnisse oder Fehler hinzu
- Konfiguriert anzuwendende Anwendungen zum Ausführen
- Zeigt InterPro-Begriffe als Qualifikatoren an

ABGLEICH:
- Sucht nach Protein-Domänen mit InterProScan | Kern: nein | App: nein | Keine externe Datenbankssuche oder Integration von InterPro vorhanden (nur lokale Analyse wie `bio/protein.rs`)
- Fügt 'goterms'-Option zur Datenbanksuche hinzu | Kern: nein | App: nein | Keine GO-Terminologie-Datenbank oder Schnittstelle vorhanden
- Fügt 'pathways'-Option zur Datenbsuchsuche hinzu | Kern: nein | App: nein | Keine Pathway-Datenbank oder Schnittstelle vorhanden
- Wählt Darstellung der InterPro-Features aus | Kern: nein | App: nein | Keine spezifische Feature-Darstellung für InterPro vorhanden (Features sind allgemein in `model/feature.rs`)
- Erkennt funktionale Domänen in Proteinen | Kern: teilweise | App: ja | `bio/protein.rs` berechnet Eigenschaften, aber keine Domänen-Erkennung via Datenbank; `view/feature_detect.rs` erkennt ORFs/Motive, aber keine InterPro-Domänen
- Zeigt InterPro-Begriffe als Qualifikatoren an | Kern: nein | App: nein | Keine Integration von InterPro-Qualifikatoren vorhanden
- Fügt Features ohne Ergebnisse oder Fehler hinzu | Kern: nein | App: nein | Keine automatische Feature-Erstellung basierend auf fehlenden externen Ergebnissen vorhanden
- Konfiguriert anzuwendende Anwendungen zum Ausführen | Kern: teilweise | App: ja | `tools.rs` und `bio/backends.rs` verwalten externe Programme, aber keine spezifische Konfiguration für InterPro-Anwendungspipelines
- Zeigt InterPro-Begriffe als Qualifikatoren an | Kern: nein | App: nein | Duplikat der oben genannten Funktion, keine Implementierung vorhanden
