//! Merge paired-end reads by name.

use std::collections::BTreeMap;

/// Result of pairing (indices into the name list).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairResult {
    /// `(forward read, reverse read)`, ascending by the index of the forward read.
    pub pairs: Vec<(usize, usize)>,
    /// Reads without a mate or without recognizable pair information, ascending.
    pub unpaired: Vec<usize>,
}

/// Detects pairs: a name ends with `/1` or `/2` (key = name without these two characters), or it contains a space and
/// the part after it starts with `1:` or `2:` (key = part before the first space). Reads with the same key and the
/// numbers 1 and 2 form a pair (if a number occurs more than once, the first occurrence counts, the others are unpaired).
/// All other reads are unpaired.
pub fn pair_reads(names: &[String]) -> PairResult {
    // Map for found pairs: key -> (index_of_1, index_of_2)
    // We use a BTreeMap to be deterministic.
    let mut pairs_map: BTreeMap<String, (Option<usize>, Option<usize>)> = BTreeMap::new();
    // List of unpaired reads (indices)
    let mut unpaired_indices: Vec<usize> = Vec::new();

    for (idx, name) in names.iter().enumerate() {
        let (key, num) = match parse_pair_info(name) {
            Some(info) => info,
            None => {
                unpaired_indices.push(idx);
                continue;
            }
        };

        // Get or create the entry for the key
        let entry = pairs_map.entry(key).or_insert((None, None));

        match num {
            1 => {
                if entry.0.is_none() {
                    // First /1 found for this key
                    entry.0 = Some(idx);
                } else {
                    // Duplicate of /1 -> unpaired
                    unpaired_indices.push(idx);
                }
            }
            2 => {
                if entry.1.is_none() {
                    // First /2 found for this key
                    entry.1 = Some(idx);
                } else {
                    // Duplicate of /2 -> unpaired
                    unpaired_indices.push(idx);
                }
            }
            _ => {} // Should not happen thanks to parse_pair_info
        }
    }

    // Extract pairs
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    for (_, (first_opt, second_opt)) in pairs_map {
        if let (Some(first), Some(second)) = (first_opt, second_opt) {
            pairs.push((first, second));
        } else {
            // If only one is present (e.g. only /1 but no /2), are both unpaired?
            // The task statement says: "Reads with the same key and the numbers 1 and 2 form a pair".
            // "All other reads are unpaired."
            // So if we have a /1 but no matching /2 (because the key differs or is missing),
            // this /1 is unpaired. The same for /2 without /1.
            if let Some(first) = first_opt {
                unpaired_indices.push(first);
            }
            if let Some(second) = second_opt {
                unpaired_indices.push(second);
            }
        }
    }

    // Sort pairs by the index of the forward read (first value in the tuple)
    pairs.sort_by_key(|&(f, _)| f);

    // Sort the unpaired list (ascending by index)
    unpaired_indices.sort();

    PairResult {
        pairs,
        unpaired: unpaired_indices,
    }
}

/// Parses a read name for pair information.
/// Returns `(key, number)`, where number is 1 or 2.
fn parse_pair_info(name: &str) -> Option<(String, u8)> {
    // Rule 1: ends with `/1` or `/2`
    if let Some(stripped) = name.strip_suffix("/1") {
        return Some((stripped.to_string(), 1));
    }
    if let Some(stripped) = name.strip_suffix("/2") {
        return Some((stripped.to_string(), 2));
    }

    // Rule 2: contains a space, the part after it starts with `1:` or `2:`
    if let Some(space_pos) = name.find(' ') {
        let suffix = &name[space_pos + 1..];
        if suffix.starts_with("1:") {
            // Key is the part before the first space
            return Some((name[..space_pos].to_string(), 1));
        }
        if suffix.starts_with("2:") {
            return Some((name[..space_pos].to_string(), 2));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn schraegstrich_und_illumina() {
        let names = v(&["r1/1", "r2/1", "r1/2", "r3 2:N:0", "r3 1:N:0", "x", "r4/2"]);
        let r = pair_reads(&names);
        assert_eq!(r.pairs, vec![(0, 2), (4, 3)]);
        assert_eq!(r.unpaired, vec![1, 5, 6]);
    }

    #[test]
    fn doppelte_und_leer() {
        let r = pair_reads(&v(&["a/1", "a/1", "a/2"]));
        assert_eq!(r.pairs, vec![(0, 2)]);
        assert_eq!(r.unpaired, vec![1]);
        let leer = pair_reads(&[]);
        assert!(leer.pairs.is_empty() && leer.unpaired.is_empty());
    }
}
