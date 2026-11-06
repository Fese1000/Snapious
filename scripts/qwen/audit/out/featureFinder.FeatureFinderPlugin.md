GENEIOUS-FUNKTIONEN:
- Annotiert Sequenzen basierend auf Ähnlichkeit zu einer Datenbank
- Sucht nach Gateway-Rekombinationsstellen (AttB, AttP, AttR, AttL)
- Vergleicht Sequenzen mit Nukleotid-Referenzdatenbanken
- Vergleicht translatierte Sequenzen mit Protein-Datenbanken (6 Frames)
- Übertrift Annotationen von Quellsequenzen auf Zielsequenzen
- Passt CDS-Grenzen an nächste gültige ORFs an
- Ignoriert oder berücksichtigt überlappende Annotationen vom selben Typ
- Indiziert Sequenzen zur schnellen Suche nach Wortübereinstimmungen
- Filtert Annotationen nach Typ oder ignoriert spezifische Typen
- Behandelt unannotierte Sequenzen als "Misc Feature" bei Bedarf

ABGLEICH:
- Annotiert Sequenzen basierend auf Ähnlichkeit zu einer Datenbank | Kern: nein | App: nein | Keine automatische Annotation durch DB-Ähnlichkeit (FeatureDetect erkennt nur ORFs/Motive lokal, keine DB-Suche)
- Sucht nach Gateway-Rekombinationsstellen (AttB, AttP, AttR, AttL) | Kern: ja | App: ja | cloning/gateway.rs (BP- und LR-Reaktion über att-Stellen)
- Vergleicht Sequenzen mit Nukleotid-Datenbanken | Kern: ja | App: ja | db/ncbi.rs (NCBI Entrez), io/fasta.rs, bio/blast_remote.rs
- Vergleicht translatierte Sequenzen mit Protein-Datenbanken (6 Frames) | Kern: ja | App: teilweise | bio/blast_remote.rs (BLASTp), bio/genetic_code.rs (Translation), aber keine integrierte 6-Frame-Protein-Datenbank-Suche wie im Plugin beschrieben
- Übertrift Annotationen von Quellsequenzen auf Zielsequenzen | Kern: ja | App: nein | Keine automatische Annotation-Übertragungsfunktion; manuelles Kopieren von Features ist nicht als spezifische Operation gelistet
- Passt CDS-Grenzen an nächste gültige ORFs an | Kern: ja | App: nein | FeatureDetect findet ORFs, aber keine automatische Anpassung von CDS-Grenzen an diese
- Ignoriert oder berücksichtigt überlappende Annotationen vom selben Typ | Kern: ja | App: teilweise | view/feature_layout.rs (Verteilung auf Spuren, damit sie sich nicht überdecken), aber keine explizite Logik zum Ignorieren/Bevorzugen von Overlaps im Modell
- Indiziert Sequenzen zur schnellen Suche nach Wortübereinstimmungen | Kern: ja | App: teilweise | project/search.rs (lokale Suche), db/cache.rs (Zwischenspeicher), aber kein dedizierter "Index" für Wortübereinstimmungen wie im Plugin
- Filtert Annotationen nach Typ oder ignoriert spezifische Typen | Kern: ja | App: teilweise | view/feature_layout.rs (Filterung für Anzeige), ops/builtin.rs (Bearbeiten), aber keine explizite "Ignore/Include Type"-Logik im Plugin-Stil
- Behandelt unannotierte Sequenzen als "Misc Feature" bei Bedarf | Kern: ja | App: nein | Keine automatische Zuordnung zu "Misc Feature" für unannotierte Sequenzen
