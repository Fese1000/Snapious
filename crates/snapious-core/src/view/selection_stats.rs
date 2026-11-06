//! Kennzahlen und Textformen für die aktuelle Auswahl (Statusleiste, Kopieren).

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::primer::tm::{gc_fraction, marmur_doty, wallace};

/// Kennzahlen einer Auswahl.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionStats {
    pub length: usize,
    /// GC-Anteil in Prozent (0 bis 100).
    pub gc_percent: f64,
    /// Schmelztemperatur in °C: bis einschließlich 13 Basen nach Wallace, darüber nach Marmur/Doty.
    pub tm_celsius: f64,
}

/// Berechnet die Kennzahlen. Leerer Text ergibt `length 0`, `gc_percent 0.0`, `tm_celsius 0.0`.
/// Verwende `gc_fraction`, `wallace` und `marmur_doty` aus `crate::primer::tm`.
pub fn selection_stats(bases: &str) -> SelectionStats {
    let len = bases.len();
    if len == 0 {
        return SelectionStats {
            length: 0,
            gc_percent: 0.0,
            tm_celsius: 0.0,
        };
    }

    let gc = gc_fraction(bases);
    let tm = if len <= 13 {
        wallace(bases)
    } else {
        marmur_doty(bases)
    };

    SelectionStats {
        length: len,
        gc_percent: gc * 100.0,
        tm_celsius: tm,
    }
}

/// FASTA-Text: Kopfzeile `>name`, danach die Basen in Zeilen zu 60 Zeichen; jede Zeile endet mit `\n`.
/// Bei leeren Basen steht nur die Kopfzeile mit `\n` da.
pub fn as_fasta(name: &str, bases: &str) -> String {
    let mut result = format!(">{}\n", name);
    for chunk in bases.as_bytes().chunks(60) {
        let line: String = chunk.iter().map(|&b| b as char).collect();
        result.push_str(&line);
        result.push('\n');
    }
    result
}

/// Reverse-Komplement als Text (Großbuchstaben), z. B. `ATGC` → `GCAT`.
/// Verwende `iupac_reverse_complement`.
pub fn as_reverse_complement(bases: &str) -> String {
    iupac_reverse_complement(bases).to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn kurze_sequenz_nach_wallace() {
        let s = selection_stats("ATGC");
        assert_eq!(s.length, 4);
        assert!(close(s.gc_percent, 50.0));
        assert!(close(s.tm_celsius, 12.0));
    }

    #[test]
    fn grenze_13_wallace_14_marmur() {
        let dreizehn = selection_stats(&"G".repeat(13));
        assert!(close(dreizehn.tm_celsius, 52.0)); // 13 * 4
        let vierzehn = selection_stats(&"G".repeat(14));
        // 64.9 + 41 * (14 - 16.4) / 14
        assert!(close(
            vierzehn.tm_celsius,
            64.9 + 41.0 * (14.0 - 16.4) / 14.0
        ));
        assert!(close(vierzehn.gc_percent, 100.0));
    }

    #[test]
    fn zwanzig_g() {
        let s = selection_stats(&"G".repeat(20));
        assert!(close(s.tm_celsius, 72.28));
        assert_eq!(s.length, 20);
    }

    #[test]
    fn leere_auswahl() {
        let s = selection_stats("");
        assert_eq!(s.length, 0);
        assert!(close(s.gc_percent, 0.0));
        assert!(close(s.tm_celsius, 0.0));
    }

    #[test]
    fn fasta_ausgabe() {
        let text = as_fasta("x", &"A".repeat(70));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ">x");
        assert_eq!(lines[1].len(), 60);
        assert_eq!(lines[2].len(), 10);
        assert!(text.ends_with('\n'));
        assert_eq!(as_fasta("leer", ""), ">leer\n");
        assert_eq!(as_fasta("k", "ACGT"), ">k\nACGT\n");
        // genau 60 Zeichen: eine Zeile, kein Leerzeilenrest
        assert_eq!(as_fasta("g", &"C".repeat(60)).lines().count(), 2);
    }

    #[test]
    fn reverse_komplement() {
        assert_eq!(as_reverse_complement("ATGC"), "GCAT");
        assert_eq!(as_reverse_complement("atgc"), "GCAT");
        assert_eq!(as_reverse_complement("AACCGGTT"), "AACCGGTT");
        assert_eq!(as_reverse_complement(""), "");
    }
}
