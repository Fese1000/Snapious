GENEIOUS-FUNKTIONEN:
- Entfernen von Adaptersequenzen aus Sequenzlesungen (BBDuk)
- Korrektur und Normalisierung von Sequenzlesungstiefe (BBNorm)
- Zusammenführung überlappender gepaarter Lesungen (BBMerge)
- Zuordnung von Lesungen an Referenzgenome (BBMap)
- Assemblerung von Lesungen zu Contigs (BBTadpole)
- Identifikation und Entfernung duplizierter Lesungen (BBDedupe)
- Erstellung von Kmer-Sketches zur Datenklassifizierung (BBSketch)
- Visualisierung und Analyse von Kmer-Sketch-Ergebnissen (BBSketchViewer)
- Berechnung der genomischen Vollständigkeit über Kmer-Häufigkeit
- Analyse der Sequenzentropie zur Erkennung niedriger Komplexität

ABGLEICH:
- Entfernen von Adaptersequenzen aus Sequenzlesungen (BBDuk) | Kern: ja/teilweise/nein | App: ja | ngs/trim.rs (Trimming von Reads, enthält Adapter-Entfernung)
- Korrektur und Normalisierung von Sequenzlesungstiefe (BBNorm) | Kern: ja/teilweise/nein | App: nein | coverage.rs bietet Coverage-Analyse, aber keine Normalisierung/Downsampling-Funktion wie BBNorm
- Zusammenführung überlappender gepaarter Lesungen (BBMerge) | Kern: ja/teilweise/nein | App: nein | assembly.rs bietet einfachen Zusammenbau, aber keine spezifische Paired-Ends-Zusammenführung (Merge) wie BBMerge
- Zuordnung von Lesungen an Referenzgenome (BBMap) | Kern: ja/teilweise/nein | App: ja | ngs/mapper.rs (Aufruf von Mapping-Programmen wie minimap2/BWA) und read_map.rs
- Assemblerung von Lesungen zu Contigs (BBTadpole) | Kern: ja/teilweise/nein | App: ja | ngs/spades.rs (Aufruf von SPAdes) und assembly.rs (einfacher Zusammenbau)
- Identifikation und Entfernung duplizierter Lesungen (BBDedupe) | Kern: ja/teilweise/nein | App: nein | project/search.rs bietet Dublettensuche für Dokumente, aber keine deduplizierung von Sequenzlesungen (Reads)
- Erstellung von Kmer-Sketches zur Datenklassifizierung (BBSketch) | Kern: ja/teilweise/nein | App: nein | Keine Funktion zur Kmer-Sketch-Erstellung vorhanden
- Visualisierung und Analyse von Kmer-Sketch-Ergebnissen (BBSketchViewer) | Kern: ja/teilweise/nein | App: nein | Keine Funktion zur Kmer-Analyse oder Visualisierung vorhanden
- Berechnung der genomischen Vollständigkeit über Kmer-Häufigkeit | Kern: ja/teilweise/nein | App: nein | Keine Funktion zur genomischen Vollständigkeitsberechnung über Kmer vorhanden
- Analyse der Sequenzentropie zur Erkennung niedriger Komplexität | Kern: ja/teilweise/nein | App: ja | bio/align_stats.rs (berechnet Entropie) und seq_stats.rs (Statistiken über Nukleotidsequenzen, kann Komplexität indirekt abbilden, aber keine spezifische "low complexity" Filterfunktion wie BBTools)
