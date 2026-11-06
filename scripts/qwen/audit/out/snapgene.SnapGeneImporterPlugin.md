GENEIOUS-FUNKTIONEN:
- Importiert SnapGene .dna Dateien in Geneious Prime
- Verarbeitet SnapGene Features und Annotationen
- Importiert Primer-Sequenzen und Namen aus SnapGene
- Importiert Restriktionsenzym-Sätze für Sequenzen
- Verarbeitet SnapGene-Historien-Daten und Farben
- Liest Notizen aus SnapGene-Dateien aus
- Konvertiert SnapGene-Sequenzklassen und -typen
- Importiert Pubmed-Referenzen aus SnapGene-Dokumenten
- Verarbeitet DNA-Strang-Historien und Produktfarben
- Importiert Enzyme für Proteinsequenzen (falls leer)
- Parsen von SnapGene-Annotation-Intervallen und Qualifizierern
- Keine biologische Analyse oder Berechnungsfunktion vorhanden

ABGLEICH:
- Importiert SnapGene .dna Dateien in Geneious Prime | Kern: ja | App: ja | io/snapgene.rs (`snapgene_open` impliziert via `io/snapgene.rs`, App-Funktion `open` oder spezifischer Import über `import_file`)
- Verarbeitet SnapGene Features und Annotationen | Kern: ja | App: ja | io/snapgene.rs (liest Features), model/feature.rs (Datenstruktur)
- Importiert Primer-Sequenzen und Namen aus SnapGene | Kern: ja | App: ja | io/snapgene.rs (liest Primer als Features), primer/picker.rs (Primer-Datenstruktur)
- Importiert Restriktionsenzym-Sätze für Sequenzen | Kern: ja | App: ja | io/snapgene.rs (liest Enzym-Sätze), cloning/enzymes.rs (Enzym-Datenbank)
- Verarbeitet SnapGene-Historien-Daten und Farben | Kern: teilweise | App: nein | io/snapgene.rs (liest Historie/Farben, aber Snapious hat keine visuelle "Farb-Historie"-Anzeige wie SnapGene; `io/snapgene.rs` liest es, aber keine App-Funktion zur Darstellung der Historie als solches)
- Liest Notizen aus SnapGene-Dateien aus | Kern: ja | App: ja | io/snapgene.rs (liest Notizen), store/folders.rs (`note`-Feld in `folders(id, name, parent_id)` und `doc_folders`)
- Konvertiert SnapGene-Sequenzklassen und -typen | Kern: ja | App: ja | io/snapgene.rs (konvertiert Typen), model/sequence.rs (DNA/RNA/Protein)
- Importiert Pubmed-Referenzen aus SnapGene-Dokumenten | Kern: ja | App: nein | io/snapgene.rs (liest Referenzen), db/ncbi.rs (kann PubMed-Daten abrufen, aber keine direkte Import-Funktion für SnapGene-Pubmed-IDs in die App-Oberfläche genannt)
- Verarbeitet DNA-Strang-Historien und Produktfarben | Kern: teilweise | App: nein | io/snapgene.rs (liest Strang/Historie), model/sequence.rs (Strang-Info), aber keine App-Funktion zur "Produktfarb-Historie"
- Importiert Enzyme für Proteinsequenzen (falls leer) | Kern: ja | App: ja | io/snapgene.rs (liest Enzyme), cloning/enzymes.rs (Enzym-Datenbank)
- Parsen von SnapGene-Annotation-Intervallen und Qualifizierern | Kern: ja | App: ja | io/snapgene.rs (parsen Intervalle), model/feature.rs (Intervalle, Positionen)
