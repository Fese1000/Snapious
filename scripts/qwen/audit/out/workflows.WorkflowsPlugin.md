GENEIOUS-FUNKTIONEN:
- Extrahiert Referenzsequenzen aus Contigs oder Alignments
- Reverse-Komplementierung aller Nukleotidsequenzen und Alignments
- Translation von Sequenzen (Nukleotid zu Protein)
- Gruppiert Sequenzen in eine Sequenzlisten-Dokument
- Führt Operationen für jede Sequunez einzeln aus
- Kombiniert Ergebnisse mit früheren Dokumenten im Workflow
- Gruppiert Dokumente für gemeinsame Weiterverarbeitung
- Benennt Dokumente während des Workflows um
- Speichert Ergebnisse und verzweigt den Workflow
- Führt benutzerdefinierten Java-Code aus (Custom Code)
- Filtert Sequenzen basierend auf Referenz oder Nicht-Referenz

ABGLEICH:
- Extrahiert Referenzsequenzen aus Contigs oder Alignments | Kern: ja | App: nein | model/sequence.rs (Basisoperation), aber keine spezifische UI-Funktion "Extrahiere Referenz aus Alignment" in der Schnittstellenliste; `alignment` und `assemble_reads` existieren, aber keine direkte "Extract Reference"-Operation als separater Workflow-Schritt in der Liste. -> **nein** (Da keine explizite Funktion `extract_reference_from_alignment` o.ä. in der Schnittstelle steht, und `alignment` nur das Alignment selbst zurückgibt).
- Reverse-Komplementierung aller Nukleotidsequenzen und Alignments | Kern: ja | App: ja | ops/builtin.rs ("Reverse-Komplement"), view/translation.rs (implizit für Strang-Handling), model/sequence.rs.
- Translation von Sequenzen (Nukleotid zu Protein) | Kern: ja | App: ja | bio/genetic_code.rs ("Translation"), view/translation.rs, ops/more.rs.
- Gruppiert Sequenzen in eine Sequenzlisten-Dokument | Kern: ja | App: nein | Keine Funktion zum Erstellen einer "Liste von Sequenzen" als neues Dokumentobjekt aus bestehenden. `documents` liest nur, `create_sequence` erstellt eins. -> **nein**
- Führt Operationen für jede Sequunez einzeln aus | Kern: ja | App: teilweise | ops/workflow.rs ("Folgen von Operationen"), aber keine explizite "ForEach" oder "Apply to all" Funktion in der API-Liste. Workflows sind sequentiell, nicht automatisch iterativ über eine Liste ohne explizite Schleifenoperation. -> **teilweise** (Workflows erlauben Sequenz, aber keine explizite "Apply to all" Funktion in der Schnittstelle).
- Kombiniert Ergebnisse mit früheren Dokumenten im Workflow | Kern: ja | App: ja | ops/workflow.rs ("deren Ergebnisse weitergereicht werden"), ops/runner.rs.
- Gruppiert Dokumente für gemeinsame Weiterverarbeitung | Kern: ja | App: nein | Keine Funktion zum Gruppieren von Dokumenten in einen Container für Batch-Verarbeitung. `create_folder` gruppiert nur visuell/strukturell, nicht für Operationen. -> **nein**
- Benennt Dokumente während des Workflows um | Kern: ja | App: ja | rename_document (in `project/search.rs` und Schnittstelle), aber keine explizite "Rename in Workflow"-Operation. Man kann `rename_document` aufrufen, aber es ist keine integrierte Workflow-Step-Funktion "Umbenenne Ergebnis". -> **teilweise** (Die Funktion existiert, ist aber kein integrierter Workflow-Block wie "Rename Step").
- Speichert Ergebnisse und verzweigt den Workflow | Kern: ja | App: teilweise | workflow_save, save_candidate. Aber keine explizite "Branch" oder "Save and Branch"-Operation im Workflow-Kontext. -> **teilweise**
- Führt benutzerdefinierten Java-Code aus (Custom Code) | Kern: ja | App: nein | Snapious ist in Rust geschrieben, keine Java-Integration. -> **nein**
- Filtert Sequenzen basierend auf Referenz oder Nicht-Referenz | Kern: ja | App: nein | Keine Funktion zum Filtern einer Liste von Sequenzen basierend auf einem Referenzkriterium. `search_documents` sucht Text, nicht biologische Filterung von Listen. -> **nein**
