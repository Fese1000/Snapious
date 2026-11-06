GENEIOUS-FUNKTIONEN:
- Filtert Dokumente basierend auf Feldinhalten
- Sortiert Dokumente nach Feldwerten oder Namen
- Filtert Sequenzen, falls es sich um Paarlese handelt
- Filtert nach spezifischen Dokumenttypen (z.B. Alignment)
- Filtert Dokumente, die ein Feld nicht enthalten
- Filtert nach Übereinstimmung aller oder beliebiger Felder
- Sortiert Dokumente ohne Feldwert ans Ende der Liste
- Filtert jeden Dokument- oder Sequenzinhalt einzeln

ABGLEICH:
- Filtert Dokumente basierend auf Feldinhalten | Kern: nein | App: nein | <fehlt>
- Sortiert Dokumente nach Feldwerten oder Namen | Kern: ja | App: ja | project/search.rs (Stapelumbenennung/Dublettensuche impliziert Sortierung/Gruppierung, aber explizite Feld-Sortierung fehlt; streng: nein) -> Korrektur: `project/search.rs` enthält keine explizite Sortierung nach Feldwerten. App-Funktion fehlt. -> Kern: nein | App: nein | <fehlt>
- Filtert Sequenzen, falls es sich um Paarlese handelt | Kern: ja | App: nein | <fehlt>
- Filtert nach spezifischen Dokumenttypen (z.B. Alignment) | Kern: ja | App: ja | project/search.rs (Stapelumbenennung/Dublettensuche impliziert Filterung, aber keine explizite Typ-Filterung in der API; streng: nein) -> Korrektur: Es gibt keine Funktion `filter_by_type`. App-Funktion fehlt. -> Kern: nein | App: nein | <fehlt>
- Filtert Dokumente, die ein Feld nicht enthalten | Kern: ja | App: nein | <fehlt>
- Filtert nach Übereinstimmung aller oder beliebiger Felder | Kern: ja | App: nein | <fehlt>
- Sortiert Dokumente ohne Feldwert ans Ende der Liste | Kern: ja | App: nein | <fehlt>
- Filtert jeden Dokument- oder Sequenzinhalt einzeln | Kern: ja | App: nein | <fehlt>
