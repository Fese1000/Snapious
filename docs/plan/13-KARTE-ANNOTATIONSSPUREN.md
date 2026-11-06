# Karte 1.3: Annotationsspuren (Feature-Lane-Zuordnung)

Datei: `crates/snapious-core/src/view/feature_layout.rs` (Gerüst und Tests vorhanden)
Voraussetzung: Karte 0.1 (Feature-Modell mit `Location`, `Interval`, `Strand`)
Ziel: Features so auf Spuren (Lanes) verteilen, dass sich keine Segmente derselben Spur überlappen — wie ein Intervall-Färbungsproblem.

Funktionen:
  `PlacedFeature::new(feature, segment, lane) -> Self`      // Ein Feature-Segment auf einer bestimmten Spur
  `FeatureLayout::assign_lanes(features) -> Vec<PlacedFeature>` // Verteilt alle Intervalle aller Features auf Spuren

Algorithmus (greedy, nach Start sortiert, dann Länge absteigend):
  1. Für jedes Feature alle Intervalle extrahieren als `(start, end, feature_index)`.
  2. Sortiere alle Intervalle nach `start` aufsteigend, dann nach `end - start` absteigend.
  3. Für jedes Intervall: finde die erste Spur, deren letztes Ende `<= interval.start` ist; lege dort hinein.
     Andernfalls: neue Spur anlegen (Index = aktuelle Anzahl der Spuren).
  4. Alle Intervalle **eines** Features müssen in derselben Spur landen:
     - Berechne zuerst für jedes Feature die minimale Spur, in der **alle** seine Intervalle passen.
     - Dazu: sortiere Features nach `location.start()`, dann Länge absteigend.
     - Für jedes Feature prüfe, ob alle seine Intervalle in die bisherige Spur passen (keine Überlappung).
     - Wenn nicht, lege eine neue Spur an und weise alle Segmente dieses Features dieser neuen Spur zu.

Datenstrukturen:
  `struct PlacedFeature { feature: Feature, segment: Interval, lane: usize }`
  `struct FeatureLayout;`

Randfälle: Leere Liste → leere Ausgabe, `lane_count = 0`; zwei disjunkte Features → beide Spur 0;
drei überlappende Features `[0,10)`, `[5,15)`, `[10,20)` → Spuren 0, 1, 0;
Join-Feature `[0,5)+[20,25)` und Feature `[10,15)`: alle passen in Spur 0.

Fertig, wenn: `cargo test -p snapious-core feature_layout` grün.
Nicht tun: Tests ändern; andere Dateien ändern; `unwrap()`.
