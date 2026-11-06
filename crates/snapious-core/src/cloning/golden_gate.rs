//! Golden-Gate-Zusammenbau mit einem Typ-IIS-Enzym (BsaI, BsmBI, Esp3I, BbsI, SapI, …).

use std::collections::{BTreeMap, HashSet};

use crate::cloning::enzymes::{iupac_reverse_complement, EnzymeDb};
use crate::cloning::fragment::{digest, End, Fragment};
use crate::cloning::ligation::join_fragments;
use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::Feature;
use crate::model::sequence::{Sequence, SequenceType};
use crate::util::{new_id, now_secs};

/// Ergebnis eines Zusammenbaus.
#[derive(Clone, Debug, PartialEq)]
pub struct GoldenGateResult {
    pub construct: SequenceDocument,
    /// Die verbundenen Fragmente in Reihenfolge der Kette.
    pub parts: Vec<Fragment>,
    pub warnings: Vec<String>,
}

/// Wählt aus dem Verdau eines Teils das Fragment, das ins Konstrukt kommt, und meldet Auffälligkeiten.
///
/// - `fragments` ist das Ergebnis von `digest(teil, db, [enzym])`, `recognition` die Erkennungssequenz des Enzyms.
/// - Linear: bei mindestens 3 Fragmenten (mindestens 2 Schnitte) sind die Innenfragmente alle außer dem ersten und letzten.
///   Zirkulär: bei mindestens 2 Fragmenten alle Fragmente.
/// - Gibt es mehrere Innenfragmente, wird zuerst unter denen gewählt, deren `top` weder `recognition` noch deren
///   Reverse-Komplement enthält; unter den verbliebenen (oder, wenn keines übrig ist, unter allen) das längste, bei Gleichstand
///   das erste. Gab es mehr Innenfragmente als bei genau zwei Schnitten (linear: mehr als 3 Fragmente, zirkulär: mehr als 2),
///   kommt die Warnung `Interne Erkennungsstelle in {name}`.
/// - Hat das Teil zu wenige Schnitte (linear weniger als 3 Fragmente, zirkulär weniger als 2), kommt die Warnung
///   `Teil {name} hat weniger als 2 Schnittstellen` und das ganze Teil wird als Fragment mit stumpfen Enden verwendet
///   (`top` = alle Basen, Features des Teils, `source_name` = Name, `source_start 0`, `source_end` = Länge).
///
/// Rückgabe: das gewählte Fragment und die Warnungen.
pub fn pick_part_fragment(
    part: &SequenceDocument,
    fragments: &[Fragment],
    recognition: &str,
) -> (Fragment, Vec<String>) {
    let is_circ = part.topology() == Topology::Circular;
    let min_len = if is_circ { 2 } else { 3 };

    if fragments.is_empty() || fragments.len() < min_len {
        let warn = format!("Teil {} hat weniger als 2 Schnittstellen", part.name());
        return (
            Fragment {
                top: part.bases().to_string(),
                left: End::blunt(),
                right: End::blunt(),
                features: part.features().to_vec(),
                source_name: part.name().to_string(),
                source_start: 0,
                source_end: part.len(),
            },
            vec![warn],
        );
    }

    let inner_frags: Vec<&Fragment> = fragments
        .iter()
        .skip(if is_circ { 0 } else { 1 })
        .take(if is_circ {
            fragments.len()
        } else {
            fragments.len().saturating_sub(2)
        })
        .collect();

    let rec_upper = recognition.to_ascii_uppercase();
    let rc_rec = iupac_reverse_complement(&rec_upper);

    let mut candidates: Vec<&Fragment> = inner_frags
        .iter()
        .filter(|f| {
            let t = f.top.to_ascii_uppercase();
            !t.contains(&rec_upper) && !t.contains(&rc_rec)
        })
        .copied()
        .collect();

    if candidates.is_empty() {
        candidates = inner_frags;
    }

    let best = candidates.iter().max_by_key(|f| f.top.len()).copied();
    let best = match best {
        Some(f) => f,
        None => {
            let fallback = Fragment {
                top: String::new(),
                left: End::blunt(),
                right: End::blunt(),
                features: Vec::new(),
                source_name: String::new(),
                source_start: 0,
                source_end: 0,
            };
            return (fallback, Vec::new());
        }
    };

    let mut warnings = Vec::new();
    if fragments.len() > (if is_circ { 2 } else { 3 }) {
        warnings.push(format!("Interne Erkennungsstelle in {}", part.name()));
    }

    (best.clone(), warnings)
}

