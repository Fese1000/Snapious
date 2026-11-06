//! Kennzahlen der Abdeckung (Tiefe je Position).

/// Kennzahlen.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageStats {
    pub mean: f64,
    /// Median (bei gerader Zahl der Mittelwert der beiden mittleren Werte).
    pub median: f64,
    pub min: u32,
    pub max: u32,
    /// Je Schwelle der Anteil der Positionen mit Tiefe `>= Schwelle` (0.0–1.0), in der Reihenfolge von `thresholds`.
    pub fractions: Vec<f64>,
}

/// Berechnet die Kennzahlen. Keine Positionen → `mean 0.0`, `median 0.0`, `min 0`, `max 0` und `fractions` mit `0.0` je Schwelle.
/// Beispiel `[0, 0, 5, 10, 10, 20]` mit Schwellen `[1, 10]` → Mittel 7.5, Median 7.5, min 0, max 20, Anteile `[4/6, 3/6]`.
pub fn coverage_stats(depth: &[u32], thresholds: &[u32]) -> CoverageStats {
    let n = depth.len();

    if n == 0 {
        return CoverageStats {
            mean: 0.0,
            median: 0.0,
            min: 0,
            max: 0,
            fractions: vec![0.0; thresholds.len()],
        };
    }

    // Mittelwert berechnen
    let sum: u64 = depth.iter().map(|&d| d as u64).sum();
    let mean = sum as f64 / n as f64;

    // Min und Max finden (sicher durch n > 0)
    let min = depth.iter().min().copied().unwrap_or(0);
    let max = depth.iter().max().copied().unwrap_or(0);

    // Median berechnen (sortierte Kopie)
    let mut sorted = depth.to_vec();
    sorted.sort_unstable();

    let median = if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        let mid1 = sorted[n / 2 - 1] as f64;
        let mid2 = sorted[n / 2] as f64;
        (mid1 + mid2) / 2.0
    };

    // Anteile berechnen (fractions)
    let fractions: Vec<f64> = thresholds
        .iter()
        .map(|&t| {
            let count = depth.iter().filter(|&&d| d >= t).count();
            count as f64 / n as f64
        })
        .collect();

    CoverageStats {
        mean,
        median,
        min,
        max,
        fractions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beispiel() {
        let s = coverage_stats(&[0, 0, 5, 10, 10, 20], &[1, 10]);
        assert!((s.mean - 7.5).abs() < 1e-9);
        assert!((s.median - 7.5).abs() < 1e-9);
        assert_eq!((s.min, s.max), (0, 20));
        assert!((s.fractions[0] - 4.0 / 6.0).abs() < 1e-9);
        assert!((s.fractions[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn leer_und_ungerade() {
        let e = coverage_stats(&[], &[5]);
        assert_eq!((e.mean, e.median, e.min, e.max), (0.0, 0.0, 0, 0));
        assert_eq!(e.fractions, vec![0.0]);
        let o = coverage_stats(&[9, 1, 5], &[]);
        assert_eq!(o.median, 5.0);
        assert!(o.fractions.is_empty());
        assert_eq!(coverage_stats(&[3, 3], &[3, 4]).fractions, vec![1.0, 0.0]);
    }
}
