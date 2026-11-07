//! Doppelsträngige Fragmente und virtueller Restriktionsverdau.

use crate::cloning::enzymes::{iupac_reverse_complement, CutSite, EnzymeDb};
use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location};

/// Art eines Fragmentendes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EndType {
    Blunt,
    FivePrime,
    ThreePrime,
}

/// Ein Ende eines Fragments. `overhang` steht in Oberstrang-Schreibweise (5'→3' des Oberstrangs); bei `Blunt` leer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct End {
    pub end_type: EndType,
    pub overhang: String,
}

impl End {
    /// Stumpfes Ende.
    pub fn blunt() -> End {
        End {
            end_type: EndType::Blunt,
            overhang: String::new(),
        }
    }

    /// Ende mit Typ und Überhang. Ist der Überhang leer, entsteht ein stumpfes Ende (egal welcher Typ).
    pub fn new(end_type: EndType, overhang: &str) -> End {
        if overhang.is_empty() {
            End {
                end_type: EndType::Blunt,
                overhang: String::new(),
            }
        } else {
            End {
                end_type,
                overhang: overhang.to_string(),
            }
        }
    }

    /// Gleicher Typ, Überhang mit `iupac_reverse_complement` umgekehrt komplementiert.
    pub fn reverse_complement(&self) -> End {
        End::new(self.end_type, &iupac_reverse_complement(&self.overhang))
    }

    /// Wahr, wenn das Ende stumpf ist.
    pub fn is_blunt(&self) -> bool {
        self.end_type == EndType::Blunt
    }

    /// Wahr, wenn beide stumpf sind oder beide denselben Typ (`FivePrime`/`ThreePrime`) und denselben Überhang haben.
    pub fn pairs_with(&self, other: &End) -> bool {
        if self.end_type == EndType::Blunt && other.end_type == EndType::Blunt {
            return true;
        }
        self.end_type == other.end_type && self.overhang == other.overhang
    }
}

/// Doppelsträngiges Fragment. `top` ist der Oberstrang von seinem 5'-Ende bis zum 3'-Ende (nur die Basen zwischen
/// den beiden Oberstrang-Schnitten). `features` liegen in Fragmentkoordinaten (0-basiert).
/// `source_start`/`source_end` sind die Schnittpositionen im Quelldokument; bei einem Fragment über den Nullpunkt
/// einer zirkulären Quelle ist `source_end < source_start`.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    pub top: String,
    pub left: End,
    pub right: End,
    pub features: Vec<Feature>,
    pub source_name: String,
    pub source_start: usize,
    pub source_end: usize,
}

impl Fragment {
    /// Länge des Oberstrangs.
    pub fn length(&self) -> usize {
        self.top.len()
    }

    /// Das Fragment von der anderen Seite gelesen: `top` reverse-komplementiert (`iupac_reverse_complement`),
    /// linkes Ende = reverse-komplementiertes altes rechtes Ende und umgekehrt, Features gespiegelt
    /// (Intervall `[s, e)` wird zu `[len − e, len − s)`, Reihenfolge der Intervalle umgekehrt,
    /// Strang mit `Strand::flipped()` gedreht, `partial_start`/`partial_end` getauscht).
    /// `source_name`, `source_start` und `source_end` bleiben. Zweimal angewendet ergibt das Original.
    /// Fehler (`Error`) nur, wenn ein Feature nicht gespiegelt werden kann.
    pub fn reverse_complement(&self) -> Result<Fragment> {
        let len = self.length();
        let new_top = iupac_reverse_complement(&self.top);

        // Reverse complement ends: new left = rc(old right), new right = rc(old left)
        let new_left = self.right.reverse_complement();
        let new_right = self.left.reverse_complement();

        // Mirror features: [s, e) -> [len - e, len - s), flip strand, swap partials
        let mut new_features = Vec::with_capacity(self.features.len());
        for f in &self.features {
            let loc = f.location();
            let mut new_intervals = Vec::with_capacity(loc.intervals().len());
            for interval in loc.intervals() {
                let s = interval.start();
                let e = interval.end();
                // Check bounds: len - e could be negative? No, s >= 0, e <= len.
                // But if e > len, it's invalid? The spec says "Fehler nur, wenn ein Feature nicht gespiegelt werden kann."
                // Assuming features are within fragment bounds.
                let new_s = len.saturating_sub(e);
                let new_e = len.saturating_sub(s);
                if new_s > new_e {
                    return Err(Error::InvalidInput(
                        "Feature cannot be mirrored".to_string(),
                    ));
                }
                new_intervals.push(Interval::new(new_s, new_e)?);
            }

            // Reverse order of intervals? The spec says "Reihenfolge der Intervalle umgekehrt".
            new_intervals.reverse();

            let new_loc = Location::new(
                new_intervals,
                loc.strand().flipped(),
                loc.partial_end(), // swap partials
                loc.partial_start(),
            )?;

            new_features.push(f.with_location(new_loc));
        }

        Ok(Fragment {
            top: new_top,
            left: new_left,
            right: new_right,
            features: new_features,
            source_name: self.source_name.clone(),
            source_start: self.source_start,
            source_end: self.source_end,
        })
    }

