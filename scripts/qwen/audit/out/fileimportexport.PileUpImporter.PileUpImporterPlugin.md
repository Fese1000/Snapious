GENEIOUS-FUNKTIONEN:
- Importiert PileUp-Dateien in Geneious Prime
- Bietet PileUp-Import-Funktionalität über Plugin
- Erstellt Dokumentobjekte aus PileUp-Daten

ABGLEICH:
- Importiert PileUp-Dateien in Geneious Prime | Kern: nein | App: nein | Datei `io/pileup.rs` existiert nicht in der Liste; Snapious unterstützt Clustal und PHYLIP (`io/alignment_formats.rs`), aber kein PileUp-Format.
- Bietet PileUp-Import-Funktionalität über Plugin | Kern: nein | App: nein | Snapious hat ein Plugin-System (`plugin.rs`), aber keine spezifische Implementierung oder Datei für PileUp-Import ist in der Funktionsübersicht enthalten.
- Erstellt Dokumentobjekte aus PileUp-Daten | Kern: nein | App: nein | Snapious erstellt Dokumente über `create_sequence` oder `import_file`, aber es gibt keine Funktion, die spezifisch aus PileUp-Daten ein Dokumentobjekt erstellt.
