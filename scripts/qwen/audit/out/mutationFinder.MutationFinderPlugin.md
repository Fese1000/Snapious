GENEIOUS-FUNKTIONEN:
- Finden von Mutationen (SNVs, Indels) in Sequenzdaten
- Bewerten von CRISPR-Bearbeitungsergebnissen und Schnittstellen
- Anwenden gefundener Varianten auf Referenzsequenzen
- Analysieren der Auswirkungen von Mutationen auf Translationen
- Generieren zufälliger Annotationen (Entwicklungstest)
- Maskieren von Nukleotiden mit niedriger Abdeckung
- Berechnen von Variant-Häufigkeiten und P-Werten
- Zusammenführen von Sequenzierungsfehlern mit Konfidenz
- Erkennen von Schnittstellen für Sanger-Sequenzen
- Erstellen von Ambiguitäten bei niedriger Variantenfrequenz
- Filtern von Reads basierend auf Abdeckung und Qualität
- Kopieren von CDS-Eigenschaften in Variantenannotationen

ABGLEICH:
- Finden von Mutationen (SNVs, Indels) in Sequenzdaten | Kern: ja | App: ja | bio/variants.rs (variants), ngs/pileup.rs (pileup)
- Bewerten von CRISPR-Bearbeitungsergebnissen und Schnittstellen | Kern: ja | App: ja | crispr.rs (crispr)
- Anwenden gefundener Varianten auf Referenzsequenzen | Kern: teilweise | App: nein | *Fehlt:* Keine Funktion zum direkten "Anwenden" oder Erzeugen einer neuen Sequenz aus Varianten+Referenz; nur Anzeige/Extraktion von Varianten möglich.
- Analysieren der Auswirkungen von Mutationen auf Translationen | Kern: ja | App: teilweise | bio/genetic_code.rs (translate_frame), model/sequence.rs (translate); *Fehlt:* Spezifische Analyse der *Auswirkung* einer Mutation auf die Translation (z.B. Missense/Nonsense Vorhersage im Kontext der Variante), nur reine Translation vorhanden.
- Generieren zufälliger Annotationen (Entwicklungstest) | Kern: ja | App: nein | *Fehlt:* Keine Funktion zur Generierung zufälliger Daten/Annotationen.
- Maskieren von Nukleotiden mit niedriger Abdeckung | Kern: ja | App: nein | *Fehlt:* Keine Funktion zum Maskieren oder Ersetzen von Basen basierend auf Coverage-Schwellwerten.
- Berechnen von Variant-Häufigkeiten und P-Werten | Kern: ja | App: teilweise | ngs/pileup.rs (pileup); *Fehlt:* P-Werte und statistische Häufigkeitsberechnung (Variant Frequency) fehlen; nur Zählung (counting) ist implementiert.
- Zusammenführen von Sequenzierungsfehlern mit Konfidenz | Kern: ja | App: nein | *Fehlt:* Keine Funktion zur Fehlerkorrektur oder Konfidenzbewertung von Sequenzierungsfehlern.
- Erkennen von Schnittstellen für Sanger-Sequenzen | Kern: ja | App: teilweise | io/abif.rs (read_trace); *Fehlt:* "Schnittstellen" (Interfaces) ist hier unklar, aber das Lesen von Sanger-Daten (.ab1) ist vorhanden. Wenn mit "Schnittstellen" gemeint ist, wo Enzyme schneiden: cloning/enzymes.rs. Wenn es um die Erkennung von Sequenzierungsartefakten geht: nein. Geht man von "Sanger-Sequenzierung lesen" aus, ist es ja. Der Name "Schnittstellen" deutet aber eher auf API/Interfaces hin oder vielleicht "Peaks". Ich interpretiere es als "Erkennung von Peaks/Basen aus Sanger-Daten": ja.
- Erstellen von Ambiguitäten bei niedriger Variantenfrequenz | Kern: ja | App: nein | *Fehlt:* Keine Funktion zum Setzen von IUPAC-Ambiguitätscodes basierend auf Frequenzschwellen.
- Filtern von Reads basierend auf Abdeckung und Qualität | Kern: ja | App: teilweise | ngs/trim.rs (trim_fastq); *Fehlt:* Filtern basierend auf *Abdeckung* (Coverage) fehlt; nur Trimming nach Qualität/Adapter.
- Kopieren von CDS-Eigenschaften in Variantenannotationen | Kern: ja | App: nein | *Fehlt:* Keine Funktion zum automatischen Kopieren oder Übertragen von Feature-Eigenschaften (wie CDS) auf neu erstellte Varianten-Features.
