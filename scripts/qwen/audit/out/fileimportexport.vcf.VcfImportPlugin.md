GENEIOUS-FUNKTIONEN:
- Importiert VCF-Dateien (Variant Call Format) in Geneious
- Exportiert Variantenannotationen als VCF-Datei (v4.2)
- Exportiert Original-VCF-Quelldateien ohne Änderungen
- Generiert neue VCF-Dateien aus Geneious-Annotationen
- Exportiert Sequenzen im FASTA-Format
- Verarbeitet Polymorphismus-Typen (Varianten)
- Prüft Referenzsequenzen und Kontig-Informationen
- Parsen und Validieren von VCF-Zeilen und Metainformationen
- Unterstützt Einzelproben-Varianten relativ zu Referenzsequenzen
- Verarbeitet Format-Felder und Alternative Allelen-Zählungen

ABGLEICH:
- Importiert VCF-Dateien (Variant Call Format) in Geneious | Kern: ja | App: ja | io/vcf.rs (VCF lesen), store/project_store.rs (Speicherung)
- Exportiert Variantenannotationen als VCF-Datei (v4.2) | Kern: ja | App: ja | io/vcf.rs (VCF schreiben), variants.rs (Varianten aus Alignment)
- Exportiert Original-VCF-Quelldateien ohne Änderungen | Kern: ja | App: nein | store/external.rs (Verknüpfung externer Dateien, aber kein direkter "Export ohne Änderung" als separater Export-Workflow beschrieben; eher Referenzierung)
- Generiert neue VCF-Dateien aus Geneious-Annotationen | Kern: ja | App: teilweise | io/vcf.rs (Schreiben), variants.rs (Erstellung von Varianten-Datenstrukturen)
- Exportiert Sequenzen im FASTA-Format | Kern: ja | App: ja | io/fasta.rs (FASTA schreiben), export/text.rs (Textdarstellung/Export)
- Verarbeitet Polymorphismus-Typen (Varianten) | Kern: ja | App: ja | variants.rs (Unterschiede jeder Zeile gegenüber einer Referenzzeile), ngs/pileup.rs (Variantenerkennung)
- Prüft Referenzsequenzen und Kontig-Informationen | Kern: ja | App: ja | model/sequence.rs (Sequenz-Datenstruktur), io/vcf.rs (Referenz-Prüfung im VCF-Kontext)
- Parsen und Validieren von VCF-Zeilen und Metainformationen | Kern: ja | App: ja | io/vcf.rs (Lesen/Validieren)
- Unterstützt Einzelproben-Varianten relativ zu Referenzsequenzen | Kern: ja | App: ja | variants.rs, ngs/pileup.rs (SNVs aus Alignments)
- Verarbeitet Format-Felder und Alternative Allelen-Zählungen | Kern: ja | App: teilweise | io/vcf.rs (Format-Felder lesen), variants.rs (Unterschiede berechnen, aber explizite "Allelen-Zählung" aus VCF-Format-Feldern ist implizit im Leseprozess von io/vcf.rs enthalten)
