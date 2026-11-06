//! Liste der zuletzt geöffneten Projekte.

/// Setzt `path` an den Anfang der Liste: ein vorhandener gleicher Eintrag (genau gleicher Text) wird zuvor entfernt, danach wird die
/// Liste auf höchstens `max` Einträge gekürzt (die ältesten fallen weg). `max == 0` → leere Liste. Leerer `path` ändert die Liste nicht.
pub fn add_recent(list: &[String], path: &str, max: usize) -> Vec<String> {
    if path.is_empty() {
        return list.to_vec();
    }

    if max == 0 {
        return vec![];
    }

    let mut result: Vec<String> = list
        .iter()
        .filter(|p| p.as_str() != path)
        .cloned()
        .collect();

    result.insert(0, path.to_string());

    if result.len() > max {
        result.truncate(max);
    }

    result
}

/// Entfernt alle Einträge, für die `exists` falsch liefert; die Reihenfolge der übrigen bleibt.
pub fn remove_missing(list: &[String], exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    list.iter()
        .filter(|p| exists(p.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn hinzufuegen() {
        assert_eq!(
            add_recent(&v(&["a", "b", "c"]), "d", 3),
            v(&["d", "a", "b"])
        );
        assert_eq!(
            add_recent(&v(&["a", "b", "c"]), "b", 3),
            v(&["b", "a", "c"])
        );
        assert_eq!(add_recent(&[], "x", 5), v(&["x"]));
        assert_eq!(add_recent(&v(&["a"]), "", 5), v(&["a"]));
        assert!(add_recent(&v(&["a"]), "b", 0).is_empty());
        assert_eq!(add_recent(&v(&["a", "b"]), "c", 1), v(&["c"]));
    }

    #[test]
    fn fehlende() {
        let r = remove_missing(&v(&["/a", "/b", "/c"]), &|p| p != "/b");
        assert_eq!(r, v(&["/a", "/c"]));
        assert_eq!(remove_missing(&v(&["/a"]), &|_| true), v(&["/a"]));
        assert!(remove_missing(&[], &|_| false).is_empty());
    }
}
