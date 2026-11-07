//! Translation von Sequenzbereichen und Features in Aminosäuren (für die Anzeige unter der Sequenz).

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::model::feature::{Feature, Strand};
use crate::model::sequence::amino_acid_for;

/// Eine Aminosäure mit der Position ihres ersten Basenbuchstabens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AminoAcidCell {
    /// Position der ersten Base des Codons im Dokument (0-basiert; bei Features auf dem Gegenstrang die
    /// Position der Base, die in Leserichtung des Features zuerst kommt, also die höchste Koordinate).
    pub base_position: usize,
    /// Einbuchstabencode; Stopp = `*`, unbekannt = `X`.
    pub amino_acid: char,
    /// Das Codon ist `ATG`.
    pub is_start: bool,
    /// Das Codon ist ein Stopcodon (`TAA`, `TAG`, `TGA`).
    pub is_stop: bool,
}

/// Translation im Leserahmen `frame` (0, 1, 2; größere Werte zählen als `frame % 3`) auf dem Vorwärtsstrang.
/// Es werden nur Codons übersetzt, die vollständig in `[from, to)` liegen (`to` wird auf die Länge von `bases`
/// begrenzt) und deren erste Position `p` die Bedingung `p % 3 == frame % 3` erfüllt. `U` zählt wie `T`,
/// Groß-/Kleinschreibung egal. Ergebnis aufsteigend nach `base_position`.
pub fn translate_frame(bases: &str, frame: usize, from: usize, to: usize) -> Vec<AminoAcidCell> {
    let frame = frame % 3;
    let len = bases.len();

    // Finde die erste Base, die im Rahmen ist und >= from.
    let mut pos = from.min(len);
    // Finde die erste Base, deren Position modulo 3 zum Rahmen passt und >= from.
    while pos < len && pos % 3 != frame % 3 {
        pos += 1;
    }

    let mut cells = Vec::new();
    while pos + 2 < to.min(len) {
        // Konvertiere zu Uppercase, zähle U wie T.
        let codon: String = (pos..pos + 3)
            .map(|i| match bases.as_bytes()[i].to_ascii_uppercase() {
                b'U' => 'T',
                c => c as char,
            })
            .collect();

        let aa = amino_acid_for(codon.as_bytes());
        cells.push(AminoAcidCell {
            base_position: pos,
            amino_acid: aa as char,
            is_start: codon == "ATG",
            is_stop: codon == "TAA" || codon == "TAG" || codon == "TGA",
        });

        pos += 3;
    }

    cells
}

