GENEIOUS-FUNKTIONEN:
- Ausrichtung von Sequenzen oder Alignments mittels Profilen
- Zwei Alignments untereinander mit Profilen ausrichten
- Gap-Kosten für Proteinsequenzen und -alignments festlegen
- Freie Endgaps bei der Ausrichtung erlauben
- Warnung vor Alignments mit duplizierten Taxa anzeigen
- Profil-Ausrichtung durchführen (Profile Alignment)
- Ausrichtung fehlgeschlagen-Fehlermeldung anzeigen
- Plugin-Komponente für Profil-Ausrichtungsoperationen bereitstellen
- Operation zur Ausführung der Profil-Ausrichtung definieren

ABGLEICH:
- Ausrichtung von Sequenzen oder Alignments mittels Profilen | Kern: ja | App: ja | bio/align_tools.rs (Profile verbinden), ops/phase3.rs
- Zwei Alignments untereinander mit Profilen ausrichten | Kern: ja | App: ja | bio/align_tools.rs (Profile verbinden), ops/phase3.rs
- Gap-Kosten für Proteinsequenzen und -alignments festlegen | Kern: ja | App: nein | bio/align.rs (Parameter der Algorithmen, aber keine explizite UI-Funktion "Gap-Kosten setzen" in der Schnittstelle)
- Freie Endgaps bei der Ausrichtung erlauben | Kern: ja | App: ja | bio/align.rs (semiglobal), ops/phase3.rs
- Warnung vor Alignments mit duplizierten Taxa anzeigen | Kern: ja | App: nein | (Keine explizite Warnfunktion in der Schnittstelle, nur `alignment` und `align_documents`)
- Profil-Ausrichtung durchführen (Profile Alignment) | Kern: ja | App: ja | bio/align_tools.rs, ops/phase3.rs
- Ausrichtung fehlgeschlagen-Fehlermeldung anzeigen | Kern: ja | App: nein | view/error_hint.rs (allgemeine Fehlerbehandlung, keine spezifische "Profil-Ausrichtung fehlgeschlagen"-Meldung)
- Plugin-Komponente für Profil-Ausrichtungsoperationen bereitstellen | Kern: ja | App: nein | (Snapious nutzt `ops/registry.rs` und `ops/phase3.rs`, keine separate Plugin-Komponente im Sinne des Geneious-Plugins)
- Operation zur Ausführung der Profil-Ausrichtung definieren | Kern: ja | App: ja | ops/phase3.rs (definiert Alignment-Operationen), ops/registry.rs
