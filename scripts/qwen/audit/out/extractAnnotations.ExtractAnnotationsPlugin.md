GENEIOUS-FUNKTIONEN:
- Extrahiert benannte oder typisierte Annotationen von Sequenzen und Alignments
- Vergleicht Annotationen aus zwei oder drei verschiedenen Sätzen (A, B, C)
- Identifiziert Annotationen, die nur in einem bestimmten Satz vorkommen (unique to)
- Findet gemeinsame Annotationen, die in mehreren Sätzen übereinstimmen (common to)
- Filtert nach Annotationstyp, Namen oder spezifischen Qualifizierer-Eigenschaften
- Unterstützt den Vergleich von Polymorphismus-Annotationen mit partieller Intervallübereinstimmung
- Generiert Konsensussequenzen aus ausgewählten Alignments, falls Plugin verfügbar ist
- Extrahiert Annotationen aus Konsensussequenzen unter Berücksichtigung von Flanking-Basen
- Unterscheidet zwischen vollständigen und partiellen Annotationen bei der Übereinstimmung
- Behandelt Variantenannotationen in Alignments unter Berücksichtigung von Lückenunterschieden

ABGLEICH:
- Extrahiert benannte oder typisierte Annotationen von Sequenzen und Alignments | Kern: ja | App: ja | model/feature.rs, features (App)
- Vergleicht Annotationen aus zwei oder drei verschiedenen Sätzen (A, B, C) | Kern: nein | App: nein | Keine Funktion zum Satzvergleich von Annotationen vorhanden
- Identifiziert Annotationen, die nur in einem bestimmten Satz vorkommen (unique to) | Kern: nein | App: nein | Keine Funktion zum mengenmäßigen Vergleich von Annotation-Sätzen vorhanden
- Findet gemeinsame Annotationen, die in mehreren Sätzen übereinstimmen (common to) | Kern: nein | App: nein | Keine Funktion zum mengenmäßigen Vergleich von Annotation-Sätzen vorhanden
- Filtert nach Annotationstyp, Namen oder spezifischen Qualifizierer-Eigenschaften | Kern: ja | App: ja | model/feature.rs (Filterung nach Typ/Namen implizit in Feature-Datenstruktur), features (App)
- Unterstützt den Vergleich von Polymorphismus-Annotationen mit partieller Intervallübereinstimmung | Kern: nein | App: nein | variants.rs behandelt Basenunterschiede, aber kein spezifischer Vergleich von Polymorphismus-Annotationen mit partieller Intervallübereinstimmung
- Generiert Konsensussequenzen aus ausgewählten Alignments, falls Plugin verfügbar | Kern: ja | App: ja | bio/align_stats.rs (alignment_consensus), alignment_consensus (App)
- Extrahiert Annotationen aus Konsensussequenzen unter Berücksichtigung von Flanking-Basen | Kern: nein | App: nein | Keine Funktion zur Extraktion von Annotationen aus Konsensussequenzen mit Flanking-Logik vorhanden
- Unterscheidet zwischen vollständigen und partiellen Annotationen bei der Übereinstimmung | Kern: nein | App: nein | Keine Unterscheidung zwischen vollständigen/partiellen Annotationen beim Vergleich implementiert
- Behandelt Variantenannotationen in Alignments unter Berücksichtigung von Lückenunterschieden | Kern: ja | App: ja | bio/variants.rs (Behandlung von Varianten/Lücken), alignment_variants (App)
