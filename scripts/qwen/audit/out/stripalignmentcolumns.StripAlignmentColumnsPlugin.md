GENEIOUS-FUNKTIONEN:
- Maskieren von Sequenzstellen nach Inhalt (z.B. Gaps, konstante Stellen)
- Maskieren oder Entfernen von Codon-Positionen (1., 2. oder 3.)
- Maskieren basierend auf vorhandenen „Masked“-Annotationen im Alignment
- Verwenden von NEXUS-style CHARSETs (Standard- oder Vector-Stil) zum Maskieren
- Erstellen neuer Alignments mit entfernten (gestrippten) Stellen
- Hinzufügen von Masked-Annotationen zu ausgewählten Regionen eines Alignments
- Unterstützung der Importierung mehrerer Masken aus NEXUS EXSETs
- Berücksichtigung von Trimmed-Regionen bei der Inhalts-basierten Maskierung
- Optionales Beibehalten oder Ignorieren der ursprünglichen Sequenznummerierung
- Warnung bei ungültigen CHARSET-Eingaben (zu lang, zu kurz, Indexfehler)
- Unterstützung für lineare und zirkulare Contigs (mit Einschränkungen beim Maskieren)
- Keine direkten biologischen Analysefunktionen wie Phylogenie oder BLAST sichtbar

ABGLEICH:
- Maskieren von Sequenzstellen nach Inhalt (z.B. Gaps, konstante Stellen) | Kern: teilweise | App: ja | bio/align_tools.rs (Teilbereich neu), view/edit_session.rs (Bearbeitung) – *Hinweis: Keine direkte "Maskieren"-Funktion, aber manuelle Löschung/Ersetzung über edit_replace/edit_delete möglich; keine automatische Maskierung basierend auf Gaps/Konsens wie im Plugin.*
- Maskieren oder Entfernen von Codon-Positionen (1., 2. oder 3.) | Kern: nein | App: ja | bio/align_tools.rs (Teilbereich neu), view/edit_session.rs – *Hinweis: Keine spezifische Codon-Position-Maskierung, manuelle Selektion und Löschung über view/selection.rs und edit_delete möglich.*
- Maskieren basierend auf vorhandenen „Masked“-Annotationen im Alignment | Kern: nein | App: ja | bio/align_tools.rs, view/edit_session.rs – *Hinweis: Keine automatische Verknüpfung von Maskierung und Annotation-Typ "Masked"; manuelle Handarbeit nötig.*
- Verwenden von NEXUS-style CHARSETs (Standard- oder Vector-Stil) zum Maskieren | Kern: nein | App: nein | *Keine NEXUS-CHARSET-Maskierungsfunktion vorhanden.*
- Erstellen neuer Alignments mit entfernten (gestrippten) Stellen | Kern: ja | App: ja | bio/align_tools.rs (Teilbereich neu), export_document – *Hinweis: "Teilbereich neu" kann genutzt werden, um eine subregion zu extrahieren und als neues Alignment zu speichern.*
- Hinzufügen von Masked-Annotationen zu ausgewählten Regionen eines Alignments | Kern: nein | App: ja | add_features, view/edit_session.rs – *Hinweis: Features können hinzugefügt werden, aber kein vordefinierter Typ "Masked" für automatische Verarbeitung.*
- Unterstützung der Importierung mehrerer Masken aus NEXUS EXSETs | Kern: nein | App: nein | *Keine Unterstützung für NEXUS EXSET-Import.*
- Berücksichtigung von Trimmed-Regionen bei der Inhalts-basierten Maskierung | Kern: nein | App: ja | bio/align_tools.rs, view/edit_session.rs – *Hinweis: Manuelle Bearbeitung möglich, keine automatische Berücksichtigung von Trim-Status für Maskierung.*
- Optionales Beibehalten oder Ignorieren der ursprünglichen Sequenznummerierung | Kern: nein | App: ja | bio/align_tools.rs, view/edit_session.rs – *Hinweis: Bei manueller Löschung (edit_delete) ändert sich die Länge, die Nummerierung der verbleibenden Basen bleibt relativ zur Referenz erhalten; keine explizige Option "Nummerierung ignorieren" wie im Plugin.*
- Warnung bei ungültigen CHARSET-Eingaben (zu lang, zu kurz, Indexfehler) | Kern: nein | App: ja | view/error_hint.rs – *Hinweis: Allgemeine Fehlerbehandlung vorhanden, keine spezifische CHARSET-Validierung.*
- Unterstützung für lineare und zirkulare Contigs (mit Einschränkungen beim Maskieren) | Kern: ja | App: ja | bio/align_tools.rs, model/document.rs (Topologie) – *Hinweis: Snapious unterstützt lineare und zirkuläre Dokumente; Maskierung ist manuell über Bearbeitung möglich.*
- Keine direkten biologischen Analysefunktionen wie Phylogenie oder BLAST sichtbar | Kern: ja | App: ja | *Bestätigung der Aussage.*
