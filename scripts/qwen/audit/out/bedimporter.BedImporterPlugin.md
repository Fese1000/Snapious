GENEIOUS-FUNKTIONEN:
- Importiert BED-Annotationsdateien in Geneious Prime
- Exportiert Sequenzdokumente im FASTA-Format
- Unterstützt den Import von Genen, mRNA und CDS-Typen
- Konvertiert andere Annotationen als misc_feature Typen
- Beschränkt Scores auf legale Werte (0 - 1000)
- Zeigt alle Features als misc_feature Annotationen an
- Erstellt Hauptintervall und dickes Intervall als separate Annotationen
- Nutzt eine dritte Annotation für Blöcke bei bestimmten Typen
- Prüft auf doppelte Sequenznamen vor dem Export
- Zeigt Fehlermeldungen bei nicht umbenannten Duplikaten an
- Bietet Batch-Umbenennung zur Lösung von Namenskonflikten
- Stellt Unterstützung für den Import und Export von BED-Dateien bereit

ABGLEICH:
- Importiert BED-Annotationsdateien in Geneious Prime | Kern: ja | App: ja | io/bed.rs
- Exportiert Sequenzdokumente im FASTA-Format | Kern: ja | App: ja | io/fasta.rs
- Unterstützt den Import von Genen, mRNA und CDS-Typen | Kern: ja | App: ja | io/gff3.rs (GFF3 enthält diese Feature-Typen; GenBank/EMBL ebenfalls möglich)
- Konvertiert andere Annotationen als misc_feature Typen | Kern: ja | App: ja | io/gff3.rs (Umwandlung in Features)
- Beschränkt Scores auf legale Werte (0 - 1000) | Kern: ja | App: nein | Keine spezifische Logik zur Score-Beschränkung auf 0-1000 in den genannten Dateien gefunden
- Zeigt alle Features als misc_feature Annotationen an | Kern: ja | App: teilweise | io/gff3.rs (Features werden importiert, aber keine explizite Konvertierung *aller* zu `misc_feature` als Standardanzeige beschrieben, eher Typ-Beibehaltung)
- Erstellt Hauptintervall und dickes Intervall als separate Annotationen | Kern: ja | App: nein | Keine spezifische Logik zur Aufteilung in "Haupt-/Dickes Intervall" als separate Annotationen beim Import gefunden
- Nutzt eine dritte Annotation für Blöcke bei bestimmten Typen | Kern: ja | App: nein | Keine spezifische Logik zur Nutzung einer dritten Annotation für Block-Blöcke gefunden
- Prüft auf doppelte Sequenznamen vor dem Export | Kern: ja | App: teilweise | project/search.rs (Dublettensuche existiert, aber keine explizite "Prüfung vor Export"-Logik im Export-Modul selbst genannt; eher eine allgemeine Suchfunktion)
- Zeigt Fehlermeldungen bei nicht umbenannten Duplikaten an | Kern: ja | App: teilweise | project/search.rs (Dublettensuche vorhanden; view/error_hint.rs für Fehlermeldungen, aber keine explizite Verknüpfung "Export -> Duplikatsprüfung -> Fehler" in der Funktionsliste)
- Bietet Batch-Umbenennung zur Lösung von Namenskonflikten | Kern: ja | App: ja | project/search.rs (Stapelumbenennung)
