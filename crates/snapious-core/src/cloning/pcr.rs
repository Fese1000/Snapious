//! PCR simulation: find primer binding and compute products from primer pairs.

use crate::cloning::enzymes::iupac_reverse_complement;

/// A primer (sequence 5'→3').
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Primer {
    pub name: String,
    pub sequence: String,
}

/// Binding of a primer to the template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimerBinding {
    pub primer_name: String,
    /// Bound region of the template `[start, end)` without the 5' tail. `start` lies in `0..length`;
    /// for circular templates `end` can be greater than the length (binding across the origin).
    pub start: usize,
    pub end: usize,
    /// `true`: primer corresponds to the top strand (forward primer); `false`: binds to the top strand, its
    /// reverse complement is in the top strand.
    pub forward: bool,
    /// Mismatches in the bound region.
    pub mismatches: usize,
    /// Length of the 5' part of the primer that does not bind.
    pub tail_length: usize,
}

/// A PCR product.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcrProduct {
    /// Forward primer (whole, with tail) + template region in between + reverse complement of the reverse primer (whole).
    pub sequence: String,
    /// Start of the forward binding (`forward.start`) and end of the reverse binding in unrolled coordinates
    /// (for circular templates `template_end` can be greater than the template length).
    pub template_start: usize,
    pub template_end: usize,
    pub forward: PrimerBinding,
    pub reverse: PrimerBinding,
}

/// Settings of the simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcrSimulator {
    /// This many bases at the 3' end of the primer must match exactly (at least 1).
    pub min_three_prime_match: usize,
    /// Maximum number of mismatches in the 5'-side bound part.
    pub max_mismatches: usize,
    /// Products with a longer sequence are dropped.
    pub max_product_length: usize,
}

impl Default for PcrSimulator {
    /// 12, 2, 10000.
    fn default() -> Self {
        PcrSimulator {
            min_three_prime_match: 12,
            max_mismatches: 2,
            max_product_length: 10000,
        }
    }
}

impl PcrSimulator {
    pub fn new(
        min_three_prime_match: usize,
        max_mismatches: usize,
        max_product_length: usize,
    ) -> Self {
        PcrSimulator {
            min_three_prime_match,
            max_mismatches,
            max_product_length,
        }
    }