    /// Wahr, wenn dieses Fragment rechts an `next` passt: `self.right.pairs_with(&next.left)`.
    pub fn can_join(&self, next: &Fragment) -> bool {
        self.right.pairs_with(&next.left)
    }
}

/// Virtueller Verdau von `doc` mit den Enzymen `enzyme_names` aus `db`.
///
/// 1. Jeder Name muss in `db` vorkommen, sonst `Error::InvalidInput`. Schnittstellen kommen aus
///    `db.find_in_document(doc)`, behalten werden nur die der gewählten Enzyme.
/// 2. Schnitte sind die Paare `(cut_top, cut_bottom)` der Schnittstellen. Mehrere Schnitte mit gleichem `cut_top` zählen
///    einmal (der erste in der Reihenfolge der Schnittstellenliste gilt).
/// 3. Ende am Schnitt: `cut_bottom > cut_top` → `FivePrime`, kleiner → `ThreePrime`, gleich → `Blunt`; der Überhang ist
///    `CutSite::overhang` (bei leerem Überhang `Blunt`). Das rechte Ende des Fragments links vom Schnitt und das linke Ende
///    des Fragments rechts vom Schnitt bekommen dasselbe Ende (gleiche Werte).
/// 4. Linear: Schnitte mit `cut_top <= 0` oder `cut_top >= Länge` werden ignoriert. Mit den sortierten Schnitten
///    `c1 < … < ck` entstehen die Fragmente `[0,c1)`, `[c1,c2)`, …, `[ck,Länge)`; das linke Ende des ersten und das rechte
///    des letzten Fragments sind stumpf. Ohne Schnitt: ein Fragment mit der ganzen Sequenz, beide Enden stumpf.
/// 5. Zirkulär: ohne Schnitt ein Fragment (ganze Sequenz, beide Enden stumpf, `source_start 0`, `source_end Länge`).
///    Mit `k >= 1` verschiedenen Schnitten `c1 < … < ck` (die `cut_top` der Schnittstellen liegen schon in `0..Länge`) entstehen
///    `k` Fragmente `[c1,c2)`, …, `[ck,Länge)+[0,c1)` (das letzte läuft über den Nullpunkt). Bei `k == 1` ist das die um
///    `c1` gedrehte ganze Sequenz, beide Enden gleich dem Ende des einen Schnitts.
/// 6. Features: nur solche, deren Hülle (`location().start()` bis `location().end()`) ganz im Fragment liegt, werden
///    übernommen und um den Fragmentanfang verschoben (alle Intervalle, Strang und Partial-Flags bleiben). Bei dem Fragment
///    über den Nullpunkt gilt: Hülle ganz in `[ck, Länge)` → um `ck` verschieben; ganz in `[0, c1)` → um `Länge − ck`
///    nach rechts verschieben. Alle anderen Features entfallen. Bei `k == 1` zirkulär gilt dasselbe mit `ck = c1`.
/// 7. `source_name` ist der Name des Dokuments; `source_start`/`source_end` sind die Schnittpositionen (Linear: 0 und Länge an den Rändern).
pub fn digest(
    doc: &SequenceDocument,
    db: &EnzymeDb,
    enzyme_names: &[&str],
) -> Result<Vec<Fragment>> {
    for name in enzyme_names {
        if db.get(name).is_none() {
            return Err(Error::InvalidInput(format!("Unknown enzyme: {}", name)));
        }
    }
    let bases = doc.bases();
    let len = bases.len();
    let circular = doc.topology() == Topology::Circular;

    // Schnitte: (cut_top, Ende), je cut_top einmal, aufsteigend sortiert
    let mut cuts: Vec<(usize, End)> = Vec::new();
    for site in db.find_in_document(doc) {
        if !enzyme_names.contains(&site.enzyme.as_str()) || site.cut_top < 0 {
            continue;
        }
        let pos = site.cut_top as usize;
        let inside = if circular {
            pos < len
        } else {
            pos > 0 && pos < len
        };
        if inside && !cuts.iter().any(|(p, _)| *p == pos) {
            cuts.push((pos, cut_end(&site)));
        }
    }
    cuts.sort_by_key(|c| c.0);

    // Fragmente als (Anfang, Ende, linkes Ende, rechtes Ende)
    let mut spans: Vec<(usize, usize, End, End)> = Vec::new();
    if cuts.is_empty() {
        spans.push((0, len, End::blunt(), End::blunt()));
    } else if circular {
        for (i, (start, left)) in cuts.iter().enumerate() {
            let (end, right) = cuts
                .get((i + 1) % cuts.len())
                .cloned()
                .unwrap_or((*start, left.clone()));
            spans.push((*start, end, left.clone(), right));
        }
    } else {
        let mut start = 0;
        let mut left = End::blunt();
        for (pos, end) in &cuts {
            spans.push((start, *pos, left.clone(), end.clone()));
            start = *pos;
            left = end.clone();
        }
        spans.push((start, len, left, End::blunt()));
    }

    let mut fragments = Vec::new();
    for (start, end, left, right) in spans {
        let wraps = circular && !cuts.is_empty() && start >= end;
        let top = if wraps {
            format!(
                "{}{}",
                bases.get(start..).unwrap_or(""),
                bases.get(..end).unwrap_or("")
            )
        } else {
            bases.get(start..end).unwrap_or("").to_string()
        };
        fragments.push(Fragment {
            top,
            left,
            right,
            features: fragment_features(doc.features(), start, end, len, wraps)?,
            source_name: doc.name().to_string(),
            source_start: start,
            source_end: end,
        });
    }
    Ok(fragments)
}

