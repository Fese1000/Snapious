//! Read GCG sequence files (single sequence).

use crate::error::{Error, Result};

/// Reads a GCG sequence. Looks for the first line that ends (trimmed) with `..`; the name is its first whitespace-separated
/// word. All lines after it are sequence: only letters are kept (line numbers, whitespace etc. are dropped), in
/// uppercase. Result `(name, sequence)`. `Error::InvalidInput` if there is no such line or the sequence is empty.
pub fn parse_gcg(text: &str) -> Result<(String, String)> {
    let mut found_marker = false;
    let mut name = String::new();
    let mut sequence = Vec::new();

    for line in text.lines() {
        if !found_marker {
            let trimmed = line.trim();
            if trimmed.ends_with("..") {
                // Name is the first whitespace-separated word
                if let Some(first_word) = trimmed.split_whitespace().next() {
                    name = first_word.to_string();
                } else {
                    return Err(Error::InvalidInput("No name in marker line".to_string()));
                }
                found_marker = true;
            }
        } else {
            // Keep only letters, ignore everything else (digits, spaces etc.)
            for c in line.chars() {
                if c.is_ascii_alphabetic() {
                    sequence.push(c.to_ascii_uppercase());
                }
            }
        }
    }

    if !found_marker {
        return Err(Error::InvalidInput(
            "No GCG marker line (.. at the end) found".to_string(),
        ));
    }

    if sequence.is_empty() {
        return Err(Error::InvalidInput("Sequence is empty".to_string()));
    }

    Ok((name, sequence.into_iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lesen() {
        let t = "!!NA_SEQUENCE 1.0\n\nsome description\n\n  seq1  Length: 12  Type: N  Check: 123  ..\n\n        1 ACGTACGTAC GT\n";
        assert_eq!(
            parse_gcg(t).unwrap(),
            ("seq1".to_string(), "ACGTACGTACGT".to_string())
        );
        let t2 = "x ..\n 1 acg\n 4 tt\n";
        assert_eq!(
            parse_gcg(t2).unwrap(),
            ("x".to_string(), "ACGTT".to_string())
        );
    }

    #[test]
    fn fehler() {
        assert!(parse_gcg("ACGT\n").is_err());
        assert!(parse_gcg("name ..\n   1 \n").is_err());
        assert!(parse_gcg("").is_err());
    }
}
