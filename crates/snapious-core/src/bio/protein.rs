//! Eigenschaften von Proteinsequenzen: Masse, Ladung, isoelektrischer Punkt, Extinktion, Hydropathie.
//!
//! Eingaben sind Einbuchstabencodes in beliebiger Schreibweise; es zählen nur die 20 Standard-Aminosäuren
//! `ACDEFGHIKLMNPQRSTVWY`. Alle anderen Zeichen (z. B. `X`, `B`, `*`, Leerzeichen) werden ignoriert, soweit nicht anders gesagt.

use std::collections::BTreeMap;

/// Durchschnittliche Restmassen in Da: G 57.0519, A 71.0788, S 87.0782, P 97.1167, V 99.1326, T 101.1051, C 103.1388,
/// L 113.1594, I 113.1594, N 114.1038, D 115.0886, Q 128.1307, K 128.1741, E 129.1155, M 131.1926, H 137.1411,
/// F 147.1766, R 156.1875, Y 163.1760, W 186.2132.
/// Rückgabe: Masse des Restes oder `None` für ein Zeichen, das keine Standard-Aminosäure ist (Uppercase erwartet).
pub fn residue_mass(aa: char) -> Option<f64> {
    match aa {
        'A' => Some(71.0788),
        'C' => Some(103.1388),
        'D' => Some(115.0886),
        'E' => Some(129.1155),
        'F' => Some(147.1766),
        'G' => Some(57.0519),
        'H' => Some(137.1411),
        'I' => Some(113.1594),
        'K' => Some(128.1741),
        'L' => Some(113.1594),
        'M' => Some(131.1926),
        'N' => Some(114.1038),
        'P' => Some(97.1167),
        'Q' => Some(128.1307),
        'R' => Some(156.1875),
        'S' => Some(87.0782),
        'T' => Some(101.1051),
        'V' => Some(99.1326),
        'W' => Some(186.2132),
        'Y' => Some(163.1760),
        _ => None,
    }
}

/// Kyte-Doolittle-Wert: A 1.8, R −4.5, N −3.5, D −3.5, C 2.5, Q −3.5, E −3.5, G −0.4, H −3.2, I 4.5, L 3.8, K −3.9,
/// M 1.9, F 2.8, P −1.6, S −0.8, T −0.7, W −0.9, Y −1.3, V 4.2. Sonst `None` (Uppercase erwartet).
pub fn hydropathy_value(aa: char) -> Option<f64> {
    match aa {
        'A' => Some(1.8),
        'C' => Some(2.5),
        'D' => Some(-3.5),
        'E' => Some(-3.5),
        'F' => Some(2.8),
        'G' => Some(-0.4),
        'H' => Some(-3.2),
        'I' => Some(4.5),
        'K' => Some(-3.9),
        'L' => Some(3.8),
        'M' => Some(1.9),
        'N' => Some(-3.5),
        'P' => Some(-1.6),
        'Q' => Some(-3.5),
        'R' => Some(-4.5),
        'S' => Some(-0.8),
        'T' => Some(-0.7),
        'V' => Some(4.2),
        'W' => Some(-0.9),
        'Y' => Some(-1.3),
        _ => None,
    }
}

/// Durchschnittliche Masse in Da: Summe der Restmassen aller Standard-Aminosäuren plus Wasser (18.0153).
/// Ohne eine einzige Standard-Aminosäure: 0.0.
pub fn molecular_weight(protein: &str) -> f64 {
    let mut mass = 0.0_f64;
    let mut has_standard = false;

    for c in protein.to_ascii_uppercase().chars() {
        if let Some(m) = residue_mass(c) {
            mass += m;
            has_standard = true;
        }
    }

    if has_standard {
        mass + 18.0153
    } else {
        0.0
    }
}

/// Anzahl je Standard-Aminosäure (Uppercase als Schlüssel); nur Aminosäuren, die vorkommen.
pub fn composition(protein: &str) -> BTreeMap<char, usize> {
    let mut map = BTreeMap::new();

    for c in protein.to_ascii_uppercase().chars() {
        if residue_mass(c).is_some() {
            *map.entry(c).or_insert(0) += 1;
        }
    }

    map
}

