GENEIOUS-FUNKTIONEN:
- Importiert ACE (PHRAP)-Dateien in das System
- Exportiert Daten ins ACE-Format (PHRAP)
- Erstellt Konsensussequenz-Dokumente aus Contig-Daten
- Verarbeitet und parst Contigs aus ACE-Dateien
- Fügt Contigs in die Datenbank ein
- Handelt Assemblys mit Referenzsequenzen (exkludiert diese)
- Benennt exportierte Daten um (Leerzeichen zu Unterstrichen)
- Generiert Konsensussequenzen gemäß Exportoptionen
- Ignoriert Cluster ohne zugehörige Contig-Daten
- Verwirft ACE-Export bei nicht unterstützten Referenzen

ABGLEICH:
- Importiert ACE (PHRAP)-Dateien in das System | Kern: nein | App: nein | Keine ACE-Import-Funktion vorhanden (io/ enthält fasta, fastq, genbank, embl, gff3, bed, sam, bam, vcf, abif, pdb, nexus, newick; kein ACE-Parser)
- Exportiert Daten ins ACE-Format (PHRAP) | Kern: nein | App: nein | Keine ACE-Export-Funktion vorhanden (export/ enthält text, markdown; keine Binär-/PHRAP-Exporte)
- Erstellt Konsensussequenz-Dokumente aus Contig-Daten | Kern: ja | App: ja | bio/align_stats.rs (alignment_consensus), assembly.rs (assemble_reads -> Contigs); Konsensbildung ist implementiert
- Verarbeitet und parst Contigs aus ACE-Dateien | Kern: nein | App: nein | Keine ACE-Parsing-Logik vorhanden (nur FASTA/FASTQ/SAM/BAM/VCF/etc. Parser)
- Fügt Contigs in die Datenbank ein | Kern: ja | App: ja | store/project_store.rs (Speicherung von Dokumenten/Sequenzen in SQLite); assembly.rs liefert Contigs
- Handelt Assemblys mit Referenzsequenzen (exkludiert diese) | Kern: teilweise | App: ja | bio/assembly.rs (gieriger Überlappungsansatz), bio/read_map.rs (Mapping auf Referenz); Exkludierung ist implizit durch Mapping/Assembly, aber keine explizite "Exkludiere Referenz"-Option im API-Listing
- Benennt exportierte Daten um (Leerzeichen zu Unterstrichen) | Kern: nein | App: ja | project/search.rs (batch_rename); keine spezifische "Leerzeichen zu Unterstrich"-Funktion, aber Umbenennung ist möglich
- Generiert Konsensussequenzen gemäß Exportoptionen | Kern: ja | App: ja | bio/align_stats.rs (alignment_consensus); Export ist über export_document/export_text möglich
- Ignoriert Cluster ohne zugehörige Contig-Daten | Kern: nein | App: nein | Keine ACE-spezifische Cluster-Verarbeitung vorhanden
- Verwirft ACE-Export bei nicht unterstützten Referenzen | Kern: nein | App: nein | Keine ACE-Export-Funktionalität vorhanden, daher keine spezifische Validierung für ACE-Referenzen
