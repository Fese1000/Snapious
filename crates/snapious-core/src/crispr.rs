//! CRISPR gRNA search: find protospacers next to a PAM, score them simply and count off-target hits.

use crate::cloning::enzymes::iupac_reverse_complement;

/// Checks whether `seq` (concrete bases) matches the IUPAC pattern `pattern`.
fn matches_iupac(pattern: &str, seq: &str) -> bool {
    pattern.chars().zip(seq.chars()).all(|(p, s)| {
        let p = p.to_ascii_uppercase();
        let s = s.to_ascii_uppercase();
        match p {
            'N' => true,
            'A' => s == 'A',
            'C' => s == 'C',
            'G' => s == 'G',
            'T' => s == 'T',
            'R' => s == 'A' || s == 'G',
            'Y' => s == 'C' || s == 'T',
            'S' => s == 'G' || s == 'C',
            'W' => s == 'A' || s == 'T',
            'K' => s == 'G' || s == 'T',
            'M' => s == 'A' || s == 'C',
            'B' => s == 'C' || s == 'G' || s == 'T',
            'D' => s == 'A' || s == 'G' || s == 'T',
            'H' => s == 'A' || s == 'C' || s == 'T',
            'V' => s == 'A' || s == 'C' || s == 'G',
            _ => false,
        }
    })
}

/// Description of a Cas nuclease.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PamSpec {
    pub name: String,
    /// PAM pattern with IUPAC codes (`N` all, `R` A/G, `Y` C/T, `S` C/G, `W` A/T, `K` G/T, `M` A/C,
    /// `B` C/G/T, `D` A/G/T, `H` A/C/T, `V` A/C/G).
    pub pattern: String,
    /// Length of the protospacer in bases.
    pub guide_length: usize,
    /// `true`: the PAM lies directly after (3') the protospacer, otherwise directly before (5').
    pub pam_three_prime: bool,
}

impl PamSpec {
    /// SpCas9: `NGG`, 20 bases, PAM at the 3' end.
    pub fn sp_cas9() -> PamSpec {
        PamSpec {
            name: "SpCas9".into(),
            pattern: "NGG".into(),
            guide_length: 20,
            pam_three_prime: true,
        }
    }

    /// SaCas9: `NNGRRT`, 21 bases, PAM at the 3' end.
    pub fn sa_cas9() -> PamSpec {
        PamSpec {
            name: "SaCas9".into(),
            pattern: "NNGRRT".into(),
            guide_length: 21,
            pam_three_prime: true,
        }
    }

    /// Cas12a: `TTTV`, 23 bases, PAM at the 5' end.
    pub fn cas12a() -> PamSpec {
        PamSpec {
            name: "Cas12a".into(),
            pattern: "TTTV".into(),
            guide_length: 23,
            pam_three_prime: false,
        }
    }
}

/// A guide site that was found.
#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    /// Protospacer as read on its own strand (so reverse-complemented when `forward == false`).
    pub protospacer: String,
    /// PAM sequence read on the same strand.
    pub pam: String,
    /// Start and end (exclusive) of protospacer plus PAM in top-strand coordinates.
    /// If the site of a circular sequence wraps around the origin, `end < start`.
    pub start: usize,
    pub end: usize,
    pub forward: bool,
    /// GC fraction of the protospacer in percent (0 to 100).
    pub gc_percent: f64,
    /// Score 0 to 100, see `score_protospacer`.
    pub score: f64,
    pub warnings: Vec<String>,
}

/// Scores a protospacer (deliberately simple, not a prediction model). Starts at 100 points:
/// - GC fraction below 40 % or above 70 %: −20, warning `GC fraction outside 40–70 %`
/// - contains `TTTT`: −30, warning `TTTT in the protospacer (termination signal of U6 polymerase)`
/// - five or more identical letters in a row (independent of the TTTT rule): −10,
///   warning `Homopolymer run of at least 5 bases`
///
/// Returns: (GC fraction in percent, points, warnings in the order of the rules above).
/// Case-insensitive. Empty text: GC 0, 100 points minus the GC rule.
pub fn score_protospacer(protospacer: &str) -> (f64, f64, Vec<String>) {
    let s = protospacer.to_ascii_uppercase();
    let len = s.len() as f64;
    let gc_count = s.chars().filter(|c| *c == 'G' || *c == 'C').count() as f64;
    let gc_percent = if len > 0.0 {
        (gc_count / len) * 100.0
    } else {
        0.0
    };

    let mut score = 100.0;
    let mut warnings: Vec<String> = Vec::new();

    if len > 0.0 && (gc_percent < 40.0 || gc_percent > 70.0) {
        score -= 20.0;
        warnings.push("GC fraction outside 40–70 %".to_string());
    }

    if s.contains("TTTT") {
        score -= 30.0;
        warnings.push("TTTT in the protospacer (termination signal of U6 polymerase)".to_string());
    }

    let mut max_run: usize = 0;
    let mut current_run: usize = 1;
    if len > 0.0 {
        let chars: Vec<char> = s.chars().collect();
        for i in 1..chars.len() {
            if chars[i] == chars[i - 1] {
                current_run += 1;
            } else {
                if current_run > max_run {
                    max_run = current_run;
                }
                current_run = 1;
            }
        }
        if current_run > max_run {
            max_run = current_run;
        }
    }

    if max_run >= 5 {
        score -= 10.0;
        warnings.push("Homopolymer run of at least 5 bases".to_string());
    }

    (gc_percent, score, warnings)
}

