# Card 1.3: Annotation tracks (feature lane assignment)

File: `crates/snapious-core/src/view/feature_layout.rs` (skeleton and tests exist)
Prerequisite: Card 0.1 (feature model with `Location`, `Interval`, `Strand`)
Goal: Distribute features onto tracks (lanes) so that no segments of the same track overlap, like an interval coloring problem.

Functions:
  `PlacedFeature::new(feature, segment, lane) -> Self`      // A feature segment on a specific lane
  `FeatureLayout::assign_lanes(features) -> Vec<PlacedFeature>` // Distributes all intervals of all features onto lanes

Algorithm (greedy, sorted by start, then length descending):
  1. For each feature, extract all intervals as `(start, end, feature_index)`.
  2. Sort all intervals by `start` ascending, then by `end - start` descending.
  3. For each interval: find the first lane whose last end is `<= interval.start`; place it there.
     Otherwise create a new lane (index = current number of lanes).
  4. All intervals of **one** feature must end up in the same lane:
     - First compute, for each feature, the lowest lane in which **all** its intervals fit.
     - To do so: sort features by `location.start()`, then length descending.
     - For each feature check whether all its intervals fit into the existing lane (no overlap).
     - If not, create a new lane and assign all segments of this feature to that new lane.

Data structures:
  `struct PlacedFeature { feature: Feature, segment: Interval, lane: usize }`
  `struct FeatureLayout;`

Edge cases: empty list → empty output, `lane_count = 0`; two disjoint features → both lane 0;
three overlapping features `[0,10)`, `[5,15)`, `[10,20)` → lanes 0, 1, 0;
join feature `[0,5)+[20,25)` and feature `[10,15)`: all fit into lane 0.

Done when: `cargo test -p snapious-core feature_layout` is green.
Do not: change tests; change other files; use `unwrap()`.
