//! Split reads by barcode (MID) at the start.

/// Result for one read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarcodeHit {
    /// Index of the matching barcode in the barcode list, `None` if no match.
    pub barcode: Option<usize>,
    /// The read without barcode (on a match), otherwise the unchanged read.
    pub trimmed: String,
}

/// Checks each read against the barcodes `(name, sequence)`. A barcode of length `L` matches if the read has at least `L` characters
/// and the first `L` characters differ from the barcode in at most `max_mismatch` positions (case-insensitive).
/// The barcode with the fewest differences is chosen, on a tie the one with the smaller index. Empty barcodes never match.
/// The result has as many entries as `reads`.
pub fn assign_barcodes(
    reads: &[String],
    barcodes: &[(String, String)],
    max_mismatch: usize,
) -> Vec<BarcodeHit> {
    reads
        .iter()
        .map(|read| {
            let r = read.as_bytes();
            let mut best: Option<(usize, usize)> = None; // (differences, index)
            for (i, (_, code)) in barcodes.iter().enumerate() {
                let b = code.as_bytes();
                if b.is_empty() || r.len() < b.len() {
                    continue;
                }
                let diff = b
                    .iter()
                    .zip(r)
                    .filter(|(x, y)| !x.eq_ignore_ascii_case(y))
                    .count();
                if diff <= max_mismatch && best.is_none_or(|(d, _)| diff < d) {
                    best = Some((diff, i));
                }
            }
            match best {
                Some((_, i)) => BarcodeHit {
                    barcode: Some(i),
                    trimmed: read.get(barcodes[i].1.len()..).unwrap_or("").to_string(),
                },
                None => BarcodeHit {
                    barcode: None,
                    trimmed: read.clone(),
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bc() -> Vec<(String, String)> {
        vec![
            ("b1".to_string(), "ACGT".to_string()),
            ("b2".to_string(), "TTGA".to_string()),
        ]
    }

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn genau_und_mit_fehlern() {
        let reads = v(&["ACGTGGGG", "ACCTGGGG", "ttgaCC", "AC", "GGGGGGGG"]);
        let r = assign_barcodes(&reads, &bc(), 0);
        assert_eq!(
            r[0],
            BarcodeHit {
                barcode: Some(0),
                trimmed: "GGGG".into()
            }
        );
        assert_eq!(
            r[1],
            BarcodeHit {
                barcode: None,
                trimmed: "ACCTGGGG".into()
            }
        );
        assert_eq!(
            r[2],
            BarcodeHit {
                barcode: Some(1),
                trimmed: "CC".into()
            }
        );
        assert_eq!(
            r[3],
            BarcodeHit {
                barcode: None,
                trimmed: "AC".into()
            }
        );
        assert_eq!(r[4].barcode, None);
        let r1 = assign_barcodes(&reads, &bc(), 1);
        assert_eq!(
            r1[1],
            BarcodeHit {
                barcode: Some(0),
                trimmed: "GGGG".into()
            }
        );
    }

    #[test]
    fn gleichstand_und_leer() {
        let b = vec![
            ("a".to_string(), "AAAA".to_string()),
            ("b".to_string(), "AAAT".to_string()),
            ("leer".to_string(), String::new()),
        ];
        let r = assign_barcodes(&v(&["AAACGG", "AAATGG"]), &b, 1);
        assert_eq!(r[0].barcode, Some(0)); // both 1 difference
        assert_eq!(r[1].barcode, Some(1)); // b exact (0), a has 1
        assert!(assign_barcodes(&[], &b, 1).is_empty());
    }
}
