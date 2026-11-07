//! Konsens aus mehreren ausgerichteten Sequenzierungen (z. B. Sanger-Reads) mit Qualitätswerten.

/// Konsens und Qualität je Konsensbase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadConsensus {
    /// Konsenssequenz ohne Lücken (Uppercase).
    pub bases: String,
    /// Qualität je Base von `bases` (Phred-ähnlich, 0–93).
    pub qualities: Vec<u8>,
}

/// Bildet den Konsens aus ausgerichteten Zeilen `rows` (Lücke = `-`, Groß-/Kleinschreibung egal). `quals[i]` enthält je **Nicht-Lücken-Base**
/// der Zeile `i` einen Qualitätswert, in der Reihenfolge der Basen (Lücken verbrauchen keinen Wert; fehlen Werte, gilt Qualität 0).
///
/// Je Spalte zählt jede Base `A`, `C`, `G`, `T` aus einer Zeile mit ihrer Qualität als Gewicht, sofern die Qualität `>= min_quality` ist
/// (alle anderen Zeichen und Basen unter der Schwelle zählen nicht). Gewonnen hat die Base mit dem größten Gewicht (bei Gleichstand die in der Reihenfolge
/// `A`, `C`, `G`, `T` erste). Die Qualität der Konsensbase ist `gewicht(Sieger) - Summe der Gewichte der übrigen Basen`, mindestens 0 und höchstens 93.
/// Gibt es in der Spalte keine zählende Base, aber mindestens eine Nicht-Lücken-Base (z. B. `N` oder zu niedrige Qualität), ist die Konsensbase `N`
/// mit Qualität 0. Spalten, in denen alle Zeilen eine Lücke haben, entfallen. Alle Zeilen sollten gleich lang sein (kürzere Zeilen zählen
/// am Ende als Lücke). Keine Zeilen → leeres Ergebnis.
pub fn quality_consensus(rows: &[String], quals: &[Vec<u8>], min_quality: u8) -> ReadConsensus {
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let bytes: Vec<&[u8]> = rows.iter().map(|r| r.as_bytes()).collect();
    // je Zeile: Zeiger auf den nächsten Qualitätswert (rückt nur bei Nicht-Lücken-Basen vor)
    let mut next_q = vec![0usize; rows.len()];
    let mut out = ReadConsensus {
        bases: String::new(),
        qualities: Vec::new(),
    };
    for col in 0..width {
        let mut weights = [0u32; 4];
        let mut any_base = false;
        for (r, row) in bytes.iter().enumerate() {
            let Some(&b) = row.get(col) else { continue };
            if b == b'-' {
                continue;
            }
            any_base = true;
            let q = quals
                .get(r)
                .and_then(|v| v.get(next_q[r]))
                .copied()
                .unwrap_or(0);
            next_q[r] += 1;
            if q < min_quality {
                continue;
            }
            if let Some(idx) = b"ACGT".iter().position(|x| *x == b.to_ascii_uppercase()) {
                weights[idx] += q as u32;
            }
        }
        if !any_base {
            continue;
        }
        let total: u32 = weights.iter().sum();
        if total == 0 {
            out.bases.push('N');
            out.qualities.push(0);
            continue;
        }
        // erster Index mit dem größten Gewicht (Reihenfolge A, C, G, T)
        let mut best = 0;
        for i in 1..4 {
            if weights[i] > weights[best] {
                best = i;
            }
        }
        out.bases.push(b"ACGT"[best] as char);
        out.qualities
            .push((weights[best] as i64 - (total - weights[best]) as i64).clamp(0, 93) as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn gewichtet() {
        let rows = v(&["ACGT", "ACGT", "ACTT"]);
        let q = vec![
            vec![40, 40, 40, 40],
            vec![30, 30, 30, 30],
            vec![10, 10, 10, 10],
        ];
        let c = quality_consensus(&rows, &q, 0);
        assert_eq!(c.bases, "ACGT");
        assert_eq!(c.qualities, vec![80, 80, 60, 80]);
    }

    #[test]
    fn luecken_und_schwelle() {
        let rows = v(&["AC-T", "ACGT"]);
        let q = vec![vec![40, 40, 40], vec![20, 20, 20, 20]];
        let c = quality_consensus(&rows, &q, 0);
        assert_eq!(c.bases, "ACGT");
        assert_eq!(c.qualities, vec![60, 60, 20, 60]);
        let c = quality_consensus(&rows, &q, 25);
        assert_eq!(c.bases, "ACNT");
        assert_eq!(c.qualities, vec![40, 40, 0, 40]);
    }

    #[test]
    fn randfaelle() {
        assert_eq!(
            quality_consensus(&[], &[], 0),
            ReadConsensus {
                bases: String::new(),
                qualities: vec![]
            }
        );
        // nur Lückenspalte entfällt
        let c = quality_consensus(&v(&["A-C", "A-C"]), &[vec![10, 10], vec![10, 10]], 0);
        assert_eq!(c.bases, "AC");
        // Gleichstand: A vor G
        let c = quality_consensus(&v(&["A", "G"]), &[vec![30], vec![30]], 0);
        assert_eq!((c.bases.as_str(), c.qualities[0]), ("A", 0));
        // Qualität höchstens 93
        let c = quality_consensus(&v(&["A", "A", "A"]), &[vec![60], vec![60], vec![60]], 0);
        assert_eq!(c.qualities, vec![93]);
        // fehlende Werte zählen 0, Base N zählt nicht
        let c = quality_consensus(&v(&["AN"]), &[vec![40]], 0);
        assert_eq!(c.bases, "AN");
    }
}
