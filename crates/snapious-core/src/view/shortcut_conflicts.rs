//! Tastenkürzel vereinheitlichen und Doppelbelegungen finden (Kürzel-Übersicht, ⌘/).

/// Ein Kürzel mit Kennung und Tastenangabe wie `"cmd+shift+k"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub id: String,
    pub keys: String,
}

/// Vereinheitlicht eine Tastenangabe: Kleinschreibung, Teile an `+` trennen und trimmen. Modifikatoren (`cmd`/`command`,
/// `ctrl`/`control`, `alt`/`opt`/`option`, `shift`) werden zu `cmd`, `ctrl`, `alt`, `shift` vereinheitlicht, doppelte
/// zusammengefasst und in dieser Reihenfolge (cmd, ctrl, alt, shift) vor die Taste gesetzt; Ergebnis mit `+` verbunden.
/// Es muss genau eine Nicht-Modifikator-Taste (nicht leer) geben, sonst `None` (auch bei leerer Eingabe oder leeren Teilen).
pub fn normalize_keys(keys: &str) -> Option<String> {
    let parts: Vec<&str> = keys.split('+').collect();
    for part in &parts {
        if part.trim().is_empty() {
            return None;
        }
    }

    let mut modifiers = Vec::new();
    let mut key: Option<String> = None;

    for part in parts {
        let p = part.trim().to_lowercase();
        match p.as_str() {
            "command" | "cmd" => modifiers.push("cmd"),
            "control" | "ctrl" => modifiers.push("ctrl"),
            "alt" | "option" | "opt" => modifiers.push("alt"),
            "shift" => modifiers.push("shift"),
            other => {
                if key.is_some() {
                    return None;
                }
                key = Some(other.to_string());
            }
        }
    }

    let k = key?;

    let mut has = [false; 4];
    for m in &modifiers {
        match *m {
            "cmd" => has[0] = true,
            "ctrl" => has[1] = true,
            "alt" => has[2] = true,
            "shift" => has[3] = true,
            _ => {}
        }
    }

    let mut result = Vec::new();
    if has[0] {
        result.push("cmd");
    }
    if has[1] {
        result.push("ctrl");
    }
    if has[2] {
        result.push("alt");
    }
    if has[3] {
        result.push("shift");
    }
    result.push(&k);

    Some(result.join("+"))
}

/// Alle Paare `(i, j)` mit `i < j`, deren vereinheitlichte Tasten gleich sind (ungültige Angaben zählen nicht),
/// sortiert nach `i`, dann `j`.
pub fn find_conflicts(shortcuts: &[Shortcut]) -> Vec<(usize, usize)> {
    let mut conflicts = Vec::new();
    for (i, shortcut) in shortcuts.iter().enumerate() {
        if let Some(norm_i) = normalize_keys(&shortcut.keys) {
            for (j, other) in shortcuts.iter().enumerate().skip(i + 1) {
                if let Some(norm_j) = normalize_keys(&other.keys) {
                    if norm_i == norm_j {
                        conflicts.push((i, j));
                    }
                }
            }
        }
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(id: &str, keys: &str) -> Shortcut {
        Shortcut {
            id: id.to_string(),
            keys: keys.to_string(),
        }
    }

    #[test]
    fn vereinheitlichen() {
        assert_eq!(
            normalize_keys("Shift+Cmd+K").as_deref(),
            Some("cmd+shift+k")
        );
        assert_eq!(
            normalize_keys("option+ctrl+x").as_deref(),
            Some("ctrl+alt+x")
        );
        assert_eq!(normalize_keys(" command + k ").as_deref(), Some("cmd+k"));
        assert_eq!(normalize_keys("cmd+cmd+k").as_deref(), Some("cmd+k"));
        assert_eq!(normalize_keys("k").as_deref(), Some("k"));
        assert_eq!(normalize_keys("cmd+"), None);
        assert_eq!(normalize_keys("cmd"), None);
        assert_eq!(normalize_keys("k+j"), None);
        assert_eq!(normalize_keys(""), None);
    }

    #[test]
    fn konflikte() {
        let list = vec![
            sc("a", "cmd+k"),
            sc("b", "Cmd+K"),
            sc("c", "cmd+j"),
            sc("d", "k+j"),
            sc("e", "shift+cmd+k"),
        ];
        assert_eq!(find_conflicts(&list), vec![(0, 1)]);
        let drei = vec![sc("a", "cmd+k"), sc("b", "cmd+k"), sc("c", "CMD+K")];
        assert_eq!(find_conflicts(&drei), vec![(0, 1), (0, 2), (1, 2)]);
        assert!(find_conflicts(&[]).is_empty());
    }
}
