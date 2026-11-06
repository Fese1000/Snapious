GENEIOUS-FUNKTIONEN:
- Importiert Vector NTI Advance-Datenbanken
- Importiert Vector NTI Express-Datenbanken
- Verarbeitet Vector NTI-Sequenzarchive
- Importiert Oligo-Archive und -Parser-Daten
- Lädt Enzym-Archive für Restriktionsanalyse
- Importiert Gel-Archivdaten
- Verarbeitet Alignment- und Contig-Daten
- Extrahiert Primer-Eigenschaften aus Sequenzen
- Handhabt VEMOLFEATURES und VEMOLRELATIONS
- Verwaltet Subsets und Metadaten-Felder
- Konvertiert Sequenzen bei fehlenden Daten
- Prüft Datenbankzugriffsrechte und Format

ABGLEICH:
- Importiert Vector NTI Advance-Datenbanken | Kern: nein | App: nein | Keine Datei vorhanden
- Importiert Vector NTI Express-Datenbanken | Kern: nein | App: nein | Keine Datei vorhanden
- Verarbeitet Vector NTI-Sequenzarchive | Kern: nein | App: nein | Keine Datei vorhanden
- Importiert Oligo-Archive und -Parser-Daten | Kern: nein | App: nein | Keine Datei vorhanden
- Lädt Enzym-Archive für Restriktionsanalyse | Kern: ja | App: ja | cloning/enzymes.rs (Datenbank und Suche)
- Importiert Gel-Archivdaten | Kern: ja | App: ja | cloning/gel.rs (Agarosegel-Simulation)
- Verarbeitet Alignment- und Contig-Daten | Kern: ja | App: ja | bio/align.rs (Alignments), assembly.rs (Contigs)
- Extrahiert Primer-Eigenschaften aus Sequenzen | Kern: ja | App: ja | primer/tm.rs (Schmelztemperatur), primer/checks.rs (Qualitätsprüfungen)
- Handhabt VEMOLFEATURES und VEMOLRELATIONS | Kern: nein | App: nein | Keine Datei vorhanden
- Verwaltet Subsets und Metadaten-Felder | Kern: ja | App: ja | model/document.rs (Metadaten), store/folders.rs (Ordner/Notizen)
- Konvertiert Sequenzen bei fehlenden Daten | Kern: ja | App: ja | model/sequence.rs (Grundoperationen), io/fasta.rs/io/fastq.rs (Lesen/Schreiben)
- Prüft Datenbankzugriffsrechte und Format | Kern: ja | App: ja | io/*.rs (Format-Parser wie fasta, fastq, genbank etc.), db/ncbi.rs/db/uniprot.rs (API-Zugriff)
