//! Feature table: filter and sort rows.

/// A row of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureRow {
    pub feature_type: String,
    pub name: String,
    pub start: usize,
    pub end: usize,
}

/// Column to sort by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortColumn {
    Type,
    Name,
    Start,
    End,
    /// `end - start` (0 if `end < start`).
    Length,
}

/// Indices of the rows that match the filter, in the order of `rows`. `type_filter` empty = any type, otherwise the type must be equal
/// ignoring case. `text` empty = any name, otherwise the name must contain the text
/// ignoring case.
pub fn filter_features(rows: &[FeatureRow], type_filter: &str, text: &str) -> Vec<usize> {
    let t = type_filter.to_lowercase();
    let q = text.to_lowercase();
    rows.iter()
        .enumerate()
        .filter(|(_, r)| {
            (t.is_empty() || r.feature_type.to_lowercase() == t)
                && (q.is_empty() || r.name.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Sorts the indices `indices` (rows from `rows`) by `column`, ascending or descending. Text is compared ignoring
/// case. Equal values keep their order from `indices` (stable sort, also when descending).
/// Indices outside `rows` are removed.
pub fn sort_features(
    rows: &[FeatureRow],
    indices: &[usize],
    column: SortColumn,
    ascending: bool,
) -> Vec<usize> {
    let mut idx: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|i| *i < rows.len())
        .collect();
    idx.sort_by(|&a, &b| {
        let (x, y) = (&rows[a], &rows[b]);
        let ord = match column {
            SortColumn::Type => x
                .feature_type
                .to_lowercase()
                .cmp(&y.feature_type.to_lowercase()),
            SortColumn::Name => x.name.to_lowercase().cmp(&y.name.to_lowercase()),
            SortColumn::Start => x.start.cmp(&y.start),
            SortColumn::End => x.end.cmp(&y.end),
            SortColumn::Length => x
                .end
                .saturating_sub(x.start)
                .cmp(&y.end.saturating_sub(y.start)),
        };
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<FeatureRow> {
        let r = |t: &str, n: &str, s: usize, e: usize| FeatureRow {
            feature_type: t.into(),
            name: n.into(),
            start: s,
            end: e,
        };
        vec![
            r("gene", "lacZ", 100, 400),
            r("CDS", "AmpR", 50, 950),
            r("promoter", "Plac", 10, 40),
            r("gene", "bla", 50, 350),
        ]
    }

    #[test]
    fn filtern() {
        let r = rows();
        assert_eq!(filter_features(&r, "", ""), vec![0, 1, 2, 3]);
        assert_eq!(filter_features(&r, "GENE", ""), vec![0, 3]);
        assert_eq!(filter_features(&r, "", "LA"), vec![0, 2, 3]);
        assert_eq!(filter_features(&r, "gene", "z"), vec![0]);
        assert!(filter_features(&r, "cds", "x").is_empty());
    }

    #[test]
    fn sortieren() {
        let r = rows();
        let all = [0usize, 1, 2, 3];
        assert_eq!(
            sort_features(&r, &all, SortColumn::Name, true),
            vec![1, 3, 0, 2]
        );
        assert_eq!(
            sort_features(&r, &all, SortColumn::Start, true),
            vec![2, 1, 3, 0]
        );
        assert_eq!(
            sort_features(&r, &all, SortColumn::Start, false),
            vec![0, 1, 3, 2]
        );
        assert_eq!(
            sort_features(&r, &all, SortColumn::Length, false),
            vec![1, 0, 3, 2]
        );
        assert_eq!(
            sort_features(&r, &all, SortColumn::Type, true),
            vec![1, 0, 3, 2]
        );
        assert_eq!(
            sort_features(&r, &all, SortColumn::End, true),
            vec![2, 3, 0, 1]
        );
        assert_eq!(
            sort_features(&r, &[3, 9, 0], SortColumn::Name, true),
            vec![3, 0]
        );
    }
}
