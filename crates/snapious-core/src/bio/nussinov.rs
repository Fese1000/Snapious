//! Einfache Faltungsvorhersage nach Nussinov (größtmögliche Zahl von Basenpaaren).

/// Findet die größtmögliche Zahl nicht kreuzender Basenpaare und gibt `(Zahl, Punkt-Klammer-Schreibweise)` zurück. Erlaubte Paare
/// (Groß-/Kleinschreibung egal, `T` wie `U`): A–U, U–A, G–C, C–G, G–U, U–G. Zwischen den gepaarten Positionen `i < j` müssen mindestens
/// `min_loop` Basen liegen (`j - i > min_loop`). Rekursion für `N[i][j]` (Bereich `i..=j`):
/// `N[i][j] = max(N[i+1][j], N[i][j-1], N[i+1][j-1] + 1 wenn i, j paaren dürfen, max über k in i..j von N[i][k] + N[k+1][j])`; für `j <= i` gilt 0.
/// Rückverfolgung für den Bereich `(i, j)` in dieser Reihenfolge der Fälle: (1) `N[i][j] == N[i+1][j]` → `i` ungepaart; (2) sonst
/// `N[i][j] == N[i][j-1]` → `j` ungepaart; (3) sonst, wenn `i`, `j` paaren dürfen und `N[i][j] == N[i+1][j-1] + 1` → Paar `(i, j)`;
/// (4) sonst das kleinste `k` mit `N[i][j] == N[i][k] + N[k+1][j]` aufteilen. Gepaarte Positionen werden `(` und `)`, alle anderen `.`.
/// Leere Sequenz → `(0, "")`. Zeichen außer A, C, G, T, U paaren nie.
pub fn max_pairs(seq: &str, min_loop: usize) -> (usize, String) {
    let s: Vec<u8> = seq
        .bytes()
        .map(|b| match b.to_ascii_uppercase() {
            b'T' => b'U',
            o => o,
        })
        .collect();
    let n = s.len();
    if n == 0 {
        return (0, String::new());
    }
    let can = |i: usize, j: usize| {
        matches!(
            (s[i], s[j]),
            (b'A', b'U') | (b'U', b'A') | (b'G', b'C') | (b'C', b'G') | (b'G', b'U') | (b'U', b'G')
        )
    };
    // t[i][j] = beste Paarzahl im Bereich i..=j; für j <= i gilt 0
    let mut t = vec![vec![0usize; n]; n];
    for len in 1..n {
        for i in 0..n - len {
            let j = i + len;
            let mut best = t[i + 1][j].max(t[i][j - 1]);
            if can(i, j) && j - i > min_loop {
                let inner = if i < j - 1 { t[i + 1][j - 1] } else { 0 };
                best = best.max(inner + 1);
            }
            for k in i..j {
                best = best.max(t[i][k] + t[k + 1][j]);
            }
            t[i][j] = best;
        }
    }
    let get = |i: usize, j: usize| {
        if i > j || i >= n || j >= n {
            0
        } else {
            t[i][j]
        }
    };
    let mut out = vec![b'.'; n];
    let mut stack = vec![(0usize, n - 1)];
    while let Some((i, j)) = stack.pop() {
        if j <= i {
            continue;
        }
        let v = t[i][j];
        if v == get(i + 1, j) {
            stack.push((i + 1, j));
        } else if v == get(i, j - 1) {
            stack.push((i, j - 1));
        } else if can(i, j) && j - i > min_loop && v == get(i + 1, j.saturating_sub(1)) + 1 {
            out[i] = b'(';
            out[j] = b')';
            if i + 1 < j {
                stack.push((i + 1, j - 1));
            }
        } else if let Some(k) = (i..j).find(|&k| v == get(i, k) + get(k + 1, j)) {
            stack.push((i, k));
            stack.push((k + 1, j));
        }
    }
    let pairs = out.iter().filter(|b| **b == b'(').count();
    (pairs, String::from_utf8_lossy(&out).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haarnadel() {
        assert_eq!(max_pairs("GGGAAACCC", 3), (3, "(((...)))".to_string()));
        assert_eq!(max_pairs("AAAA", 3), (0, "....".to_string()));
        assert_eq!(max_pairs("", 3), (0, String::new()));
        assert_eq!(max_pairs("ggguuuccc", 3), (3, "(((...)))".to_string()));
    }

    #[test]
    fn schleife_und_eigenschaften() {
        // zu kurze Schleife: GC mit nur einer Base dazwischen darf bei min_loop 3 nicht paaren
        assert_eq!(max_pairs("GAC", 3).0, 0);
        assert_eq!(max_pairs("GAC", 0), (1, "(.)".to_string()));
        // beliebige Eingaben ergeben ausgeglichene Klammern, deren Zahl der Paarzahl entspricht
        for s in [
            "GCGCAAUGCGC",
            "AUGCAUGCAUGC",
            "GGGGCCCCUUUUAAAA",
            "NNNN",
            "GCAUGCAUGCAUGCAU",
        ] {
            let (n, d) = max_pairs(s, 3);
            assert_eq!(d.len(), s.len());
            let mut depth = 0i32;
            let mut opens = 0;
            for c in d.chars() {
                match c {
                    '(' => {
                        depth += 1;
                        opens += 1;
                    }
                    ')' => depth -= 1,
                    _ => {}
                }
                assert!(depth >= 0, "{s} {d}");
            }
            assert_eq!(depth, 0, "{s} {d}");
            assert_eq!(opens, n, "{s} {d}");
        }
    }
}
