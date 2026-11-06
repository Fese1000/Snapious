GENEIOUS-FUNKTIONEN:
- Importiert MAQ-Alignments aus mapview
- Erstellt Contig-Dokumente aus Reads
- Verarbeitet referenzbasierte Sequenzalignments
- Parst gelesene DNA/RNA-Sequenzen (Reads)
- Verknüpft Paired-End-Lesungen (Mates)
- Validiert Referenzsequenzen auf Nukleotid-Typ
- Speichert generierte Contig-Datenstrukturen
- Zeigt nicht gemappte Reads an

ABGLEICH:
- Importiert MAQ-Alignments aus mapview | Kern: nein | App: nein | Keine Unterstützung für das spezifische MAQ-Format oder `mapview`
- Erstellt Contig-Dokumente aus Reads | Kern: ja | App: assemble_reads | assembly.rs (gieriger Überlappungsansatz)
- Verarbeitet referenzbasierte Sequenzalignments | Kern: ja | App: map_reads_to_reference, seqcheck.rs | read_map.rs (Mapping ohne Lücken), bio/align.rs
- Parst gelesene DNA/RNA-Sequenzen (Reads) | Kern: ja | App: import_file, check_read_file | io/fastq.rs (FASTQ lesen), io/fasta.rs
- Verknüpft Paired-End-Lesungen (Mates) | Kern: nein | App: nein | Keine explizite Paarungsverwaltung oder "Mates"-Logik in der Liste
- Validiert Referenzsequenzen auf Nukleotid-Typ | Kern: ja | App: nein | Keine explizite Validierungsfunktion genannt; implizit in `io/` oder `model/sequence.rs`, aber keine spezifische App-Funktion
- Speichert generierte Contig-Datenstrukturen | Kern: ja | App: save_candidate, export_document | store/project_store.rs (Speicherung), assembly.rs (Erstellung)
- Zeigt nicht gemappte Reads an | Kern: ja | App: nein | Keine spezifische Funktion zum Filtern/Anzeigen von "unmapped reads" in der App-Schnittstelle genannt
