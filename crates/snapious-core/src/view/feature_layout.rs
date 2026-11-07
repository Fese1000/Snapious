//! Distribution of features onto lanes so that nothing overlaps in the display.

use crate::model::feature::{Feature, Interval};

/// A piece (segment) of a feature with its lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedFeature {
    /// Index of the feature in the passed list.
    pub feature_index: usize,
    pub segment: Interval,
    pub lane: usize,
}

/// Assigns a lane to each feature. Rules:
/// 1. Features are processed in the order `location().start()` ascending, ties by
///    `location().length()` descending, further ties by list index.
/// 2. Each feature gets the smallest lane in which none of its segments overlaps a segment already
///    there (`Interval::overlaps`). All segments of a feature lie in the same lane.
/// 3. Result: one `PlacedFeature` per segment, sorted by `feature_index`, then by segment order within the feature.
pub fn assign_lanes(features: &[Feature]) -> Vec<PlacedFeature> {
    let count = features.len();
    if count == 0 {
        return Vec::new();
    }

    // Determine the processing order of the features: by smallest start of all segments,
    // then largest total length, then feature index.
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

    // Collect all segments with their feature index.
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

    // Collect all segment indices for each feature.
    let mut feature_segments: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (i, seg) in segments.iter().enumerate() {
        feature_segments[seg.feature_index].push(i);
    }

    // Lane assignment: put all segments of a feature into the same lane.
    let mut spuren: Vec<Vec<Interval>> = Vec::new();

    // Result: placeholder for each segment.
    let mut result: Vec<Option<PlacedFeature>> = vec![None; segments.len()];

    // Process features in the sorted order.
    for &fi in &feature_order {
        let seg_indices = &feature_segments[fi];

        // Find the smallest lane in which ALL segments of this feature fit.
        let mut lane: usize = spuren.len();
        for candidate in 0..=spuren.len() {
            if candidate == spuren.len() {
                // Create a new lane.
                spuren.push(Vec::new());
                lane = candidate;
                break;
            }

            // Check whether all segments of this feature fit into this lane.
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

        // Add all segments of this feature to the lane.
        for &si in seg_indices {
            spuren[lane].push(segments[si].interval);
            result[si] = Some(PlacedFeature {
                feature_index: segments[si].feature_index,
                segment: segments[si].interval,
                lane,
            });
        }
    }

    // Result sorted by feature_index, then segment_order.
    let mut placed: Vec<PlacedFeature> = result.into_iter().flatten().collect();
    placed.sort_by_key(|p| p.feature_index);
    placed
}

/// Number of lanes used (0 for an empty list).
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
        // B (0..10) is longer and comes first in lane 0; A (0..5) must go into lane 1
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
        // Segments of the join feature: index 0, then 1; then the second feature
        assert_eq!(p.len(), 3);
        assert_eq!((p[0].feature_index, p[0].lane), (0, 0));
        assert_eq!((p[1].feature_index, p[1].lane), (0, 0));
        assert_eq!(p[1].segment, Interval::new(20, 25).unwrap());
        // the second feature overlaps no segment and fits into lane 0
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
        // The first feature (0..40) lies in lane 0 and overlaps both segments
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
        // Lane 0: [0,10) and [12,20'); lane 1: [5,15)
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
