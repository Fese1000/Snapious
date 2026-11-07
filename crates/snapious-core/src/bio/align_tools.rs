//! Alignment-Werkzeuge auf Zeilenlisten: Profile verbinden, Translations-Alignment, Teilbereich neu ausrichten.
//! Lücken sind `-`; alle Zeilen eines Alignments sind gleich lang.

use crate::error::{Error, Result};
use crate::model::sequence::amino_acid_for;

/// Spaltenbewertung zweier Zeichen: beide `-` → 0, genau eines `-` → −1, gleiche Zeichen (Groß-/Kleinschreibung egal) → 2, sonst −1.
fn pair_score(x: char, y: char) -> f64 {
    match (x == '-', y == '-') {
        (true, true) => 0.0,
        (true, false) | (false, true) => -1.0,
        _ if x.eq_ignore_ascii_case(&y) => 2.0,
        _ => -1.0,
    }
}

const GAP_COST: f64 = -2.0;

fn check_group(rows: &[String], what: &str) -> Result<usize> {
    let Some(first) = rows.first() else {
        return Err(Error::InvalidInput(format!("{} is empty", what)));
    };
    let n = first.chars().count();
    if rows.iter().any(|r| r.chars().count() != n) {
        return Err(Error::InvalidInput(format!(
            "Rows of {} have different lengths",
            what
        )));
    }
    Ok(n)
}

/// Verbindet zwei Alignments `a` und `b` zu einem, ohne die Lückenstruktur innerhalb von `a` und `b` zu ändern
/// (es werden nur reine Lückenspalten in der jeweils anderen Gruppe eingefügt). Die Zeilen von `a` stehen zuerst, dann die von `b`.
/// Verfahren: globales Alignment der Spaltenfolgen (Needleman-Wunsch):
/// * Wert zweier Spalten (eine aus `a`, eine aus `b`) = Mittelwert von `pair_score` über alle Paare (Zeichen aus `a`, Zeichen aus `b`).
/// * Eine eingefügte Lückenspalte kostet −2.0.
/// * Matrix `S[i][j]`: `S[i][0] = i * −2.0`, `S[0][j] = j * −2.0`, sonst Maximum aus Diagonale `S[i−1][j−1] + Spaltenwert`,
///   `S[i−1][j] − 2.0` (Spalte von `a` bekommt Lückenspalte in `b`) und `S[i][j−1] − 2.0` (umgekehrt).
/// * Rückverfolgung ab `(n, m)` bei Gleichstand in der Reihenfolge Diagonale, dann „oben“ (Lücke in `b`), dann „links“ (Lücke in `a`).
///
/// Fehler `Error::InvalidInput`, wenn `a` oder `b` leer ist oder die Zeilen innerhalb einer Gruppe verschieden lang sind.
pub fn align_profiles(a: &[String], b: &[String]) -> Result<Vec<String>> {
    let n = check_group(a, "Profile A")?;
    let m = check_group(b, "Profile B")?;
    let ca: Vec<Vec<char>> = a.iter().map(|r| r.chars().collect()).collect();
    let cb: Vec<Vec<char>> = b.iter().map(|r| r.chars().collect()).collect();
    let col_score = |i: usize, j: usize| -> f64 {
        let mut sum = 0.0;
        for ra in &ca {
            for rb in &cb {
                sum += pair_score(ra[i], rb[j]);
            }
        }
        sum / (ca.len() * cb.len()) as f64
    };
    let mut s = vec![vec![0.0f64; m + 1]; n + 1];
    // 0 = Diagonale, 1 = oben, 2 = links
    let mut t = vec![vec![0u8; m + 1]; n + 1];
    for i in 1..=n {
        s[i][0] = i as f64 * GAP_COST;
        t[i][0] = 1;
    }
    for j in 1..=m {
        s[0][j] = j as f64 * GAP_COST;
        t[0][j] = 2;
    }
    for i in 1..=n {
        for j in 1..=m {
            let d = s[i - 1][j - 1] + col_score(i - 1, j - 1);
            let u = s[i - 1][j] + GAP_COST;
            let l = s[i][j - 1] + GAP_COST;
            let best = d.max(u).max(l);
            s[i][j] = best;
            t[i][j] = if best == d {
                0
            } else if best == u {
                1
            } else {
                2
            };
        }
    }
    let (mut i, mut j) = (n, m);
    let mut path = Vec::new();
    while i > 0 || j > 0 {
        let step = t[i][j];
        path.push(step);
        match step {
            0 => {
                i -= 1;
                j -= 1;
            }
            1 => i -= 1,
            _ => j -= 1,
        }
    }
    path.reverse();
    let mut out: Vec<String> = vec![String::new(); a.len() + b.len()];
    let (mut i, mut j) = (0, 0);
    for step in path {
        for (k, row) in ca.iter().enumerate() {
            out[k].push(if step == 2 { '-' } else { row[i] });
        }
        for (k, row) in cb.iter().enumerate() {
            out[a.len() + k].push(if step == 1 { '-' } else { row[j] });
        }
        if step != 2 {
            i += 1;
        }
        if step != 1 {
            j += 1;
        }
    }
    Ok(out)
}