/// Translation eines Features (z. B. CDS), dem Verlauf seiner Intervalle und seinem Strang folgend:
/// 1. Basen der Intervalle in aufsteigender Reihenfolge aneinanderhängen und merken, welche Dokumentposition
///    jede Base hat.
/// 2. Bei `Strand::Reverse` Basen und Positionsliste umdrehen und die Basen reverse-komplementieren
///    (`iupac_reverse_complement`).
/// 3. Qualifier `codon_start` (1, 2 oder 3; Standard 1) gibt an, wie viele Basen (`codon_start - 1`) am Anfang
///    übersprungen werden.
/// 4. Ab da in Dreiergruppen übersetzen; ein unvollständiges Schluss-Codon entfällt.
///
/// Fehler: Feature reicht über `bases` hinaus → `Error::OutOfRange`; `codon_start` ungültig → `Error::InvalidInput`.
pub fn translate_feature(bases: &str, feature: &Feature) -> Result<Vec<AminoAcidCell>> {
    let intervals = feature.location().intervals();

    // Prüfe, dass alle Intervalle innerhalb der Sequenz liegen.
    let seq_len = bases.len();
    for interval in intervals {
        if interval.end() > seq_len {
            return Err(Error::OutOfRange(format!(
                "Feature interval [{}..{}) lies beyond the end of the sequence (length {})",
                interval.start(),
                interval.end(),
                seq_len
            )));
        }
    }

    // Hole codon_start aus Qualifiern (Standard 1). Prüfe auf ungültige Werte.
    let codon_start: usize = match feature.qualifier("codon_start") {
        None => 1,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|&v| (1..=3).contains(&v))
            .ok_or_else(|| {
                Error::InvalidInput(format!(
                    "Invalid codon_start value '{}'. Must be 1, 2 or 3.",
                    v
                ))
            })?,
    };

    // Bauen: hänge alle Basen der Intervalle aneinander, merke die Dokument-Position jeder Base.
    let mut merged_bases: Vec<u8> = Vec::new();
    let mut doc_positions: Vec<usize> = Vec::new();

    for interval in intervals {
        for i in interval.start()..interval.end() {
            merged_bases.push(bases.as_bytes()[i]);
            doc_positions.push(i);
        }
    }

    // Bei Reverse-Strang: Positionsliste umkehren (höchste Koordinate zuerst),
    // Basen reverse-komplementieren (`iupac_reverse_complement` macht bereits Reverse + Komplement).
    let (working_bases, working_positions) = match feature.location().strand() {
        Strand::Reverse => {
            let rc = iupac_reverse_complement(&String::from_utf8_lossy(&merged_bases));
            let rc_bytes: Vec<u8> = rc.bytes().collect();
            doc_positions.reverse();
            (rc_bytes, doc_positions)
        }
        _ => (merged_bases, doc_positions),
    };

    // codon_start - 1 Basen überspringen.
    let skip = codon_start - 1;

    let mut cells = Vec::new();
    let mut i = skip;
    while i + 2 < working_bases.len() {
        let codon: String = (i..i + 3)
            .map(|j| match working_bases[j].to_ascii_uppercase() {
                b'U' => 'T',
                c => c as char,
            })
            .collect();

        let aa = amino_acid_for(codon.as_bytes());
        // Dokument-Position der ersten Base dieses Codons.
        let doc_pos = working_positions[i];

        cells.push(AminoAcidCell {
            base_position: doc_pos,
            amino_acid: aa as char,
            is_start: codon == "ATG",
            is_stop: codon == "TAA" || codon == "TAG" || codon == "TGA",
        });

        i += 3;
    }

    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Interval, Location};

    fn cds(intervals: &[(usize, usize)], strand: Strand, codon_start: Option<&str>) -> Feature {
        let loc = Location::new(
            intervals
                .iter()
                .map(|(a, b)| Interval::new(*a, *b).unwrap())
                .collect(),
            strand,
            false,
            false,
        )
        .unwrap();
        let q = codon_start
            .map(|c| vec![("codon_start".to_string(), vec![c.to_string()])])
            .unwrap_or_default();
        Feature::new("CDS", "c", loc, q)
    }

    fn summary(cells: &[AminoAcidCell]) -> (String, Vec<usize>) {
        (
            cells.iter().map(|c| c.amino_acid).collect(),
            cells.iter().map(|c| c.base_position).collect(),
        )
    }

    #[test]
    fn rahmen_null() {
        let c = translate_frame("ATGAAATAG", 0, 0, 9);
        assert_eq!(summary(&c), ("MK*".to_string(), vec![0, 3, 6]));
        assert!(c[0].is_start && !c[0].is_stop);
        assert!(!c[1].is_start && !c[1].is_stop);
        assert!(c[2].is_stop);
    }

    #[test]
    fn rahmen_eins_und_zwei() {
        let c = translate_frame("AATGAAATAG", 1, 0, 10);
        assert_eq!(summary(&c), ("MK*".to_string(), vec![1, 4, 7]));
        let d = translate_frame("AAATGAAATAG", 2, 0, 11);
        assert_eq!(summary(&d), ("MK*".to_string(), vec![2, 5, 8]));
    }

    #[test]
    fn bereich_und_grenzen() {
        // from = 2: erstes Codon im Rahmen 0 beginnt bei 3
        let c = translate_frame("ATGAAATAG", 0, 2, 9);
        assert_eq!(summary(&c), ("K*".to_string(), vec![3, 6]));
        // to schneidet ein Codon ab: nur vollständige Codons zählen
        let d = translate_frame("ATGAAATAG", 0, 0, 7);
        assert_eq!(summary(&d), ("MK".to_string(), vec![0, 3]));
        // to hinter dem Ende wird begrenzt
        assert_eq!(translate_frame("ATGAAA", 0, 0, 100).len(), 2);
        assert!(translate_frame("ATG", 0, 5, 9).is_empty());
        assert!(translate_frame("", 0, 0, 0).is_empty());
        assert!(translate_frame("ATGAAA", 0, 4, 2).is_empty());
    }

    #[test]
    fn frame_modulo_drei_und_schreibweise() {
        let c = translate_frame("atgaaauag", 3, 0, 9);
        assert_eq!(summary(&c), ("MK*".to_string(), vec![0, 3, 6]));
        let r = translate_frame("AUGAAAUAG", 0, 0, 9);
        assert_eq!(summary(&r), ("MK*".to_string(), vec![0, 3, 6]));
    }

    #[test]
    fn unbekanntes_codon() {
        let c = translate_frame("ANGAAA", 0, 0, 6);
        assert_eq!(c[0].amino_acid, 'X');
        assert!(!c[0].is_start && !c[0].is_stop);
    }

    #[test]
    fn feature_vorwaerts() {
        let c = translate_feature("ATGAAATAG", &cds(&[(0, 9)], Strand::Forward, None)).unwrap();
        assert_eq!(summary(&c), ("MK*".to_string(), vec![0, 3, 6]));
        assert!(c[0].is_start && c[2].is_stop);
    }

    #[test]
    fn feature_rueckwaerts() {
        // Reverse-Komplement von CTATTTCAT ist ATGAAATAG
        let c = translate_feature("CTATTTCAT", &cds(&[(0, 9)], Strand::Reverse, None)).unwrap();
        assert_eq!(summary(&c), ("MK*".to_string(), vec![8, 5, 2]));
    }

    #[test]
    fn feature_mit_join() {
        // Basen 0..3 und 6..9 ergeben ATG + TAG
        let c =
            translate_feature("ATGCCCTAG", &cds(&[(0, 3), (6, 9)], Strand::Forward, None)).unwrap();
        assert_eq!(summary(&c), ("M*".to_string(), vec![0, 6]));
    }

    #[test]
    fn feature_teilbereich_mitten_in_der_sequenz() {
        let c =
            translate_feature("GGGATGAAATAGCCC", &cds(&[(3, 12)], Strand::Forward, None)).unwrap();
        assert_eq!(summary(&c), ("MK*".to_string(), vec![3, 6, 9]));
    }

    #[test]
    fn codon_start_ueberspringt_basen() {
        let c =
            translate_feature("ATGAAATAG", &cds(&[(0, 9)], Strand::Forward, Some("2"))).unwrap();
        // TGA AAT AG → '*' (Position 1), 'N' (Position 4); "AG" ist unvollständig
        assert_eq!(summary(&c), ("*N".to_string(), vec![1, 4]));
        assert!(c[0].is_stop);
    }

    #[test]
    fn fehlerfaelle() {
        let ausserhalb = translate_feature("ATGAAA", &cds(&[(0, 20)], Strand::Forward, None));
        assert!(matches!(ausserhalb, Err(Error::OutOfRange(_))));
        for ungueltig in ["0", "4", "x", ""] {
            let r = translate_feature(
                "ATGAAATAG",
                &cds(&[(0, 9)], Strand::Forward, Some(ungueltig)),
            );
            assert!(matches!(r, Err(Error::InvalidInput(_))), "{}", ungueltig);
        }
    }

    #[test]
    fn zu_kurzes_feature_ergibt_keine_zellen() {
        let c = translate_feature("ATGAAATAG", &cds(&[(0, 2)], Strand::Forward, None)).unwrap();
        assert!(c.is_empty());
    }

    #[test]
    fn strand_none_wird_wie_vorwaerts_behandelt() {
        let c = translate_feature("ATGAAATAG", &cds(&[(0, 9)], Strand::None, None)).unwrap();
        assert_eq!(c.len(), 3);
    }
}
