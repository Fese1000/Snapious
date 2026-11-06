//! Weitere Proteinkennzahlen: Aliphatic Index und GRAVY.

use crate::bio::protein::hydropathy_value;

/// Aliphatic Index: `X_A + 2.9 * X_V + 3.9 * (X_I + X_L)`, wobei `X` der Molanteil in Prozent (0–100) der Aminosäure an **allen**
/// Zeichen der Sequenz ist (Großbuchstaben; Groß-/Kleinschreibung egal; Leerzeichen zählen nicht mit). Leere Sequenz → `0.0`.
/// Beispiel: `AVIL` → 25 + 2.9·25 + 3.9·50 = 292.5.
pub fn aliphatic_index(protein: &str) -> f64 {
    let chars: Vec<char> = protein
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if chars.is_empty() {
        return 0.0;
    }
    let pct = |set: &[char]| {
        100.0 * chars.iter().filter(|c| set.contains(c)).count() as f64 / chars.len() as f64
    };
    pct(&['A']) + 2.9 * pct(&['V']) + 3.9 * (pct(&['I']) + pct(&['L']))
}

/// GRAVY: Mittelwert der Kyte-Doolittle-Werte (`hydropathy_value`) über alle Aminosäuren, für die es einen Wert gibt
/// (Groß-/Kleinschreibung egal; andere Zeichen zählen weder im Zähler noch im Nenner). Keine bewertbare Aminosäure → `0.0`.
/// Beispiel: `AIL` → (1.8 + 4.5 + 3.8) / 3.
pub fn gravy(protein: &str) -> f64 {
    let values: Vec<f64> = protein
        .chars()
        .filter_map(|c| hydropathy_value(c.to_ascii_uppercase()))
        .collect();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliphatisch() {
        assert!((aliphatic_index("AVIL") - 292.5).abs() < 1e-9);
        assert!((aliphatic_index("avil") - 292.5).abs() < 1e-9);
        assert!((aliphatic_index("AKKK") - 25.0).abs() < 1e-9);
        assert_eq!(aliphatic_index("KKKK"), 0.0);
        assert_eq!(aliphatic_index(""), 0.0);
        assert!((aliphatic_index("A V") - 0.5 * 100.0 * (1.0 + 2.9)).abs() < 1e-9);
    }

    #[test]
    fn gravy_werte() {
        assert!((gravy("AIL") - (1.8 + 4.5 + 3.8) / 3.0).abs() < 1e-9);
        assert!((gravy("ail") - (1.8 + 4.5 + 3.8) / 3.0).abs() < 1e-9);
        assert!((gravy("AXZ") - 1.8).abs() < 1e-9);
        assert_eq!(gravy(""), 0.0);
        assert_eq!(gravy("XXX"), 0.0);
    }
}
