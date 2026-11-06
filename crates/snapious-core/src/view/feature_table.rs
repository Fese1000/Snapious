//! Feature-Tabelle: Zeilen filtern und sortieren.

/// Eine Zeile der Tabelle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureRow {
    pub feature_type: String,
    pub name: String,
    pub start: usize,
    pub end: usize,
}

/// Spalte, nach der sortiert wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortColumn {
    Type,
    Name,
    Start,
    End,
    /// `end - start` (bei `end < start` 0).
    Length,
}

/// Indizes der Zeilen, die zum Filter passen, in der Reihenfolge von `rows`. `type_filter` leer = jeder Typ, sonst muss der Typ
/// ohne Beachtung der Groß-/Kleinschreibung gleich sein. `text` leer = jeder Name, sonst muss der Name den Text ohne Beachtung der
/// Groß-/Kleinschreibung enthalten.
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

/// Sortiert die Indizes `indices` (Zeilen aus `rows`) nach `column`, aufsteigend oder absteigend. Text wird ohne Beachtung der
/// Groß-/Kleinschreibung verglichen. Gleiche Werte behalten ihre Reihenfolge aus `indices` (stabile Sortierung, auch bei absteigend).
/// Indizes außerhalb von `rows` werden entfernt.
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
