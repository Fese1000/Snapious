GENEIOUS-FUNKTIONEN:
- Importiert Clustal(W)-Dateien in das System
- Verarbeitet Alignment-Daten im Clustal-Format
- Stellt biologische Sequenzdaten dar

ABGLEICH:
- Importiert Clustal(W)-Dateien in das System | Kern: ja | App: ja | io/alignment_formats.rs (liest Clustal-Formate)
- Verarbeitet Alignment-Daten im Clustal-Format | Kern: ja | App: ja | io/alignment_formats.rs (liest und schreibt Clustal)
- Stellt biologische Sequenzdaten dar | Kern: ja | App: ja | model/sequence.rs, view/seq_layout.rs
