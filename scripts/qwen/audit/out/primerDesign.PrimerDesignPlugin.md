GENEIOUS-FUNKTIONEN:
- Designen neuer Primer und Sonden für Sequenzen
- Testen existierender Primer auf Bindung an Zielregionen
- Konvertieren von Sequenzen in Oligo-Dokumente (Primer)
- Berechnen thermodynamischer Eigenschaften von Primern
- Suchen passender Primer aus der lokalen Datenbank
- Testen auf unspezifische Bindung (Off-target Sites)
- Erstellen von Sequenzier-Primern für lange DNA
- Hinzufügen von Primer-Annotationen an Sequenzen
- Analysieren von Primer-Paarungen und Überlappungen
- Handhaben degenerierter Primer mit unerkennbaren Zeichen
- Prüfen auf Haarnadeln und Primer-Dimer-Bildung
- Generieren von Erweiterungen (Extensions) an Primern

ABGLEICH:
- Designen neuer Primer und Sonden für Sequenzen | Kern: ja | App: ja | primer/picker.rs (pick_primers)
- Testen existierender Primer auf Bindung an Zielregionen | Kern: ja | App: ja | primer/checks.rs (Prüfung der Bindungseigenschaften, implizit über checks)
- Konvertieren von Sequenzen in Oligo-Dokumente (Primer) | Kern: ja | App: nein | primer/picker.rs erstellt Primer, aber keine explizite "Konvertierung in Oligo-Dokument" als separater App-Befehl genannt (nur pick_primers)
- Berechnen thermodynamischer Eigenschaften von Primern | Kern: ja | App: ja | primer/tm.rs (Tm-Berechnung)
- Suchen passender Primer aus der lokalen Datenbank | Kern: ja | App: ja | primer/picker.rs (Auswahl/Picker)
- Testen auf unspezifische Bindung (Off-target Sites) | Kern: ja | App: teilweise | primer/checks.rs prüft spezifische Eigenschaften, aber keine explizite "Off-target"-Suche gegen Genom im App-Befehlssatz genannt (search.rs ist BLAST-ähnlich, aber nicht primär für Primer-Off-target) -> Streng: nein (da keine spezifische App-Funktion "test_off_target" genannt, nur checks.rs)
- Erstellen von Sequier-Primern für lange DNA | Kern: ja | App: ja | primer/picker.rs (Primer-Auswahl für Zielfragment)
- Hinzufügen von Primer-Annotationen an Sequenzen | Kern: ja | App: ja | primer/annotate.rs (annotate_features)
- Analysieren von Primer-Paarungen und Überlappungen | Kern: ja | App: teilweise | primer/checks.rs prüft Dimer/Haarnadel, aber keine explizite "Paarungsanalyse" als App-Befehl (overlap_primers.rs existiert, aber kein passender App-Befehl wie "analyze_pair" genannt) -> Streng: nein
- Handhaben degenerierter Primer mit unerkennbaren Zeichen | Kern: ja | App: ja | primer/picker.rs / primer/checks.rs (IUPAC-Codes werden in motif.rs und cloning/enzymes.rs behandelt, primer/picker.rs nutzt diese)
- Prüfen auf Haarnadeln und Primer-Dimer-Bildung | Kern: ja | App: ja | primer/checks.rs (checks.rs explizit genannt)
- Generieren von Erweiterungen (Extensions) an Primern | Kern: ja | App: ja | primer/cloning_primers.rs (Klonierungsprimer mit Überhang/Extension)
