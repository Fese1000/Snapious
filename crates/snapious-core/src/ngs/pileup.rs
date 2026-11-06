//! Einfache Variantenerkennung (SNVs) aus Alignments: Basen je Referenzposition zählen und Abweichungen melden.

use crate::error::{Error, Result};
use crate::io::sam::SamRecord;
use crate::io::vcf::VariantCall;

/// Einstellungen. Standard: Tiefe 10, Anteil 0,2, Basenqualität 20, MAPQ 20.
#[derive(Clone, Debug, PartialEq)]
pub struct CallOptions {
    pub min_depth: usize,
    pub min_alt_fraction: f64,
    pub min_base_quality: i32,
    pub min_mapq: u8,
}

impl CallOptions {
    pub fn standard() -> CallOptions {
        CallOptions {
            min_depth: 10,
            min_alt_fraction: 0.2,
            min_base_quality: 20,
            min_mapq: 20,
        }
    }
}

/// Eine erkannte Einzelbasenvariante.
#[derive(Clone, Debug, PartialEq)]
pub struct Snv {
    /// 0-basiert.
    pub position: usize,
    pub ref_base: char,
    pub alt_base: char,
    pub depth: usize,
    pub alt_count: usize,
    pub alt_fraction: f64,
    /// Phred-artige Güte `−10·log10(P)` mit `P` aus `binomial_tail`.
    pub quality: f64,
}

/// `P(X >= k)` für `X ~ Binomial(n, p)`; Ergebnis als `-10 * log10(P)`. Exakt als Summe der Einzelwahrscheinlichkeiten
/// (Logarithmen der Fakultäten als Summe von `ln(i)`; `P` nicht kleiner als `1e-300`). Ist `n > 1000`, werden stattdessen
/// `n = 1000` und `k = round(k * 1000 / n_alt)` verwendet (`n_alt` = ursprüngliches `n`). Ist `k == 0`, ist `P = 1` und das Ergebnis 0.
/// `k > n` ergibt 3000.0 (`P = 1e-300`).
pub fn binomial_phred(n: usize, k: usize, p: f64) -> f64 {
    if k == 0 {
        return 0.0;
    }
    if k > n {
        return 3000.0;
    }
    let (n, k) = if n > 1000 {
        (1000usize, ((k as f64) * 1000.0 / n as f64).round() as usize)
    } else {
        (n, k)
    };
    if k == 0 {
        return 0.0;
    }
    let mut ln_fact = vec![0.0f64; n + 1];
    for i in 1..=n {
        ln_fact[i] = ln_fact[i - 1] + (i as f64).ln();
    }
    let (lp, lq) = (p.ln(), (1.0 - p).ln());
    let mut sum = 0.0f64;
    for i in k..=n {
        let ln_term =
            ln_fact[n] - ln_fact[i] - ln_fact[n - i] + i as f64 * lp + (n - i) as f64 * lq;
        sum += ln_term.exp();
    }
    -10.0 * sum.clamp(1e-300, 1.0).log10()
}

