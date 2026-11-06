GENEIOUS-FUNKTIONEN:
- Klonen von DNA-Sequenzen mittels Restriktionsenzymen
- Gibson-Assembly zur nahtlosen Verknüpfung von DNA-Fragmenten
- Golden-Gate-Klonung über spezifische Überhänge (Overhangs)
- Gateway-Klonung mit attB-, bp- und lr-Reaktionen
- In-Fusion-Klonung durch homologe Enden (Homologous Recombination)
- Topo-Klonung für schnelle Klonierung ohne Enzyme
- PCR-Produkt-Design und Amplifikationssimulation
- Restriktionsenzym-Analyse und Schnittstellenbestimmung
- Erstellung und Verwaltung von Enzym-Sets (Enzyme Sets)
- Silent-Mutationsanalyse zur Vermeidung unerwünschter Änderungen
- Parts-Klonung für modulare DNA-Zusammenstellungen
- SLIC/CPEC/SLiCE Simulation ohne spezifische Exonukleasen

ABGLEICH:
- Klonen von DNA-Sequenzen mittels Restriktionsenzymen | Kern: ja | App: ja | cloning/enzymes.rs, cloning/ligation.rs
- Gibson-Assembly zur nahtlosen Verknüpfung von DNA-Fragmenten | Kern: ja | App: ja | cloning/overlap.rs
- Golden-Gate-Klonung über spezifische Überhänge (Overhangs) | Kern: ja | App: ja | cloning/golden_gate.rs
- Gateway-Klonung mit attB-, bp- und lr-Reaktionen | Kern: ja | App: ja | cloning/gateway.rs
- In-Fusion-Klonung durch homologe Enden (Homologous Recombination) | Kern: ja | App: ja | cloning/overlap.rs
- Topo-Klonung für schnelle Klonierung ohne Enzyme | Kern: ja | App: ja | cloning/topo.rs
- PCR-Produkt-Design und Amplifikationssimulation | Kern: ja | App: ja | cloning/pcr.rs
- Restriktionsenzym-Analyse und Schnittstellenbestimmung | Kern: ja | App: ja | cloning/enzymes.rs, cloning/site_check.rs
- Erstellung und Verwaltung von Enzyme-Sets (Enzyme Sets) | Kern: teilweise | App: ja | cloning/enzymes.rs (enzyme_list), jedoch keine explizite "Ensemble"-Verwaltungsfunktion in der App-Liste, nur Einzelaufruf
- Silent-Mutationsanalyse zur Vermeidung unerwünschter Änderungen | Kern: nein | App: nein | Keine spezifische Funktion zur automatischen Erkennung von Silent Mutations im Kontext der Klonierung; primer/mutagenesis.rs erlaubt manuelle Änderungen, aber keine automatische Analyse auf "Silent" im Klonierungskontext
- Parts-Klonung für modulare DNA-Zusammenstellungen | Kern: ja | App: teilweise | cloning/overlap.rs (Überlappung), jedoch keine explizite "Parts"-Bibliothek oder -Verwaltung in der App-Liste
- SLIC/CPEC/SLiCE Simulation ohne spezifische Exonukleasen | Kern: ja | App: ja | cloning/overlap.rs (SLIC wird explizit genannt)
