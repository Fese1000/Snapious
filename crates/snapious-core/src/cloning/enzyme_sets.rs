//! Enzymsets: benannte Gruppen von Restriktionsenzymen (für Verdau und Klonierung).

use std::collections::HashSet;

/// Ein benanntes Set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnzymeSet {
    pub name: String,
    pub enzymes: Vec<String>,
}

/// Eingebaute Sets in dieser Reihenfolge:
/// 1. `Häufige Klonierung`: EcoRI, BamHI, HindIII, XbaI, SpeI, PstI, NotI, XhoI, NcoI, NdeI
/// 2. `Golden Gate (Typ IIS)`: BsaI, BsmBI, BbsI, SapI, Esp3I
/// 3. `Glatte Enden`: EcoRV, SmaI, PvuII, HpaI, ScaI
pub fn builtin_sets() -> Vec<EnzymeSet> {
    vec![
        EnzymeSet {
            name: "Häufige Klonierung".to_string(),
            enzymes: vec![
                "EcoRI".into(),
                "BamHI".into(),
                "HindIII".into(),
                "XbaI".into(),
                "SpeI".into(),
                "PstI".into(),
                "NotI".into(),
                "XhoI".into(),
                "NcoI".into(),
                "NdeI".into(),
            ],
        },
        EnzymeSet {
            name: "Golden Gate (Typ IIS)".to_string(),
            enzymes: vec![
                "BsaI".into(),
                "BsmBI".into(),
                "BbsI".into(),
                "SapI".into(),
                "Esp3I".into(),
            ],
        },
        EnzymeSet {
            name: "Glatte Enden".to_string(),
            enzymes: vec![
                "EcoRV".into(),
                "SmaI".into(),
                "PvuII".into(),
                "HpaI".into(),
                "ScaI".into(),
            ],
        },
    ]
}

/// Zerlegt einen Text in Enzymnamen: getrennt an Komma, Semikolon und Leerraum; leere Teile entfallen; doppelte Namen
/// (Groß-/Kleinschreibung egal) nur beim ersten Vorkommen, in der Schreibweise des ersten.
pub fn parse_enzyme_list(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for part in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if seen.insert(lower) {
            result.push(trimmed.to_string());
        }
    }
    result
}

/// Fügt Namen mit `", "` zusammen (Umkehrung von `parse_enzyme_list` für saubere Listen).
pub fn format_enzyme_list(enzymes: &[String]) -> String {
    enzymes.join(", ")
}

/// Vereinigung mehrerer Sets: Namen in der Reihenfolge des ersten Auftretens, doppelte (Groß-/Kleinschreibung egal) nur einmal.
pub fn union_sets(sets: &[EnzymeSet]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for set in sets {
        for enzyme in &set.enzymes {
            let lower = enzyme.to_lowercase();
            if seen.insert(lower) {
                result.push(enzyme.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebaut() {
        let s = builtin_sets();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].name, "Häufige Klonierung");
        assert_eq!(s[0].enzymes.len(), 10);
        assert_eq!(s[1].enzymes, vec!["BsaI", "BsmBI", "BbsI", "SapI", "Esp3I"]);
        assert_eq!(s[2].enzymes[0], "EcoRV");
    }

    #[test]
    fn lesen_schreiben_vereinen() {
        assert_eq!(
            parse_enzyme_list("EcoRI, bamHI;  EcoRI\nHindIII,,"),
            vec!["EcoRI", "bamHI", "HindIII"]
        );
        assert!(parse_enzyme_list("  ,; ").is_empty());
        let l = vec!["EcoRI".to_string(), "BamHI".to_string()];
        assert_eq!(format_enzyme_list(&l), "EcoRI, BamHI");
        assert_eq!(format_enzyme_list(&[]), "");
        let a = EnzymeSet {
            name: "a".into(),
            enzymes: vec!["EcoRI".into(), "BamHI".into()],
        };
        let b = EnzymeSet {
            name: "b".into(),
            enzymes: vec!["bamhi".into(), "XbaI".into()],
        };
        assert_eq!(union_sets(&[a, b]), vec!["EcoRI", "BamHI", "XbaI"]);
    }
}
