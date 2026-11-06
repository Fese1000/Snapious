//! Primer für den Überlappungs-Zusammenbau: Überhang zum Vektor plus bindender Teil am Insert.

use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};
use crate::primer::tm::nearest_neighbor;

/// Ein Primerpaar mit Überhängen.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlapPrimers {
    /// Vorwärtsprimer 5'→3': Vektorüberhang + bindender Teil.
    pub forward: String,
    /// Rückwärtsprimer 5'→3': Reverse-Komplement des Vektorüberhangs rechts der Schnittstelle + bindender Teil.
    pub reverse: String,
    pub overlap_length: usize,
    /// Länge des bindenden Teils des Vorwärtsprimers und des Rückwärtsprimers.
    pub forward_binding_length: usize,
    pub reverse_binding_length: usize,
}

/// Wählt die Länge `L` (18 bis 30) eines bindenden Teils. `tm_of(L)` liefert die Schmelztemperatur der Länge `L`
/// (`None`, wenn nicht berechenbar; solche Längen zählen nicht). Gewählt wird die kleinste Länge mit `58.0 <= tm <= 62.0`;
/// gibt es keine, die Länge mit dem kleinsten `|tm − 60|` (bei Gleichstand die kleinere). Gibt es gar keine berechenbare Länge: `None`.
pub fn choose_binding_length(tm_of: &dyn Fn(usize) -> Option<f64>) -> Option<usize> {
    let mut best_len: Option<usize> = None;
    let mut min_diff = f64::MAX;

    for l in 18..=30 {
        if let Some(tm) = tm_of(l) {
            if (58.0..=62.0).contains(&tm) {
                return Some(l);
            }
            let diff = (tm - 60.0).abs();
            if diff < min_diff {
                min_diff = diff;
                best_len = Some(l);
            }
        }
    }
    best_len
}

