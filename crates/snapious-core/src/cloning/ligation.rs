//! Restriktionsklonierung: Fragmente verbinden und mögliche Konstrukte aus Vektor und Insert bilden.

use std::collections::{BTreeMap, HashSet};

use crate::cloning::enzymes::EnzymeDb;
use crate::cloning::fragment::{digest, Fragment};
use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::sequence::{Sequence, SequenceType};
use crate::util::{new_id, now_secs};

/// Ein mögliches Klonierungsprodukt.
#[derive(Clone, Debug, PartialEq)]
pub struct CloningCandidate {
    /// Das Vektor-Rückgrat (größtes Fragment des Vektorverdaus).
    pub vector: Fragment,
    /// Das Insert-Fragment in der Orientierung, in der es eingebaut ist.
    pub insert: Fragment,
    /// `true`, wenn das Insert reverse-komplementiert eingebaut wurde.
    pub insert_reversed: bool,
    /// `true`, wenn die beiden Vektorenden verschieden sind (dann ist nur eine Orientierung möglich).
    pub directional: bool,
    /// Zirkuläres Konstrukt.
    pub construct: SequenceDocument,
    pub warnings: Vec<String>,
}

/// Verbindet Fragmente in der gegebenen Reihenfolge zu einem Oberstrang: Summe der `top`-Texte.
/// - Leere Liste: `Error::InvalidInput`.
/// - Für jedes benachbarte Paar muss `fragments[i].can_join(&fragments[i+1])` gelten, sonst `Error::InvalidInput`
///   (Meldung nennt beide Nummern 1-basiert: `Fragmente {i} und {i+1} passen nicht zusammen`).
/// - Ist `circular`, muss zusätzlich das letzte Fragment ans erste passen (bei einem einzelnen Fragment an sich selbst),
///   sonst `Error::InvalidInput`.
pub fn join_fragments(fragments: &[Fragment], circular: bool) -> Result<String> {
    if fragments.is_empty() {
        return Err(Error::InvalidInput("Leere Fragmentliste".to_string()));
    }

    let mut result = String::new();
    for (i, frag) in fragments.iter().enumerate() {
        result.push_str(&frag.top);

        if i < fragments.len() - 1 {
            let next = fragments
                .get(i + 1)
                .ok_or_else(|| Error::InvalidInput("Logikfehler".to_string()))?;
            if !frag.can_join(next) {
                return Err(Error::InvalidInput(format!(
                    "Fragmente {} und {} passen nicht zusammen",
                    i + 1,
                    i + 2
                )));
            }
        }
    }

    if circular {
        let last = fragments
            .last()
            .ok_or_else(|| Error::InvalidInput("Logikfehler: leere Liste".to_string()))?;
        let first = fragments
            .first()
            .ok_or_else(|| Error::InvalidInput("Logikfehler".to_string()))?;
        if !last.can_join(first) {
            return Err(Error::InvalidInput(format!(
                "Fragmente {} und {} passen nicht zusammen",
                fragments.len(),
                1
            )));
        }
    }

    Ok(result)
}