/// Zählt Basen je Position und meldet SNVs für die Referenz `reference_name` mit den Basen `reference` (Großbuchstaben).
/// * Berücksichtigt werden Datensätze mit `reference == reference_name`, ausgerichtet (nicht `is_unmapped`), nicht `is_secondary`, nicht
///   `is_duplicate`, mit `mapq >= min_mapq`, `position >= 1` und lesbarem CIGAR (`cigar_ops`; sonst `Error::InvalidInput` weitergeben).
/// * CIGAR-Lauf: `M`, `=`, `X` verbrauchen Read und Referenz (Basen werden gezählt), `I` und `S` nur den Read, `D` und `N` nur die Referenz,
///   `H` und `P` nichts. Referenzposition beginnt bei `position − 1`. Positionen hinter dem Ende von `reference` werden ignoriert.
/// * Eine Base zählt nur mit Qualitätszeichen: Phred = Zeichen − 33 `>= min_base_quality`; fehlt die Qualität (`*`), gilt Phred 0.
///   Basen werden in Großbuchstaben gezählt; nur A, C, G, T zählen (N usw. nicht).
/// * An jeder Position: `depth` = Zahl gezählter Basen. Alt-Base = die häufigste Base, die nicht der Referenzbase entspricht
///   (bei Gleichstand die in der Reihenfolge A, C, G, T erste). Variante, wenn `depth >= min_depth` und `alt_count / depth >= min_alt_fraction`
///   (und `alt_count > 0`). Referenzbasen außer A, C, G, T werden übersprungen. `quality = binomial_phred(depth, alt_count, 0.01)`.
/// * Ergebnis aufsteigend nach `position`.
pub fn call_snvs(
    reference_name: &str,
    reference: &str,
    records: &[SamRecord],
    o: &CallOptions,
) -> Result<Vec<Snv>> {
    const BASES: [char; 4] = ['A', 'C', 'G', 'T'];
    let ref_chars: Vec<char> = reference.to_ascii_uppercase().chars().collect();
    let mut counts = vec![[0usize; 4]; ref_chars.len()];
    for r in records {
        if r.reference != reference_name
            || r.is_unmapped()
            || r.is_secondary()
            || r.is_duplicate()
            || r.mapq < o.min_mapq
            || r.position < 1
        {
            continue;
        }
        let ops = r
            .cigar_ops()
            .map_err(|e| Error::InvalidInput(e.to_string()))?;
        let seq: Vec<char> = r.sequence.to_ascii_uppercase().chars().collect();
        let qual: Vec<u8> = if r.quality == "*" {
            Vec::new()
        } else {
            r.quality.bytes().collect()
        };
        let mut ref_pos = r.position - 1;
        let mut read_pos = 0usize;
        for (len, op) in ops {
            match op {
                'M' | '=' | 'X' => {
                    for _ in 0..len {
                        if ref_pos < counts.len() {
                            let phred = qual.get(read_pos).map_or(0, |q| *q as i32 - 33);
                            if phred >= o.min_base_quality {
                                if let Some(base) = seq.get(read_pos) {
                                    if let Some(i) = BASES.iter().position(|b| b == base) {
                                        counts[ref_pos][i] += 1;
                                    }
                                }
                            }
                        }
                        ref_pos += 1;
                        read_pos += 1;
                    }
                }
                'I' | 'S' => read_pos += len,
                'D' | 'N' => ref_pos += len,
                'H' | 'P' => {}
                other => {
                    return Err(Error::InvalidInput(format!(
                        "Unbekannte CIGAR-Operation '{}' in {}",
                        other, r.read_name
                    )))
                }
            }
        }
    }
    let mut out = Vec::new();
    for (pos, c) in counts.iter().enumerate() {
        let Some(ri) = BASES.iter().position(|b| *b == ref_chars[pos]) else {
            continue;
        };
        let depth: usize = c.iter().sum();
        if depth < o.min_depth || depth == 0 {
            continue;
        }
        let mut best: Option<usize> = None;
        for i in 0..4 {
            if i != ri && c[i] > 0 && best.is_none_or(|b| c[i] > c[b]) {
                best = Some(i);
            }
        }
        let Some(alt) = best else { continue };
        let alt_count = c[alt];
        let fraction = alt_count as f64 / depth as f64;
        if fraction < o.min_alt_fraction {
            continue;
        }
        out.push(Snv {
            position: pos,
            ref_base: BASES[ri],
            alt_base: BASES[alt],
            depth,
            alt_count,
            alt_fraction: fraction,
            quality: binomial_phred(depth, alt_count, 0.01),
        });
    }
    Ok(out)
}

