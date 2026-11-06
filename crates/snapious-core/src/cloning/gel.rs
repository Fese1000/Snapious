//! Agarosegel-Simulation: Laufstrecke aus der Fragmentgröße und Banden einer Spur.

/// Eine Bande. `mobility` 0.0 = ganz oben (große Fragmente), 1.0 = ganz unten.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    pub size_bp: usize,
    pub mobility: f64,
    /// Relative Helligkeit, die hellste Bande einer Spur hat 1.0.
    pub intensity: f64,
}

/// Eine Spur auf dem Gel.
#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub label: String,
    /// Von oben nach unten sortiert (größte Fragmente zuerst).
    pub bands: Vec<Band>,
}

/// Größenstandard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ladder {
    pub name: String,
    pub sizes: Vec<usize>,
}

impl Ladder {
    /// `Generic 1 kb`: 250, 500, 750, 1000, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 8000, 10000.
    pub fn generic_1kb() -> Ladder {
        Ladder {
            name: "Generic 1 kb".to_string(),
            sizes: vec![
                250, 500, 750, 1000, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 8000, 10000,
            ],
        }
    }

    /// `Generic 100 bp`: 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1200, 1500.
    pub fn generic_100bp() -> Ladder {
        Ladder {
            name: "Generic 100 bp".to_string(),
            sizes: vec![
                100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1200, 1500,
            ],
        }
    }
}

/// Simulation für eine Agarosekonzentration.
#[derive(Clone, Debug, PartialEq)]
pub struct GelSimulator {
    agarose_percent: f64,
}

/// Stützstellen `(Prozent, kleinste auflösbare Größe, größte auflösbare Größe)`.
const TABLE: [(f64, f64, f64); 4] = [
    (0.7, 500.0, 15000.0),
    (1.0, 300.0, 10000.0),
    (1.5, 150.0, 5000.0),
    (2.0, 100.0, 3000.0),
];

impl GelSimulator {
    /// Merkt die Konzentration; Werte außerhalb von 0,5 bis 3,0 werden auf diesen Bereich begrenzt.
    pub fn new(agarose_percent: f64) -> GelSimulator {
        let clamped = agarose_percent.clamp(0.5, 3.0);
        GelSimulator {
            agarose_percent: clamped,
        }
    }

    pub fn agarose_percent(&self) -> f64 {
        self.agarose_percent
    }

    /// Auflösungsbereich `(smin, smax)` in bp. Zwischen den Stützstellen aus `TABLE` wird `log10` der Größe linear
    /// in der Prozentzahl interpoliert; unterhalb der ersten und oberhalb der letzten Stützstelle gelten deren Werte.
    pub fn range(&self) -> (f64, f64) {
        let p = self.agarose_percent;
        let mut smin_log = 0.0f64;
        let mut smax_log = 0.0f64;

        for i in 0..TABLE.len() {
            if p <= TABLE[i].0 {
                smin_log = TABLE[i].1.log10();
                smax_log = TABLE[i].2.log10();
                break;
            } else if i + 1 < TABLE.len() {
                let p0 = TABLE[i].0;
                let p1 = TABLE[i + 1].0;
                if p <= p1 {
                    let t = (p - p0) / (p1 - p0);
                    smin_log =
                        TABLE[i].1.log10() + t * (TABLE[i + 1].1.log10() - TABLE[i].1.log10());
                    smax_log =
                        TABLE[i].2.log10() + t * (TABLE[i + 1].2.log10() - TABLE[i].2.log10());
                    break;
                }
            } else {
                smin_log = TABLE[i].1.log10();
                smax_log = TABLE[i].2.log10();
            }
        }
        (10f64.powf(smin_log), 10f64.powf(smax_log))
    }

