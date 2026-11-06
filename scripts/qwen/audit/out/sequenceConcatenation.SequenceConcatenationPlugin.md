GENEIOUS-FUNKTIONEN:
- Verknüpfung von Aminosäuresequenzen zu einer einzigen Sequenz
- Zusammenführung von Nukleotidgraph-Sequenzen in einem Ergebnis
- Konkatenation einfacher Nukleotidsequenzen zu einer Gesamtssequenz
- Ausführung von Konkatenationsoperationen auf biologischen Daten
- Verschmelzung von Sequenzalignments zu einem einzigen Alignment
- Plugin-Verwaltung für die Sequenzkonkatenierungsfunktion
- Anzeige von verknüpften Sequenzalignments in der Ansicht
- Dialog zur Auswahl zu verknüpfender Sequenzen oder Alignments
- Konkatenation von Listen gleicher Molekültypen (Nukleotid/Protein)
- Fehlerprüfung auf gültige Residue-Indizes während der Operation
- Validierung der Graphposition innerhalb der verknüpften Sequenz
- Sicherstellung, dass Eingabesequenzen vom Typ NucleotideGraphSequence sind

ABGLEICH:
- Verknüpfung von Aminosäuresequenzen zu einer einzigen Sequenz | Kern: ja | App: nein | bio/sequence.rs (concat-Logik implizit in `ops/builtin` oder manuelle Bearbeitung, aber keine direkte "Concatenate Sequences"-Operation in der Schnittstelle)
- Zusammenführung von Nukleotidgraph-Sequenzen in einem Ergebnis | Kern: nein | App: nein | Snapious nutzt lineare Sequenzmodelle (`model/sequence.rs`), keine Graphen-Struktur für Assembly-Konkatenation wie in Geneious
- Konkatenation einfacher Nukleotidsequenzen zu einer Gesamtssequenz | Kern: ja | App: nein | `ops/builtin.rs` (edit_insert/edit_replace existieren, aber keine dedizierte "Concatenate" Operation in der API)
- Ausführung von Konkatenationsoperationen auf biologischen Daten | Kern: ja | App: nein | Siehe oben; keine spezifische API-Funktion für generelle Konkatenation
- Verschmelzung von Sequenzalignments zu einem einzigen Alignment | Kern: ja | App: ja | `bio/align_tools.rs` (Funktion `alignment_strip` oder manuelle Handhabung über `model/alignment.rs`, jedoch keine direkte "merge alignments" API-Funktion in der Liste, daher eher "teilweise" oder fehlend an direkter App-Abdeckung) -> Korrektur: `bio/align_tools.rs` hat "Profile verbinden", aber keine direkte "Alignment Concatenation" in der App-Schnittstelle. -> **Kern: ja | App: nein** (Keine direkte `align_concatenate` Funktion in der Schnittstellenliste)
- Plugin-Verwaltung für die Sequenzkonkatenierungsfunktion | Kern: ja | App: ja | `plugin.rs` (Allgemeine Plugin-Verwaltung vorhanden, aber kein spezifisches Plugin für diese Funktion)
- Anzeige von verknüpften Sequenzalignments in der Ansicht | Kern: ja | App: ja | `view/seq_layout.rs` (Darstellung von Alignments vorhanden, aber keine spezifische "Concatenated View"-Logik)
- Dialog zur Auswahl zu verknüpfender Sequenzen oder Alignments | Kern: ja | App: nein | Keine spezifische Dialog-Funktion in der API-Liste für "Select for Concatenation"
- Konkatenation von Listen gleicher Molekültypen (Nukleotid/Protein) | Kern: ja | App: nein | Keine spezifische API-Funktion für "Concatenate List"
- Fehlerprüfung auf gültige Residue-Indizes während der Operation | Kern: ja | App: ja | `model/sequence.rs` und `ops/builtin.rs` (Validierung ist Teil der Basisoperationen)
- Validierung der Graphposition innerhalb der verknüpften Sequenz | Kern: ja | App: nein | Snapious hat keine Graph-Positionierung für Konkatenation (`model/sequence.rs` ist linear)
- Sicherstellung, dass Eingabesequenzen vom Typ NucleotideGraphSequence sind | Kern: ja | App: nein | Snapious nutzt `model/sequence.rs` (DNA/RNA/Protein), keine `NucleotideGraphSequence`
