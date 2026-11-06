//! Statistik für Expressionsdaten: Normalisierung und Korrektur für Mehrfachtests.

/// Benjamini-Hochberg-Korrektur (angepasste p-Werte, FDR). Gültige Werte sind die ohne NaN; mit `n` = Zahl der gültigen Werte und
/// Rang `r` (1 = kleinster p-Wert; bei gleichen p-Werten zählt die Reihenfolge in der Eingabe) ist der Rohwert `p * n / r`; der angepasste Wert ist
/// das Minimum der Rohwerte aller Ränge ab `r` bis zum größten Rang, höchstens `1.0`. NaN bleibt NaN. Ergebnis in der Reihenfolge der Eingabe.
/// Beispiel `[0.01, 0.04, 0.03, 0.005]` → `[0.02, 0.04, 0.04, 0.02]`.
pub fn benjamini_hochberg(p: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..p.len()).filter(|&i| !p[i].is_nan()).collect();
    order.sort_by(|&a, &b| p[a].total_cmp(&p[b]).then(a.cmp(&b)));
    let n = order.len();
    let mut out = p.to_vec();
    let mut running = f64::INFINITY;
    for (rank0, &i) in order.iter().enumerate().rev() {
        let raw = p[i] * n as f64 / (rank0 + 1) as f64;
        running = running.min(raw);
        out[i] = running.min(1.0);
    }
    out
}

/// Größenfaktoren nach der Methode „Median der Verhältnisse“. `counts[gene][probe]` sind Zählwerte (alle Gene haben gleich viele
/// Proben, sonst zählt die Zahl der Proben des ersten Gens und kürzere Gene werden übersprungen). Nur Gene, bei denen **alle** Proben
/// größer als 0 sind, zählen. Je Gen ist der Referenzwert das geometrische Mittel (`exp(Mittel der ln-Werte)`); je Probe ist der
/// Größenfaktor der Median (bei gerader Zahl der Mittelwert der beiden mittleren Werte) von `Zählwert / Referenzwert` über diese Gene.
/// Keine Gene oder keine verwendbaren Gene → alle Faktoren `1.0` (Länge = Zahl der Proben, bei keinen Genen leer).
/// Beispiel `[[10,20],[100,200],[0,5]]` → `[0.7071…, 1.4142…]` (das dritte Gen hat eine 0 und zählt nicht).
pub fn median_of_ratios(counts: &[Vec<f64>]) -> Vec<f64> {
    let Some(first) = counts.first() else {
        return Vec::new();
    };
    let probes = first.len();
    let usable: Vec<(&Vec<f64>, f64)> = counts
        .iter()
        .filter(|g| g.len() >= probes && g[..probes].iter().all(|v| *v > 0.0))
        .map(|g| {
            let ln_mean = g[..probes].iter().map(|v| v.ln()).sum::<f64>() / probes as f64;
            (g, ln_mean.exp())
        })
        .collect();
    if usable.is_empty() {
        return vec![1.0; probes];
    }
    (0..probes)
        .map(|j| {
            let mut ratios: Vec<f64> = usable.iter().map(|(g, geo)| g[j] / geo).collect();
            ratios.sort_by(|a, b| a.total_cmp(b));
            let m = ratios.len();
            if m % 2 == 1 {
                ratios[m / 2]
            } else {
                (ratios[m / 2 - 1] + ratios[m / 2]) / 2.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 1e-9, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn bh() {
        close(
            &benjamini_hochberg(&[0.01, 0.04, 0.03, 0.005]),
            &[0.02, 0.04, 0.04, 0.02],
        );
        close(&benjamini_hochberg(&[0.5, 0.9]), &[0.9, 0.9]);
        close(&benjamini_hochberg(&[0.8, 0.9, 1.0]), &[1.0, 1.0, 1.0]);
        let r = benjamini_hochberg(&[0.01, f64::NAN, 0.02]);
        assert!((r[0] - 0.02).abs() < 1e-9 && r[1].is_nan() && (r[2] - 0.02).abs() < 1e-9);
        assert!(benjamini_hochberg(&[]).is_empty());
    }

    #[test]
    fn faktoren() {
        let f = median_of_ratios(&[vec![10.0, 20.0], vec![100.0, 200.0], vec![0.0, 5.0]]);
        close(&f, &[0.5_f64.sqrt(), 2.0_f64.sqrt()]);
        close(&median_of_ratios(&[vec![0.0, 1.0]]), &[1.0, 1.0]);
        assert!(median_of_ratios(&[]).is_empty());
        close(&median_of_ratios(&[vec![4.0, 4.0, 4.0]]), &[1.0, 1.0, 1.0]);
    }
}