/// Finds guides on both strands.
/// - `bases` is read in uppercase with `T` instead of `U`. `to` is capped at the length.
/// - Forward strand: for each start position `st` with room for protospacer plus PAM (`guide_length + pattern.len()`)
///   the PAM is checked against the pattern (IUPAC). With `pam_three_prime` the protospacer precedes the PAM, otherwise it follows it.
///   `start = st`, `end = st + guide_length + pattern.len()`.
/// - Reverse strand: the same on the reverse complement (`iupac_reverse_complement`); coordinates are converted back:
///   `start = len − end'`, `end = len − start'`. `forward = false`.
/// - Non-circular: only sites with `from <= start` and `end <= to` count.
/// - Circular: `from`/`to` are ignored, the whole sequence is searched across the origin
///   (start positions `0..len`, sequence extended at the end by `guide_length + pattern.len() − 1` bases from the start).
///   On the forward strand `end = start + total length`; if `end > len`, then `end −= len`. On the reverse strand:
///   `start = (len − end') mod len` (with `rem_euclid`, `end'` = end on the reverse complement before trimming), then
///   `end = start + total length`, and likewise `end −= len` if `end > len`.
/// - Scoring with `score_protospacer`.
/// - Sorting: `start`, then `end`, then forward before reverse strand.
pub fn find_guides(
    spec: &PamSpec,
    bases: &str,
    from: usize,
    to: usize,
    circular: bool,
) -> Vec<Guide> {
    let bases = bases.to_ascii_uppercase().replace('U', "T");
    let g_len = spec.guide_length;
    let p_len = spec.pattern.len();
    let total = g_len + p_len;

    if bases.is_empty() || total > bases.len() {
        return Vec::new();
    }

    let len = bases.len();
    let mut guides: Vec<Guide> = Vec::new();

    // forward strand
    {
        let limit = if circular { len } else { len - total };
        let ext = if circular {
            format!("{}{}", bases, &bases[..total.saturating_sub(1)])
        } else {
            bases.to_string()
        };

        for st in 0..=limit {
            let pam_slice = if spec.pam_three_prime {
                ext.get(st + g_len..st + total)
            } else {
                ext.get(st..st + p_len)
            };
            if let Some(pam_slice) = pam_slice {
                if matches_iupac(&spec.pattern, pam_slice) {
                    let proto_slice = if spec.pam_three_prime {
                        ext.get(st..st + g_len)
                    } else {
                        ext.get(st + p_len..st + total)
                    };
                    if let Some(proto_slice) = proto_slice {
                        let (gc, score, warnings) = score_protospacer(proto_slice);
                        let mut end = st + total;
                        if circular && end > len {
                            end -= len;
                        }

                        guides.push(Guide {
                            protospacer: proto_slice.to_string(),
                            pam: pam_slice.to_string(),
                            start: st,
                            end,
                            forward: true,
                            gc_percent: gc,
                            score,
                            warnings,
                        });
                    }
                }
            }
        }
    }

    // reverse strand
    {
        let rc = iupac_reverse_complement(&bases);
        let rc_len = rc.len();
        let limit = if circular { rc_len } else { rc_len - total };
        let ext = if circular {
            format!("{}{}", rc, &rc[..total.saturating_sub(1)])
        } else {
            rc.to_string()
        };

        for st in 0..=limit {
            let pam_slice = if spec.pam_three_prime {
                ext.get(st + g_len..st + total)
            } else {
                ext.get(st..st + p_len)
            };
            if let Some(pam_slice) = pam_slice {
                if matches_iupac(&spec.pattern, pam_slice) {
                    let proto_slice = if spec.pam_three_prime {
                        ext.get(st..st + g_len)
                    } else {
                        ext.get(st + p_len..st + total)
                    };
                    if let Some(proto_slice) = proto_slice {
                        let (gc, score, warnings) = score_protospacer(proto_slice);
                        let end_prime = st + total;
                        let start = if circular {
                            (len as i64 - end_prime as i64).rem_euclid(len as i64) as usize
                        } else {
                            len.saturating_sub(end_prime)
                        };
                        let mut end = if circular {
                            start + total
                        } else {
                            len.saturating_sub(st)
                        };
                        if circular && end > len {
                            end -= len;
                        }

                        guides.push(Guide {
                            protospacer: proto_slice.to_string(),
                            pam: pam_slice.to_string(),
                            start,
                            end,
                            forward: false,
                            gc_percent: gc,
                            score,
                            warnings,
                        });
                    }
                }
            }
        }
    }

    if !circular {
        guides.retain(|g| g.start >= from && g.end <= to);
    }

    guides.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| a.end.cmp(&b.end))
            .then_with(|| {
                if a.forward == b.forward {
                    std::cmp::Ordering::Equal
                } else if a.forward {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
    });

    guides
}