/// Alle zirkulären Produkte der Restriktionsklonierung.
///
/// 1. `digest(vector, db, vector_enzymes)` und `digest(insert, db, insert_enzymes)` (Fehler werden weitergegeben).
/// 2. Wurde der Vektor nicht geschnitten (genau ein Fragment mit `source_start == 0` und `source_end == Länge des Vektors`),
///    gibt es keine Kandidaten (leere Liste). Rückgrat `V` ist das längste Vektorfragment (bei Gleichstand das erste).
/// 3. Für jedes Insert-Fragment `I` und beide Orientierungen (`I`, dann `I.reverse_complement()`) entsteht ein Kandidat, wenn
///    `V.can_join(&I')` und `I'.can_join(&V)` gelten.
/// 4. `directional = V.left != V.right`.
/// 5. Konstrukt: Oberstrang `V.top + I'.top`; Name `"{Vektorname} + {Insertname}"`; neue ID (`new_id()`), `Topology::Circular`;
///    Sequenz vom Typ DNA (Id und Name der Sequenz gleich dem Dokumentnamen); Features: die von `V` und die von `I'`,
///    letztere um `V.length()` nach rechts verschoben (`Location::shifted`, `Feature::with_location`);
///    Metadaten `source = "clone.restriction"`; Erstellzeit `now_secs()`.
/// 6. Warnungen (in dieser Reihenfolge):
///    - `Insert-Orientierung nicht gerichtet (beide Orientierungen möglich)`, wenn für dasselbe Insert-Fragment beide
///      Orientierungen einen Kandidaten ergeben haben (steht an beiden Kandidaten);
///    - `Vektor-Rückligation möglich`, wenn `V.right.pairs_with(&V.left)`.
/// 7. Sortierung: gerichtete zuerst, dann weniger Warnungen, dann kürzeres Konstrukt, dann nicht umgekehrt vor umgekehrt,
///    dann kleinerer `source_start` des Inserts.
pub fn restriction_candidates(
    vector: &SequenceDocument,
    vector_enzymes: &[&str],
    insert: &SequenceDocument,
    insert_enzymes: &[&str],
    db: &EnzymeDb,
) -> Result<Vec<CloningCandidate>> {
    let vector_frags = digest(vector, db, vector_enzymes)?;
    let insert_frags = digest(insert, db, insert_enzymes)?;

    // Schritt 2: Vektor-Prüfung
    let vec_len = vector.len();
    if vector_frags.len() == 1 {
        let first_frag = vector_frags
            .first()
            .ok_or_else(|| Error::InvalidInput("Logikfehler".to_string()))?;
        if first_frag.source_start == 0 && first_frag.source_end == vec_len {
            return Ok(Vec::new());
        }
    }

    // Längstes Vektorfragment finden (bei Gleichstand erstes)
    let mut v_idx = 0;
    let mut v_len = 0;
    for (i, frag) in vector_frags.iter().enumerate() {
        let l = frag.length();
        if l > v_len {
            v_len = l;
            v_idx = i;
        }
    }
    let v_frag = vector_frags
        .get(v_idx)
        .ok_or_else(|| Error::InvalidInput("Logikfehler: kein Vektorfragment".to_string()))?;

    // Schritt 3: Kandidaten generieren
    let mut raw_candidates: Vec<(usize, Fragment, bool)> = Vec::new();

    for (i_frag, base_i) in insert_frags.iter().enumerate() {
        // Orientierung 1: normal
        let i_prime = base_i.clone();
        if v_frag.can_join(&i_prime) && i_prime.can_join(v_frag) {
            raw_candidates.push((i_frag, i_prime.clone(), false));
        }

        // Orientierung 2: reverse-komplementiert
        let i_rev = base_i.reverse_complement()?;
        if v_frag.can_join(&i_rev) && i_rev.can_join(v_frag) {
            raw_candidates.push((i_frag, i_rev.clone(), true));
        }
    }

    if raw_candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Schritt 6: Warnungen bestimmen
    let mut insert_indices_with_both = HashSet::new();
    for (idx, _, reversed) in &raw_candidates {
        let has_other = raw_candidates
            .iter()
            .any(|(i, _, r)| *i == *idx && *r != *reversed);
        if has_other {
            insert_indices_with_both.insert(*idx);
        }
    }

    let mut candidates = Vec::new();

    for (idx, frag, reversed) in &raw_candidates {
        let mut warnings_list = Vec::new();
        if insert_indices_with_both.contains(idx) {
            warnings_list.push(
                "Insert-Orientierung nicht gerichtet (beide Orientierungen möglich)".to_string(),
            );
        }
        if v_frag.right.pairs_with(&v_frag.left) {
            warnings_list.push("Vektor-Rückligation möglich".to_string());
        }

        let construct_bases = join_fragments(&[v_frag.clone(), frag.clone()], true)?;
        let construct_name = format!("{} + {}", vector.name(), insert.name());

        let construct_seq = Sequence::new(
            &construct_name,
            &construct_name,
            &construct_bases,
            SequenceType::Dna,
        )?;

        let mut construct_features = v_frag.features.clone();
        for f in &frag.features {
            let shifted_loc = f.location().shifted(v_frag.length() as isize)?;
            construct_features.push(f.with_location(shifted_loc));
        }

        let mut metadata = BTreeMap::new();
        metadata.insert("source".to_string(), "clone.restriction".to_string());

        let construct_doc = SequenceDocument::new(
            &new_id(),
            &construct_name,
            construct_seq,
            Topology::Circular,
            construct_features,
            metadata,
            now_secs(),
        );

        candidates.push(CloningCandidate {
            vector: v_frag.clone(),
            insert: frag.clone(),
            insert_reversed: *reversed,
            directional: v_frag.left != v_frag.right,
            construct: construct_doc,
            warnings: warnings_list,
        });
    }

    // Schritt 7: Sortierung
    candidates.sort_by(|a, b| {
        a.directional
            .cmp(&b.directional)
            .reverse() // gerichtete zuerst (true > false)
            .then_with(|| a.warnings.len().cmp(&b.warnings.len())) // weniger Warnungen zuerst
            .then_with(|| a.construct.len().cmp(&b.construct.len())) // kürzeres Konstrukt zuerst
            .then_with(|| a.insert_reversed.cmp(&b.insert_reversed)) // nicht umgekehrt zuerst (false < true)
            .then_with(|| a.insert.source_start.cmp(&b.insert.source_start)) // kleinerer source_start zuerst
    });

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Feature, Interval, Location, Strand};

    fn doc(name: &str, bases: &str, topology: Topology) -> SequenceDocument {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        SequenceDocument::of(name, seq).with_topology(topology)
    }

    fn feat(start: usize, end: usize) -> Feature {
        let loc = Location::new(
            vec![Interval::new(start, end).unwrap()],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        Feature::new("gene", "g", loc, Vec::new())
    }

    fn circular_contains(c: &SequenceDocument, site: &str) -> bool {
        let b = c.bases();
        format!("{}{}", b, &b[..site.len() - 1]).contains(site)
    }

    fn mcs_vector() -> SequenceDocument {
        let b = format!(
            "{}GGATCC{}GAATTC{}",
            "A".repeat(10),
            "T".repeat(8),
            "C".repeat(10)
        );
        doc("vec", &b, Topology::Circular).with_features(vec![feat(31, 35)])
    }

    fn directional_insert() -> SequenceDocument {
        doc("ins", "CCGGATCCACGTACGTGAATTCGG", Topology::Linear).with_features(vec![feat(5, 9)])
    }

    #[test]
    fn fragmente_verbinden() {
        let d = doc("s", "AAGGAATTCGATTAGCTAGC", Topology::Linear);
        let f = digest(&d, &EnzymeDb::standard(), &["EcoRI"]).unwrap();
        assert_eq!(join_fragments(&f, false).unwrap(), "AAGGAATTCGATTAGCTAGC");
        // f0 an f0: rechtes Ende AATT passt nicht an das stumpfe linke Ende
        let doppelt = vec![f[0].clone(), f[0].clone()];
        let msg = match join_fragments(&doppelt, false) {
            Err(Error::InvalidInput(m)) => m,
            other => panic!("unerwartet: {:?}", other),
        };
        assert!(msg.contains('1') && msg.contains('2'));
        assert!(matches!(
            join_fragments(&[], false),
            Err(Error::InvalidInput(_))
        ));
        // zirkulär: ein einzelnes Fragment mit Enden AATT/stumpf passt nicht an sich selbst
        assert!(matches!(
            join_fragments(&f[..1], true),
            Err(Error::InvalidInput(_))
        ));
        // beide Fragmente zirkulär: Ende (stumpf) passt an Anfang (stumpf)
        assert_eq!(join_fragments(&f, true).unwrap(), "AAGGAATTCGATTAGCTAGC");
    }

    #[test]
    fn gerichtete_klonierung() {
        let db = EnzymeDb::standard();
        let c = restriction_candidates(
            &mcs_vector(),
            &["BamHI", "EcoRI"],
            &directional_insert(),
            &["BamHI", "EcoRI"],
            &db,
        )
        .unwrap();
        assert_eq!(c.len(), 1);
        let k = &c[0];
        assert!(k.directional);
        assert!(!k.insert_reversed);
        assert!(k.warnings.is_empty());
        assert_eq!(k.vector.length(), 26);
        assert_eq!(k.insert.length(), 14);
        assert_eq!(k.construct.len(), 40);
        assert_eq!(k.construct.topology(), Topology::Circular);
        assert_eq!(k.construct.name(), "vec + ins");
        assert_eq!(
            k.construct.metadata().get("source").map(String::as_str),
            Some("clone.restriction")
        );
        assert!(circular_contains(&k.construct, "GAATTC"));
        assert!(circular_contains(&k.construct, "GGATCC"));
        // Features: Vektor 31..35 liegt im Rückgrat (ab 25) → 6..10; Insert 5..9 liegt im Fragment (ab 3) → 2..6, plus 26
        let mut starts: Vec<usize> = k
            .construct
            .features()
            .iter()
            .map(|f| f.location().start())
            .collect();
        starts.sort();
        assert_eq!(starts, vec![6, 28]);
        assert_eq!(
            k.construct.bases(),
            format!("{}{}", k.vector.top, k.insert.top)
        );
    }

    #[test]
    fn ein_enzym_zwei_orientierungen() {
        let db = EnzymeDb::standard();
        let vec = doc(
            "vec",
            &format!("{}GAATTC{}", "A".repeat(20), "C".repeat(14)),
            Topology::Circular,
        );
        let ins = doc("ins", "CCGAATTCACGTACGTGAATTCGG", Topology::Linear);
        let c = restriction_candidates(&vec, &["EcoRI"], &ins, &["EcoRI"], &db).unwrap();
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|k| !k.directional));
        assert!(c.iter().all(|k| k.construct.len() == 54));
        assert!(c.iter().all(|k| k.warnings
            == vec![
                "Insert-Orientierung nicht gerichtet (beide Orientierungen möglich)".to_string(),
                "Vektor-Rückligation möglich".to_string()
            ]));
        assert!(!c[0].insert_reversed && c[1].insert_reversed);
    }

    #[test]
    fn hybridstellen_bamhi_bglii() {
        let db = EnzymeDb::standard();
        let vec = doc(
            "vec",
            &format!("{}GGATCC{}", "A".repeat(10), "C".repeat(14)),
            Topology::Circular,
        );
        let ins = doc("ins", "TTAGATCTACGTACGTAGATCTTT", Topology::Linear);
        let c = restriction_candidates(&vec, &["BamHI"], &ins, &["BglII"], &db).unwrap();
        assert_eq!(c.len(), 2);
        let vorwaerts = c.iter().find(|k| !k.insert_reversed).unwrap();
        assert!(circular_contains(&vorwaerts.construct, "GGATCT"));
        assert!(circular_contains(&vorwaerts.construct, "AGATCC"));
        assert!(!circular_contains(&vorwaerts.construct, "GGATCC"));
    }

    #[test]
    fn unvertraegliche_enden_und_ungeschnittener_vektor() {
        let db = EnzymeDb::standard();
        let vec = doc(
            "vec",
            &format!("{}GAATTC{}", "A".repeat(20), "C".repeat(14)),
            Topology::Circular,
        );
        let ins = doc("ins", "CCGGATCCACGTACGTGGATCCGG", Topology::Linear);
        assert!(
            restriction_candidates(&vec, &["EcoRI"], &ins, &["BamHI"], &db)
                .unwrap()
                .is_empty()
        );
        // Vektor ohne Schnittstelle des gewählten Enzyms
        assert!(
            restriction_candidates(&vec, &["BamHI"], &ins, &["BamHI"], &db)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            restriction_candidates(&vec, &["Nix"], &ins, &["BamHI"], &db),
            Err(Error::InvalidInput(_))
        ));
    }
}
