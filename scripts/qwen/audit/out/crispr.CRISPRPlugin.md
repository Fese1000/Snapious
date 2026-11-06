GENEIOUS-FUNKTIONEN:
- Sucht nach CRISPR-Zielstellen in Nukleotidsequenzen
- Definiert PAM-Motive (Protospacer Adjacent Motif)
- Berechnet Aktivitätswerte basierend auf Sequenzmerkmalen
- Bewertet Spezifität gegen Off-Target-Datenbanken
- Färbt CRISPR-Zielstellen nach ihren Scores ein
- Parert Zielstellen für Single-Strand-Nicking-Enzyme
- Generiert CRISPR-Annotationen auf Originalsequenzen
- Unterstützt Doench-Algorithmen (2014, 2016, 2022)
- Nutzt Zhang-Spezifitätsscore für 3'-PAMs
- Prüft Off-Target-Interaktionen in ausgewählten Regionen
- Verarbeitet IUPAC-Codes für ambiguous Basen
- Zeigt Off-Target-Mismatches und Indels rot an

ABGLEICH:
- Sucht nach CRISPR-Zielstellen in Nukleotidsequenzen | Kern: ja | App: ja | `crispr.rs` (Funktion `find_guides`)
- Definiert PAM-Motive (Protospacer Adjacent Motif) | Kern: ja | App: ja | `crispr.rs` (Parameter der Suche nach PAM)
- Berechnet Aktivitätswerte basierend auf Sequenzmerkmalen | Kern: ja | App: nein | `crispr.rs` enthält nur "einfach bewerten" und "Nebentreffer zählen", keine spezifischen Aktivitätsalgorithmen (Doench/Zhang) implementiert
- Bewertet Spezifität gegen Off-Target-Datenbanken | Kern: ja | App: nein | `crispr.rs` zählt Nebentreffer, aber keine externe Datenbankbewertung oder spezifische Off-Target-Score-Berechnung wie in Geneious
- Färbt CRISPR-Zielstellen nach ihren Scores ein | Kern: ja | App: nein | `crispr.rs` findet Treffer, aber keine visuelle Farbkodierung nach Score-Stufen in der Schnittstelle oder `view`-Modulen spezifiziert
- Parert Zielstellen für Single-Strand-Nicking-Enzyme | Kern: ja | App: nein | `crispr.rs` unterstützt keine spezifische Nicking-Enzym-Logik (nur Standard-Cut)
- Generiert CRISPR-Annotationen auf Originalsequenzen | Kern: ja | App: teilweise | `crispr.rs` findet Treffer, aber keine explizite Funktion zum Hinzufügen von Features (`add_features`) für CRISPR-Zielstellen in der Liste der erreichbaren App-Funktionen genannt (nur `find_guides`)
- Unterstützt Doench-Algorithmen (2014, 2016, 2022) | Kern: ja | App: nein | `crispr.rs` erwähnt keine Doench-Algorithmen
- Nutzt Zhang-Spezifitätsscore für 3'-PAMs | Kern: ja | App: nein | `crispr.rs` erwähnt keinen Zhang-Score
- Prüft Off-Target-Interaktionen in ausgewählten Regionen | Kern: ja | App: teilweise | `crispr.rs` zählt Nebentreffer, aber keine detaillierte Interaktionsprüfung wie im Plugin beschrieben
- Verarbeitet IUPAC-Codes für ambiguous Basen | Kern: ja | App: ja | `crispr.rs` (Explizit erwähnt: "IUPAC-Codes")
- Zeigt Off-Target-Mismatches und Indels rot an | Kern: ja | App: nein | `crispr.rs` zählt Treffer, aber keine spezifische visuelle Anzeige von Mismatches/Indels mit Farbcodierung (rot) spezifiziert