/// Warnungen zu den Überhängen der gewählten Fragmente (Reihenfolge: doppelte, palindromische, falsche Orientierung).
fn overhang_warnings(fragments: &[Fragment]) -> Vec<String> {
    let mut out = Vec::new();
    // Enden in der Reihenfolge Fragment 1 links, Fragment 1 rechts, Fragment 2 links, …
    let ends: Vec<(bool, &End)> = fragments
        .iter()
        .flat_map(|f| [(false, &f.left), (true, &f.right)])
        .collect();

    // 1. doppelte Enden (je Seite)
    let mut seen: HashSet<String> = HashSet::new();
    for (side, e) in &ends {
        if e.is_blunt() {
            continue;
        }
        let count = ends.iter().filter(|(s, o)| s == side && *o == *e).count();
        if count > 1 && seen.insert(e.overhang.clone()) {
            out.push(format!("Doppelter Überhang {}", e.overhang));
        }
    }
    // 2. palindromische Überhänge
    let mut seen: HashSet<String> = HashSet::new();
    for (_, e) in &ends {
        if !e.is_blunt()
            && e.overhang == iupac_reverse_complement(&e.overhang)
            && seen.insert(e.overhang.clone())
        {
            out.push(format!("Palindromischer Überhang {}", e.overhang));
        }
    }
    // 3. falsche Orientierung (gleiche Seite, Reverse-Komplement eines anderen Fragments)
    let mut seen: HashSet<String> = HashSet::new();
    for (idx, (side, e)) in ends.iter().enumerate() {
        if e.is_blunt() {
            continue;
        }
        let rc = iupac_reverse_complement(&e.overhang);
        if rc == e.overhang {
            continue;
        }
        let other = ends.iter().enumerate().any(|(k, (s, o))| {
            k != idx && s == side && !o.is_blunt() && o.end_type == e.end_type && o.overhang == rc
        });
        if other && seen.insert(e.overhang.clone()) {
            out.push(format!(
                "Überhang {} passt zum Reverse-Komplement eines anderen Teils (falsche Orientierung)",
                e.overhang
            ));
        }
    }
    out
}

