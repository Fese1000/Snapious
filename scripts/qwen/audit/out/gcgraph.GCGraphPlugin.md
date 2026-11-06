GENEIOUS-FUNKTIONEN:
- Erstellen von GC-Grafen für Sequenzanalysen
- Visualisierung der GC-Konzentrationsverteilung in DNA
- Analyse von Sequenzzusammensetzungsmerkmalen
- Unterstützung spezifischer Bioinformatik-Dateiformate

ABGLEICH:
- Erstellen von GC-Grafen für Sequenzanalysen | Kern: ja | App: ja | seq_stats.rs (Funktion `gc_profile`)
- Visualisierung der GC-Konzentrationsverteilung in DNA | Kern: ja | App: ja | seq_stats.rs (Funktion `gc_profile`)
- Analyse von Sequenzzusammensetzungsmerkmalen | Kern: ja | App: ja | seq_stats.rs (Funktionen `gc_profile`, `gc_skew`, `dinucleotides`, `cpg_island_regions`)
- Unterstützung spezifischer Bioinformatik-Dateiformate | Kern: ja | App: ja | io/fasta.rs, io/fastq.rs, io/genbank.rs, io/embl.rs, io/gff3.rs, io/vcf.rs, io/sam.rs, io/abif.rs (Lesen/Schreiben verschiedener Formate)
