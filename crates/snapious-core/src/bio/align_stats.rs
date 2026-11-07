//! Alignment metrics: consensus, conservation, entropy, identity matrix, sequence logo, column removal.
//!
//! An alignment here is a list of equal-length rows (`&[String]`); `-` and `.` are gaps.
//! Case is ignored (uppercased internally, `U` is left unchanged).

use crate::error::{Error, Result};
use std::collections::BTreeMap;

/// One column of a sequence logo.
#[derive(Clone, Debug, PartialEq)]
pub struct LogoColumn {
    /// Symbols with their heights in bits, ascending by height, alphabetical on ties.
    /// Only symbols that occur in the column.
    pub symbols: Vec<(char, f64)>,
}

/// Checks that all rows have the same length (in characters); error `Error::InvalidInput`.
/// Returns the number of columns (0 if there are no rows).
pub fn column_count(rows: &[String]) -> Result<usize> {
    if rows.is_empty() {
        return Ok(0);
    }
    let len = match rows.first() {
        Some(r) => r.chars().count(),
        None => 0,
    };
    if rows.iter().any(|r| r.chars().count() != len) {
        return Err(Error::InvalidInput(
            "Rows have different lengths".into(),
        ));
    }
    Ok(len)
}

/// IUPAC code for a set of bases (order, repeats and case do not matter):
/// single → the base itself; {A,G}→R, {C,T}→Y, {G,T}→K, {A,C}→M, {C,G}→S, {A,T}→W, {C,G,T}→B, {A,G,T}→D, {A,C,T}→H,
/// {A,C,G}→V, all four → N. If the set contains a character other than A, C, G, T → `X`. Empty set → `-`.
pub fn iupac_code(bases: &[char]) -> char {
    if bases.is_empty() {
        return '-';
    }
    let mut unique: BTreeMap<char, bool> = BTreeMap::new();
    for &c in bases {
        let up = c.to_ascii_uppercase();
        match up {
            'A' | 'C' | 'G' | 'T' => {
                unique.insert(up, true);
            }
            _ => return 'X',
        }
    }
    if unique.is_empty() {
        return '-';
    }
    let keys: Vec<char> = unique.keys().copied().collect();
    match keys.as_slice() {
        ['A'] => 'A',
        ['C'] => 'C',
        ['G'] => 'G',
        ['T'] => 'T',
        ['A', 'G'] => 'R',
        ['C', 'T'] => 'Y',
        ['G', 'T'] => 'K',
        ['A', 'C'] => 'M',
        ['C', 'G'] => 'S',
        ['A', 'T'] => 'W',
        ['C', 'G', 'T'] => 'B',
        ['A', 'G', 'T'] => 'D',
        ['A', 'C', 'T'] => 'H',
        ['A', 'C', 'G'] => 'V',
        ['A', 'C', 'G', 'T'] => 'N',
        _ => 'X',
    }
}

/// Consensus sequence, one character per column.
/// - The non-gap symbols of the column are counted. A pure gap column → `-`.
/// - Denominator of the fraction: the number of non-gap rows if `ignore_gaps`, otherwise the number of all rows.
/// - The most frequent symbol (alphabetically smallest on ties) is taken if its fraction is `>= threshold`.
///   Otherwise the `iupac_code` of the symbols present in the column (for proteins, i.e. if a symbol outside ACGT occurs, this gives `X`).
pub fn consensus(rows: &[String], threshold: f64, ignore_gaps: bool) -> Result<String> {
    let len = column_count(rows)?;
    if len == 0 {
        return Ok(String::new());
    }

    let char_rows: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let num_rows = char_rows.len();

    let mut result = String::new();
    for i in 0..len {
        let mut counts: BTreeMap<char, usize> = BTreeMap::new();
        let mut non_gap_count: usize = 0;

        for r in &char_rows {
            if let Some(&c) = r.get(i) {
                let up = c.to_ascii_uppercase();
                if up == '-' || up == '.' {
                    continue;
                }
                non_gap_count += 1;
                *counts.entry(up).or_insert(0) += 1;
            }
        }

        if non_gap_count == 0 {
            result.push('-');
            continue;
        }

        let denom = if ignore_gaps { non_gap_count } else { num_rows };
        let mut best_sym = ' ';
        let mut best_freq: usize = 0;

        for (&sym, &freq) in counts.iter() {
            if freq > best_freq || (freq == best_freq && sym < best_sym) {
                best_freq = freq;
                best_sym = sym;
            }
        }

        if (best_freq as f64) / (denom as f64) >= threshold {
            result.push(best_sym);
        } else {
            let syms: Vec<char> = counts.keys().copied().collect();
            result.push(iupac_code(&syms));
        }
    }
    Ok(result)
}