    /// All bindings of `primer` to `template` (uppercase, `U` as `T`; bases other than A, C, G, T never match).
    ///
    /// Forward primer (length `m`, `k = min_three_prime_match`, primer shorter than `k` → no binding): for every end position
    /// `e` (exclusive end of the binding, non-circular `k..=length`, circular `1..=length` with wrap-around) the last `k`
    /// bases of the primer must exactly equal `template[e−k..e]`. Then it is extended base by base towards 5' (`j = k, k+1, …, m−1`):
    /// `primer[m−1−j]` and `template[e−1−j]` are compared; non-circular, the extension stops when the template has ended
    /// (index < 0). Each mismatch increments the counter; if it exceeds `max_mismatches`, the extension stops. The
    /// bound length is the largest `j + 1` (at least `k`) at which the last compared base **matches** (the bound part
    /// never ends with a mismatch on the 5' side). `mismatches` counts only mismatches within the bound part,
    /// `tail_length = m − bound length`, `start = (e − bound length) mod length` (circular) or `e − bound length`,
    /// `end = start + bound length`.
    ///
    /// Reverse primer: the reverse complement `r` of the primer is searched. The first `k` bases of `r` (which is the
    /// 3' end of the primer) must match `template[s..s+k]` exactly, `s` in `0..length` (circular with wrap-around, non-circular
    /// only with `s + k <= length`). Extension to the right with the same rules: `r[j]` against `template[s+j]`. `start = s`,
    /// `end = s + bound length`, `forward = false`.
    ///
    /// Result sorted by `start`, then `end`, forward bindings before reverse bindings on a tie.
    pub fn find_bindings(
        &self,
        template: &str,
        circular: bool,
        primer: &Primer,
    ) -> Vec<PrimerBinding> {
        let text: Vec<u8> = template.to_ascii_uppercase().replace('U', "T").into_bytes();
        let fwd: Vec<u8> = primer
            .sequence
            .to_ascii_uppercase()
            .replace('U', "T")
            .into_bytes();
        let rev: Vec<u8> = iupac_reverse_complement(&String::from_utf8_lossy(&fwd)).into_bytes();
        let (len, m) = (text.len() as isize, fwd.len());
        let k = self.min_three_prime_match.max(1);
        if len == 0 || m < k {
            return Vec::new();
        }
        // Template base at position i (circular with wrap-around), None outside
        let at = |i: isize| -> Option<u8> {
            let idx = if circular { i.rem_euclid(len) } else { i };
            if idx < 0 || idx >= len {
                return None;
            }
            text.get(idx as usize).copied()
        };
        let same = |t: Option<u8>, c: Option<&u8>| -> bool {
            matches!((t, c), (Some(a), Some(b)) if a == *b && matches!(a, b'A' | b'C' | b'G' | b'T'))
        };
        // Extends from the exact 3' part: `base(j)` returns (template base, primer base) for the j-th base from the 3' end.
        let extend = |base: &dyn Fn(usize) -> (Option<u8>, Option<u8>)| -> (usize, usize) {
            let (mut best, mut best_mm, mut mm) = (k, 0usize, 0usize);
            let mut j = k;
            while j < m {
                let (t, c) = base(j);
                if t.is_none() {
                    break;
                }
                let ok = same(t, c.as_ref());
                if !ok {
                    mm += 1;
                    if mm > self.max_mismatches {
                        break;
                    }
                }
                j += 1;
                if ok {
                    best = j;
                    best_mm = mm;
                }
            }
            (best, best_mm)
        };

        let mut found = Vec::new();
        // Forward primer: the last k bases end at e (exclusive)
        let ends = if circular {
            1..=len
        } else {
            (k as isize)..=len
        };
        for e in ends {
            let exact = (0..k).all(|i| same(at(e - k as isize + i as isize), fwd.get(m - k + i)));
            if !exact {
                continue;
            }
            let base = |j: usize| (at(e - 1 - j as isize), fwd.get(m - 1 - j).copied());
            let (best, mismatches) = extend(&base);
            let start = if circular {
                (e - best as isize).rem_euclid(len)
            } else {
                e - best as isize
            } as usize;
            found.push(PrimerBinding {
                primer_name: primer.name.clone(),
                start,
                end: start + best,
                forward: true,
                mismatches,
                tail_length: m - best,
            });
        }
        // Reverse primer: the reverse complement is in the top strand, its first k bases start at s
        let starts = if circular {
            0..len
        } else {
            0..(len - k as isize + 1).max(0)
        };
        for s in starts {
            let exact = (0..k).all(|i| same(at(s + i as isize), rev.get(i)));
            if !exact {
                continue;
            }
            let base = |j: usize| (at(s + j as isize), rev.get(j).copied());
            let (best, mismatches) = extend(&base);
            found.push(PrimerBinding {
                primer_name: primer.name.clone(),
                start: s as usize,
                end: s as usize + best,
                forward: false,
                mismatches,
                tail_length: m - best,
            });
        }
        found.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then(a.end.cmp(&b.end))
                .then(b.forward.cmp(&a.forward))
        });
        found
    }

    /// Computes all products from all pairs (forward binding F, reverse binding R) of the primers in `primers`
    /// (the same primer may take both roles).
    /// - Non-circular: only pairs with `R.start >= F.end`.
    /// - Circular: `R.start` is increased by the template length until it is `>= F.end` (smallest such shift;
    ///   `R.end` is shifted along, the product carries the shifted reverse binding in `reverse`).
    /// - Sequence: primer of F (whole) + template region from `F.end` to `R.start` (with wrap-around if circular) + reverse complement
    ///   of the primer of R (whole).
    /// - `template_start = F.start`, `template_end = R.end` (shifted).
    /// - Products whose sequence is longer than `max_product_length` are dropped.
    /// - Sorting by sequence length, then `template_start`, then name of the forward primer.
    pub fn simulate(&self, template: &str, circular: bool, primers: &[Primer]) -> Vec<PcrProduct> {
        let text = template.to_ascii_uppercase().replace('U', "T");
        let len = text.len();
        // Bindings with the sequence of their primer
        let mut forwards: Vec<(PrimerBinding, String)> = Vec::new();
        let mut reverses: Vec<(PrimerBinding, String)> = Vec::new();
        for p in primers {
            let sequence = p.sequence.to_ascii_uppercase().replace('U', "T");
            for b in self.find_bindings(template, circular, p) {
                if b.forward {
                    forwards.push((b, sequence.clone()));
                } else {
                    reverses.push((b, sequence.clone()));
                }
            }
        }

        let mut products = Vec::new();
        for (f, f_seq) in &forwards {
            for (r, r_seq) in &reverses {
                let shift = if circular && len > 0 {
                    let mut s = 0;
                    while r.start + s < f.end {
                        s += len;
                    }
                    s
                } else {
                    0
                };
                let (r_start, r_end) = (r.start + shift, r.end + shift);
                if r_start < f.end {
                    continue;
                }
                let middle: String = (f.end..r_start)
                    .filter_map(|i| text.as_bytes().get(i % len.max(1)).map(|b| *b as char))
                    .collect();
                let sequence = format!("{}{}{}", f_seq, middle, iupac_reverse_complement(r_seq));
                if sequence.len() > self.max_product_length {
                    continue;
                }
                products.push(PcrProduct {
                    sequence,
                    template_start: f.start,
                    template_end: r_end,
                    forward: f.clone(),
                    reverse: PrimerBinding {
                        start: r_start,
                        end: r_end,
                        ..r.clone()
                    },
                });
            }
        }
        products.sort_by(|a, b| {
            a.sequence
                .len()
                .cmp(&b.sequence.len())
                .then_with(|| a.template_start.cmp(&b.template_start))
                .then_with(|| a.forward.primer_name.cmp(&b.forward.primer_name))
        });
        products
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = "ATGCCGTAGCTAGCATCGATCGGATCCGATCGATCGTAGCTAGCTAGGCTAGC";

    fn primer(name: &str, seq: &str) -> Primer {
        Primer {
            name: name.to_string(),
            sequence: seq.to_string(),
        }
    }

    fn rc(s: &str) -> String {
        iupac_reverse_complement(s)
    }

    fn sim() -> PcrSimulator {
        PcrSimulator::default()
    }

    #[test]
    fn einstellungen() {
        let s = PcrSimulator::default();
        assert_eq!(
            (
                s.min_three_prime_match,
                s.max_mismatches,
                s.max_product_length
            ),
            (12, 2, 10000)
        );
        assert_eq!(PcrSimulator::new(10, 1, 500).max_product_length, 500);
        assert_eq!(T.len(), 53);
    }

    #[test]
    fn volle_bindung_ergibt_die_vorlage() {
        let f = primer("F", &T[..14]);
        let r = primer("R", &rc(&T[39..]));
        let fb = sim().find_bindings(T, false, &f);
        assert_eq!(fb.len(), 1);
        assert_eq!(
            (
                fb[0].start,
                fb[0].end,
                fb[0].forward,
                fb[0].mismatches,
                fb[0].tail_length
            ),
            (0, 14, true, 0, 0)
        );
        let rb = sim().find_bindings(T, false, &r);
        assert_eq!((rb[0].start, rb[0].end, rb[0].forward), (39, 53, false));
        let p = sim().simulate(T, false, &[f, r]);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].sequence, T);
        assert_eq!((p[0].template_start, p[0].template_end), (0, 53));
        assert_eq!(p[0].forward.primer_name, "F");
        assert_eq!(p[0].reverse.primer_name, "R");
    }

    #[test]
    fn schwanz_wird_mitgenommen() {
        let f = primer("F", &format!("GAATTC{}", &T[..14]));
        let r = primer("R", &rc(&T[39..]));
        let p = sim().simulate(T, false, &[f, r]);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].sequence.len(), 59);
        assert!(p[0].sequence.starts_with("GAATTC"));
        assert_eq!(p[0].forward.tail_length, 6);
        assert_eq!(p[0].forward.start, 0);
        assert_eq!(p[0].template_start, 0);
    }

    #[test]
    fn falsches_dreiprimer_ende_bindet_nicht() {
        let mut s = T[..14].to_string();
        s.pop();
        s.push('A');
        assert_ne!(&s, &T[..14]);
        assert!(sim().find_bindings(T, false, &primer("x", &s)).is_empty());
    }

    #[test]
    fn abweichung_im_fuenf_strich_bereich() {
        let mut s: Vec<char> = T[..14].chars().collect();
        s[1] = 'A';
        let s: String = s.into_iter().collect();
        let strikt = PcrSimulator::new(12, 0, 10000).find_bindings(T, false, &primer("x", &s));
        assert_eq!(
            (
                strikt[0].start,
                strikt[0].end,
                strikt[0].mismatches,
                strikt[0].tail_length
            ),
            (2, 14, 0, 2)
        );
        let locker = PcrSimulator::new(12, 1, 10000).find_bindings(T, false, &primer("x", &s));
        assert_eq!(
            (
                locker[0].start,
                locker[0].end,
                locker[0].mismatches,
                locker[0].tail_length
            ),
            (0, 14, 1, 0)
        );
        // Mismatch at the very edge: bound part ends earlier, base 0 becomes tail
        let mut s2: Vec<char> = T[..14].chars().collect();
        s2[0] = 'C';
        let s2: String = s2.into_iter().collect();
        let b = PcrSimulator::new(12, 1, 10000).find_bindings(T, false, &primer("x", &s2));
        assert_eq!((b[0].start, b[0].mismatches, b[0].tail_length), (1, 0, 1));
    }

    #[test]
    fn zwei_vorwaertsprimer_ergeben_nichts() {
        let p = sim().simulate(T, false, &[primer("F", &T[..14]), primer("F2", &T[5..19])]);
        assert!(p.is_empty());
    }

    #[test]
    fn produkt_zu_lang() {
        let f = primer("F", &T[..14]);
        let r = primer("R", &rc(&T[39..]));
        assert!(PcrSimulator::new(12, 2, 52)
            .simulate(T, false, &[f.clone(), r.clone()])
            .is_empty());
        assert_eq!(
            PcrSimulator::new(12, 2, 53)
                .simulate(T, false, &[f, r])
                .len(),
            1
        );
    }

    #[test]
    fn primer_kuerzer_als_mindestbindung_oder_leer() {
        assert!(sim()
            .find_bindings(T, false, &primer("x", "ATGCC"))
            .is_empty());
        assert!(sim().find_bindings(T, false, &primer("x", "")).is_empty());
        assert!(sim()
            .find_bindings("", false, &primer("x", &T[..14]))
            .is_empty());
    }

    #[test]
    fn zirkulaer_ueber_den_nullpunkt() {
        let tt = format!("{}{}", T, T);
        let f = primer("F", &tt[47..61]);
        let r = primer("R", &rc(&T[14..28]));
        let fb = sim().find_bindings(T, true, &f);
        assert_eq!(fb.len(), 1);
        assert_eq!((fb[0].start, fb[0].end), (47, 61));
        let p = sim().simulate(T, true, &[f, r]);
        let echt: Vec<&PcrProduct> = p.iter().filter(|x| x.forward.primer_name == "F").collect();
        assert_eq!(echt.len(), 1);
        assert_eq!(echt[0].sequence, tt[47..81]);
        assert_eq!(echt[0].sequence.len(), 34);
        assert_eq!((echt[0].template_start, echt[0].template_end), (47, 81));
        // same template linear: forward primer across the origin finds nothing
        assert!(sim()
            .find_bindings(T, false, &primer("F", &tt[47..61]))
            .is_empty());
    }
}
