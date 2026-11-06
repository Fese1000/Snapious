//! Verteilung von Features auf Spuren (Lanes), damit sich in der Anzeige nichts überdeckt.

use crate::model::feature::{Feature, Interval};

/// Ein Teilstück (Segment) eines Features mit seiner Spur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedFeature {
    /// Index des Features in der übergebenen Liste.
    pub feature_index: usize,
    pub segment: Interval,
    pub lane: usize,
}

/// Ordnet jedem Feature eine Spur zu. Regeln:
/// 1. Features werden in der Reihenfolge `location().start()` aufsteigend, bei Gleichstand nach
///    `location().length()` absteigend, bei weiterem Gleichstand nach Listenindex bearbeitet.
/// 2. Jedes Feature bekommt die kleinste Spur, in der keines seiner Segmente ein bereits dort liegendes
///    Segment überlappt (`Interval::overlaps`). Alle Segmente eines Features liegen in derselben Spur.
/// 3. Ergebnis: ein `PlacedFeature` je Segment, sortiert nach `feature_index`, dann nach Segmentreihenfolge im Feature.
pub fn assign_lanes(features: &[Feature]) -> Vec<PlacedFeature> {
    let count = features.len();
    if count == 0 {
        return Vec::new();
    }

    // Bestimme die Reihenfolge der Feature-Bearbeitung: nach kleinstem Start aller Segmente,
    // dann größte Gesamtlänge, dann Feature-Index.
    let mut feature_order: Vec<usize> = (0..count).collect();
    feature_order.sort_by(|&a, &b| {
        let a_intervals = features[a].location().intervals();
        let b_intervals = features[b].location().intervals();
        let a_start = *a_intervals
            .iter()
            .min_by_key(|i| i.start())
            .unwrap_or(&Interval::new(0, 0).unwrap());
        let b_start = *b_intervals
            .iter()
            .min_by_key(|i| i.start())
            .unwrap_or(&Interval::new(0, 0).unwrap());
        match a_start.start().cmp(&b_start.start()) {
            std::cmp::Ordering::Equal => {
                let a_len: usize = a_intervals.iter().map(|i| i.length()).sum();
                let b_len: usize = b_intervals.iter().map(|i| i.length()).sum();
                match b_len.cmp(&a_len) {
                    std::cmp::Ordering::Equal => a.cmp(&b),
                    other => other,
                }
            }
            other => other,
        }
    });

    // Sammle alle Segmente mit ihrem Feature-Index.
    struct SegmentEntry {
        feature_index: usize,
        interval: Interval,
    }

    let mut segments: Vec<SegmentEntry> = Vec::with_capacity(count * 2);
    for (fi, feature) in features.iter().enumerate() {
        let intervals = feature.location().intervals();
        for interval in intervals.iter() {
            segments.push(SegmentEntry {
                feature_index: fi,
                interval: *interval,
            });
        }
    }

    // Für jedes Feature alle Segment-Indizes sammeln.
    let mut feature_segments: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (i, seg) in segments.iter().enumerate() {
        feature_segments[seg.feature_index].push(i);
    }

    // Spur-Zuweisung: pro Feature alle Segmente in dieselbe Spur legen.
    let mut spuren: Vec<Vec<Interval>> = Vec::new();

    // Ergebnis: Platzhalter für jedes Segment.
    let mut result: Vec<Option<PlacedFeature>> = vec![None; segments.len()];

    // Verarbeite Features in der sortierten Reihenfolge.
    for &fi in &feature_order {
        let seg_indices = &feature_segments[fi];

        // Finde die kleinste Spur, in der ALLE Segmente dieses Features passen.
        let mut lane: usize = spuren.len();
        for candidate in 0..=spuren.len() {
            if candidate == spuren.len() {
                // Neue Spur anlegen.
                spuren.push(Vec::new());
                lane = candidate;
                break;
            }

            // Prüfe, ob alle Segmente dieses Features in diese Spur passen.
            let mut fits = true;
            for &si in seg_indices {
                if spuren[candidate]
                    .iter()
                    .any(|existing| existing.overlaps(&segments[si].interval))
                {
                    fits = false;
                    break;
                }
            }
            if fits {
                lane = candidate;
                break;
            }
        }

        // Füge alle Segmente dieser Feature in die Spur.
        for &si in seg_indices {
            spuren[lane].push(segments[si].interval);
            result[si] = Some(PlacedFeature {
                feature_index: segments[si].feature_index,
                segment: segments[si].interval,
                lane,
            });
        }
    }

    // Ergebnis sortiert nach feature_index, dann segment_order.
    let mut placed: Vec<PlacedFeature> = result.into_iter().flatten().collect();
    placed.sort_by_key(|p| p.feature_index);
    placed
}

