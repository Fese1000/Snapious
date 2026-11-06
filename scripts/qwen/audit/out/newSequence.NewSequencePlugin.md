GENEIOUS-FUNKTIONEN:
- Neue Sequenzdokumente durch Einfügen oder Tippen von Resten erstellen
- Leere Sequenzen des korrekten Typs erstellen und in den Sequenz-Viewer einfügen
- Maximale Größe des Sequenz-Einfügens im Dialog festlegen

ABGLEICH:
- Neue Sequenzdokumente durch Einfügen oder Tippen von Resten erstellen | Kern: ja | App: ja | create_sequence, edit_insert (view/edit_session.rs)
- Leere Sequenzen des korrekten Typs erstellen und in den Sequenz-Viewer einfügen | Kern: ja | App: ja | create_sequence (ops/builtin.rs, view/edit_session.rs)
- Maximale Größe des Sequenz-Einfügens im Dialog festlegen | Kern: nein | App: nein | settings.rs (enthält nur JSON-Einstellungen, keine spezifische Dialog-Größenbegrenzung für Einfügeoperationen)