    /// `clamp((log10(smax) − log10(size)) / (log10(smax) − log10(smin)), 0, 1)`; Größe 0 zählt wie 1.
    pub fn mobility(&self, size_bp: usize) -> f64 {
        let (smin, smax) = self.range();
        let size = if size_bp == 0 { 1 } else { size_bp };
        let log_size = (size as f64).log10();
        let log_smin = smin.log10();
        let log_smax = smax.log10();

        if log_size <= log_smin {
            return 1.0;
        }
        if log_size >= log_smax {
            return 0.0;
        }

        let num = log_smax - log_size;
        let den = log_smax - log_smin;
        (num / den).clamp(0.0, 1.0)
    }

    /// Spur für Fragmente. Gleiche Größen werden zu einer Bande zusammengefasst. Rohintensität einer Größe =
    /// `mass_ng · size · anzahl / summe_aller_größen`; danach durch die größte Rohintensität geteilt (hellste Bande = 1.0).
    /// Größen 0 werden ignoriert. Keine Fragmente oder `mass_ng <= 0`: alle Intensitäten 0.0 (bei keinen Fragmenten keine Banden).
    /// Banden nach Größe absteigend.
    pub fn lane(&self, label: &str, fragment_sizes: &[usize], mass_ng: f64) -> Lane {
        let mut counts = std::collections::BTreeMap::<usize, usize>::new();
        for &s in fragment_sizes {
            if s > 0 {
                *counts.entry(s).or_insert(0) += 1;
            }
        }

        if counts.is_empty() || mass_ng <= 0.0 {
            let mut bands: Vec<Band> = counts
                .keys()
                .map(|&s| Band {
                    size_bp: s,
                    mobility: self.mobility(s),
                    intensity: 0.0,
                })
                .collect();

            bands.sort_by_key(|a| std::cmp::Reverse(a.size_bp));
            return Lane {
                label: label.to_string(),
                bands,
            };
        }

        let total_size_sum: f64 = counts.iter().map(|(&s, &c)| (s as f64) * (c as f64)).sum();
        let raw_intensities: Vec<(usize, f64)> = counts
            .iter()
            .map(|(&s, &c)| {
                let raw = (mass_ng * (s as f64) * (c as f64)) / total_size_sum;
                (s, raw)
            })
            .collect();

        let max_raw = raw_intensities
            .iter()
            .map(|&(_, r)| r)
            .fold(0.0f64, f64::max);
        let mut bands: Vec<Band> = raw_intensities
            .into_iter()
            .map(|(s, r)| Band {
                size_bp: s,
                mobility: self.mobility(s),
                intensity: if max_raw > 0.0 { r / max_raw } else { 0.0 },
            })
            .collect();

        bands.sort_by_key(|a| std::cmp::Reverse(a.size_bp));
        Lane {
            label: label.to_string(),
            bands,
        }
    }

