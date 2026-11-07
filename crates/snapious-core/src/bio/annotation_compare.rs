//! Compare two sets of annotations: shared or only in one set.

/// An annotation (interval `[start, end)` and type).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: String,
}

/// When two annotations count as equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchMode {
    /// Same type, same start, same end.
    Exact,
    /// Same type and overlapping intervals (at least one shared position).
    Overlap,
}

/// Result: indices into the passed lists, each ascending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// Entries from `a` with no counterpart in `b`.
    pub only_a: Vec<usize>,
    /// Entries from `b` with no counterpart in `a`.
    pub only_b: Vec<usize>,
    /// Entries from `a` with at least one counterpart in `b`.
    pub common_a: Vec<usize>,
    /// Entries from `b` with at least one counterpart in `a`.
    pub common_b: Vec<usize>,
}

/// Compares `a` and `b` according to `mode`. Types are compared case-insensitively. Two intervals
/// overlap if `a.start < b.end && b.start < a.end`.
pub fn compare_sets(a: &[Span], b: &[Span], mode: MatchMode) -> Comparison {
    let mut common_a = Vec::new();
    let mut only_a = Vec::new();
    let mut matched_b = vec![false; b.len()];

    for (i, span_a) in a.iter().enumerate() {
        let mut found = false;
        for (j, span_b) in b.iter().enumerate() {
            if matches_span(span_a, span_b, mode) {
                found = true;
                matched_b[j] = true;
            }
        }
        if found {
            common_a.push(i);
        } else {
            only_a.push(i);
        }
    }

    let mut common_b = Vec::new();
    let mut only_b = Vec::new();
    for (j, _span_b) in b.iter().enumerate() {
        if matched_b[j] {
            common_b.push(j);
        } else {
            only_b.push(j);
        }
    }

    Comparison {
        only_a,
        only_b,
        common_a,
        common_b,
    }
}

fn matches_span(a: &Span, b: &Span, mode: MatchMode) -> bool {
    if !a.kind.eq_ignore_ascii_case(&b.kind) {
        return false;
    }
    match mode {
        MatchMode::Exact => a.start == b.start && a.end == b.end,
        MatchMode::Overlap => a.start < b.end && b.start < a.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(start: usize, end: usize, kind: &str) -> Span {
        Span {
            start,
            end,
            kind: kind.to_string(),
        }
    }

    #[test]
    fn genau() {
        let a = vec![s(0, 10, "gene"), s(20, 30, "CDS"), s(40, 50, "gene")];
        let b = vec![s(0, 10, "Gene"), s(20, 31, "CDS"), s(60, 70, "gene")];
        let r = compare_sets(&a, &b, MatchMode::Exact);
        assert_eq!(r.common_a, vec![0]);
        assert_eq!(r.common_b, vec![0]);
        assert_eq!(r.only_a, vec![1, 2]);
        assert_eq!(r.only_b, vec![1, 2]);
    }

    #[test]
    fn ueberlappung() {
        let a = vec![s(0, 10, "gene"), s(20, 30, "CDS"), s(40, 50, "gene")];
        let b = vec![
            s(5, 15, "gene"),
            s(30, 40, "CDS"),
            s(45, 46, "gene"),
            s(48, 60, "promoter"),
        ];
        let r = compare_sets(&a, &b, MatchMode::Overlap);
        assert_eq!(r.common_a, vec![0, 2]);
        assert_eq!(r.common_b, vec![0, 2]);
        assert_eq!(r.only_a, vec![1]);
        assert_eq!(r.only_b, vec![1, 3]);
        let leer = compare_sets(&[], &b, MatchMode::Overlap);
        assert_eq!(leer.only_b, vec![0, 1, 2, 3]);
        assert!(leer.common_a.is_empty());
    }
}
