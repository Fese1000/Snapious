GENEIOUS-FUNKTIONEN:
- Extrahiert Konsensus-Sequenzen aus Ausrichtungen oder Contigs
- Exportiert extrahierte Konsensus-Sequenzen in Dokumente
- Verarbeitet eine oder mehrere biologische Ausrichtungen
- Verarbeitet eine oder mehrere biologische Contigs

ABGLEICH:
- Extrahiert Konsensus-Sequenzen aus Ausrichtungen oder Contigs | Kern: ja | App: ja | bio/align_stats.rs (alignment_consensus), model/document.rs (Contigs als Sequenzdokumente)
- Exportiert extrahierte Konsensus-Sequenzen in Dokumente | Kern: ja | App: ja | export/text.rs (export_document), model/document.rs
- Verarbeitet eine oder mehrere biologische Ausrichtungen | Kern: ja | App: ja | bio/align.rs, ops/phase3.rs (alignment)
- Verarbeitet eine oder mehrere biologische Contigs | Kern: ja | App: ja | bio/assembly.rs (assemble_reads), model/document.rs
