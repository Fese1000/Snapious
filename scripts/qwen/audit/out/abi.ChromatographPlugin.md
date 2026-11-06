GENEIOUS-FUNKTIONEN:
- Importiert und exportiert Chromatogramme (Traces)
- Liest ABI-Dateien ein (ChromatographPluginDocument)
- Verarbeitet SCF-Datenformate (SCFPluginDocument)
- Handelt CAF-Zusammenführungsdaten (CAFDocumentImporter)
- Zeigt Rohdaten-Traces bei Parsing-Fehlern an
- Korrigiert ungültige Chromatogramme (InvalidChromatogramFixer)
- Exportiert Original-Chromatogramm bei Sequenzänderungen
- Prüft auf ungültige Datenobjekttypen und -daten
- Validiert BaseQuality-Sektionen und Lesernamen
- Prüft Speicherfreiheit beim Importieren großer Dokumente
- Erkennt fehlende Graphwerte für Nukleotidzustände
- Prüft Directory-Einträge auf korrekte Datentypen

ABGLEICH:
- Importiert und exportiert Chromatogramme (Traces) | Kern: teilweise | App: ja | io/abif.rs (liest .ab1), export_document/export_text (kein spezifischer Chromatogramm-Export genannt, nur Text/FASTA)
- Liest ABI-Dateien ein (ChromatographPluginDocument) | Kern: ja | App: ja | io/abif.rs (liest Basen, Qualitäten, Peakpositionen)
- Verarbeitet SCF-Datenformate (SCFPluginDocument) | Kern: nein | App: nein | Keine SCF-Unterstützung in der Liste gefunden
- Handelt CAF-Zusammenführungsdaten (CAFDocumentImporter) | Kern: nein | App: nein | Keine CAF-Unterstützung in der Liste gefunden
- Zeigt Rohdaten-Traces bei Parsing-Fehlern an | Kern: nein | App: nein | view/error_hint.rs zeigt Fehlermeldungen, aber keine Rohdaten-Traces-Anzeige spezifiziert
- Korrigiert ungültige Chromatogramme (InvalidChromatogramFixer) | Kern: nein | App: nein | Keine automatische Korrektur von Chromatogramm-Datenstrukturen beschrieben
- Exportiert Original-Chromatogramm bei Sequenzänderungen | Kern: nein | App: nein | Kein Mechanismus zum Exportieren des Original-Chromatogramms neben der Sequenz beschrieben
- Prüft auf ungültige Datenobjekttypen und -daten | Kern: teilweise | App: ja | io/abif.rs (Validierung beim Lesen), view/error_hint.rs (Fehlerbehandlung)
- Validiert BaseQuality-Sektionen und Lesernamen | Kern: ja | App: ja | io/abif.rs (liest Qualitäten), io/fastq.rs (Qualität Phred+33)
- Prüft Speicherfreiheit beim Importieren großer Dokumente | Kern: nein | App: nein | Keine explizite Speicherkapazitätsprüfung vor Import beschrieben (nur "Strom lesen" bei FASTQ)
- Erkennt fehlende Graphwerte für Nukleotidzustände | Kern: ja | App: teilweise | io/abif.rs (liest Peakpositionen und Spuren; Fehlerbehandlung impliziert, aber keine spezifische "fehlende Graphwerte"-Prüfung als Feature genannt)
- Prüft Directory-Einträge auf korrekte Datentypen | Kern: ja | App: ja | io/abif.rs (Validierung beim Einlesen), db/cache.rs, store/project_store.rs (Datenintegrität)
