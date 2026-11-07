//! Built-in codon tables (Kazusa Codon Usage Database), embedded from `data/codon/*.json`.

use crate::bio::codon_optimize::CodonTable;
use crate::bio::kazusa::table_from_json;
use crate::error::{Error, Result};

/// Names of the built-in tables, alphabetical: `ecoli`, `human`, `mouse`, `yeast`.
pub fn builtin_names() -> Vec<&'static str> {
    vec!["ecoli", "human", "mouse", "yeast"]
}

/// Loads a built-in table. The JSON texts come via `include_str!("../../data/codon/<name>.json")` (path relative to this file; the
/// files are in `crates/snapious-core/data/codon/`). Name is case-insensitive and ignores surrounding whitespace.
/// Unknown name → `Error::InvalidInput` with the name and the list of valid names. Parse errors are propagated.
pub fn builtin_table(name: &str) -> Result<CodonTable> {
    let normalized = name.trim().to_lowercase();
    match normalized.as_str() {
        "ecoli" => table_from_json(include_str!("../../data/codon/ecoli.json")),
        "human" => table_from_json(include_str!("../../data/codon/human.json")),
        "mouse" => table_from_json(include_str!("../../data/codon/mouse.json")),
        "yeast" => table_from_json(include_str!("../../data/codon/yeast.json")),
        _ => Err(Error::InvalidInput(format!(
            "Unknown table '{}'. Valid names: {}",
            name,
            builtin_names().join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namen() {
        assert_eq!(builtin_names(), vec!["ecoli", "human", "mouse", "yeast"]);
    }

    #[test]
    fn tabellen_laden() {
        for n in builtin_names() {
            let t = builtin_table(n).unwrap();
            assert_eq!(t.frequency("ATG"), 1.0, "{}", n);
            assert_eq!(t.frequency("TGG"), 1.0, "{}", n);
            // all codons of an amino acid sum to roughly 1 (leucine: 6 codons)
            let leu: f64 = ["TTA", "TTG", "CTT", "CTC", "CTA", "CTG"]
                .iter()
                .map(|c| t.frequency(c))
                .sum();
            assert!((leu - 1.0).abs() < 0.03, "{} {}", n, leu);
        }
        assert!((builtin_table("ecoli").unwrap().frequency("GCT") - 0.16).abs() < 1e-9);
        assert!((builtin_table("human").unwrap().frequency("GCT") - 0.27).abs() < 1e-9);
        assert!((builtin_table("yeast").unwrap().frequency("GCT") - 0.38).abs() < 1e-9);
        assert!((builtin_table("mouse").unwrap().frequency("GCT") - 0.29).abs() < 1e-9);
        // E. coli prefers CTG for leucine
        let e = builtin_table(" EColi ").unwrap();
        assert!(e.frequency("CTG") > 0.4);
    }

    #[test]
    fn unbekannt() {
        let e = builtin_table("hefe").unwrap_err();
        assert!(matches!(e, Error::InvalidInput(_)));
        assert!(e.to_string().contains("hefe"));
        assert!(e.to_string().contains("ecoli"));
    }
}
