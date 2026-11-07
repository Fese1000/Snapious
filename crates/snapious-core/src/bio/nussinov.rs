//! Simple folding prediction after Nussinov (maximum number of base pairs).

/// Finds the maximum number of non-crossing base pairs and returns `(count, dot-bracket notation)`. Allowed pairs
/// (case-insensitive, `T` as `U`): A–U, U–A, G–C, C–G, G–U, U–G. Between the paired positions `i < j` there must be at least
/// `min_loop` bases (`j - i > min_loop`). Recursion for `N[i][j]` (range `i..=j`):
/// `N[i][j] = max(N[i+1][j], N[i][j-1], N[i+1][j-1] + 1 if i, j may pair, max over k in i..j of N[i][k] + N[k+1][j])`; for `j <= i` it is 0.
/// Traceback for the range `(i, j)` in this order of cases: (1) `N[i][j] == N[i+1][j]` → `i` unpaired; (2) otherwise
/// `N[i][j] == N[i][j-1]` → `j` unpaired; (3) otherwise, if `i`, `j` may pair and `N[i][j] == N[i+1][j-1] + 1` → pair `(i, j)`;
/// (4) otherwise split at the smallest `k` with `N[i][j] == N[i][k] + N[k+1][j]`. Paired positions become `(` and `)`, all others `.`.
/// Empty sequence → `(0, "")`. Characters other than A, C, G, T, U never pair.
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
    // t[i][j] = best pair count in range i..=j; for j <= i it is 0
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
        // loop too short: GC with only one base between must not pair at min_loop 3
        assert_eq!(max_pairs("GAC", 3).0, 0);
        assert_eq!(max_pairs("GAC", 0), (1, "(.)".to_string()));
        // arbitrary inputs give balanced brackets whose count equals the pair count
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
