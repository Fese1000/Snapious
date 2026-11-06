GENEIOUS-FUNKTIONEN:
- Führt BLAST-Suchen auf Sequenzen durch
- Ruft Taxonomieinformationen für BLAST-Ergebnisse ab
- Baut taxonomische Bäume aus BLAST-Daten
- Zeigt evolutionäre Ansichten der Suchergebnisse an
- Verarbeitet Nukleotid- oder Aminosäuresequenzen als Abfrage
- Filtert Ergebnisse basierend auf maximalen E-Werten
- Begrenzt die Anzahl der angeforderten BLAST-Treffer
- Generiert Dokumente aus BLAST-Abfrageergebnissen
- Gruppiert BLAST-Ergebnisse in einem einzigen Baum
- Zeigt Linnaeus-Blast-Ergebnisansichten an
- Validiert BLAST-Dienste und Abfrageleerheit
- Verarbeitet Zugriffskennungen als Sequenzabfragen

ABGLEICH:
- Führt BLAST-Suchen auf Sequenzen durch | Kern: ja | App: ja | bio/blast.rs (lokaler BLAST+), bio/blast_remote.rs (NCBI API)
- Ruft Taxonomieinformationen für BLAST-Ergebnisse ab | Kern: nein | App: nein | Keine Taxonomie-Abfrage oder -Anreicherung vorhanden
- Baut taxonomische Bäume aus BLAST-Daten | Kern: ja | App: ja | bio/tree.rs (Baumdatenstruktur, NJ/UPGMA), ops/phase3.rs (Alignment/Baum-Operationen)
- Zeigt evolutionäre Ansichten der Suchergebnisse an | Kern: ja | App: ja | view/tree_layout.rs (Baumlayout), bio/newick.rs (Newick-Format)
- Verarbeitet Nukleotid- oder Aminosäuresequenzen als Abfrage | Kern: ja | App: ja | bio/blast.rs (blastn/blastp), model/sequence.rs
- Filtert Ergebnisse basierend auf maximalen E-Werten | Kern: ja | App: nein | bio/blast.rs liest Tabellarisches, aber keine explizite E-Wert-Filterung in der API-Schnittstelle genannt
- Begrenzt die Anzahl der angeforderten BLAST-Treffer | Kern: ja | App: nein | bio/blast.rs liest Ausgabe, aber keine explizite Limitierungsfunktion in der API-Schnittstelle genannt
- Generiert Dokumente aus BLAST-Abfrageergebnissen | Kern: ja | App: ja | model/document.rs, ops/network.rs (Datensatz laden), io/fasta.rs/io/genbank.rs
- Gruppiert BLAST-Ergebnisse in einem einzigen Baum | Kern: ja | App: teilweise | bio/tree.rs (Baumoperationen), aber keine explizite "Gruppierung aus BLAST" Funktion in der API
- Zeigt Linnaeus-Blast-Ergebnisansichten an | Kern: ja | App: nein | Keine spezifische "Linnaeus-Blast-Ansicht" in der Snapious-App vorhanden
- Validiert BLAST-Dienste und Abfrageleerheit | Kern: ja | App: teilweise | bio/blast.rs/bio/blast_remote.rs prüfen Aufrufe, aber keine explizite "Validierung von Diensten" als separate API-Funktion
- Verarbeitet Zugriffskennungen als Sequenzabfragen | Kern: ja | App: nein | Keine Funktion zur Verarbeitung von Zugriffskennungen als Abfrageobjekte
