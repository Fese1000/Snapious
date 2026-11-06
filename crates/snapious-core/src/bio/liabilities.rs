//! Bekannte Schwachstellen in Proteinsequenzen (z. B. Antikörper): Motive, die chemisch oder durch Glykosylierung Probleme machen.

/// Eine gefundene Schwachstelle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Liability {
    /// Beginn (0-basiert) in der Proteinsequenz.
    pub position: usize,
    /// Länge des Motivs (1–3).
    pub length: usize,
    /// Art, genau einer der Texte: `N-Glykosylierung`, `Deamidierung`, `Isomerisierung`, `Oxidation`, `Freies Cystein`.
    pub kind: String,
    /// Die Aminosäuren des Motivs in Großbuchstaben.
    pub motif: String,
}

/// Sucht alle Schwachstellen (Großbuchstaben erwartet, andere Zeichen werden nur als Platzhalter gezählt; Groß-/Kleinschreibung egal).
/// Regeln, überlappende Treffer sind erlaubt:
/// * `N-Glykosylierung`: drei Aminosäuren `N`, dann irgendeine außer `P`, dann `S` oder `T` (Länge 3).
/// * `Deamidierung`: `NG` oder `NS` (Länge 2).
/// * `Isomerisierung`: `DG` oder `DS` (Länge 2).
/// * `Oxidation`: jedes `M` und jedes `W` (Länge 1).
/// * `Freies Cystein`: jedes `C` (Länge 1).
///
/// Sortierung: nach `position`, bei gleicher Position in der Reihenfolge der obigen Liste (Glykosylierung, Deamidierung,
/// Isomerisierung, Oxidation, Cystein). Beispiel `ANSTGDSMWC`: Glykosylierung `NST` (1), Deamidierung `NS` (1), Isomerisierung `DS` (5),
/// Oxidation `M` (7) und `W` (8), Cystein `C` (9).
pub fn find_liabilities(protein: &str) -> Vec<Liability> {
    let seq: Vec<u8> = protein.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let mut results: Vec<Liability> = Vec::new();

    for i in 0..seq.len() {
        let c = seq[i];

        // Regel 1: N-Glykosylierung (Länge 3)
        if c == b'N' && i + 2 < seq.len() {
            let s = seq[i + 1];
            if s != b'P' {
                let t = seq[i + 2];
                if t == b'S' || t == b'T' {
                    results.push(Liability {
                        position: i,
                        length: 3,
                        kind: "N-Glykosylierung".to_string(),
                        motif: String::from_utf8_lossy(&seq[i..i + 3]).to_string(),
                    });
                }
            }
        }

        // Regel 2: Deamidierung (Länge 2)
        if c == b'N' && i + 1 < seq.len() {
            let next = seq[i + 1];
            if next == b'G' || next == b'S' {
                results.push(Liability {
                    position: i,
                    length: 2,
                    kind: "Deamidierung".to_string(),
                    motif: String::from_utf8_lossy(&seq[i..i + 2]).to_string(),
                });
            }
        }

        // Regel 3: Isomerisierung (Länge 2)
        if c == b'D' && i + 1 < seq.len() {
            let next = seq[i + 1];
            if next == b'G' || next == b'S' {
                results.push(Liability {
                    position: i,
                    length: 2,
                    kind: "Isomerisierung".to_string(),
                    motif: String::from_utf8_lossy(&seq[i..i + 2]).to_string(),
                });
            }
        }

        // Regel 4: Oxidation (Länge 1)
        if c == b'M' || c == b'W' {
            results.push(Liability {
                position: i,
                length: 1,
                kind: "Oxidation".to_string(),
                motif: String::from_utf8_lossy(&seq[i..i + 1]).to_string(),
            });
        }

        // Regel 5: Freies Cystein (Länge 1)
        if c == b'C' {
            results.push(Liability {
                position: i,
                length: 1,
                kind: "Freies Cystein".to_string(),
                motif: String::from_utf8_lossy(&seq[i..i + 1]).to_string(),
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(position: usize, length: usize, kind: &str, motif: &str) -> Liability {
        Liability {
            position,
            length,
            kind: kind.to_string(),
            motif: motif.to_string(),
        }
    }

    #[test]
    fn beispiel() {
        assert_eq!(
            find_liabilities("ANSTGDSMWC"),
            vec![
                l(1, 3, "N-Glykosylierung", "NST"),
                l(1, 2, "Deamidierung", "NS"),
                l(5, 2, "Isomerisierung", "DS"),
                l(7, 1, "Oxidation", "M"),
                l(8, 1, "Oxidation", "W"),
                l(9, 1, "Freies Cystein", "C"),
            ]
        );
    }

    #[test]
    fn prolin_klein_und_leer() {
        assert!(find_liabilities("NPT").is_empty());
        assert!(find_liabilities("").is_empty());
        assert!(find_liabilities("AKLV").is_empty());
        assert_eq!(
            find_liabilities("ngt"),
            vec![
                l(0, 3, "N-Glykosylierung", "NGT"),
                l(0, 2, "Deamidierung", "NG")
            ]
        );
        // zwei Glykosylierungsstellen überlappen: NNSS
        let r = find_liabilities("NNST");
        assert_eq!(r[0], l(0, 3, "N-Glykosylierung", "NNS"));
        assert_eq!(r[1], l(1, 3, "N-Glykosylierung", "NST"));
    }
}
