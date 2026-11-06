//! Expressionswerte aus Read-Zählungen: RPKM/FPKM, TPM und log2-Verhältnis.

/// RPKM (gleich FPKM bei einzelnen Reads): `count * 1e9 / (length_bp * total_reads)`. Ist `length_bp <= 0.0` oder
/// `total_reads <= 0.0`, ist das Ergebnis `0.0`.
pub fn rpkm(count: f64, length_bp: f64, total_reads: f64) -> f64 {
    if length_bp <= 0.0 || total_reads <= 0.0 {
        return 0.0;
    }
    count * 1e9 / (length_bp * total_reads)
}

/// TPM für alle Gene: `rate_i = count_i / length_i`, `TPM_i = rate_i / Σ rate * 1e6`. Gene mit `length <= 0.0` haben die Rate 0.
/// Ist die Summe der Raten 0, sind alle Werte `0.0`. `counts` und `lengths` sind gleich lang (sonst werden nur die ersten
/// `min(len)` Paare berücksichtigt und der Rest der Ausgabe fehlt: Länge der Ausgabe = `min(len)`).
pub fn tpm(counts: &[f64], lengths: &[f64]) -> Vec<f64> {
    let len = counts.len().min(lengths.len());
    let mut rates: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        if lengths[i] <= 0.0 || counts[i] <= 0.0 {
            rates.push(0.0);
        } else {
            rates.push(counts[i] / lengths[i]);
        }
    }

    let sum_rates: f64 = rates.iter().sum();

    if sum_rates == 0.0 {
        return vec![0.0; len];
    }

    rates.iter().map(|&r| r / sum_rates * 1e6).collect()
}

/// `log2((a + pseudo) / (b + pseudo))`. Ist `a + pseudo <= 0.0` oder `b + pseudo <= 0.0`, ist das Ergebnis `f64::NAN`.
pub fn log2_ratio(a: f64, b: f64, pseudo: f64) -> f64 {
    let a_adj = a + pseudo;
    let b_adj = b + pseudo;

    if a_adj <= 0.0 || b_adj <= 0.0 {
        return f64::NAN;
    }

    (a_adj / b_adj).log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} vs {b}");
    }

    #[test]
    fn rpkm_werte() {
        close(rpkm(10.0, 1000.0, 30.0), 10.0 * 1e9 / (1000.0 * 30.0));
        close(rpkm(20.0, 2000.0, 30.0), 333333.333333);
        assert_eq!(rpkm(5.0, 0.0, 10.0), 0.0);
        assert_eq!(rpkm(5.0, 10.0, 0.0), 0.0);
    }

    #[test]
    fn tpm_und_log() {
        let t = tpm(&[10.0, 20.0], &[1000.0, 2000.0]);
        close(t[0], 500000.0);
        close(t[1], 500000.0);
        let u = tpm(&[30.0, 10.0], &[1000.0, 1000.0]);
        close(u[0], 750000.0);
        close(u[1], 250000.0);
        assert_eq!(tpm(&[0.0, 0.0], &[100.0, 100.0]), vec![0.0, 0.0]);
        assert_eq!(tpm(&[5.0], &[0.0]), vec![0.0]);
        assert_eq!(tpm(&[1.0, 2.0, 3.0], &[10.0, 10.0]).len(), 2);
        close(log2_ratio(8.0, 2.0, 0.0), 2.0);
        close(log2_ratio(7.0, 1.0, 1.0), 2.0);
        assert!(log2_ratio(0.0, 1.0, 0.0).is_nan());
    }
}