/// Aligner für benannte Proteinzeilen.
pub type ProteinAligner = dyn Fn(&[(String, String)]) -> Result<Vec<(String, String)>>;

/// Translations-Alignment: Nukleotidzeilen `(Name, Basen)` werden im Leseraster 1 in Codons zerlegt und mit
/// `amino_acid_for(&[u8])` (liefert das Aminosäurezeichen als Byte, Stopp `*`) übersetzt. Die Proteine `(Name, Protein)` gehen an
/// `aligner`, das ausgerichtete Proteine `(Name, Zeile)` zurückgibt (beliebige Reihenfolge, Zuordnung über den Namen).
/// Rückübersetzung: jede Aminosäure der ausgerichteten Zeile wird durch das Original-Codon ersetzt (der Reihe nach), jedes `-` durch `---`.
/// Das Ergebnis hat die Zeilen in der Reihenfolge der Eingabe. Basen in Uppercase.
/// Fehler `Error::InvalidInput`: leere Eingabe; Länge einer Zeile kein Vielfaches von 3 (Meldung enthält den Namen);
/// `aligner` liefert eine Zeile nicht zurück oder mit einer Zahl Aminosäuren (ohne `-`), die nicht zum Protein passt. Fehler des `aligner` werden weitergegeben.
pub fn translation_align(
    rows: &[(String, String)],
    aligner: &ProteinAligner,
) -> Result<Vec<(String, String)>> {
    if rows.is_empty() {
        return Err(Error::InvalidInput("No sequences provided".to_string()));
    }
    let mut codons: Vec<Vec<String>> = Vec::new();
    let mut proteins: Vec<(String, String)> = Vec::new();
    for (name, bases) in rows {
        let up = bases.to_ascii_uppercase();
        if up.len() % 3 != 0 || !up.is_ascii() {
            return Err(Error::InvalidInput(format!(
                "The length of '{}' is not a multiple of 3",
                name
            )));
        }
        let cs: Vec<String> = up
            .as_bytes()
            .chunks(3)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect();
        let prot: String = cs
            .iter()
            .map(|c| amino_acid_for(c.as_bytes()) as char)
            .collect();
        codons.push(cs);
        proteins.push((name.clone(), prot));
    }
    let aligned = aligner(&proteins)?;
    let mut out = Vec::new();
    for (idx, (name, _)) in rows.iter().enumerate() {
        let Some((_, line)) = aligned.iter().find(|(n, _)| n == name) else {
            return Err(Error::InvalidInput(format!(
                "The alignment does not contain row '{}'",
                name
            )));
        };
        let mut next = 0;
        let mut dna = String::new();
        for c in line.chars() {
            if c == '-' {
                dna.push_str("---");
            } else {
                let Some(codon) = codons[idx].get(next) else {
                    return Err(Error::InvalidInput(format!(
                        "Row '{}' does not match the protein",
                        name
                    )));
                };
                dna.push_str(codon);
                next += 1;
            }
        }
        if next != codons[idx].len() {
            return Err(Error::InvalidInput(format!(
                "Row '{}' does not match the protein",
                name
            )));
        }
        out.push((name.clone(), dna));
    }
    Ok(out)
}