/// Identity per column: fraction of non-gap rows carrying the most frequent non-gap symbol (0 to 1);
/// pure gap column → 0.0.
pub fn column_identity(rows: &[String]) -> Result<Vec<f64>> {
    let len = column_count(rows)?;
    if len == 0 {
        return Ok(Vec::new());
    }

    let char_rows: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let mut identities = Vec::with_capacity(len);

    for i in 0..len {
        let mut counts: BTreeMap<char, usize> = BTreeMap::new();
        let mut non_gap_count: usize = 0;

        for r in &char_rows {
            if let Some(&c) = r.get(i) {
                let up = c.to_ascii_uppercase();
                if up == '-' || up == '.' {
                    continue;
                }
                non_gap_count += 1;
                *counts.entry(up).or_insert(0) += 1;
            }
        }

        if non_gap_count == 0 {
            identities.push(0.0);
        } else {
            let max_freq = counts
                .values()
                .copied()
                .fold(0usize, |a, b| if a > b { a } else { b });
            identities.push(max_freq as f64 / non_gap_count as f64);
        }
    }
    Ok(identities)
}

/// Shannon entropy per column in bits over the non-gap symbols (`−Σ p·log2 p`); pure gap column → 0.0.
pub fn column_entropy_bits(rows: &[String]) -> Result<Vec<f64>> {
    let len = column_count(rows)?;
    if len == 0 {
        return Ok(Vec::new());
    }

    let char_rows: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let mut entropies = Vec::with_capacity(len);

    for i in 0..len {
        let mut counts: BTreeMap<char, usize> = BTreeMap::new();
        let mut non_gap_count: usize = 0;

        for r in &char_rows {
            if let Some(&c) = r.get(i) {
                let up = c.to_ascii_uppercase();
                if up == '-' || up == '.' {
                    continue;
                }
                non_gap_count += 1;
                *counts.entry(up).or_insert(0) += 1;
            }
        }

        if non_gap_count == 0 {
            entropies.push(0.0);
        } else {
            let mut h: f64 = 0.0;
            for &freq in counts.values() {
                let p = freq as f64 / non_gap_count as f64;
                if p > 0.0 {
                    h -= p * p.log2();
                }
            }
            entropies.push(h);
        }
    }
    Ok(entropies)
}

/// Pairwise identity in percent: number of columns where neither row has a gap and both are equal, divided by the
/// number of columns where neither has a gap, times 100. If there is no such column, 0.0. The diagonal is always 100.0.
pub fn identity_matrix_percent(rows: &[String]) -> Result<Vec<Vec<f64>>> {
    let len = column_count(rows)?;
    if len == 0 || rows.is_empty() {
        return Ok(Vec::new());
    }

    let char_rows: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let num_rows = char_rows.len();

    let mut matrix = vec![vec![0.0; num_rows]; num_rows];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = 100.0;
    }

    for i in 0..num_rows {
        for j in (i + 1)..num_rows {
            let mut matches: usize = 0;
            let mut total_non_gap_pairs: usize = 0;

            for col in 0..len {
                if let (Some(&a), Some(&b)) = (char_rows[i].get(col), char_rows[j].get(col)) {
                    let ua = a.to_ascii_uppercase();
                    let ub = b.to_ascii_uppercase();
                    if ua != '-' && ua != '.' && ub != '-' && ub != '.' {
                        total_non_gap_pairs += 1;
                        if ua == ub {
                            matches += 1;
                        }
                    }
                }
            }

            let percent = if total_non_gap_pairs == 0 {
                0.0
            } else {
                (matches as f64 / total_non_gap_pairs as f64) * 100.0
            };
            matrix[i][j] = percent;
            matrix[j][i] = percent;
        }
    }
    Ok(matrix)
}

/// Sequence logo after Schneider & Stephens (1990). `s` = 4 (DNA) or 20 (`protein == true`), `n` = number of non-gap symbols in the column,
/// `H` = entropy in bits, `e = (s − 1) / (2 · ln 2 · n)` if `small_sample_correction` and `n > 0`, otherwise 0.
/// Information content `R = max(0, log2(s) − (H + e))`; height of a symbol = frequency · `R`. Pure gap column → `symbols` empty.
pub fn sequence_logo(
    rows: &[String],
    protein: bool,
    small_sample_correction: bool,
) -> Result<Vec<LogoColumn>> {
    let len = column_count(rows)?;
    if len == 0 {
        return Ok(Vec::new());
    }

    let char_rows: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let s: f64 = if protein { 20.0 } else { 4.0 };
    let log2_s = s.log2();

    let mut logo = Vec::with_capacity(len);
    for i in 0..len {
        let mut counts: BTreeMap<char, usize> = BTreeMap::new();
        let mut non_gap_count: usize = 0;

        for r in &char_rows {
            if let Some(&c) = r.get(i) {
                let up = c.to_ascii_uppercase();
                if up == '-' || up == '.' {
                    continue;
                }
                non_gap_count += 1;
                *counts.entry(up).or_insert(0) += 1;
            }
        }

        if non_gap_count == 0 {
            logo.push(LogoColumn {
                symbols: Vec::new(),
            });
            continue;
        }

        let mut h: f64 = 0.0;
        for &freq in counts.values() {
            let p = freq as f64 / non_gap_count as f64;
            if p > 0.0 {
                h -= p * p.log2();
            }
        }

        let e = if small_sample_correction && non_gap_count > 0 {
            (s - 1.0) / (2.0 * non_gap_count as f64 * 2f64.ln())
        } else {
            0.0
        };

        let r_val = (log2_s - (h + e)).max(0.0);

        let mut symbols: Vec<(char, f64)> = counts
            .iter()
            .map(|(&sym, &freq)| {
                let p = freq as f64 / non_gap_count as f64;
                (sym, p * r_val)
            })
            .collect();

        symbols.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        logo.push(LogoColumn { symbols });
    }
    Ok(logo)
}

