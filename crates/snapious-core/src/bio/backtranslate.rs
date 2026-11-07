//! Back-translation: protein → DNA (preferred codons or ambiguity codes).

use crate::error::{Error, Result};

/// Back-translation with one fixed codon per amino acid (most frequent codon of E. coli):
/// A GCG, R CGT, N AAC, D GAT, C TGC, Q CAG, E GAA, G GGC, H CAT, I ATT, L CTG, K AAA, M ATG, F TTT, P CCG, S AGC, T ACC,
/// W TGG, Y TAT, V GTG, stop `*` TAA. Case-insensitive. Empty input → "".
/// Error `Error::InvalidInput` for any other character.
pub fn back_translate_preferred(protein: &str) -> Result<String> {
    let mut dna = String::with_capacity(protein.len() * 3);
    for c in protein.chars().map(|ch| ch.to_ascii_uppercase()) {
        let codon = match c {
            'A' => "GCG",
            'R' => "CGT",
            'N' => "AAC",
            'D' => "GAT",
            'C' => "TGC",
            'Q' => "CAG",
            'E' => "GAA",
            'G' => "GGC",
            'H' => "CAT",
            'I' => "ATT",
            'L' => "CTG",
            'K' => "AAA",
            'M' => "ATG",
            'F' => "TTT",
            'P' => "CCG",
            'S' => "AGC",
            'T' => "ACC",
            'W' => "TGG",
            'Y' => "TAT",
            'V' => "GTG",
            '*' => "TAA",
            _ => return Err(Error::InvalidInput(format!("Unknown character: '{}'", c))),
        };
        dna.push_str(codon);
    }
    Ok(dna)
}

/// Back-translation with IUPAC ambiguity codes (like EMBOSS backtranseq), standard code:
/// A GCN, R MGN, N AAY, D GAY, C TGY, Q CAR, E GAR, G GGN, H CAY, I ATH, L YTN, K AAR, M ATG, F TTY, P CCN, S WSN, T ACN,
/// W TGG, Y TAY, V GTN, stop `*` TRR. Otherwise as `back_translate_preferred` (same errors and input rules).
pub fn back_translate_degenerate(protein: &str) -> Result<String> {
    let mut dna = String::with_capacity(protein.len() * 3);
    for c in protein.chars().map(|ch| ch.to_ascii_uppercase()) {
        let codon = match c {
            'A' => "GCN",
            'R' => "MGN",
            'N' => "AAY",
            'D' => "GAY",
            'C' => "TGY",
            'Q' => "CAR",
            'E' => "GAR",
            'G' => "GGN",
            'H' => "CAY",
            'I' => "ATH",
            'L' => "YTN",
            'K' => "AAR",
            'M' => "ATG",
            'F' => "TTY",
            'P' => "CCN",
            'S' => "WSN",
            'T' => "ACN",
            'W' => "TGG",
            'Y' => "TAY",
            'V' => "GTN",
            '*' => "TRR",
            _ => return Err(Error::InvalidInput(format!("Unknown character: '{}'", c))),
        };
        dna.push_str(codon);
    }
    Ok(dna)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bevorzugt() {
        assert_eq!(back_translate_preferred("MKV*").unwrap(), "ATGAAAGTGTAA");
        assert_eq!(back_translate_preferred("mAS").unwrap(), "ATGGCGAGC");
        assert_eq!(back_translate_preferred("").unwrap(), "");
        assert!(back_translate_preferred("MKX").is_err());
        assert!(back_translate_preferred("M K").is_err());
    }

    #[test]
    fn mehrdeutig() {
        assert_eq!(back_translate_degenerate("MKV").unwrap(), "ATGAARGTN");
        assert_eq!(back_translate_degenerate("LR").unwrap(), "YTNMGN");
        assert_eq!(back_translate_degenerate("S*").unwrap(), "WSNTRR");
        assert!(back_translate_degenerate("B").is_err());
    }
}
