//! PCR-Simulation: Primerbindung finden und Produkte aus Primerpaaren berechnen.

use crate::cloning::enzymes::iupac_reverse_complement;

/// Ein Primer (Sequenz 5'→3').
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Primer {
    pub name: String,
    pub sequence: String,
}

/// Bindung eines Primers an die Vorlage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimerBinding {
    pub primer_name: String,
    /// Gebundener Bereich der Vorlage `[start, end)` ohne den 5'-Schwanz. `start` liegt in `0..Länge`;
    /// bei zirkulären Vorlagen kann `end` größer als die Länge sein (Bindung über den Nullpunkt).
    pub start: usize,
    pub end: usize,
    /// `true`: Primer entspricht dem Oberstrang (Vorwärtsprimer); `false`: bindet am Oberstrang, sein
    /// Reverse-Komplement steht im Oberstrang.
    pub forward: bool,
    /// Abweichungen im gebundenen Bereich.
    pub mismatches: usize,
    /// Länge des 5'-Teils des Primers, der nicht bindet.
    pub tail_length: usize,
}

/// Ein PCR-Produkt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcrProduct {
    /// Vorwärtsprimer (ganz, mit Schwanz) + Vorlagenbereich dazwischen + Reverse-Komplement des Rückwärtsprimers (ganz).
    pub sequence: String,
    /// Beginn der Vorwärtsbindung (`forward.start`) und Ende der Rückwärtsbindung in aufgerollten Koordinaten
    /// (bei Zirkularität kann `template_end` größer als die Vorlagenlänge sein).
    pub template_start: usize,
    pub template_end: usize,
    pub forward: PrimerBinding,
    pub reverse: PrimerBinding,
}

/// Einstellungen der Simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcrSimulator {
    /// So viele Basen am 3'-Ende des Primers müssen exakt passen (mindestens 1).
    pub min_three_prime_match: usize,
    /// Höchstzahl der Abweichungen im 5'-seitigen gebundenen Teil.
    pub max_mismatches: usize,
    /// Produkte mit längerer Sequenz entfallen.
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

    /// Alle Bindungen von `primer` an `template` (Uppercase, `U` wie `T`; Basen außerhalb von A, C, G, T passen nie).
    ///
    /// Vorwärtsprimer (Länge `m`, `k = min_three_prime_match`, Primer kürzer als `k` → keine Bindung): Für jede Endposition
    /// `e` (exklusives Ende der Bindung, nicht zirkulär `k..=Länge`, zirkulär `1..=Länge` mit Umlauf) müssen die letzten `k`
    /// Basen des Primers exakt `template[e−k..e]` entsprechen. Dann wird Base für Base nach 5' erweitert (`j = k, k+1, …, m−1`):
    /// verglichen werden `primer[m−1−j]` und `template[e−1−j]`; nicht zirkulär bricht die Erweiterung ab, wenn die Vorlage zu Ende
    /// ist (Index < 0). Jede Abweichung erhöht den Zähler; übersteigt er `max_mismatches`, bricht die Erweiterung ab. Die
    /// gebundene Länge ist das größte `j + 1` (mindestens `k`), bei dem die zuletzt verglichene Base **passt** (der gebundene Teil
    /// endet 5'-seitig nie mit einer Abweichung). `mismatches` zählt nur Abweichungen innerhalb des gebundenen Teils,
    /// `tail_length = m − gebundene Länge`, `start = (e − gebundene Länge) mod Länge` (zirkulär) bzw. `e − gebundene Länge`,
    /// `end = start + gebundene Länge`.
    ///
    /// Rückwärtsprimer: Das Reverse-Komplement `r` des Primers wird gesucht. Die ersten `k` Basen von `r` (das ist das
    /// 3'-Ende des Primers) müssen exakt an `template[s..s+k]` passen, `s` in `0..Länge` (zirkulär mit Umlauf, nicht zirkulär
    /// nur mit `s + k <= Länge`). Erweiterung nach rechts mit denselben Regeln: `r[j]` gegen `template[s+j]`. `start = s`,
    /// `end = s + gebundene Länge`, `forward = false`.
    ///
    /// Ergebnis nach `start`, dann `end` sortiert, Vorwärtsbindungen vor Rückwärtsbindungen bei Gleichstand.
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
        // Basis der Vorlage an Position i (zirkulär mit Umlauf), None außerhalb
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
        // Erweitert ab dem exakten 3'-Teil: `base(j)` liefert (Vorlagenbase, Primerbase) für die j-te Base vom 3'-Ende.
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
        // Vorwärtsprimer: letzte k Basen enden bei e (exklusiv)
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
        // Rückwärtsprimer: Reverse-Komplement steht im Oberstrang, die ersten k Basen davon beginnen bei s
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

    /// Berechnet alle Produkte aus allen Paaren (Vorwärtsbindung F, Rückwärtsbindung R) der Primer in `primers`
    /// (derselbe Primer darf beide Rollen haben).
    /// - Nicht zirkulär: nur Paare mit `R.start >= F.end`.
    /// - Zirkulär: `R.start` wird um die Vorlagenlänge erhöht, bis es `>= F.end` ist (kleinste solche Verschiebung;
    ///   `R.end` wird mit verschoben, das Produkt trägt die verschobene Rückwärtsbindung in `reverse`).
    /// - Sequenz: Primer von F (ganz) + Vorlagenbereich von `F.end` bis `R.start` (bei Zirkularität mit Umlauf) + Reverse-Komplement
    ///   des Primers von R (ganz).
    /// - `template_start = F.start`, `template_end = R.end` (verschoben).
    /// - Produkte, deren Sequenz länger als `max_product_length` ist, entfallen.
    /// - Sortierung nach Sequenzlänge, dann `template_start`, dann Name des Vorwärtsprimers.
    pub fn simulate(&self, template: &str, circular: bool, primers: &[Primer]) -> Vec<PcrProduct> {
        let text = template.to_ascii_uppercase().replace('U', "T");
        let len = text.len();
        // Bindungen mit der Sequenz ihres Primers
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
        // Abweichung ganz außen: gebundener Teil endet vorher, Base 0 wird Schwanz
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
        // dieselbe Vorlage linear: Vorwärtsprimer über den Nullpunkt findet nichts
        assert!(sim()
            .find_bindings(T, false, &primer("F", &tt[47..61]))
            .is_empty());
    }
}