/// Richtet den Spaltenbereich `[from, to)` für die Zeilen `selected` (Indizes) neu aus.
/// * Für jede gewählte Zeile wird das Stück `rows[i][from..to]` ohne `-` genommen und die Liste (in der Reihenfolge von `selected`)
///   an `aligner` gegeben, der gleich lange ausgerichtete Zeilen (Breite `w`) in derselben Reihenfolge liefert.
/// * Neue Breite des Bereichs `W = max(w, to − from)`. Gewählte Zeilen bekommen ihr ausgerichtetes Stück rechts mit `-` auf `W` aufgefüllt;
///   nicht gewählte Zeilen behalten ihr Stück `rows[i][from..to]` und werden rechts mit `-` auf `W` aufgefüllt.
/// * Ergebnis je Zeile: `rows[i][..from] + Stück + rows[i][to..]`.
///
/// Fehler `Error::InvalidInput`: `rows` leer, Zeilen verschieden lang, `selected` leer, ein Index ungültig oder doppelt;
/// `Error::OutOfRange`: `from >= to` oder `to` größer als die Spaltenzahl. `aligner` muss gleich viele Zeilen zurückgeben, sonst `Error::InvalidInput`.
pub fn realign_region(
    rows: &[String],
    from: usize,
    to: usize,
    selected: &[usize],
    aligner: &dyn Fn(&[String]) -> Result<Vec<String>>,
) -> Result<Vec<String>> {
    let width = check_group(rows, "the alignment")?;
    if selected.is_empty() {
        return Err(Error::InvalidInput("No rows selected".to_string()));
    }
    for (k, &i) in selected.iter().enumerate() {
        if i >= rows.len() || selected[..k].contains(&i) {
            return Err(Error::InvalidInput(format!("Invalid row index {}", i)));
        }
    }
    if from >= to || to > width {
        return Err(Error::OutOfRange(format!(
            "Range {}..{} is not within 0..{}",
            from, to, width
        )));
    }
    let chars: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let pieces: Vec<String> = selected
        .iter()
        .map(|&i| chars[i][from..to].iter().filter(|c| **c != '-').collect())
        .collect();
    let aligned = aligner(&pieces)?;
    if aligned.len() != pieces.len() {
        return Err(Error::InvalidInput(
            "The aligner returned the wrong number of rows".to_string(),
        ));
    }
    let w = aligned.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let region = w.max(to - from);
    let mut out = Vec::new();
    for (i, row) in chars.iter().enumerate() {
        let mut piece: String = match selected.iter().position(|&s| s == i) {
            Some(k) => aligned[k].clone(),
            None => row[from..to].iter().collect(),
        };
        while piece.chars().count() < region {
            piece.push('-');
        }
        let head: String = row[..from].iter().collect();
        let tail: String = row[to..].iter().collect();
        out.push(format!("{}{}{}", head, piece, tail));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn named(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn spaltenwert() {
        assert_eq!(pair_score('A', 'A'), 2.0);
        assert_eq!(pair_score('a', 'A'), 2.0);
        assert_eq!(pair_score('A', 'C'), -1.0);
        assert_eq!(pair_score('A', '-'), -1.0);
        assert_eq!(pair_score('-', '-'), 0.0);
    }

    #[test]
    fn profile_verbinden() {
        assert_eq!(
            align_profiles(&v(&["ACGT", "ACGT"]), &v(&["AGT"])).unwrap(),
            v(&["ACGT", "ACGT", "A-GT"])
        );
        assert_eq!(
            align_profiles(&v(&["ACT"]), &v(&["ACGT"])).unwrap(),
            v(&["AC-T", "ACGT"])
        );
        assert_eq!(
            align_profiles(&v(&["AC-T", "ACGT"]), &v(&["ACGT"])).unwrap(),
            v(&["AC-T", "ACGT", "ACGT"])
        );
        assert_eq!(
            align_profiles(&v(&["AAAA"]), &v(&["CCCC"])).unwrap(),
            v(&["AAAA", "CCCC"])
        );
        assert_eq!(
            align_profiles(&v(&["ACGTACGT", "ACGAACGT"]), &v(&["ACGTCGT", "ACGTCGT"])).unwrap(),
            v(&["ACGTACGT", "ACGAACGT", "ACGT-CGT", "ACGT-CGT"])
        );
    }

    #[test]
    fn profile_ohne_ungappte_aenderung() {
        let a = v(&["AC-GT", "ACAGT"]);
        let b = v(&["A-GT", "ATGT"]);
        let r = align_profiles(&a, &b).unwrap();
        assert_eq!(r.len(), 4);
        let w = r[0].len();
        assert!(r.iter().all(|x| x.len() == w));
        for (orig, res) in a.iter().chain(b.iter()).zip(&r) {
            assert_eq!(res.replace('-', ""), orig.replace('-', ""));
        }
    }

    #[test]
    fn profile_fehler() {
        assert!(matches!(
            align_profiles(&[], &v(&["AC"])),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            align_profiles(&v(&["AC", "A"]), &v(&["AC"])),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn translation_alignment() {
        let rows = named(&[("a", "ATGAAATAG"), ("b", "ATGAAGTAG")]);
        let aligner = |p: &[(String, String)]| -> Result<Vec<(String, String)>> {
            assert_eq!(p[0].1, "MK*");
            Ok(vec![
                ("b".to_string(), "M-K*".to_string()),
                ("a".to_string(), "MK-*".to_string()),
            ])
        };
        let r = translation_align(&rows, &aligner).unwrap();
        assert_eq!(r[0], ("a".to_string(), "ATGAAA---TAG".to_string()));
        assert_eq!(r[1], ("b".to_string(), "ATG---AAGTAG".to_string()));
    }

    #[test]
    fn translation_fehler() {
        let ident = |p: &[(String, String)]| -> Result<Vec<(String, String)>> { Ok(p.to_vec()) };
        let e = translation_align(&named(&[("kurz", "ATGA")]), &ident).unwrap_err();
        assert!(e.to_string().contains("kurz"));
        assert!(translation_align(&[], &ident).is_err());
        let falsch = |_: &[(String, String)]| -> Result<Vec<(String, String)>> {
            Ok(vec![("a".to_string(), "MK".to_string())])
        };
        assert!(translation_align(&named(&[("a", "ATGAAATAG")]), &falsch).is_err());
    }

    #[test]
    fn bereich_neu_ausrichten() {
        let rows = v(&["AC-GT", "ACGGT", "AAAAA"]);
        let pad = |s: &[String]| -> Result<Vec<String>> {
            let w = s.iter().map(|x| x.len()).max().unwrap_or(0);
            Ok(s.iter().map(|x| format!("{x:-<w$}")).collect())
        };
        let r = realign_region(&rows, 1, 4, &[0, 1], &pad).unwrap();
        assert_eq!(r, v(&["ACG-T", "ACGGT", "AAAAA"]));
        let wide = |s: &[String]| -> Result<Vec<String>> {
            Ok(s.iter().map(|x| format!("{x:-<5}")).collect())
        };
        let r = realign_region(&rows, 1, 4, &[0, 1], &wide).unwrap();
        assert_eq!(r, v(&["ACG---T", "ACGG--T", "AAAA--A"]));
    }

    #[test]
    fn bereich_fehler() {
        let rows = v(&["ACGT", "ACGT"]);
        let ident = |s: &[String]| -> Result<Vec<String>> { Ok(s.to_vec()) };
        assert!(matches!(
            realign_region(&rows, 2, 2, &[0], &ident),
            Err(Error::OutOfRange(_))
        ));
        assert!(matches!(
            realign_region(&rows, 1, 9, &[0], &ident),
            Err(Error::OutOfRange(_))
        ));
        assert!(matches!(
            realign_region(&rows, 0, 2, &[], &ident),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            realign_region(&rows, 0, 2, &[5], &ident),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            realign_region(&rows, 0, 2, &[1, 1], &ident),
            Err(Error::InvalidInput(_))
        ));
    }
}
