//! Reads nach Barcode (MID) am Anfang aufteilen.

/// Ergebnis für einen Read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarcodeHit {
    /// Index des passenden Barcodes in der Barcodeliste, `None` ohne Treffer.
    pub barcode: Option<usize>,
    /// Der Read ohne Barcode (bei Treffer), sonst der unveränderte Read.
    pub trimmed: String,
}

/// Prüft jeden Read gegen die Barcodes `(Name, Sequenz)`. Ein Barcode der Länge `L` passt, wenn der Read mindestens `L` Zeichen hat
/// und sich die ersten `L` Zeichen in höchstens `max_mismatch` Stellen vom Barcode unterscheiden (Groß-/Kleinschreibung egal).
/// Gewählt wird der Barcode mit den wenigsten Unterschieden, bei Gleichstand der mit dem kleineren Index. Leere Barcodes passen nie.
/// Das Ergebnis hat so viele Einträge wie `reads`.
pub fn assign_barcodes(
    reads: &[String],
    barcodes: &[(String, String)],
    max_mismatch: usize,
) -> Vec<BarcodeHit> {
    reads
        .iter()
        .map(|read| {
            let r = read.as_bytes();
            let mut best: Option<(usize, usize)> = None; // (Unterschiede, Index)
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
        assert_eq!(r[0].barcode, Some(0)); // beide 1 Unterschied
        assert_eq!(r[1].barcode, Some(1)); // b genau (0), a hat 1
        assert!(assign_barcodes(&[], &b, 1).is_empty());
    }
}
