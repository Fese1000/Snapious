//! Überlappungs-Zusammenbau (Gibson, In-Fusion, SLIC): Fragmente über gleiche Enden verbinden.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location};
use crate::model::sequence::{Sequence, SequenceType};
use crate::primer::tm::nearest_neighbor;

/// Ein gefundener Überlapp zwischen zwei Fragmenten.
#[derive(Clone, Debug, PartialEq)]
pub struct Overlap {
    /// Index des Fragments, an dessen Ende der Überlapp steht, und des Fragments, an dessen Anfang er steht.
    pub from_index: usize,
    pub to_index: usize,
    pub length: usize,
    /// Schmelztemperatur des Überlapps in °C (`nearest_neighbor(seq, 250.0, 50.0)`).
    pub tm: f64,
}

/// Ergebnis eines Zusammenbaus.
#[derive(Clone, Debug, PartialEq)]
pub struct AssemblyResult {
    pub construct: SequenceDocument,
    pub overlaps: Vec<Overlap>,
    pub warnings: Vec<String>,
}

/// Einstellungen: erlaubte Überlapplängen (beide Grenzen eingeschlossen).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlapAssembly {
    pub min_overlap: usize,
    pub max_overlap: usize,
}

impl OverlapAssembly {
    pub fn new(min_overlap: usize, max_overlap: usize) -> OverlapAssembly {
        OverlapAssembly {
            min_overlap,
            max_overlap,
        }
    }

    /// 15 bis 40.
    pub fn gibson() -> OverlapAssembly {
        Self::new(15, 40)
    }

    /// 15 bis 20.
    pub fn in_fusion() -> OverlapAssembly {
        Self::new(15, 20)
    }

    /// 25 bis 80.
    pub fn slic() -> OverlapAssembly {
        Self::new(25, 80)
    }

    /// Längster Überlapp `k` mit `min_overlap <= k <= max_overlap` (und `k <=` Länge beider Texte), bei dem die letzten
    /// `k` Basen von `left` gleich den ersten `k` Basen von `right` sind (Großbuchstaben, `U` wie `T`).
    /// `None`, wenn es keinen gibt.
    pub fn find_overlap(&self, left: &str, right: &str) -> Option<usize> {
        let l = left.len();
        let r = right.len();
        let max_k = self.max_overlap.min(l).min(r);

        for k in (self.min_overlap..=max_k).rev() {
            let left_slice = &left.as_bytes()[l - k..];
            let right_slice = &right.as_bytes()[..k];

            if Self::matches_case_insensitive(left_slice, right_slice) {
                return Some(k);
            }
        }
        None
    }

    fn matches_case_insensitive(a: &[u8], b: &[u8]) -> bool {
        a.iter().zip(b.iter()).all(|(&x, &y)| {
            let x = x.to_ascii_uppercase();
            let y = y.to_ascii_uppercase();
            (x == y) || (x == b'T' && y == b'U') || (x == b'U' && y == b'T')
        })
    }

