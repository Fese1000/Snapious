GENEIOUS-FUNKTIONEN:
- Importiert GCG-Dateien in das System
- Verarbeitet AA_SEQUENCE-Daten (Aminosäuren)
- Verarbeitet NA_SEQUENCE-Daten (Nukleinsäuren)
- Zeigt Warnung bei fehlender ".."-Zeile
- Erzeugt Sequenzobjekte aus GCG-Daten
- Zeigt Fehler bei leerer Datei an
- Zeigt Fehler bei fehlender Sequenz an
- Bietet GCG-Import-Funktionalität im Plugin

ABGLEICH:
- Importiert GCG-Dateien in das System | Kern: nein | App: nein | Keine Datei oder Funktion für GCG-Import vorhanden
- Verarbeitet AA_SEQUENCE-Daten (Aminosäuren) | Kern: ja | App: ja | `io/fasta.rs` (liest FASTA, enthält AA), `model/sequence.rs`
- Verarbeitet NA_SEQUENCE-Daten (Nukleinsäuren) | Kern: ja | App: ja | `io/fasta.rs` (liest FASTA, enthält NA), `model/sequence.rs`
- Zeigt Warnung bei fehlender ".."-Zeile | Kern: nein | App: nein | Keine spezifische GCG-Parser-Warnung implementiert
- Erzeugt Sequenzobjekte aus GCG-Daten | Kern: ja | App: ja | `io/fasta.rs` (Parsing), `model/sequence.rs` (Objekt-Erzeugung)
- Zeigt Fehler bei leerer Datei an | Kern: ja | App: ja | `io/fasta.rs` (Fehlerbehandlung beim Lesen), `view/error_hint.rs`
- Zeigt Fehler bei fehlender Sequenz an | Kern: ja | App: ja | `io/fasta.rs` (Validierung), `view/error_hint.rs`
- Bietet GCG-Import-Funktionalität im Plugin | Kern: nein | App: nein | Kein spezifischer GCG-Importer vorhanden; nur generische Importe (FASTA, GenBank etc.)