/// Counts all sites in `reference` (both strands, non-circular) where a matching PAM (per `spec`) is present and the
/// adjacent protospacer has at most `max_mismatches` differences from `guide.protospacer`. The exact site
/// of the guide itself counts. With `pam_three_prime == false` the protospacer follows the PAM.
pub fn count_sites(reference: &str, guide: &Guide, spec: &PamSpec, max_mismatches: usize) -> usize {
    let ref_upper = reference.to_ascii_uppercase().replace('U', "T");
    let g_len = spec.guide_length;
    let p_len = spec.pattern.len();
    let total = g_len + p_len;

    if ref_upper.is_empty() || total > ref_upper.len() {
        return 0;
    }

    let len = ref_upper.len();
    let mut count: usize = 0;

    // forward strand (non-circular)
    for st in 0..=len - total {
        let pam_slice = if spec.pam_three_prime {
            ref_upper.get(st + g_len..st + total)
        } else {
            ref_upper.get(st..st + p_len)
        };
        if let Some(pam_slice) = pam_slice {
            if matches_iupac(&spec.pattern, pam_slice) {
                let proto_slice = if spec.pam_three_prime {
                    ref_upper.get(st..st + g_len)
                } else {
                    ref_upper.get(st + p_len..st + total)
                };
                if let Some(proto_slice) = proto_slice {
                    let mismatches = proto_slice
                        .chars()
                        .zip(guide.protospacer.chars())
                        .filter(|(a, b)| a != b)
                        .count();
                    if mismatches <= max_mismatches {
                        count += 1;
                    }
                }
            }
        }
    }

    // reverse strand (non-circular)
    let rc = iupac_reverse_complement(&ref_upper);
    for st in 0..=rc.len() - total {
        let pam_slice = if spec.pam_three_prime {
            rc.get(st + g_len..st + total)
        } else {
            rc.get(st..st + p_len)
        };
        if let Some(pam_slice) = pam_slice {
            if matches_iupac(&spec.pattern, pam_slice) {
                let proto_slice = if spec.pam_three_prime {
                    rc.get(st..st + g_len)
                } else {
                    rc.get(st + p_len..st + total)
                };
                if let Some(proto_slice) = proto_slice {
                    let mismatches = proto_slice
                        .chars()
                        .zip(guide.protospacer.chars())
                        .filter(|(a, b)| a != b)
                        .count();
                    if mismatches <= max_mismatches {
                        count += 1;
                    }
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: &str = "GCATGCATCGATCGTAGCTA";

    fn seq() -> String {
        format!("AA{}TGGCC", P)
    }

    fn rc(s: &str) -> String {
        iupac_reverse_complement(s)
    }

    #[test]
    fn vorlagen() {
        let sp = PamSpec::sp_cas9();
        assert_eq!(
            (sp.pattern.as_str(), sp.guide_length, sp.pam_three_prime),
            ("NGG", 20, true)
        );
        let sa = PamSpec::sa_cas9();
        assert_eq!(
            (sa.pattern.as_str(), sa.guide_length, sa.pam_three_prime),
            ("NNGRRT", 21, true)
        );
        let c = PamSpec::cas12a();
        assert_eq!(
            (c.pattern.as_str(), c.guide_length, c.pam_three_prime),
            ("TTTV", 23, false)
        );
    }

    #[test]
    fn bewertung() {
        let (gc, score, w) = score_protospacer(P);
        assert!((gc - 50.0).abs() < 1e-9);
        assert!((score - 100.0).abs() < 1e-9);
        assert!(w.is_empty());
        let (_, s, w) = score_protospacer("GCATGCTTTTCGATCGTAGC");
        assert!((s - 70.0).abs() < 1e-9);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("TTTT"));
        let (gc, s, w) = score_protospacer("ATATATATATATATATATAT");
        assert!((gc - 0.0).abs() < 1e-9 && (s - 80.0).abs() < 1e-9);
        assert!(w[0].contains("GC fraction"));
        let (_, s, w) = score_protospacer("gcgcaaaaagcgctagctag");
        assert!((s - 90.0).abs() < 1e-9);
        assert!(w[0].contains("Homopolymer"));
        // run of 5 T: both rules apply (−30 and −10)
        let (_, s, w) = score_protospacer("GCATGCTTTTTGATCGTAGC");
        assert!((s - 60.0).abs() < 1e-9);
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn spcas9_vorwaerts() {
        let g = find_guides(&PamSpec::sp_cas9(), &seq(), 0, 100, false);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].protospacer, P);
        assert_eq!(g[0].pam, "TGG");
        assert_eq!((g[0].start, g[0].end, g[0].forward), (2, 25, true));
        assert!((g[0].gc_percent - 50.0).abs() < 1e-9);
        assert!((g[0].score - 100.0).abs() < 1e-9);
    }

    #[test]
    fn spcas9_gegenstrang() {
        let g = find_guides(&PamSpec::sp_cas9(), &rc(&seq()), 0, 100, false);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].protospacer, P);
        assert_eq!(g[0].pam, "TGG");
        assert_eq!((g[0].start, g[0].end, g[0].forward), (2, 25, false));
    }

    #[test]
    fn bereich_und_leere_faelle() {
        let spec = PamSpec::sp_cas9();
        assert!(find_guides(&spec, &seq(), 0, 24, false).is_empty());
        assert_eq!(find_guides(&spec, &seq(), 0, 25, false).len(), 1);
        assert!(find_guides(&spec, &seq(), 3, 100, false).is_empty());
        assert!(find_guides(&spec, "", 0, 10, false).is_empty());
        assert!(find_guides(&spec, "TGG", 0, 3, false).is_empty());
        assert!(find_guides(&spec, &seq(), 10, 5, false).is_empty());
    }

    #[test]
    fn bewertung_im_fund() {
        let g = find_guides(&PamSpec::sp_cas9(), "GCATGCTTTTCGATCGTAGCAGG", 0, 23, false);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].pam, "AGG");
        assert!((g[0].score - 70.0).abs() < 1e-9);
        assert_eq!(g[0].warnings.len(), 1);
    }

    #[test]
    fn andere_nukleasen() {
        let c = find_guides(
            &PamSpec::cas12a(),
            "TTTAGCATGCATCGATCGTAGCTAGCTC",
            0,
            100,
            false,
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].pam, "TTTA");
        assert_eq!(c[0].protospacer, "GCATGCATCGATCGTAGCTAGCT");
        assert_eq!((c[0].start, c[0].end), (0, 27));
        let s = find_guides(
            &PamSpec::sa_cas9(),
            "GCATGCATCGATCGTAGCTAGAAGAAT",
            0,
            100,
            false,
        );
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].pam, "AAGAAT");
        assert_eq!(s[0].protospacer, "GCATGCATCGATCGTAGCTAG");
    }

    #[test]
    fn zirkulaer_ueber_den_nullpunkt() {
        let s = seq();
        let rotated = format!("{}{}", &s[10..], &s[..10]);
        let g = find_guides(&PamSpec::sp_cas9(), &rotated, 0, 0, true);
        assert_eq!(g.len(), 2);
        assert_eq!((g[0].start, g[0].end, g[0].forward), (15, 11, false));
        assert_eq!(g[0].protospacer, "AGCTACGATCGATGCATGCT");
        assert_eq!((g[1].start, g[1].end, g[1].forward), (19, 15, true));
        assert_eq!(g[1].protospacer, P);
    }

    #[test]
    fn nebentreffer() {
        let spec = PamSpec::sp_cas9();
        let reference = format!("{}TGGAAAAGCATGAATCGATCGTAGCTTAGG", P);
        let guide = find_guides(&spec, &format!("{}TGG", P), 0, 100, false).remove(0);
        assert_eq!(count_sites(&reference, &guide, &spec, 3), 2);
        assert_eq!(count_sites(&reference, &guide, &spec, 1), 1);
        assert_eq!(count_sites(&reference, &guide, &spec, 0), 1);
        assert_eq!(count_sites("", &guide, &spec, 3), 0);
        // reverse strand counts too
        assert_eq!(count_sites(&rc(&reference), &guide, &spec, 3), 2);
    }
}