    /// Spur für einen Standard: `label` ist der Name, jede Größe eine Bande mit Intensität 1.0, Größen absteigend.
    pub fn ladder_lane(&self, ladder: &Ladder) -> Lane {
        let mut bands = ladder
            .sizes
            .iter()
            .map(|&s| Band {
                size_bp: s,
                mobility: self.mobility(s),
                intensity: 1.0,
            })
            .collect::<Vec<Band>>();

        bands.sort_by_key(|a| std::cmp::Reverse(a.size_bp));
        Lane {
            label: ladder.name.clone(),
            bands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn standards() {
        let l = Ladder::generic_1kb();
        assert_eq!(l.sizes.len(), 13);
        assert_eq!((l.sizes[0], l.sizes[12]), (250, 10000));
        assert_eq!(l.name, "Generic 1 kb");
        let k = Ladder::generic_100bp();
        assert_eq!(k.sizes.len(), 12);
        assert_eq!(k.sizes[11], 1500);
        assert_eq!(k.name, "Generic 100 bp");
    }

    #[test]
    fn bereich() {
        let (a, b) = GelSimulator::new(1.0).range();
        assert!(close(a, 300.0, 1e-6) && close(b, 10000.0, 1e-6));
        let (a, b) = GelSimulator::new(1.25).range();
        assert!(close(a, (300.0f64 * 150.0).sqrt(), 1e-6));
        assert!(close(b, (10000.0f64 * 5000.0).sqrt(), 1e-6));
        let (a, b) = GelSimulator::new(0.5).range();
        assert!(close(a, 500.0, 1e-6) && close(b, 15000.0, 1e-6));
        let (a, b) = GelSimulator::new(9.0).range();
        assert!(close(a, 100.0, 1e-6) && close(b, 3000.0, 1e-6));
        assert!(close(GelSimulator::new(9.0).agarose_percent(), 3.0, 1e-12));
        assert!(close(GelSimulator::new(0.1).agarose_percent(), 0.5, 1e-12));
    }

    #[test]
    fn laufstrecke() {
        let g = GelSimulator::new(1.0);
        assert!(close(g.mobility(10000), 0.0, 1e-9));
        assert!(close(g.mobility(300), 1.0, 1e-9));
        assert!(close(g.mobility(1732), 0.5, 0.01));
        assert!(close(g.mobility(50), 1.0, 1e-9));
        assert!(close(g.mobility(50000), 0.0, 1e-9));
        assert!(g.mobility(500) > g.mobility(1000) && g.mobility(1000) > g.mobility(5000));
        assert!(close(g.mobility(0), 1.0, 1e-9));
    }

    #[test]
    fn dichteres_gel_bremst() {
        let m10 = GelSimulator::new(1.0).mobility(1000);
        let m12 = GelSimulator::new(1.25).mobility(1000);
        let m15 = GelSimulator::new(1.5).mobility(1000);
        // im dichteren Gel liegt dieselbe Bande weiter oben (kleinere Laufstrecke)
        assert!(m10 > m12 && m12 > m15);
    }

    #[test]
    fn spuren() {
        let g = GelSimulator::new(1.0);
        let l = g.lane("x", &[1000, 1000], 10.0);
        assert_eq!(l.label, "x");
        assert_eq!(l.bands.len(), 1);
        assert!(close(l.bands[0].intensity, 1.0, 1e-9));
        assert_eq!(l.bands[0].size_bp, 1000);
        assert!(close(l.bands[0].mobility, g.mobility(1000), 1e-12));

        let m = g.lane("m", &[1000, 1000, 3000], 100.0);
        assert_eq!(m.bands.len(), 2);
        assert_eq!(m.bands[0].size_bp, 3000);
        assert!(close(m.bands[0].intensity, 1.0, 1e-9));
        assert_eq!(m.bands[1].size_bp, 1000);
        assert!(close(m.bands[1].intensity, 2.0 / 3.0, 1e-9));

        let n = g.lane("n", &[1000, 2000], 5.0);
        assert!(close(n.bands[1].intensity, 0.5, 1e-9));
    }

    #[test]
    fn spuren_sonderfaelle() {
        let g = GelSimulator::new(1.0);
        assert!(g.lane("leer", &[], 10.0).bands.is_empty());
        let null = g.lane("null", &[0, 500], 10.0);
        assert_eq!(null.bands.len(), 1);
        assert_eq!(null.bands[0].size_bp, 500);
        let ohne = g.lane("ohne", &[500, 700], 0.0);
        assert!(ohne.bands.iter().all(|b| b.intensity == 0.0));
        assert_eq!(ohne.bands.len(), 2);
    }

    #[test]
    fn leiterspur() {
        let g = GelSimulator::new(1.0);
        let l = g.ladder_lane(&Ladder::generic_100bp());
        assert_eq!(l.label, "Generic 100 bp");
        assert_eq!(l.bands.len(), 12);
        assert_eq!(l.bands[0].size_bp, 1500);
        assert!(l.bands.iter().all(|b| b.intensity == 1.0));
        assert!(l.bands.windows(2).all(|w| w[0].mobility <= w[1].mobility));
    }
}