/// Wandelt in VCF-Zeilen um: `chromosome = reference_name`, `position + 1`, `id = "."`, `ref_allele`/`alt_allele` je eine Base, `quality = Some(q)`,
/// `filter = "PASS"`, `info = "DP={depth};AF={alt_fraction:.3}"`.
pub fn to_vcf(reference_name: &str, snvs: &[Snv]) -> Vec<VariantCall> {
    snvs.iter()
        .map(|s| VariantCall {
            chromosome: reference_name.to_string(),
            position: s.position + 1,
            id: ".".to_string(),
            ref_allele: s.ref_base.to_string(),
            alt_allele: s.alt_base.to_string(),
            quality: Some(s.quality),
            filter: "PASS".to_string(),
            info: format!("DP={};AF={:.3}", s.depth, s.alt_fraction),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(name: &str, pos: usize, cigar: &str, seq: &str, qual: &str) -> SamRecord {
        SamRecord {
            read_name: name.to_string(),
            flag: 0,
            reference: "ref".to_string(),
            position: pos,
            mapq: 60,
            cigar: cigar.to_string(),
            mate_reference: "*".to_string(),
            mate_position: 0,
            template_length: 0,
            sequence: seq.to_string(),
            quality: qual.to_string(),
        }
    }

    const REF: &str = "AAAAAAAAAA";

    fn reads_with_g(n_alt: usize, n_total: usize, alt_qual: char) -> Vec<SamRecord> {
        (0..n_total)
            .map(|i| {
                if i < n_alt {
                    let q: String = (0..10)
                        .map(|k| if k == 4 { alt_qual } else { 'I' })
                        .collect();
                    rec("r", 1, "10M", "AAAAGAAAAA", &q)
                } else {
                    rec("r", 1, "10M", "AAAAAAAAAA", "IIIIIIIIII")
                }
            })
            .collect()
    }

    #[test]
    fn eine_variante() {
        let v = call_snvs(
            "ref",
            REF,
            &reads_with_g(10, 20, 'I'),
            &CallOptions::standard(),
        )
        .unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].position, 4);
        assert_eq!((v[0].ref_base, v[0].alt_base), ('A', 'G'));
        assert_eq!(v[0].depth, 20);
        assert_eq!(v[0].alt_count, 10);
        assert_eq!(v[0].alt_fraction, 0.5);
        assert!(
            (v[0].quality - 147.73049566939235).abs() < 1e-6,
            "{}",
            v[0].quality
        );
    }

    #[test]
    fn schwellen() {
        let o = CallOptions::standard();
        // 2 von 20 = 0,1 < 0,2
        assert!(call_snvs("ref", REF, &reads_with_g(2, 20, 'I'), &o)
            .unwrap()
            .is_empty());
        // Basenqualität 10 ('+') zählt nicht: die 10 G-Basen fallen weg, Tiefe 10
        assert!(call_snvs("ref", REF, &reads_with_g(10, 20, '+'), &o)
            .unwrap()
            .is_empty());
        // Tiefe 5 < 10
        assert!(call_snvs("ref", REF, &reads_with_g(3, 5, 'I'), &o)
            .unwrap()
            .is_empty());
        let low = CallOptions {
            min_depth: 5,
            ..o.clone()
        };
        assert_eq!(
            call_snvs("ref", REF, &reads_with_g(3, 5, 'I'), &low)
                .unwrap()
                .len(),
            1
        );
        // MAPQ
        let mut reads = reads_with_g(10, 20, 'I');
        for r in reads.iter_mut().take(10) {
            r.mapq = 10;
        }
        assert!(call_snvs("ref", REF, &reads, &o).unwrap().is_empty());
        // andere Referenz, Duplikate, unmapped
        let mut other = reads_with_g(10, 20, 'I');
        for r in other.iter_mut() {
            r.reference = "x".to_string();
        }
        assert!(call_snvs("ref", REF, &other, &o).unwrap().is_empty());
        let mut dup = reads_with_g(10, 20, 'I');
        for r in dup.iter_mut() {
            r.flag = 0x400;
        }
        assert!(call_snvs("ref", REF, &dup, &o).unwrap().is_empty());
    }

    #[test]
    fn cigar_lauf() {
        // 2S3M1D2M ab Position 3 (1-basiert): M bei Ref 2,3,4; D bei 5; M bei 6,7
        let reference = "ACGTACGTAC";
        let mut reads = Vec::new();
        for _ in 0..10 {
            // Soft-Clip "TT", dann M: Read-Basen G,T,T gegen Ref[2..5]="GTA" -> T statt A an Ref 4; dann 2M gegen Ref[6..8]="GT" -> C,C
            reads.push(rec("r", 3, "2S3M1D2M", "TTGTTCC", "IIIIIII"));
        }
        let o = CallOptions {
            min_depth: 5,
            ..CallOptions::standard()
        };
        let v = call_snvs("ref", reference, &reads, &o).unwrap();
        let pos: Vec<usize> = v.iter().map(|s| s.position).collect();
        assert_eq!(pos, vec![4, 6, 7]);
        assert_eq!((v[0].ref_base, v[0].alt_base), ('A', 'T'));
        assert_eq!((v[1].ref_base, v[1].alt_base), ('G', 'C'));
        assert_eq!((v[2].ref_base, v[2].alt_base), ('T', 'C'));
    }

    #[test]
    fn fehler_im_cigar() {
        let r = rec("r", 1, "4Q", "ACGT", "IIII");
        assert!(matches!(
            call_snvs("ref", REF, &[r], &CallOptions::standard()),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn wahrscheinlichkeit() {
        assert!((binomial_phred(20, 10, 0.01) - 147.73049566939235).abs() < 1e-6);
        assert!((binomial_phred(20, 2, 0.01) - 17.731594918009645).abs() < 1e-6);
        assert!((binomial_phred(10, 3, 0.01) - 39.43670329916089).abs() < 1e-6);
        assert!((binomial_phred(1000, 50, 0.01) - 190.80588652478133).abs() < 1e-6);
        // n > 1000: auf 1000 und k = 50 umgerechnet
        assert!((binomial_phred(2000, 100, 0.01) - 190.80588652478133).abs() < 1e-6);
        assert_eq!(binomial_phred(10, 0, 0.01), 0.0);
        assert_eq!(binomial_phred(10, 11, 0.01), 3000.0);
    }

    #[test]
    fn vcf_zeilen() {
        let v = call_snvs(
            "ref",
            REF,
            &reads_with_g(10, 20, 'I'),
            &CallOptions::standard(),
        )
        .unwrap();
        let rows = to_vcf("ref", &v);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].chromosome, "ref");
        assert_eq!(rows[0].position, 5);
        assert_eq!(rows[0].ref_allele, "A");
        assert_eq!(rows[0].alt_allele, "G");
        assert_eq!(rows[0].filter, "PASS");
        assert_eq!(rows[0].info, "DP=20;AF=0.500");
        assert!(rows[0].quality.is_some());
    }
}
