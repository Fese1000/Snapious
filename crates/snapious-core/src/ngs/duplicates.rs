//! Mark PCR duplicates by position.

use std::collections::BTreeMap;

/// A read for duplicate detection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadKey {
    pub reference: String,
    /// Start position.
    pub position: usize,
    pub reverse: bool,
    /// Sum of the quality values (higher = better).
    pub quality: u64,
}

/// Marks duplicates: reads with the same reference, position and direction form a group. In each group the read with the
/// highest `quality` stays unmarked (on a tie the one with the smallest index), all others are duplicates. Result: per read `true` if it is a
/// duplicate (same length and order as `reads`).
pub fn mark_duplicates(reads: &[ReadKey]) -> Vec<bool> {
    if reads.is_empty() {
        return vec![];
    }

    // Group the indices by (reference, position, reverse)
    let mut groups: BTreeMap<(String, usize, bool), Vec<usize>> = BTreeMap::new();

    for (i, read) in reads.iter().enumerate() {
        let key = (read.reference.clone(), read.position, read.reverse);
        groups.entry(key).or_default().push(i);
    }

    // Initialize the result array (all false)
    let mut result = vec![false; reads.len()];

    // Go through each group and mark the best read (false), the rest true
    for indices in groups.values() {
        // Find the index of the best read in this group
        let mut best_idx = indices[0];

        for &idx in indices.iter().skip(1) {
            let current_quality = reads[idx].quality;
            let best_quality = reads[best_idx].quality;

            if current_quality > best_quality {
                // Higher quality -> new best
                best_idx = idx;
            } else if current_quality == best_quality {
                // Equal quality -> smaller index wins (already marked false in the result)
                // Since we go through the indices in ascending order (0, 1, 2...),
                // the first index found (smallest index) remains the winner.
                // So we need to do nothing if current_quality <= best_quality.
            }
        }

        // All indices in the group are duplicates except the best one
        for &idx in indices.iter() {
            if idx != best_idx {
                result[idx] = true;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(reference: &str, position: usize, reverse: bool, quality: u64) -> ReadKey {
        ReadKey {
            reference: reference.into(),
            position,
            reverse,
            quality,
        }
    }

    #[test]
    fn gruppen() {
        let reads = vec![
            k("c1", 10, false, 30),
            k("c1", 10, false, 50),
            k("c1", 10, true, 20),
            k("c2", 10, false, 20),
            k("c1", 10, false, 50),
            k("c1", 11, false, 10),
        ];
        assert_eq!(
            mark_duplicates(&reads),
            vec![true, false, false, false, true, false]
        );
    }

    #[test]
    fn leer_und_einzeln() {
        assert!(mark_duplicates(&[]).is_empty());
        assert_eq!(mark_duplicates(&[k("a", 1, false, 5)]), vec![false]);
        assert_eq!(
            mark_duplicates(&[
                k("a", 1, false, 5),
                k("a", 1, false, 5),
                k("a", 1, false, 5)
            ]),
            vec![false, true, true]
        );
    }
}
