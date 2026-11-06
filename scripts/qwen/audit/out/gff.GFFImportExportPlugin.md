GENEIOUS-FUNKTIONEN:
- Importiert und exportiert GFF-Dokumente (GFF3, Sanger)
- Verarbeitet Features mit Start-/Endpositions-Korrektur bei Fehlern
- Exportiert Sequenzen aus GFF-Dateien in FASTA-Format
- Handhabung von ungültigen Zeichen und URL-Kodierung
- Unterstützung für Nukleotid- und Proteinsequenz-Exportoptionen
- Prüfung auf leere Dateien oder fehlende Feature-Zeilen
- Behandlung von Taxon-Namen und Decoder-Fehlern
- Export in striktem GFF3-Format mit Attributprüfung
- Ignorieren von Zeilen mit mehr als 3 Teilen
- Warnung bei Features ohne gültige Sequenzdaten
- Option zum Importieren trotz ungültiger Zeichen
- Unterstützung für eingebettete Sequenzdaten in GFF

ABGLEICH:
- Importiert und exportiert GFF-Dokumente (GFF3, Sanger) | Kern: ja | App: ja | io/gff3.rs
- Verarbeitet Features mit Start-/Endpositions-Korrektur bei Fehlern | Kern: ja | App: nein | io/gff3.rs (Korrekturlogik nicht explizit genannt, nur Lesen/Schreiben)
- Exportiert Sequenzen aus GFF-Dateien in FASTA-Format | Kern: ja | App: ja | io/gff3_write.rs, io/fasta.rs
- Handhabung von ungültigen Zeichen und URL-Kodierung | Kern: ja | App: nein | io/gff3.rs (Robustheit nicht explizit in Snapious-Liste)
- Unterstützung für Nukleotid- und Proteinsequenz-Exportoptionen | Kern: ja | App: ja | io/gff3_write.rs (Features können DNA/Protein sein)
- Prüfung auf leere Dateien oder fehlende Feature-Zeilen | Kern: ja | App: nein | io/gff3.rs (Fehlerbehandlung nicht explizit als Funktion gelistet)
- Behandlung von Taxon-Namen und Decoder-Fehlern | Kern: ja | App: nein | io/gff3.rs (Spezifische Decoder-Logik nicht sichtbar)
- Export in striktem GFF3-Format mit Attributprüfung | Kern: ja | App: ja | io/gff3_write.rs
- Ignorieren von Zeilen mit mehr als 3 Teilen | Kern: ja | App: nein | io/gff3.rs (Parsing-Regeln nicht explizit gelistet)
- Warnung bei Features ohne gültige Sequenzdaten | Kern: ja | App: nein | io/gff3.rs (Warnmechanismus nicht explizit gelistet)
- Option zum Importieren trotz ungültiger Zeichen | Kern: ja | App: nein | io/gff3.rs (Fehlerbehandlungs-Option nicht explizit gelistet)
- Unterstützung für eingebettete Sequenzdaten in GFF | Kern: ja | App: ja | io/gff3.rs (Features enthalten Sequenzdaten laut model/feature.rs)