/// Ende an einem Schnitt aus den Schnittpositionen der Schnittstelle.
fn cut_end(site: &CutSite) -> End {
    let kind = match site.cut_bottom.cmp(&site.cut_top) {
        std::cmp::Ordering::Greater => EndType::FivePrime,
        std::cmp::Ordering::Less => EndType::ThreePrime,
        std::cmp::Ordering::Equal => EndType::Blunt,
    };
    End::new(kind, &site.overhang)
}

/// Features, deren Hülle ganz im Fragment liegt, in Fragmentkoordinaten.
/// Bei einem Fragment über den Nullpunkt (`wraps`) zählt die Hülle ganz in `[start, len)` oder ganz in `[0, end)`.
fn fragment_features(
    features: &[Feature],
    start: usize,
    end: usize,
    len: usize,
    wraps: bool,
) -> Result<Vec<Feature>> {
    let mut kept = Vec::new();
    for f in features {
        let (fs, fe) = (f.location().start(), f.location().end());
        let delta: Option<isize> = if !wraps {
            (fs >= start && fe <= end).then(|| -(start as isize))
        } else if fs >= start && fe <= len {
            Some(-(start as isize))
        } else if fe <= end {
            Some((len - start) as isize)
        } else {
            None
        };
        if let Some(d) = delta {
            kept.push(f.with_location(f.location().shifted(d)?));
        }
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::Strand;
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(bases: &str, topology: Topology) -> SequenceDocument {
        let seq = Sequence::new("s", "s", bases, SequenceType::Dna).unwrap();
        SequenceDocument::of("s", seq).with_topology(topology)
    }

    fn feat(start: usize, end: usize, strand: Strand) -> Feature {
        let loc = Location::new(
            vec![Interval::new(start, end).unwrap()],
            strand,
            false,
            false,
        )
        .unwrap();
        Feature::new("gene", "g", loc, Vec::new())
    }

    fn five(o: &str) -> End {
        End::new(EndType::FivePrime, o)
    }

    #[test]
    fn enden() {
        assert_eq!(
            End::blunt(),
            End {
                end_type: EndType::Blunt,
                overhang: String::new()
            }
        );
        assert_eq!(End::new(EndType::FivePrime, ""), End::blunt());
        assert_eq!(five("AATT").reverse_complement(), five("AATT"));
        assert_eq!(five("AAGC").reverse_complement(), five("GCTT"));
        assert_eq!(End::blunt().reverse_complement(), End::blunt());
        assert!(End::blunt().pairs_with(&End::blunt()));
        assert!(five("AATT").pairs_with(&five("AATT")));
        assert!(!five("AATT").pairs_with(&five("GATC")));
        assert!(!five("AATT").pairs_with(&End::new(EndType::ThreePrime, "AATT")));
        assert!(!five("AATT").pairs_with(&End::blunt()));
    }

    #[test]
    fn linear_ecori() {
        let d = doc("AAGGAATTCGATTAGCTAGC", Topology::Linear);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].top, "AAGG");
        assert_eq!(f[0].left, End::blunt());
        assert_eq!(f[0].right, five("AATT"));
        assert_eq!(f[1].top, "AATTCGATTAGCTAGC");
        assert_eq!(f[1].left, five("AATT"));
        assert_eq!(f[1].right, End::blunt());
        assert_eq!((f[0].source_start, f[0].source_end), (0, 4));
        assert_eq!((f[1].source_start, f[1].source_end), (4, 20));
        assert_eq!(f[0].source_name, "s");
        assert!(f[0].can_join(&f[1]));
        assert!(!f[0].can_join(&f[0]));
        assert_eq!(f[0].length(), 4);
    }

    #[test]
    fn pst_und_sma() {
        let d = doc("AACTGCAGTT", Topology::Linear);
        let f = digest(&d, &EnzymeDb::standard(), &["PstI"]).unwrap();
        assert_eq!(f[0].top, "AACTGCA");
        assert_eq!(f[0].right, End::new(EndType::ThreePrime, "TGCA"));
        assert_eq!(f[1].top, "GTT");
        assert_eq!(f[1].left, End::new(EndType::ThreePrime, "TGCA"));
        let s = digest(
            &doc("AACCCGGGTT", Topology::Linear),
            &EnzymeDb::standard(),
            &["SmaI"],
        )
        .unwrap();
        assert_eq!(s[0].top, "AACCC");
        assert_eq!(s[1].top, "GGGTT");
        assert_eq!(s[0].right, End::blunt());
        assert_eq!(s[1].left, End::blunt());
    }

    fn plasmid() -> String {
        format!(
            "{}GAATTC{}GGATCC{}",
            "A".repeat(100),
            "A".repeat(294),
            "A".repeat(594)
        )
    }

    #[test]
    fn zirkulaer_zwei_schnitte() {
        let p = plasmid();
        assert_eq!(p.len(), 1000);
        let d = doc(&p, Topology::Circular);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI", "BamHI"]).unwrap();
        let mut laengen: Vec<usize> = f.iter().map(|x| x.length()).collect();
        laengen.sort();
        assert_eq!(laengen, vec![300, 700]);
        assert_eq!(f[0].top.len(), 300);
        assert!(f[0].top.starts_with("AATTC"));
        assert_eq!(f[0].left, five("AATT"));
        assert_eq!(f[0].right, five("GATC"));
        assert_eq!(f[1].left, five("GATC"));
        assert_eq!(f[1].right, five("AATT"));
        assert!(f[0].can_join(&f[1]) && f[1].can_join(&f[0]));
        assert_eq!((f[0].source_start, f[0].source_end), (101, 401));
        assert_eq!((f[1].source_start, f[1].source_end), (401, 101));
    }

    #[test]
    fn zirkulaer_ein_schnitt_und_kein_schnitt() {
        let p = plasmid();
        let d = doc(&p, Topology::Circular);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].length(), 1000);
        assert_eq!(f[0].left, five("AATT"));
        assert_eq!(f[0].right, five("AATT"));
        assert!(f[0].top.starts_with("AATTC"));
        assert!(f[0].top.ends_with('G'));
        let g = digest(&d, &EnzymeDb::standard(), &["PstI"]).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].top, p);
        assert_eq!(
            (g[0].left.clone(), g[0].right.clone()),
            (End::blunt(), End::blunt())
        );
    }

    #[test]
    fn linear_ohne_treffer_und_randschnitt() {
        let d = doc("AAAAAAAA", Topology::Linear);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].top, "AAAAAAAA");
        assert_eq!((f[0].source_start, f[0].source_end), (0, 8));
        // EcoRI-Stelle ganz am Anfang: Schnitt bei 1, kein leeres Fragment
        let g = digest(
            &doc("GAATTCAAAA", Topology::Linear),
            &EnzymeDb::standard(),
            &["EcoRI"],
        )
        .unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].top, "G");
        assert_eq!(g[1].top, "AATTCAAAA");
    }

    #[test]
    fn unbekanntes_enzym() {
        let d = doc("AAAA", Topology::Linear);
        assert!(matches!(
            digest(&d, &EnzymeDb::standard(), &["NixEnzym"]),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn features_bei_verdau() {
        let base = doc("AAGGAATTCGATTAGCTAGC", Topology::Linear).with_features(vec![
            feat(0, 3, Strand::Forward),
            feat(2, 6, Strand::Forward),
            feat(10, 14, Strand::Reverse),
        ]);
        let f = digest(&base, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(f[0].features.len(), 1);
        assert_eq!(f[0].features[0].location().start(), 0);
        // das Feature 2..6 überspannt den Schnitt bei 4 und entfällt
        assert_eq!(f[1].features.len(), 1);
        assert_eq!(f[1].features[0].location().start(), 6);
        assert_eq!(f[1].features[0].location().end(), 10);
        assert_eq!(f[1].features[0].location().strand(), Strand::Reverse);
    }

    #[test]
    fn features_ueber_den_nullpunkt() {
        let p = plasmid();
        let base = doc(&p, Topology::Circular).with_features(vec![
            feat(990, 995, Strand::Forward),
            feat(5, 9, Strand::Forward),
        ]);
        let f = digest(&base, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(f.len(), 1);
        // Schnitt bei 101, Fragment beginnt dort: 990 → 889, 5 → 5 + (1000 − 101) = 904
        let mut starts: Vec<usize> = f[0].features.iter().map(|x| x.location().start()).collect();
        starts.sort();
        assert_eq!(starts, vec![889, 904]);
    }

    #[test]
    fn reverse_komplement_des_fragments() {
        let d = doc("AAGGAATTCGATTAGCTAGC", Topology::Linear).with_features(vec![feat(
            10,
            14,
            Strand::Forward,
        )]);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        let rc = f[1].reverse_complement().unwrap();
        assert_eq!(rc.length(), f[1].length());
        assert_eq!(rc.top, "GCTAGCTAATCGAATT");
        assert_eq!(rc.left, End::blunt());
        assert_eq!(rc.right, five("AATT"));
        assert_eq!(rc.features.len(), 1);
        assert_eq!(rc.features[0].location().start(), 16 - 10);
        assert_eq!(rc.features[0].location().end(), 16 - 6);
        assert_eq!(rc.features[0].location().strand(), Strand::Reverse);
        assert_eq!(rc.reverse_complement().unwrap(), f[1]);
    }
}