    /// Setzt `fragments` in der gegebenen Reihenfolge zusammen.
    pub fn assemble(
        &self,
        fragments: &[SequenceDocument],
        circular: bool,
    ) -> Result<AssemblyResult> {
        if fragments.is_empty() {
            return Err(Error::InvalidInput("Keine Fragmente angegeben".to_string()));
        }

        let n = fragments.len();
        let mut overlaps: Vec<Overlap> = Vec::with_capacity(n);

        // Pairs to check for overlaps
        let mut pairs: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        if circular {
            if n > 1 {
                pairs.push((n - 1, 0));
            } else if n == 1 {
                pairs.push((0, 0)); // Selbstüberlappung für einzelnes zirkuläres Fragment
            }
        }

        for &(from, to) in &pairs {
            let left = fragments[from].bases();
            let right = fragments[to].bases();

            match self.find_overlap(left, right) {
                Some(k) => overlaps.push(Overlap {
                    from_index: from,
                    to_index: to,
                    length: k,
                    tm: 0.0,
                }),
                None => {
                    return Err(Error::InvalidInput(format!(
                        "Kein Überlapp zwischen '{}' und '{}'",
                        fragments[from].name(),
                        fragments[to].name()
                    )));
                }
            }
        }

        // Build sequence string
        let mut bases = String::new();
        for (i, frag) in fragments.iter().enumerate() {
            if i == 0 {
                bases.push_str(frag.bases());
            } else {
                let k = overlaps[i - 1].length;
                bases.push_str(&frag.bases()[k..]);
            }
        }

        // Circular truncation of the string
        let construct_bases = if circular && n > 1 {
            let last_k = overlaps.last().map(|o| o.length).unwrap_or(0);
            bases.truncate(bases.len() - last_k);
            bases
        } else {
            bases
        };

        // Compute TM and warnings for overlaps
        let mut construct_overlaps = Vec::with_capacity(overlaps.len());
        let mut warnings = Vec::new();

        for ov in &overlaps {
            let left_bases = fragments[ov.from_index].bases();
            // Extract overlap sequence from the end of 'left'
            let ov_seq = &left_bases[left_bases.len() - ov.length..];

            // Clone first to allow mutation (fixes E0594)
            let mut new_ov = ov.clone();

            match nearest_neighbor(ov_seq, 250.0, 50.0) {
                Ok(tm_val) => new_ov.tm = tm_val,
                Err(_) => new_ov.tm = 0.0,
            }

            let mut ov_warnings = Vec::new();
            if new_ov.tm < 48.0 {
                ov_warnings.push("Tm unter 48 °C".to_string());
            }
            if new_ov.tm > 65.0 {
                ov_warnings.push("Tm über 65 °C".to_string());
            }

            // GC content check on overlap sequence
            let gc = ov_seq
                .bytes()
                .filter(|b| matches!(b.to_ascii_uppercase(), b'G' | b'C'))
                .count() as f64;
            let total = ov_seq.len() as f64;
            if total > 0.0 {
                let gc_frac = gc / total;
                if gc_frac < 0.2 || gc_frac > 0.8 {
                    ov_warnings.push("GC-Gehalt außerhalb von 20–80 %".to_string());
                }
            }

            let header = format!("Überlapp {}→{}: ", ov.from_index + 1, ov.to_index + 1);
            for w in &ov_warnings {
                warnings.push(format!("{}{}", header, w));
            }

            construct_overlaps.push(new_ov);
        }

        // Features handling
        let mut final_features: Vec<Feature> = Vec::new();
        let mut offset = 0usize;

        for (i, frag) in fragments.iter().enumerate() {
            let k = if i == 0 { 0 } else { overlaps[i - 1].length };

            for feat in frag.features() {
                let loc = feat.location();

                // Shift intervals by current offset
                let shifted_intervals: Vec<Interval> = loc
                    .intervals()
                    .iter()
                    .filter_map(|&iv| Interval::new(iv.start() + offset, iv.end() + offset).ok())
                    .collect();

                let shifted_loc = Location::new(
                    shifted_intervals,
                    loc.strand(),
                    loc.partial_start(),
                    loc.partial_end(),
                )
                .ok();
                let shifted_feat = match shifted_loc {
                    Some(l) => Feature::new(
                        feat.feature_type(),
                        feat.name(),
                        l,
                        feat.qualifiers().to_vec(),
                    ),
                    None => continue, // Should not happen with valid inputs
                };

                // Deduplication check for overlap region
                if loc.end() <= k {
                    let mut is_dup = false;
                    for existing in &final_features {
                        if existing.feature_type() == shifted_feat.feature_type()
                            && existing.name() == shifted_feat.name()
                            && existing.location().strand() == shifted_feat.location().strand()
                            && existing.location().start() == shifted_feat.location().start()
                            && existing.location().end() == shifted_feat.location().end()
                        {
                            is_dup = true;
                            break;
                        }
                    }
                    if !is_dup {
                        final_features.push(shifted_feat);
                    }
                } else {
                    final_features.push(shifted_feat);
                }
            }

            // Update offset for next fragment
            if i < n - 1 {
                offset += frag.sequence().len() - overlaps[i].length;
            }
        }

        // Circular truncation of features: drop those beyond construct length
        if circular && n > 1 {
            let construct_len = construct_bases.len();
            final_features.retain(|f| f.location().end() <= construct_len);
        }

        // Create document
        let name = fragments
            .iter()
            .map(|f| f.name())
            .collect::<Vec<&str>>()
            .join(" + ");
        let seq = Sequence::new(&name, &name, &construct_bases, SequenceType::Dna)
            .map_err(|e| Error::InvalidInput(format!("Ungültige Sequenz: {}", e)))?;

        let doc = SequenceDocument::of(&name, seq)
            .with_topology(if circular {
                Topology::Circular
            } else {
                Topology::Linear
            })
            .with_features(final_features)
            .with_metadata({
                let mut m = BTreeMap::new();
                m.insert("source".to_string(), "clone.overlap".to_string());
                m
            });

        Ok(AssemblyResult {
            construct: doc,
            overlaps: construct_overlaps,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Interval, Location, Strand};

    const O1: &str = "ACGTACGTACGTACGTACGT";
    const O2: &str = "TTGGCCAATTGGCCAATTGG";
    const O3: &str = "GGGGCCCCGGGGCCCCGGGG";

    fn doc(name: &str, bases: &str) -> SequenceDocument {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        SequenceDocument::of(name, seq)
    }

    fn x() -> String {
        "AT".repeat(15)
    }
    fn y() -> String {
        format!("{}CC", "CCGG".repeat(7))
    }
    fn z() -> String {
        "TA".repeat(15)
    }

    fn feat(start: usize, end: usize) -> Feature {
        let loc = Location::new(
            vec![Interval::new(start, end).unwrap()],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        Feature::new("misc_feature", "o", loc, Vec::new())
    }

    #[test]
    fn voreinstellungen() {
        assert_eq!(OverlapAssembly::gibson(), OverlapAssembly::new(15, 40));
        assert_eq!(OverlapAssembly::in_fusion(), OverlapAssembly::new(15, 20));
        assert_eq!(OverlapAssembly::slic(), OverlapAssembly::new(25, 80));
    }

    #[test]
    fn ueberlapp_suchen() {
        let a = OverlapAssembly::gibson();
        assert_eq!(
            a.find_overlap(&format!("TT{}", O1), &format!("{}GG", O1)),
            Some(20)
        );
        // längster Überlapp gewinnt, auch wenn kürzere (periodische) passen
        assert_eq!(
            a.find_overlap(&format!("A{}", O1), &format!("{}C", O1)),
            Some(20)
        );
        assert_eq!(a.find_overlap("AAAA", "CCCC"), None);
        // zu kurz für min_overlap 15
        assert_eq!(a.find_overlap("TTTTGATTACAGGT", "GATTACAGGTCCCC"), None);
        assert_eq!(
            OverlapAssembly::new(8, 40).find_overlap("TTTTGATTACAGGT", "GATTACAGGTCCCC"),
            Some(10)
        );
        // Schreibweise und U
        assert_eq!(
            a.find_overlap(
                &format!("tt{}", O1.to_lowercase()),
                &format!("{}GG", O1.replace('T', "U"))
            ),
            Some(20)
        );
        // obere Grenze
        assert_eq!(
            OverlapAssembly::new(15, 20)
                .find_overlap(&format!("T{}{}", O3, O3), &format!("{}{}", O3, O3)),
            Some(20)
        );
    }

    #[test]
    fn linearer_zusammenbau() {
        let a = doc("A", &format!("{}{}", x(), O1));
        let b = doc("B", &format!("{}{}{}", O1, y(), O2));
        let c = doc("C", &format!("{}{}", O2, z()));
        let r = OverlapAssembly::gibson()
            .assemble(&[a, b, c], false)
            .unwrap();
        assert_eq!(r.construct.len(), 130);
        assert_eq!(r.construct.topology(), Topology::Linear);
        assert_eq!(r.construct.name(), "A + B + C");
        assert_eq!(
            r.construct.bases(),
            format!("{}{}{}{}{}", x(), O1, y(), O2, z())
        );
        assert_eq!(r.overlaps.len(), 2);
        assert_eq!(
            (
                r.overlaps[0].from_index,
                r.overlaps[0].to_index,
                r.overlaps[0].length
            ),
            (0, 1, 20)
        );
        assert_eq!(
            (
                r.overlaps[1].from_index,
                r.overlaps[1].to_index,
                r.overlaps[1].length
            ),
            (1, 2, 20)
        );
        let tm = nearest_neighbor(O1, 250.0, 50.0).unwrap();
        assert!((r.overlaps[0].tm - tm).abs() < 1e-9);
        assert_eq!(
            r.construct.metadata().get("source").map(String::as_str),
            Some("clone.overlap")
        );
    }

    #[test]
    fn zirkulaerer_zusammenbau() {
        let a = doc("A", &format!("{}{}{}", O3, x(), O1));
        let b = doc("B", &format!("{}{}{}", O1, y(), O2));
        let c = doc("C", &format!("{}{}{}", O2, z(), O3));
        let r = OverlapAssembly::gibson()
            .assemble(&[a, b, c], true)
            .unwrap();
        assert_eq!(r.construct.len(), 210 - 60);
        assert_eq!(r.construct.topology(), Topology::Circular);
        assert_eq!(r.overlaps.len(), 3);
        assert_eq!((r.overlaps[2].from_index, r.overlaps[2].to_index), (2, 0));
        assert!(r.construct.bases().starts_with(O3));
    }

    #[test]
    fn fehler() {
        let a = doc("A", "ACGTACGTACGTACGTACGTAAAA");
        let b = doc("B", "GGGGCCCCGGGGCCCCGGGGTTTT");
        let e = OverlapAssembly::gibson().assemble(&[a, b], false);
        match e {
            Err(Error::InvalidInput(m)) => assert!(m.contains('A') && m.contains('B'), "{}", m),
            other => panic!("unerwartet: {:?}", other),
        }
        assert!(matches!(
            OverlapAssembly::gibson().assemble(&[], false),
            Err(Error::InvalidInput(_))
        ));
        let einzeln = OverlapAssembly::gibson()
            .assemble(&[doc("S", "ACGTAC")], false)
            .unwrap();
        assert_eq!(einzeln.construct.bases(), "ACGTAC");
        assert!(einzeln.overlaps.is_empty());
    }

    #[test]
    fn warnungen() {
        let poly = "A".repeat(20);
        let a = doc("A", &format!("CGCGCGCGCGCGCGCGCGCG{}", poly));
        let b = doc("B", &format!("{}GCGCGCGCGCGCGCGCGCGC", poly));
        let r = OverlapAssembly::gibson().assemble(&[a, b], false).unwrap();
        assert_eq!(r.warnings.len(), 2);
        assert!(
            r.warnings[0].starts_with("Überlapp 1→2: ") && r.warnings[0].contains("Tm unter 48 °C")
        );
        assert!(r.warnings[1].contains("GC-Gehalt außerhalb von 20–80 %"));
        let gut = OverlapAssembly::gibson()
            .assemble(
                &[
                    doc("A", &format!("{}{}", x(), O1)),
                    doc("B", &format!("{}{}", O1, y())),
                ],
                false,
            )
            .unwrap();
        assert!(gut.warnings.is_empty(), "{:?}", gut.warnings);
    }

    #[test]
    fn features_werden_uebernommen_und_dedupliziert() {
        let a = doc("A", &format!("{}{}{}", O3, x(), O1)).with_features(vec![feat(50, 70)]);
        let b = doc("B", &format!("{}{}", O1, y())).with_features(vec![feat(0, 20), feat(25, 30)]);
        let r = OverlapAssembly::gibson().assemble(&[a, b], false).unwrap();
        let mut starts: Vec<usize> = r
            .construct
            .features()
            .iter()
            .map(|f| f.location().start())
            .collect();
        starts.sort();
        // A endet bei 70, B beginnt im Konstrukt bei 70 − 20 = 50: Feature 0..20 von B ist dasselbe wie 50..70 von A und entfällt,
        // Feature 25..30 von B liegt bei 75..80
        assert_eq!(starts, vec![50, 75]);
    }
}
