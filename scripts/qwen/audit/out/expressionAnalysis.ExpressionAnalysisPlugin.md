GENEIOUS-FUNKTIONEN:
- Berechnung von Expressionsniveaus (RPKM, FPKM, TPM)
- Durchführung differentieller Expressionsanalyse mittels DESeq2
- Generierung von Volcano Plots zur Visualisierung signifikanter Gene
- Erstellung von PCA-Diagrammen zur Mustererkennung in Proben
- Zuweisung von Proben zu experimentellen Bedingungen (A/B)
- Normalisierung roher Zählwerte nach Transkriptlänge und Tiefe
- Handhabung mehrdeutig gemappter Reads (partial/full matches)
- Anzeige von Annotations-Eigenschaften wie Rohtranskriptzählung
- Export von Ergebnistabellen im CSV-Format für weitere Analyse
- Visualisierung von Log2-Ratios und absoluten Konfidenzwerten

ABGLEICH:
- Berechnung von Expressionsniveaus (RPKM, FPKM, TPM) | Kern: nein | App: nein | Keine Datei vorhanden (fehlende Berechnung von Transkriptlängen-basierter Normalisierung)
- Durchführung differentieller Expressionsanalyse mittels DESeq2 | Kern: nein | App: nein | Keine Datei vorhanden (fehlende statistische Analyse und externe Integration von DESeq2)
- Generierung von Volcano Plots zur Visualisierung signifikanter Gene | Kern: nein | App: nein | Keine Datei vorhanden (fehlende Plot-Generierung für statistische Signifikanz)
- Erstellung von PCA-Diagrammen zur Mustererkennung in Proben | Kern: nein | App: nein | Keine Datei vorhanden (fehlende Hauptkomponentenanalyse für Probenclustering)
- Zuweisung von Proben zu experimentellen Bedingungen (A/B) | Kern: nein | App: nein | Keine Datei vorhanden (fehlende Metadaten-Verwaltung für experimentelle Gruppen)
- Normalisierung roher Zählwerte nach Transkriptlänge und Tiefe | Kern: nein | App: nein | Keine Datei vorhanden (fehlende Normalisierungsalgorithmen für RNA-Seq-Daten)
- Handhabung mehrdeutig gemappter Reads (partial/full matches) | Kern: teilweise | App: ja | `ngs/mapper.rs`, `ngs/pileup.rs` (Mapping und Variantenerkennung vorhanden, aber keine spezifische Handhabung mehrdeutiger Zuweisung für Expressionsanalyse)
- Anzeige von Annotations-Eigenschaften wie Rohtranskriptzählung | Kern: nein | App: ja | `model/feature.rs`, `io/genbank.rs` (Features und Annotationen lesbar, aber keine spezifische Anzeige von Rohtranskriptzählungen)
- Export von Ergebnistabellen im CSV-Format für weitere Analyse | Kern: teilweise | App: ja | `export/text.rs` (Textausgabe möglich, aber kein spezifischer CSV-Export für statistische Ergebnisse)
- Visualisierung von Log2-Ratios und absoluten Konfidenzwerten | Kern: nein | App: nein | Keine Datei vorhanden (fehlende Visualisierungskomponente für statistische Werte)