/// Removes columns that contain at least one gap and whose gap fraction (gaps / row count) is `>= gap_fraction_threshold`.
/// A threshold of 1.0 removes only pure gap columns; gap-free columns always stay. Returns the new rows.
pub fn strip_columns(rows: &[String], gap_fraction_threshold: f64) -> Result<Vec<String>> {
    let len = column_count(rows)?;
    if len == 0 || rows.is_empty() {
        return Ok(Vec::new());
    }

    let char_rows: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let num_rows = char_rows.len();

    let mut keep_cols: Vec<usize> = Vec::new();
    for i in 0..len {
        let mut gap_count: usize = 0;
        for r in &char_rows {
            if let Some(&c) = r.get(i) {
                let up = c.to_ascii_uppercase();
                if up == '-' || up == '.' {
                    gap_count += 1;
                }
            }
        }

        let gap_fraction = if num_rows == 0 {
            0.0
        } else {
            gap_count as f64 / num_rows as f64
        };
        if gap_count > 0 && gap_fraction >= gap_fraction_threshold {
            continue;
        }
        keep_cols.push(i);
    }

    let mut result = Vec::with_capacity(num_rows);
    for r in &char_rows {
        let new_row: String = keep_cols
            .iter()
            .filter_map(|&i| r.get(i).copied())
            .collect();
        result.push(new_row);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| s.to_string()).collect()
    }

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn spaltenzahl() {
        assert_eq!(column_count(&rows(&["ACGT", "AC-T"])).unwrap(), 4);
        assert_eq!(column_count(&[]).unwrap(), 0);
        assert!(matches!(
            column_count(&rows(&["ACGT", "AC"])),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn iupac() {
        assert_eq!(iupac_code(&['A', 'G']), 'R');
        assert_eq!(iupac_code(&['g', 'a', 'a']), 'R');
        assert_eq!(iupac_code(&['C', 'T']), 'Y');
        assert_eq!(iupac_code(&['G', 'T']), 'K');
        assert_eq!(iupac_code(&['A', 'C']), 'M');
        assert_eq!(iupac_code(&['C', 'G']), 'S');
        assert_eq!(iupac_code(&['A', 'T']), 'W');
        assert_eq!(iupac_code(&['C', 'G', 'T']), 'B');
        assert_eq!(iupac_code(&['A', 'G', 'T']), 'D');
        assert_eq!(iupac_code(&['A', 'C', 'T']), 'H');
        assert_eq!(iupac_code(&['A', 'C', 'G']), 'V');
        assert_eq!(iupac_code(&['A', 'C', 'G', 'T']), 'N');
        assert_eq!(iupac_code(&['T']), 'T');
        assert_eq!(iupac_code(&['A', 'L']), 'X');
        assert_eq!(iupac_code(&[]), '-');
    }

    #[test]
    fn konsens() {
        let r = rows(&["ACGT", "ACGT", "ACGA"]);
        assert_eq!(consensus(&r, 0.7, true).unwrap(), "ACGW");
        assert_eq!(consensus(&r, 0.5, true).unwrap(), "ACGT");
        assert_eq!(consensus(&[], 0.5, true).unwrap(), "");
    }

    #[test]
    fn konsens_mit_luecken() {
        let r = rows(&["A-", "A-", "C-"]);
        assert_eq!(consensus(&r, 0.6, true).unwrap(), "A-");
        // without gap handling: fraction 2/3 stays (column 1 has no gaps)
        assert_eq!(consensus(&r, 0.6, false).unwrap(), "A-");
        let g = rows(&["AC", "A-", "A-", "A-"]);
        // Column 2: one C among four rows → ignoring: fraction 1/1 → C; otherwise 1/4 < 0.5 → ambiguity code of the single symbol → C
        assert_eq!(consensus(&g, 0.5, true).unwrap(), "AC");
        let p = rows(&["AL", "AK"]);
        assert_eq!(consensus(&p, 0.9, true).unwrap(), "AX");
    }

    #[test]
    fn identitaet_und_entropie() {
        let r = rows(&["AAA-", "AAC-", "ACG-", "A-T-", "A-A-"]);
        let id = column_identity(&r).unwrap();
        assert!(close(id[0], 1.0, 1e-9));
        assert!(close(id[1], 2.0 / 3.0, 1e-4));
        assert!(close(id[3], 0.0, 1e-9));
        let e = column_entropy_bits(&rows(&["A", "C", "G", "T"])).unwrap();
        assert!(close(e[0], 2.0, 1e-9));
        let e = column_entropy_bits(&rows(&["A", "A", "A", "A"])).unwrap();
        assert!(close(e[0], 0.0, 1e-9));
        let e = column_entropy_bits(&rows(&["-", "-"])).unwrap();
        assert!(close(e[0], 0.0, 1e-9));
        let single = column_identity(&rows(&["A", "-", "-"])).unwrap();
        assert!(close(single[0], 1.0, 1e-9));
    }

    #[test]
    fn identitaetsmatrix() {
        let m = identity_matrix_percent(&rows(&["ACGT", "ACGA", "AC-T"])).unwrap();
        assert_eq!(m.len(), 3);
        assert!(close(m[0][0], 100.0, 1e-9) && close(m[1][1], 100.0, 1e-9));
        assert!(close(m[0][1], 75.0, 1e-9));
        assert!(close(m[1][0], 75.0, 1e-9));
        assert!(close(m[0][2], 100.0, 1e-9));
        // no common non-gap column
        let n = identity_matrix_percent(&rows(&["A-", "-C"])).unwrap();
        assert!(close(n[0][1], 0.0, 1e-9));
        assert!(close(n[0][0], 100.0, 1e-9));
    }

    #[test]
    fn logo() {
        let gleich = sequence_logo(&rows(&["A", "C", "G", "T"]), false, false).unwrap();
        let summe: f64 = gleich[0].symbols.iter().map(|s| s.1).sum();
        assert!(close(summe, 0.0, 1e-9));
        assert_eq!(gleich[0].symbols.len(), 4);
        let einzeln = sequence_logo(&rows(&["A", "A", "A", "A"]), false, false).unwrap();
        assert_eq!(einzeln[0].symbols.len(), 1);
        assert_eq!(einzeln[0].symbols[0].0, 'A');
        assert!(close(einzeln[0].symbols[0].1, 2.0, 1e-9));
        let protein = sequence_logo(&rows(&["L", "L"]), true, false).unwrap();
        assert!(close(protein[0].symbols[0].1, 20f64.log2(), 1e-9));
        let korrigiert = sequence_logo(&rows(&["A", "A", "A", "A"]), false, true).unwrap();
        let e = 3.0 / (2.0 * 2f64.ln() * 4.0);
        assert!(close(korrigiert[0].symbols[0].1, 2.0 - e, 1e-9));
        let leer = sequence_logo(&rows(&["-", "-"]), false, false).unwrap();
        assert!(leer[0].symbols.is_empty());
    }

    #[test]
    fn logo_sortierung() {
        // 3 times A, 1 time C: H = 0.8113, R = 1.1887; heights 0.8915 (A) and 0.2972 (C), ascending
        let l = sequence_logo(&rows(&["A", "A", "A", "C"]), false, false).unwrap();
        assert_eq!(l[0].symbols[0].0, 'C');
        assert_eq!(l[0].symbols[1].0, 'A');
        assert!(close(
            l[0].symbols[1].1,
            0.75 * (2.0 - 0.8112781244591328),
            1e-9
        ));
    }

    #[test]
    fn spalten_entfernen() {
        let r = rows(&["A-CT", "A-CT", "A-CT", "A-C-"]);
        // Column 1: only gaps (fraction 1.0), column 3: one gap out of four (0.25)
        assert_eq!(
            strip_columns(&r, 1.0).unwrap(),
            rows(&["ACT", "ACT", "ACT", "AC-"])
        );
        assert_eq!(
            strip_columns(&r, 0.5).unwrap(),
            rows(&["ACT", "ACT", "ACT", "AC-"])
        );
        assert_eq!(
            strip_columns(&r, 0.25).unwrap(),
            rows(&["AC", "AC", "AC", "AC"])
        );
        assert_eq!(
            strip_columns(&r, 0.0).unwrap(),
            rows(&["AC", "AC", "AC", "AC"])
        );
        assert_eq!(strip_columns(&[], 0.5).unwrap(), Vec::<String>::new());
    }
}
