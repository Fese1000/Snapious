GENEIOUS-FUNKTIONEN:
- Exportiert Sequenzdokumente im FASTA-Format
- Stellt Unterstützung für QUAL-Dateiexport bereit
- Exportiert Sequenzqualitätsdaten als QUAL-Datei
- Prüft, ob Sequenzen Qualitätsinformationen enthalten
- Verhindert Export ohne Qualitätsdaten
- Zeigt Warnung bei fehlenden Qualitätsinfos an

ABGLEICH:
- Exportiert Sequenzdokumente im FASTA-Format | Kern: ja | App: ja | io/fasta.rs (export_document)
- Stellt Unterstützung für QUAL-Dateiexport bereit | Kern: nein | App: nein | QUAL-Export ist nicht implementiert (nur Import/Lesen über io/quality.rs und io/abif.rs)
- Exportiert Sequenzqualitätsdaten als QUAL-Datei | Kern: nein | App: nein | QUAL-Export ist nicht implementiert
- Prüft, ob Sequenzen Qualitätsinformationen enthalten | Kern: ja | App: nein | Keine direkte Funktion zum Prüfen auf Vorhandensein von Qual-Daten in der API gelistet (view/seq_layout.rs berechnet Layout, aber keine explizite "has_quality"-Prüf-Funktion in der Schnittstellenliste)
- Verhindert Export ohne Qualitätsdaten | Kern: ja | App: nein | Keine explizite Export-Validierungsfunktion in der API gelistet
- Zeigt Warnung bei fehlenden Qualitätsinfos | Kern: ja | App: nein | view/error_hint.rs existiert, aber keine spezifische Warnlogik für QUAL-Mangel in der API gelistet