/// Nettoladung bei `ph` mit den pKa-Werten N-Terminus 8.6, C-Terminus 3.6, D 3.9, E 4.1, C 8.5, Y 10.1, H 6.5, K 10.8, R 12.5.
/// Positive Gruppen (N-Terminus, K, R, H) tragen je `1 / (1 + 10^(ph − pKa))`, negative (C-Terminus, D, E, C, Y) je
/// `−1 / (1 + 10^(pKa − ph))`. Die beiden Termini zählen nur, wenn mindestens eine Standard-Aminosäure vorhanden ist; sonst 0.0.
pub fn charge_at_ph(protein: &str, ph: f64) -> f64 {
    let upper = protein.to_ascii_uppercase();
    let mut counts: BTreeMap<char, u32> = BTreeMap::new();

    for c in upper.chars() {
        if c.is_ascii_uppercase() && residue_mass(c).is_some() {
            *counts.entry(c).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        return 0.0;
    }

    // Termini
    let mut charge = 1.0 / (1.0 + 10.0_f64.powf(ph - 8.6)) - 1.0 / (1.0 + 10.0_f64.powf(3.6 - ph));
    // Seitenketten: (Aminosäure, pKa, positiv)
    let groups = [
        ('D', 3.9, false),
        ('E', 4.1, false),
        ('C', 8.5, false),
        ('Y', 10.1, false),
        ('H', 6.5, true),
        ('K', 10.8, true),
        ('R', 12.5, true),
    ];
    for (aa, pka, positive) in groups {
        let count = counts.get(&aa).copied().unwrap_or(0) as f64;
        if positive {
            charge += count / (1.0 + 10.0_f64.powf(ph - pka));
        } else {
            charge -= count / (1.0 + 10.0_f64.powf(pka - ph));
        }
    }
    charge
}

/// Isoelektrischer Punkt: der pH zwischen 0 und 14, bei dem `charge_at_ph` 0 ist (Bisektion; Abbruch, wenn das Intervall
/// kleiner als 0.0001 ist). Die Ladung nimmt mit steigendem pH ab. Ohne Standard-Aminosäure: 7.0.
pub fn isoelectric_point(protein: &str) -> f64 {
    let upper = protein.to_ascii_uppercase();

    if !upper.chars().any(|c| residue_mass(c).is_some()) {
        return 7.0;
    }

    let mut lo = 0.0_f64;
    let mut hi = 14.0_f64;

    while hi - lo > 0.0001 {
        let mid = (lo + hi) / 2.0;
        if charge_at_ph(protein, mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    (lo + hi) / 2.0
}

/// Extinktionskoeffizient bei 280 nm in M⁻¹ cm⁻¹ nach Pace et al. (1995): `nW · 5500 + nY · 1490 + (nC / 2) · 125`, wobei
/// `nC / 2` ganzzahlig geteilt wird; der Cystin-Term entfällt, wenn `reduced_cysteines` wahr ist.
pub fn extinction_coefficient_280(protein: &str, reduced_cysteines: bool) -> f64 {
    let mut n_w = 0u32;
    let mut n_y = 0u32;
    let mut n_c = 0u32;

    for c in protein.to_ascii_uppercase().chars() {
        match c {
            'W' => n_w += 1,
            'Y' => n_y += 1,
            'C' if residue_mass(c).is_some() => n_c += 1,
            _ => {}
        }
    }

    let mut ext = n_w as f64 * 5500.0 + n_y as f64 * 1490.0;
    if !reduced_cysteines {
        ext += (n_c / 2) as f64 * 125.0;
    }

    ext
}

/// Anteil der aromatischen Reste (F, W, Y) an allen Standard-Aminosäuren; ohne Aminosäuren 0.0.
pub fn aromaticity(protein: &str) -> f64 {
    let mut total = 0u32;
    let mut arom = 0u32;

    for c in protein.to_ascii_uppercase().chars() {
        if residue_mass(c).is_some() {
            total += 1;
            match c {
                'F' | 'W' | 'Y' => arom += 1,
                _ => {}
            }
        }
    }

    if total == 0 {
        0.0
    } else {
        arom as f64 / total as f64
    }
}

/// Kyte-Doolittle-Verlauf: ein Wert je Zeichen von `protein`. Zeichen ohne Wert (`hydropathy_value` ist `None`) zählen als 0.0.
/// Der Wert an Position `i` ist der Mittelwert über das zentrierte Fenster. Das Fenster wird auf eine ungerade Größe gebracht
/// (gerade Größe + 1, 0 wird 1); am Rand wird es gekürzt (nur vorhandene Positionen werden gemittelt). Leere Eingabe → leerer Vektor.
pub fn kyte_doolittle(protein: &str, window: usize) -> Vec<f64> {
    let win = if window == 0 {
        1
    } else if window.is_multiple_of(2) {
        window + 1
    } else {
        window
    };

    let half = (win - 1) / 2;
    let chars: Vec<char> = protein.to_ascii_uppercase().chars().collect();

    if chars.is_empty() {
        return Vec::new();
    }

    let len = chars.len();
    let mut result = Vec::with_capacity(len);

    for i in 0..len {
        let start = i.saturating_sub(half);
        let end = (i + half).min(len - 1);
        let mut sum = 0.0_f64;

        for c in chars.iter().skip(start).take(end - start + 1) {
            if let Some(h) = hydropathy_value(*c) {
                sum += h;
            }
        }

        result.push(sum / (end - start + 1) as f64);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn tabellen() {
        assert_eq!(residue_mass('G'), Some(57.0519));
        assert_eq!(residue_mass('W'), Some(186.2132));
        assert_eq!(residue_mass('B'), None);
        assert_eq!(residue_mass('g'), None);
        assert_eq!(hydropathy_value('I'), Some(4.5));
        assert_eq!(hydropathy_value('R'), Some(-4.5));
        assert_eq!(hydropathy_value('*'), None);
        let alle = "ACDEFGHIKLMNPQRSTVWY";
        assert!(alle
            .chars()
            .all(|c| residue_mass(c).is_some() && hydropathy_value(c).is_some()));
    }

    #[test]
    fn masse() {
        assert!(close(
            molecular_weight("GG"),
            2.0 * 57.0519 + 18.0153,
            0.001
        ));
        assert_eq!(molecular_weight(""), 0.0);
        assert_eq!(molecular_weight("XX*"), 0.0);
        assert!(close(molecular_weight("A*"), 71.0788 + 18.0153, 0.001));
        assert!(close(
            molecular_weight("ga"),
            57.0519 + 71.0788 + 18.0153,
            0.001
        ));
    }

    #[test]
    fn zusammensetzung() {
        let c = composition("AAB");
        assert_eq!(c.get(&'A'), Some(&2));
        assert_eq!(c.len(), 1);
        let d = composition("ggAk");
        assert_eq!(d.get(&'G'), Some(&2));
        assert_eq!(d.get(&'K'), Some(&1));
        assert!(composition("").is_empty());
    }

    #[test]
    fn ladung() {
        assert!(close(charge_at_ph("K", 7.0), 0.9757, 0.005));
        assert_eq!(charge_at_ph("", 7.0), 0.0);
        assert_eq!(charge_at_ph("XX", 7.0), 0.0);
        // bei sehr hohem pH negativ, bei sehr niedrigem positiv
        assert!(charge_at_ph("GGGG", 14.0) < -0.9);
        assert!(charge_at_ph("GGGG", 0.0) > 0.9);
    }

    #[test]
    fn isoelektrischer_punkt() {
        assert!(close(isoelectric_point("GGGG"), 6.1, 0.05));
        assert!(isoelectric_point(&"K".repeat(10)) > 10.0);
        assert!(isoelectric_point(&"D".repeat(10)) < 4.0);
        assert_eq!(isoelectric_point(""), 7.0);
        let pi = isoelectric_point("ACDEFGHIKLMNPQRSTVWY");
        assert!(close(charge_at_ph("ACDEFGHIKLMNPQRSTVWY", pi), 0.0, 0.01));
    }

    #[test]
    fn extinktion() {
        assert_eq!(extinction_coefficient_280("W", false), 5500.0);
        assert_eq!(extinction_coefficient_280("Y", false), 1490.0);
        assert_eq!(extinction_coefficient_280("CC", false), 125.0);
        assert_eq!(extinction_coefficient_280("CC", true), 0.0);
        assert_eq!(extinction_coefficient_280("CCC", false), 125.0);
        assert_eq!(
            extinction_coefficient_280("WWY", true),
            2.0 * 5500.0 + 1490.0
        );
        assert_eq!(extinction_coefficient_280("", false), 0.0);
    }

    #[test]
    fn aromatisch() {
        assert!(close(aromaticity("FWYA"), 0.75, 1e-12));
        assert_eq!(aromaticity(""), 0.0);
        assert_eq!(aromaticity("GGG"), 0.0);
    }

    #[test]
    fn hydropathie_verlauf() {
        let a = kyte_doolittle("AAAA", 3);
        assert_eq!(a.len(), 4);
        assert!(a.iter().all(|v| close(*v, 1.8, 1e-12)));
        assert_eq!(kyte_doolittle("AR", 1), vec![1.8, -4.5]);
        let b = kyte_doolittle("AAR", 3);
        assert!(close(b[0], 1.8, 1e-12));
        assert!(close(b[1], (1.8 + 1.8 - 4.5) / 3.0, 1e-12));
        assert!(close(b[2], (1.8 - 4.5) / 2.0, 1e-12));
        // gerade Fenster wird ungerade gemacht, Fenster 0 wie 1
        assert_eq!(kyte_doolittle("AR", 0), vec![1.8, -4.5]);
        assert_eq!(kyte_doolittle("AAR", 2), kyte_doolittle("AAR", 3));
        assert!(kyte_doolittle("", 5).is_empty());
        // unbekanntes Zeichen zählt als 0
        assert_eq!(kyte_doolittle("X", 1), vec![0.0]);
    }
}
