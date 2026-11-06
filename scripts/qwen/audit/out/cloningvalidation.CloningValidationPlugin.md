GENEIOUS-FUNKTIONEN:
- Gruppiert Reads automatisch zu Clones basierend auf Namensmustern
- Mappt Reads automatisch oder manuell auf Referenzsequenzen
- Validiert Klonierungsresultate durch Batch-Sequenzalignments
- Identifiziert und zeigt Varianten (SNPs) in den Sequenzen an
- Berechnet Statistiken zur Validierungsgüte der Reads/Clones
- Exportiert Validierungsergebnisse in CSV- oder TSV-Formaten
- Filtert Ergebnisse nach Regionen, Annotationstypen oder Status
- Markiert Reads manuell als valid oder nicht valid
- Gruppiert und entgruppiert Reads in der Listenansicht
- Zeigt detaillierte Varianteninformationen (Nukleotid/Aminosäure) an
- Unterstützt NGS-Daten mit begrenzter automatischer Verarbeitung
- Generiert Contigs aus den gemappten Alignments

ABGLEICH:
- Gruppiert Reads automatisch zu Clones basierend auf Namensmustern | Kern: nein | App: nein | <fehlt>
- Mappt Reads automatisch oder manuell auf Referenzsequenzen | Kern: ja | App: map_reads_to_reference, map_reads_to | bio/read_map.rs
- Validiert Klonierungsresultate durch Batch-Sequenzalignments | Kern: ja | App: align_documents, alignment_variants | bio/align.rs, ops/phase3.rs
- Identifiziert und zeigt Varianten (SNPs) in den Sequenzen an | Kern: ja | App: call_variants, alignment_variants | bio/variants.rs
- Berechnet Statistiken zur Validierungsgüte der Reads/Clones | Kern: ja | App: coverage_depth, selection_stats, summary | coverage.rs
- Exportiert Validierungsergebnisse in CSV- oder TSV-Formaten | Kern: ja | App: export_document, export_text | export/text.rs
- Filtert Ergebnisse nach Regionen, Annotationstypen oder Status | Kern: ja | App: features, selection_stats | model/feature.rs
- Markiert Reads manuell als valid oder nicht valid | Kern: nein | App: nein | <fehlt>
- Gruppiert und entgruppiert Reads in der Listenansicht | Kern: ja | App: create_folder, move_document, folders | store/folders.rs
- Zeigt detaillierte Varianteninformationen (Nukleotid/Aminosäure) an | Kern: ja | App: alignment_variants, translation | bio/variants.rs, view/translation.rs
- Unterstützt NGS-Daten mit begrenzter automatischer Verarbeitung | Kern: ja | App: check_read_file, qc_fastq, trim_fastq | ngs/qc.rs, ngs/trim.rs
- Generiert Contigs aus den gemappten Alignments | Kern: ja | App: assemble_reads, alignment_consensus | bio/assembly.rs
