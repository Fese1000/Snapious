# Mitmachen

Snapious ist ein Hobbyprojekt, Beiträge sind willkommen: Fehlermeldungen, Ideen und Pull Requests.

## Fehler und Wünsche
Eröffne ein Issue mit der passenden Vorlage. Bei Fehlern helfen: macOS-Version, Schritte zum Nachstellen, wenn möglich eine kleine Beispieldatei (keine privaten Daten).

## Code beitragen
1. Fork anlegen, Branch von `main` erstellen.
2. Änderung klein halten und beschreiben, *warum* sie nötig ist.
3. Vor dem Pull Request müssen diese Befehle grün sein:
   ```bash
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   scripts/bundle-app.sh && app/.build/debug/SnapiousSelfTest
   ```
4. Neue Rechenlogik gehört in `crates/snapious-core` und bekommt Tests mit von Hand nachgerechneten Erwartungswerten.
   Oberfläche (SwiftUI) bleibt dünn und ruft den Kern über `crates/snapious-ffi` auf.
5. Koordinaten im Kern sind 0-basiert und Ende exklusiv.

## Stil
Deutsche Texte in der Oberfläche und in Kommentaren, Standard-macOS-Look, kein eigenes Designsystem erfinden.
