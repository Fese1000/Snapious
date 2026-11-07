//! Read codon tables: Kazusa text format and a simple JSON representation.

use crate::bio::codon_optimize::CodonTable;
use crate::bio::codon_usage::CodonCounts;
use crate::error::{Error, Result};
use serde_json;

/// Reads the text representation of the Kazusa codon usage table ("Codon Usage Database").
/// Each line contains several entries of the form `UUU 17.1( 1234)` (codon with `U` instead of `T`, frequency per thousand,
/// absolute count in parentheses, the count may contain spaces inside the parentheses), separated by whitespace.
/// For each entry, codon and count are read; codons are stored in uppercase with `T` instead of `U`.
/// Lines without such entries (header lines, blank lines) are ignored.
/// Error `Error::InvalidInput` if no entry at all is found or a count is not an integer.
pub fn parse_kazusa(text: &str) -> Result<CodonCounts> {
    let mut counts = CodonCounts::new();
    for line in text.lines() {
        let mut i = 0;
        let bytes = line.as_bytes();
        while i < bytes.len() {
            if bytes[i].is_ascii_uppercase()
                && i + 2 < bytes.len()
                && bytes[i + 1].is_ascii_uppercase()
                && bytes[i + 2].is_ascii_uppercase()
            {
                let c1 = bytes[i] as char;
                let c2 = bytes[i + 1] as char;
                let c3 = bytes[i + 2] as char;
                let mut j = i + 3;
                while j < bytes.len() && bytes[j] != b'(' {
                    j += 1;
                }
                if j < bytes.len() {
                    let mut k = j + 1;
                    while k < bytes.len() && bytes[k] != b')' {
                        k += 1;
                    }
                    if k < bytes.len() {
                        let count_str = &line[j + 1..k];
                        match count_str.trim().parse::<usize>() {
                            Ok(count) => {
                                let codon: String = [c1, c2, c3]
                                    .iter()
                                    .map(|&ch| if ch == 'U' { 'T' } else { ch })
                                    .collect();
                                if (b'A'..=b'T').contains(&codon.as_bytes()[0])
                                    && (b'A'..=b'T').contains(&codon.as_bytes()[1])
                                    && (b'A'..=b'T').contains(&codon.as_bytes()[2])
                                {
                                    counts.insert(codon, count);
                                }
                            }
                            Err(_) => return Err(Error::InvalidInput("Invalid count".into())),
                        }
                    }
                }
            }
            i += 1;
        }
    }
    if counts.is_empty() {
        return Err(Error::InvalidInput(
            "No valid entries found".into(),
        ));
    }
    Ok(counts)
}

/// Reads a codon table from JSON of the form `{"organism": "...", "codons": {"ATG": 1.0, "GCT": 0.18}}`
/// (frequencies as in `CodonTable::from_frequencies`; `organism` is optional). Error `Error::InvalidInput` on invalid
/// JSON, missing `codons` or invalid codons/frequencies (pass on the core function's message).
pub fn table_from_json(json: &str) -> Result<CodonTable> {
    let val: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| Error::InvalidInput(format!("Invalid JSON: {}", e)))?;
    let codons_obj = val
        .get("codons")
        .ok_or_else(|| Error::InvalidInput("Missing field 'codons'".into()))?;
    let obj = codons_obj
        .as_object()
        .ok_or_else(|| Error::InvalidInput("Field 'codons' is not an object".into()))?;
    let pairs: Vec<(&str, f64)> = obj
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_f64().unwrap_or(f64::NAN)))
        .collect();
    CodonTable::from_frequencies(&pairs)
}

/// Writes counts as JSON for `table_from_json`: `{"organism": <name>, "codons": {...}}` with the relative frequencies
/// within the amino acid family (`CodonTable::from_counts`, then `frequency` per codon), codons sorted alphabetically,
/// frequencies rounded to 4 decimal places. All 64 codons are written.
pub fn counts_to_json(organism: &str, counts: &CodonCounts) -> Result<String> {
    let table = CodonTable::from_counts(counts);
    let letters = ['A', 'C', 'G', 'T'];
    let mut codons: Vec<String> = Vec::with_capacity(64);
    for &a in &letters {
        for &b in &letters {
            for &c in &letters {
                codons.push([a, b, c].iter().collect::<String>());
            }
        }
    }
    let mut codon_map = serde_json::Map::new();
    for c in &codons {
        let freq = table.frequency(c);
        let rounded = (freq * 10000.0).round() / 10000.0;
        codon_map.insert(c.clone(), serde_json::Value::from(rounded));
    }
    let mut root = serde_json::Map::new();
    root.insert("organism".to_string(), serde_json::Value::from(organism));
    root.insert("codons".to_string(), serde_json::Value::Object(codon_map));
    serde_json::to_string(&root).map_err(|e| Error::Io(format!("JSON: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Escherichia coli [gbbct]: 3 CDS's
UUU 22.1(  3) UCU  8.6(  1) UAU 16.3( 12) UGU  5.2(100)
UUC 16.6(  2) UCC  8.6(  9) UAC 12.2(  0) UGC  6.5(  7)
";

    #[test]
    fn kazusa_lesen() {
        let c = parse_kazusa(SAMPLE).unwrap();
        assert_eq!(c.len(), 8);
        assert_eq!(c["TTT"], 3);
        assert_eq!(c["TCT"], 1);
        assert_eq!(c["TAT"], 12);
        assert_eq!(c["TGT"], 100);
        assert_eq!(c["TAC"], 0);
        assert_eq!(c["TGC"], 7);
    }

    #[test]
    fn kazusa_fehler() {
        assert!(matches!(
            parse_kazusa("nur Text\n\n"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(parse_kazusa(""), Err(Error::InvalidInput(_))));
    }

    #[test]
    fn json_lesen() {
        let t = table_from_json(r#"{"organism":"x","codons":{"ATG":1.0,"GCT":0.25,"GCC":0.75}}"#)
            .unwrap();
        assert_eq!(t.frequency("ATG"), 1.0);
        assert_eq!(t.frequency("GCC"), 0.75);
        assert_eq!(t.frequency("AAA"), 0.0);
        assert!(table_from_json("{}").is_err());
        assert!(table_from_json("kein json").is_err());
        assert!(table_from_json(r#"{"codons":{"AT":1.0}}"#).is_err());
        assert!(table_from_json(r#"{"codons":{"ATG":-1.0}}"#).is_err());
    }

    #[test]
    fn rundlauf_ueber_zaehlungen() {
        let mut counts = CodonCounts::new();
        counts.insert("GCT".to_string(), 1);
        counts.insert("GCC".to_string(), 3);
        counts.insert("ATG".to_string(), 5);
        let json = counts_to_json("Testorganismus", &counts).unwrap();
        assert!(json.contains("Testorganismus"));
        let t = table_from_json(&json).unwrap();
        assert_eq!(t.frequency("GCT"), 0.25);
        assert_eq!(t.frequency("GCC"), 0.75);
        assert_eq!(t.frequency("ATG"), 1.0);
        assert_eq!(t.frequency("GCA"), 0.0);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["codons"].as_object().unwrap().len(), 64);
    }
}