/// Anzahl der benutzten Spuren (0 bei leerer Liste).
pub fn lane_count(placed: &[PlacedFeature]) -> usize {
    placed
        .iter()
        .map(|p| p.lane)
        .max()
        .map(|l| l + 1)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Location, Strand};

    fn feat(start: usize, end: usize) -> Feature {
        Feature::new(
            "gene",
            "g",
            Location::of(start, end, Strand::Forward).unwrap(),
            vec![],
        )
    }

    fn lanes(placed: &[PlacedFeature]) -> Vec<usize> {
        placed.iter().map(|p| p.lane).collect()
    }

    #[test]
    fn drei_features_zwei_spuren() {
        let p = assign_lanes(&[feat(0, 10), feat(5, 15), feat(10, 20)]);
        assert_eq!(lanes(&p), vec![0, 1, 0]);
        assert_eq!(lane_count(&p), 2);
        assert_eq!(p[1].feature_index, 1);
        assert_eq!(p[1].segment, Interval::new(5, 15).unwrap());
    }

    #[test]
    fn getrennte_features_teilen_eine_spur() {
        let p = assign_lanes(&[feat(0, 5), feat(10, 15), feat(20, 25)]);
        assert_eq!(lanes(&p), vec![0, 0, 0]);
        assert_eq!(lane_count(&p), 1);
    }

    #[test]
    fn leere_liste() {
        let p = assign_lanes(&[]);
        assert!(p.is_empty());
        assert_eq!(lane_count(&p), 0);
    }

    #[test]
    fn gleicher_start_laengeres_zuerst() {
        // B (0..10) ist länger und kommt zuerst in Spur 0; A (0..5) muss in Spur 1
        let p = assign_lanes(&[feat(0, 5), feat(0, 10)]);
        assert_eq!(lanes(&p), vec![1, 0]);
    }

    #[test]
    fn join_feature_belegt_eine_spur_fuer_alle_segmente() {
        let join = Feature::new(
            "CDS",
            "c",
            Location::new(
                vec![Interval::new(0, 5).unwrap(), Interval::new(20, 25).unwrap()],
                Strand::Forward,
                false,
                false,
            )
            .unwrap(),
            vec![],
        );
        let p = assign_lanes(&[join, feat(10, 15)]);
        // Segmente des Join-Features: Index 0, dann 1; danach das zweite Feature
        assert_eq!(p.len(), 3);
        assert_eq!((p[0].feature_index, p[0].lane), (0, 0));
        assert_eq!((p[1].feature_index, p[1].lane), (0, 0));
        assert_eq!(p[1].segment, Interval::new(20, 25).unwrap());
        // das zweite Feature überlappt kein Segment und passt in Spur 0
        assert_eq!((p[2].feature_index, p[2].lane), (1, 0));
    }

    #[test]
    fn join_feature_weicht_gemeinsam_aus() {
        let join = Feature::new(
            "CDS",
            "c",
            Location::new(
                vec![
                    Interval::new(10, 15).unwrap(),
                    Interval::new(30, 35).unwrap(),
                ],
                Strand::Forward,
                false,
                false,
            )
            .unwrap(),
            vec![],
        );
        // Das erste Feature (0..40) liegt in Spur 0 und überlapft beide Segmente
        let p = assign_lanes(&[feat(0, 40), join]);
        let join_lanes: Vec<usize> = p
            .iter()
            .filter(|x| x.feature_index == 1)
            .map(|x| x.lane)
            .collect();
        assert_eq!(join_lanes, vec![1, 1]);
    }

    #[test]
    fn spur_wird_wiederverwendet() {
        // Spur 0: [0,10) und [12,20'); Spur 1: [5,15)
        let p = assign_lanes(&[feat(0, 10), feat(5, 15), feat(12, 20)]);
        assert_eq!(lanes(&p), vec![0, 1, 0]);
    }

    #[test]
    fn ergebnis_ist_nach_feature_index_sortiert() {
        let p = assign_lanes(&[feat(30, 40), feat(0, 10), feat(5, 20)]);
        let idx: Vec<usize> = p.iter().map(|x| x.feature_index).collect();
        assert_eq!(idx, vec![0, 1, 2]);
    }
}
