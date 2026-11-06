GENEIOUS-FUNKTIONEN:
- Findet offene Leserahmen (ORFs) in DNA-Sequenzen
- Analysiert Sequenzen basierend auf angegebenem genetischen Code
- Konfiguriert gültige Start-Codons (z. B. ATG)
- Legt minimale ORF-Größe in Nukleotiden fest
- Schließt innere ORFs innerhalb anderer ORFs ein
- Nimmt Start-/Stopp-Codons außerhalb der Sequenz an
- Analysiert die gesamte Sequenz oder ausgewählte Regionen
- Generiert ORF-Annotationen für weitere Verarbeitung
- Nutzt Protein-Coding-Prediction-Grafik zur Identifizierung
- Prüft auf gültige Nukleotide und Codon-Länge

ABGLEICH:
- Findet offene Leserahmen (ORFs) in DNA-Sequenzen | Kern: ja | App: ja | view/feature_detect.rs (find_orfs)
- Analysiert Sequenzen basierend auf angegebenem genetischen Code | Kern: ja | App: nein | bio/genetic_code.rs (vorhanden, aber keine direkte App-Funktion zur Auswahl des Codes für ORF-Suche)
- Konfiguriert gültige Start-Codons (z. B. ATG) | Kern: ja | App: nein | bio/genetic_code.rs (vorhanden, aber keine Konfigurationsmöglichkeit in der App-Schnittstelle)
- Legt minimale ORF-Größe in Nukleotiden fest | Kern: ja | App: nein | Keine passende Snapious-Datei oder was fehlt (keine Parameterkonfiguration in der App)
- Schließt innere ORFs innerhalb anderer ORFs ein | Kern: ja | App: nein | view/feature_detect.rs (find_orfs existiert, aber Logik für "innere ORFs" ist nicht explizit als Option in der App-Schnittstelle abgebildet)
- Nimmt Start-/Stopp-Codons außerhalb der Sequenz an | Kern: ja | App: nein | view/feature_detect.rs (find_orfs existiert, aber diese spezifische Logik ist nicht als App-Option sichtbar)
- Analysiert die gesamte Sequenz oder ausgewählte Regionen | Kern: ja | App: ja | view/feature_detect.rs (find_orfs kann auf selection_stats basieren, aber keine explizite "Region"-Option in der Liste) -> *Korrektur*: Die App-Funktion `find_orfs` ist vorhanden. Die Frage ist, ob sie "ausgewählte Regionen" unterstützt. `view/feature_detect.rs` wird genannt. Die App-Funktion `find_orfs` ist in der Liste. Ob sie selektiert, steht nicht explizit da, aber `find_orfs` ist eine Operation. Ich markiere es als ja für die Basisfunktion, aber die spezifische Konfiguration fehlt.
- Generiert ORF-Annotationen für weitere Verarbeitung | Kern: ja | App: ja | view/feature_detect.rs (find_orfs generiert Features, die dann via `add_features` oder ähnlich genutzt werden können, aber `find_orfs` ist die Quelle)
- Nutzt Protein-Coding-Prediction-Grafik zur Identifizierung | Kern: ja | App: nein | view/feature_detect.rs (keine grafische Vorhersage-Engine in Snapious, nur statische Suche)
- Prüft auf gültige Nukleotide und Codon-Länge | Kern: ja | App: ja | bio/sequence.rs (Grundtypen prüfen) / view/feature_detect.rs (Validierung ist Teil der Suche)
