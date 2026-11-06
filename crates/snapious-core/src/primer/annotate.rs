//! Primerbindestellen als Features in eine Sequenz eintragen.

use crate::cloning::pcr::{PcrSimulator, Primer};
use crate::error::Result;
use crate::model::feature::{Feature, Interval, Location, Strand};

/// Sucht jeden Primer mit `PcrSimulator::find_bindings` auf der Vorlage und macht daraus Features.
///
/// - Typ `primer_bind`, Name = Primername.
/// - Strang: `Forward` für Vorwärtsbindungen, `Reverse` für Rückwärtsbindungen.
/// - Ort: `[start, end)` der Bindung. Ragt bei einer zirkulären Vorlage das Ende über die Länge hinaus (`end > Länge`), entstehen zwei
///   Intervalle: `[start, Länge)` und `[0, end − Länge)` in dieser Reihenfolge (`Location::new` mit beiden Intervallen).
///   `partial_start` und `partial_end` sind falsch.
/// - Qualifier (in dieser Reihenfolge): `("note", ["Abweichungen: {n}"])` und, wenn der Primer einen nicht bindenden 5'-Schwanz hat
///   (`tail_length > 0`), zusätzlich `("note", ["5'-Schwanz: {n} nt"])` — beide Einträge heißen `note`.
/// - Ergebnis sortiert nach `Location::start()`, dann nach Name, dann Vorwärts vor Rückwärts; Primer in der Reihenfolge der Eingabe.
///   Jede Bindung ergibt ein Feature.
pub fn annotate_primers(
    template: &str,
    circular: bool,
    primers: &[Primer],
    sim: &PcrSimulator,
) -> Result<Vec<Feature>> {
    let len = template.chars().count();
    let mut found: Vec<(usize, String, bool, Feature)> = Vec::new();
    for primer in primers {
        for b in sim.find_bindings(template, circular, primer) {
            let strand = if b.forward {
                Strand::Forward
            } else {
                Strand::Reverse
            };
            let intervals = if circular && b.end > len {
                vec![Interval::new(b.start, len)?, Interval::new(0, b.end - len)?]
            } else {
                vec![Interval::new(b.start, b.end)?]
            };
            let location = Location::new(intervals, strand, false, false)?;
            let mut qualifiers = vec![(
                "note".to_string(),
                vec![format!("Abweichungen: {}", b.mismatches)],
            )];
            if b.tail_length > 0 {
                qualifiers.push((
                    "note".to_string(),
                    vec![format!("5'-Schwanz: {} nt", b.tail_length)],
                ));
            }
            let feature = Feature::new("primer_bind", &primer.name, location, qualifiers);
            found.push((b.start, primer.name.clone(), !b.forward, feature));
        }
    }
    found.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));
    Ok(found.into_iter().map(|f| f.3).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloning::enzymes::iupac_reverse_complement;

    const T: &str = "ATGCCGTAGCTAGCATCGATCGGATCCGATCGATCGTAGCTAGCTAGGCTAGC";

    fn primer(name: &str, seq: &str) -> Primer {
        Primer {
            name: name.to_string(),
            sequence: seq.to_string(),
        }
    }

    fn span(f: &Feature) -> (usize, usize, Strand) {
        (
            f.location().start(),
            f.location().end(),
            f.location().strand(),
        )
    }

    #[test]
    fn vorwaerts_und_rueckwaerts() {
        let f = primer("fwd", &T[..14]);
        let r = primer("rev", &iupac_reverse_complement(&T[39..]));
        let feats = annotate_primers(T, false, &[f, r], &PcrSimulator::default()).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].feature_type(), "primer_bind");
        assert_eq!(feats[0].name(), "fwd");
        assert_eq!(span(&feats[0]), (0, 14, Strand::Forward));
        assert_eq!(feats[1].name(), "rev");
        assert_eq!(span(&feats[1]), (39, 53, Strand::Reverse));
        assert_eq!(feats[0].qualifiers().len(), 1);
        assert_eq!(
            feats[0].qualifiers()[0],
            ("note".to_string(), vec!["Abweichungen: 0".to_string()])
        );
    }

    #[test]
    fn schwanz_und_abweichung() {
        let p = primer("mitSchwanz", &format!("GAATTC{}", &T[..14]));
        let feats = annotate_primers(T, false, &[p], &PcrSimulator::default()).unwrap();
        assert_eq!(feats.len(), 1);
        assert_eq!(span(&feats[0]), (0, 14, Strand::Forward));
        assert_eq!(feats[0].qualifiers().len(), 2);
        assert_eq!(
            feats[0].qualifiers()[1],
            ("note".to_string(), vec!["5'-Schwanz: 6 nt".to_string()])
        );
        // Abweichung an Position 1: mit max 1 gebunden
        let mut s: Vec<char> = T[..14].chars().collect();
        s[1] = 'A';
        let s: String = s.into_iter().collect();
        let sim = PcrSimulator::new(12, 1, 10000);
        let feats = annotate_primers(T, false, &[primer("mm", &s)], &sim).unwrap();
        assert_eq!(
            feats[0].qualifiers()[0].1,
            vec!["Abweichungen: 1".to_string()]
        );
    }

    #[test]
    fn ueber_den_nullpunkt() {
        let tt = format!("{}{}", T, T);
        let p = primer("rund", &tt[47..61]);
        let feats = annotate_primers(T, true, &[p], &PcrSimulator::default()).unwrap();
        assert_eq!(feats.len(), 1);
        let loc = feats[0].location();
        assert_eq!(loc.intervals().len(), 2);
        assert_eq!(
            (loc.intervals()[0].start(), loc.intervals()[0].end()),
            (47, 53)
        );
        assert_eq!(
            (loc.intervals()[1].start(), loc.intervals()[1].end()),
            (0, 8)
        );
        assert_eq!(loc.strand(), Strand::Forward);
    }

    #[test]
    fn nichts_gefunden_und_sortierung() {
        assert!(annotate_primers(
            T,
            false,
            &[primer("x", "GGGGGGGGGGGGGGGG")],
            &PcrSimulator::default()
        )
        .unwrap()
        .is_empty());
        assert!(annotate_primers(T, false, &[], &PcrSimulator::default())
            .unwrap()
            .is_empty());
        let a = primer("b", &T[20..40]);
        let b = primer("a", &T[..14]);
        let feats = annotate_primers(T, false, &[a, b], &PcrSimulator::default()).unwrap();
        let names: Vec<&str> = feats.iter().map(|f| f.name()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }
}