/// Entwirft Primer, damit `insert[insert_start..insert_end]` an der Stelle `cut_position` (zwischen Basen des Vektors) eingebaut werden kann.
///
/// - Alle Texte werden in Großbuchstaben mit `T` statt `U` gelesen.
/// - Vorwärtsprimer = `vector[cut − overlap_length .. cut]` + `insert[insert_start .. insert_start + L]`, wobei `L` mit
///   `choose_binding_length` aus der Tm (`nearest_neighbor(teil, 250.0, 50.0)`) des Insertanfangs gewählt wird.
/// - Rückwärtsprimer = `iupac_reverse_complement(vector[cut .. cut + overlap_length])` + `iupac_reverse_complement(insert[insert_end − L' .. insert_end])`,
///   `L'` ebenfalls mit `choose_binding_length` (Tm des bindenden Teils des Rückwärtsprimers, also des Reverse-Komplements des Insertendes).
///   Dadurch endet `iupac_reverse_complement(reverse)` mit `vector[cut .. cut + overlap_length]`.
/// - Fehler `Error::InvalidInput`: `overlap_length == 0`; `overlap_length > cut_position`; `cut_position + overlap_length > vector.len()`;
///   `insert_start >= insert_end` oder `insert_end > insert.len()`; Insertbereich kürzer als 30 Basen (sonst kann `L` nicht frei gewählt
///   werden); keine berechenbare Länge für einen der beiden bindenden Teile.
///
/// Beispiel: Vektor `…ACGT|TTGG…` mit `cut_position` zwischen `T` und `T`, `overlap_length 4`: Vorwärtsprimer beginnt mit `ACGT`
/// und setzt sich mit dem Insertanfang fort; Rückwärtsprimer beginnt mit `CCAA` (Reverse-Komplement von `TTGG`).
pub fn design_overlap_primers(
    insert: &str,
    insert_start: usize,
    insert_end: usize,
    vector: &str,
    cut_position: usize,
    overlap_length: usize,
) -> Result<OverlapPrimers> {
    if overlap_length == 0 {
        return Err(Error::InvalidInput(
            "overlap_length muss größer als 0 sein".into(),
        ));
    }
    if overlap_length > cut_position {
        return Err(Error::InvalidInput(
            "overlap_length darf nicht größer als cut_position sein".into(),
        ));
    }
    if (cut_position + overlap_length) > vector.len() {
        return Err(Error::InvalidInput(
            "Schnittstelle + Überhang liegt außerhalb des Vektors".into(),
        ));
    }
    if insert_start >= insert_end {
        return Err(Error::InvalidInput(
            "insert_start muss kleiner als insert_end sein".into(),
        ));
    }
    if insert_end > insert.len() {
        return Err(Error::InvalidInput(
            "insert_end liegt außerhalb der Insert-Sequenz".into(),
        ));
    }

    let insert_len = insert_end - insert_start;
    if insert_len < 30 {
        return Err(Error::InvalidInput(
            "Insertbereich muss mindestens 30 Basen lang sein".into(),
        ));
    }

    let forward_overlap = &vector[cut_position - overlap_length..cut_position];

    // Hilfsfunktion, um die Länge für einen Sequenzabschnitt zu finden
    let find_binding_length = |seq: &str| -> Option<usize> {
        let tm_of = |l: usize| nearest_neighbor(&seq[..l], 250.0, 50.0).ok();
        choose_binding_length(&tm_of)
    };

    let forward_l = find_binding_length(&insert[insert_start..]).ok_or_else(|| {
        Error::InvalidInput("Keine berechenbare Länge für den Vorwärtsprimer".into())
    })?;

    let reverse_overlap =
        iupac_reverse_complement(&vector[cut_position..cut_position + overlap_length]);

    let reverse_l = {
        let tm_of = |l: usize| {
            let slice = &insert[insert_end - l..insert_end];
            nearest_neighbor(&iupac_reverse_complement(slice), 250.0, 50.0).ok()
        };
        choose_binding_length(&tm_of).ok_or_else(|| {
            Error::InvalidInput("Keine berechenbare Länge für den Rückwärtsprimer".into())
        })?
    };

    let forward_binding = &insert[insert_start..insert_start + forward_l];
    let reverse_binding_rc = iupac_reverse_complement(&insert[insert_end - reverse_l..insert_end]);

    let forward = format!("{}{}", forward_overlap, forward_binding);
    let reverse = format!("{}{}", reverse_overlap, reverse_binding_rc);

    Ok(OverlapPrimers {
        forward,
        reverse,
        overlap_length,
        forward_binding_length: forward_l,
        reverse_binding_length: reverse_l,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministische Testfolge mit gemischter Zusammensetzung.
    fn seq(n: usize, seed: u64) -> String {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ['A', 'C', 'G', 'T'][((s >> 33) % 4) as usize]
            })
            .collect()
    }

    fn rc(s: &str) -> String {
        iupac_reverse_complement(s)
    }

    #[test]
    fn laengenwahl() {
        // Tm steigt mit der Länge: 18 → 50, +1.0 je Base → 58 bei L = 26
        let f = |l: usize| Some(50.0 + (l as f64 - 18.0));
        assert_eq!(choose_binding_length(&f), Some(26));
        // nie im Fenster, immer zu kalt → nächste an 60 ist die längste
        let kalt = |l: usize| Some(30.0 + 0.1 * l as f64);
        assert_eq!(choose_binding_length(&kalt), Some(30));
        // immer zu heiß → kürzeste
        let heiss = |l: usize| Some(80.0 + 0.1 * l as f64);
        assert_eq!(choose_binding_length(&heiss), Some(18));
        // nicht berechenbar für alle
        assert_eq!(choose_binding_length(&|_| None), None);
        // nur einzelne Längen berechenbar
        let teils = |l: usize| if l == 20 { Some(59.0) } else { None };
        assert_eq!(choose_binding_length(&teils), Some(20));
    }

    #[test]
    fn primeraufbau() {
        let vector = seq(80, 1);
        let insert = seq(120, 2);
        let p = design_overlap_primers(&insert, 10, 100, &vector, 40, 25).unwrap();
        assert_eq!(p.overlap_length, 25);
        assert!((18..=30).contains(&p.forward_binding_length));
        assert!((18..=30).contains(&p.reverse_binding_length));
        assert!(p.forward.starts_with(&vector[15..40]));
        assert_eq!(p.forward.len(), 25 + p.forward_binding_length);
        assert_eq!(&p.forward[25..], &insert[10..10 + p.forward_binding_length]);
        assert_eq!(p.reverse.len(), 25 + p.reverse_binding_length);
        assert!(p.reverse.starts_with(&rc(&vector[40..65])));
        assert_eq!(
            &p.reverse[25..],
            rc(&insert[100 - p.reverse_binding_length..100])
        );
        assert!(rc(&p.reverse).ends_with(&vector[40..65]));
    }

    #[test]
    fn gewaehlte_laenge_folgt_der_regel() {
        let vector = seq(80, 3);
        let insert = seq(150, 4);
        let p = design_overlap_primers(&insert, 5, 140, &vector, 30, 25).unwrap();
        let tm_vorn = |l: usize| nearest_neighbor(&insert[5..5 + l], 250.0, 50.0).ok();
        assert_eq!(
            Some(p.forward_binding_length),
            choose_binding_length(&tm_vorn)
        );
        let tm_hinten = |l: usize| nearest_neighbor(&rc(&insert[140 - l..140]), 250.0, 50.0).ok();
        assert_eq!(
            Some(p.reverse_binding_length),
            choose_binding_length(&tm_hinten)
        );
    }

    #[test]
    fn kleines_beispiel_aus_der_doku() {
        let vector = format!("{}ACGTTTGG{}", "GATTACA".repeat(2), "CAGT".repeat(5));
        let cut = 14 + 4; // zwischen ACGT und TTGG
        let insert = seq(60, 5);
        let p = design_overlap_primers(&insert, 0, 60, &vector, cut, 4).unwrap();
        assert!(p.forward.starts_with("ACGT"));
        assert!(p.reverse.starts_with("CCAA"));
    }

    #[test]
    fn fehler() {
        let vector = seq(80, 1);
        let insert = seq(120, 2);
        let e = |r: Result<OverlapPrimers>| matches!(r, Err(Error::InvalidInput(_)));
        assert!(e(design_overlap_primers(&insert, 10, 100, &vector, 40, 0)));
        assert!(e(design_overlap_primers(&insert, 10, 100, &vector, 10, 25)));
        assert!(e(design_overlap_primers(&insert, 10, 100, &vector, 70, 25)));
        assert!(e(design_overlap_primers(&insert, 100, 10, &vector, 40, 25)));
        assert!(e(design_overlap_primers(&insert, 10, 200, &vector, 40, 25)));
        assert!(e(design_overlap_primers(&insert, 10, 30, &vector, 40, 25)));
        assert!(e(design_overlap_primers(
            "ACGTNNNNACGTNNNNACGTNNNNACGTNNNNACGT",
            0,
            36,
            &vector,
            40,
            25
        )));
    }
}