/// Baut aus `parts` ein Konstrukt.
///
/// 1. Das Enzym `enzyme_name` muss in `db` stehen, sonst `Error::InvalidInput`; ohne Teile `Error::InvalidInput`.
///    Jedes Teil wird mit `digest` verdaut und mit `pick_part_fragment` auf ein Fragment reduziert (Warnungen sammeln).
/// 2. Warnungen zu Überhängen (je einmal pro betroffenen Überhangtext, in dieser Reihenfolge, nach den Teilewarnungen):
///    - `Doppelter Überhang {x}`, wenn dasselbe Ende (Typ und Text, nicht stumpf) an mehreren rechten Enden oder an mehreren linken
///      Enden verschiedener Fragmente vorkommt;
///    - `Palindromischer Überhang {x}`, wenn ein nicht stumpfes Ende `x` gleich seinem eigenen Reverse-Komplement ist;
///    - `Überhang {x} passt zum Reverse-Komplement eines anderen Teils (falsche Orientierung)`, wenn ein nicht palindromisches
///      rechtes Ende `x` gleich dem Reverse-Komplement des rechten Endes eines anderen Fragments ist (ebenso für linke Enden).
///      Reihenfolge der Prüfungen: Doppelte, Palindrome, Orientierung; innerhalb einer Prüfung nach Fragmentreihenfolge, dann links vor rechts.
/// 3. Kette: Zirkulär beginnt sie beim ersten Fragment; linear bei dem ersten Fragment (in Eingabereihenfolge), dessen linkes Ende
///    zu keinem rechten Ende eines anderen Fragments passt (`pairs_with`); gibt es kein solches, `Error::InvalidInput`.
///    Weitere Glieder: das erste noch nicht benutzte Fragment (Eingabereihenfolge), dessen linkes Ende zum rechten Ende des
///    letzten Gliedes passt.
/// 4. Sind am Ende nicht alle Fragmente benutzt, oder (zirkulär) passt das letzte Glied nicht ans erste:
///    `Error::InvalidInput("Teile lassen sich nicht zu einer Kette verbinden: {Namen der Teile, die nicht benutzt wurden bzw. der Kette}")`.
/// 5. Konstrukt: `join_fragments(&kette, circular)`; Name = Teilenamen (Reihenfolge der Kette) mit `" + "`; neue ID; DNA; Topologie
///    nach `circular`; Metadaten `source = "clone.goldengate"`; Erstellzeit `now_secs()`; Features der Fragmente, um die Länge
///    der vorangehenden Fragmente verschoben (`Location::shifted`, `Feature::with_location`).
/// 6. Erkennungsstelle des Enzyms (oder deren Reverse-Komplement) im Konstrukt (bei zirkulär auch über den Nullpunkt, also in
///    `konstrukt + konstrukt[..stellenlänge−1]`): Warnung `Erkennungsstelle von {enzyme_name} im Konstrukt vorhanden`.
pub fn assemble(
    db: &EnzymeDb,
    enzyme_name: &str,
    parts: &[SequenceDocument],
    circular: bool,
) -> Result<GoldenGateResult> {
    let enzyme = db
        .get(enzyme_name)
        .ok_or_else(|| Error::InvalidInput(format!("Enzym {} nicht gefunden", enzyme_name)))?;

    if parts.is_empty() {
        return Err(Error::InvalidInput("Keine Teile angegeben".to_string()));
    }

    let mut all_fragments = Vec::new();
    let mut part_warnings = Vec::new();

    for p in parts {
        let frags = digest(p, db, &[enzyme_name])?;
        let (frag, pw) = pick_part_fragment(p, &frags, enzyme.recognition());
        all_fragments.push(frag);
        part_warnings.extend(pw);
    }

    let mut warnings = part_warnings;
    warnings.extend(overhang_warnings(&all_fragments));

    // Kette bilden
    let n = all_fragments.len();
    let unused_names = |used: &[bool]| -> String {
        parts
            .iter()
            .zip(used)
            .filter(|(_, u)| !**u)
            .map(|(p, _)| p.name())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let chain_error = |names: String| {
        Error::InvalidInput(format!(
            "Teile lassen sich nicht zu einer Kette verbinden: {}",
            names
        ))
    };
    let mut used = vec![false; n];
    let start_idx = if circular {
        0
    } else {
        let found = (0..n).find(|&i| {
            all_fragments.get(i).is_some_and(|f| {
                !all_fragments
                    .iter()
                    .enumerate()
                    .any(|(j, g)| j != i && g.right.pairs_with(&f.left))
            })
        });
        match found {
            Some(i) => i,
            None => return Err(chain_error(unused_names(&used))),
        }
    };
    let mut order = vec![start_idx];
    if let Some(u) = used.get_mut(start_idx) {
        *u = true;
    }
    while order.len() < n {
        let Some(last) = order.last().and_then(|i| all_fragments.get(*i)) else {
            break;
        };
        let next = (0..n).find(|&i| {
            !used.get(i).copied().unwrap_or(true)
                && all_fragments
                    .get(i)
                    .is_some_and(|f| last.right.pairs_with(&f.left))
        });
        match next {
            Some(i) => {
                order.push(i);
                if let Some(u) = used.get_mut(i) {
                    *u = true;
                }
            }
            None => return Err(chain_error(unused_names(&used))),
        }
    }
    let chain_frags: Vec<Fragment> = order
        .iter()
        .filter_map(|i| all_fragments.get(*i).cloned())
        .collect();
    if circular {
        let closes = match (chain_frags.last(), chain_frags.first()) {
            (Some(last), Some(first)) => last.right.pairs_with(&first.left),
            _ => false,
        };
        if !closes {
            return Err(chain_error(
                order
                    .iter()
                    .filter_map(|i| parts.get(*i))
                    .map(|p| p.name())
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }

    let construct_bases = join_fragments(&chain_frags, circular)?;
    let construct_name = order
        .iter()
        .filter_map(|i| parts.get(*i))
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(" + ");

    let mut features: Vec<Feature> = Vec::new();
    let mut offset: usize = 0;
    for f in &chain_frags {
        for feat in &f.features {
            let new_loc = feat.location().shifted(offset as isize)?;
            features.push(feat.with_location(new_loc));
        }
        offset += f.length();
    }

    let mut meta = BTreeMap::new();
    meta.insert("source".to_string(), "clone.goldengate".to_string());

    let seq = Sequence::new(
        &construct_name,
        &construct_name,
        &construct_bases,
        SequenceType::Dna,
    )?;
    let construct = SequenceDocument::new(
        &new_id(),
        &construct_name,
        seq,
        if circular {
            Topology::Circular
        } else {
            Topology::Linear
        },
        features,
        meta,
        now_secs(),
    );

    // Warnung: Erkennungsstelle im Konstrukt
    let rc_rec = iupac_reverse_complement(enzyme.recognition());
    let wrap_len = enzyme.recognition().len().saturating_sub(1);
    let check_bases = if circular {
        format!(
            "{}{}",
            construct_bases,
            construct_bases.get(..wrap_len).unwrap_or("")
        )
    } else {
        construct_bases.clone()
    };

    if check_bases
        .to_ascii_uppercase()
        .contains(enzyme.recognition())
        || check_bases.to_ascii_uppercase().contains(&rc_rec)
    {
        warnings.push(format!(
            "Erkennungsstelle von {} im Konstrukt vorhanden",
            enzyme_name
        ));
    }

    Ok(GoldenGateResult {
        construct,
        parts: chain_frags,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloning::fragment::EndType;

    fn doc(name: &str, bases: &str) -> SequenceDocument {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        SequenceDocument::of(name, seq)
    }

    /// Teil mit BsaI-Rändern: Vorwärtsstelle links, Rückwärtsstelle rechts.
    fn part(name: &str, left: &str, body: &str, right: &str) -> SequenceDocument {
        doc(
            name,
            &format!("TTGGTCTCA{}{}{}TGAGACCTT", left, body, right),
        )
    }

    fn p1() -> SequenceDocument {
        part("p1", "AATG", &"C".repeat(12), "GCTT")
    }
    fn p2() -> SequenceDocument {
        part("p2", "GCTT", &"G".repeat(12), "CGAT")
    }
    fn p3() -> SequenceDocument {
        part("p3", "CGAT", &"T".repeat(12), "AATG")
    }

    fn has_site(c: &SequenceDocument, site: &str) -> bool {
        let b = c.bases();
        let doubled = format!("{}{}", b, &b[..site.len() - 1]);
        doubled.contains(site) || doubled.contains(&iupac_reverse_complement(site))
    }

    #[test]
    fn teil_auswahl() {
        let db = EnzymeDb::standard();
        let d = p1();
        let f = digest(&d, &db, &["BsaI"]).unwrap();
        assert_eq!(f.len(), 3);
        let (g, w) = pick_part_fragment(&d, &f, "GGTCTC");
        assert!(w.is_empty());
        assert_eq!(g.top, format!("AATG{}", "C".repeat(12)));
        assert_eq!(g.left, End::new(EndType::FivePrime, "AATG"));
        assert_eq!(g.right, End::new(EndType::FivePrime, "GCTT"));
        // ohne Schnittstelle
        let kahl = doc("kahl", "ACGTACGT");
        let f = digest(&kahl, &db, &["BsaI"]).unwrap();
        let (g, w) = pick_part_fragment(&kahl, &f, "GGTCTC");
        assert_eq!(g.top, "ACGTACGT");
        assert_eq!(
            (g.left.clone(), g.right.clone()),
            (End::blunt(), End::blunt())
        );
        assert_eq!(
            w,
            vec!["Teil kahl hat weniger als 2 Schnittstellen".to_string()]
        );
    }

    #[test]
    fn zirkulaerer_zusammenbau() {
        let db = EnzymeDb::standard();
        let r = assemble(&db, "BsaI", &[p1(), p2(), p3()], true).unwrap();
        assert_eq!(r.construct.len(), 48);
        assert_eq!(r.construct.topology(), Topology::Circular);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert!(!has_site(&r.construct, "GGTCTC"));
        assert_eq!(r.construct.name(), "p1 + p2 + p3");
        assert_eq!(r.parts.len(), 3);
        assert_eq!(
            r.construct.metadata().get("source").map(String::as_str),
            Some("clone.goldengate")
        );
        assert_eq!(
            r.construct.bases(),
            format!(
                "AATG{}GCTT{}CGAT{}",
                "C".repeat(12),
                "G".repeat(12),
                "T".repeat(12)
            )
        );
        // anderer Anfang: gleiche Länge
        let s = assemble(&db, "BsaI", &[p3(), p1(), p2()], true).unwrap();
        assert_eq!(s.construct.len(), 48);
    }

    #[test]
    fn linearer_zusammenbau_mit_beliebiger_reihenfolge() {
        let db = EnzymeDb::standard();
        let r = assemble(&db, "BsaI", &[p2(), p1()], false).unwrap();
        assert_eq!(r.construct.len(), 32);
        assert_eq!(r.construct.topology(), Topology::Linear);
        assert_eq!(r.construct.name(), "p1 + p2");
        assert!(r.construct.bases().starts_with("AATG"));
    }

    #[test]
    fn doppelter_ueberhang() {
        let db = EnzymeDb::standard();
        let p1b = part("p1b", "CGAT", &"A".repeat(12), "GCTT");
        let r = assemble(&db, "BsaI", &[p1(), p1b, p2()], false).unwrap();
        assert!(
            r.warnings.iter().any(|w| w == "Doppelter Überhang GCTT"),
            "{:?}",
            r.warnings
        );
        assert_eq!(r.construct.len(), 48);
    }

    #[test]
    fn palindromischer_ueberhang() {
        let db = EnzymeDb::standard();
        let p = part("pal", "GATC", &"C".repeat(12), "TTCG");
        let r = assemble(&db, "BsaI", &[p], false).unwrap();
        assert!(
            r.warnings
                .iter()
                .any(|w| w == "Palindromischer Überhang GATC"),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn falsche_orientierung() {
        let db = EnzymeDb::standard();
        // rechtes Ende von q ist AAGC = Reverse-Komplement von GCTT (rechtes Ende von p1); die Kette p1, p2, q geht trotzdem auf
        let q = part("q", "CGAT", &"A".repeat(12), "AAGC");
        let r = assemble(&db, "BsaI", &[p1(), p2(), q], false).unwrap();
        assert!(
            r.warnings
                .iter()
                .any(|m| m == "Überhang GCTT passt zum Reverse-Komplement eines anderen Teils (falsche Orientierung)"),
            "{:?}",
            r.warnings
        );
        assert_eq!(r.construct.len(), 48);
    }

    #[test]
    fn interne_erkennungsstelle() {
        let db = EnzymeDb::standard();
        let body = format!("CC{}A{}{}", "GGTCTC", "TTAC", "CCCC");
        let p = part("intern", "AATG", &body, "GCTT");
        let r = assemble(&db, "BsaI", &[p], false).unwrap();
        assert!(
            r.warnings
                .iter()
                .any(|w| w == "Interne Erkennungsstelle in intern"),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn nicht_verbindbar_und_fehler() {
        let db = EnzymeDb::standard();
        let fremd = part("fremd", "TTTT", &"C".repeat(12), "CCCC");
        match assemble(&db, "BsaI", &[p1(), fremd], false) {
            Err(Error::InvalidInput(m)) => assert!(
                m.starts_with("Teile lassen sich nicht zu einer Kette verbinden"),
                "{}",
                m
            ),
            other => panic!("unerwartet: {:?}", other),
        }
        assert!(matches!(
            assemble(&db, "Nix", &[p1()], false),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            assemble(&db, "BsaI", &[], false),
            Err(Error::InvalidInput(_))
        ));
        // zirkulär geht nicht auf: p1 und p2 schließen sich nicht
        assert!(matches!(
            assemble(&db, "BsaI", &[p1(), p2()], true),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn kein_schnitt_warnung() {
        let db = EnzymeDb::standard();
        let r = assemble(&db, "BsaI", &[doc("kahl", "ACGTACGT")], false).unwrap();
        assert_eq!(r.construct.bases(), "ACGTACGT");
        assert_eq!(
            r.warnings,
            vec!["Teil kahl hat weniger als 2 Schnittstellen".to_string()]
        );
    }

    #[test]
    fn esp3i_wie_bsmbi() {
        let db = EnzymeDb::standard();
        let mk = |name: &str, l: &str, r: &str| {
            doc(
                name,
                &format!("TTCGTCTCA{}{}{}TGAGACGTT", l, "C".repeat(12), r),
            )
        };
        let a = assemble(
            &db,
            "BsmBI",
            &[mk("a", "AATG", "GCTT"), mk("b", "GCTT", "AATG")],
            true,
        )
        .unwrap();
        let b = assemble(
            &db,
            "Esp3I",
            &[mk("a", "AATG", "GCTT"), mk("b", "GCTT", "AATG")],
            true,
        )
        .unwrap();
        assert_eq!(a.construct.bases(), b.construct.bases());
        assert_eq!(a.construct.len(), 32);
    }

    #[test]
    fn features_werden_verschoben() {
        use crate::model::feature::{Interval, Location, Strand};
        let db = EnzymeDb::standard();
        let loc = Location::new(
            vec![Interval::new(13, 17).unwrap()],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        // im Teil p2 liegt das Innenfragment bei 9..25, das Feature 13..17 also bei 4..8 im Fragment
        let p2f = p2().with_features(vec![Feature::new("misc_feature", "f", loc, Vec::new())]);
        let r = assemble(&db, "BsaI", &[p1(), p2f, p3()], true).unwrap();
        let starts: Vec<usize> = r
            .construct
            .features()
            .iter()
            .map(|f| f.location().start())
            .collect();
        assert_eq!(starts, vec![16 + 4]);
    }
}
