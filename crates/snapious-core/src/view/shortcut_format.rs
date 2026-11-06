//! Tastenkürzel für die Anzeige im macOS-Stil schreiben.

use crate::view::shortcut_conflicts::normalize_keys;

/// Wandelt `"cmd+shift+k"` in `"⇧⌘K"` um. Zuerst `normalize_keys` (bei `None` kommt `keys` unverändert zurück). Reihenfolge der
/// Zeichen wie unter macOS: Control `⌃`, Option `⌥`, Umschalt `⇧`, Befehl `⌘`, dann die Taste. Die Taste wird großgeschrieben;
/// besondere Tasten: `return` → `↩`, `enter` → `⌅`, `delete` → `⌫`, `esc` oder `escape` → `⎋`, `tab` → `⇥`, `space` → `␣`,
/// `left` → `←`, `right` → `→`, `up` → `↑`, `down` → `↓`. Beispiel `ctrl+alt+delete` → `⌃⌥⌫`.
pub fn format_shortcut(keys: &str) -> String {
    let Some(norm) = normalize_keys(keys) else {
        return keys.to_string();
    };
    let mut out = String::new();
    let mut key = String::new();
    for part in norm.split('+') {
        match part {
            "ctrl" | "alt" | "shift" | "cmd" => {}
            other => key = other.to_string(),
        }
    }
    let parts: Vec<&str> = norm.split('+').collect();
    for (name, symbol) in [("ctrl", "⌃"), ("alt", "⌥"), ("shift", "⇧"), ("cmd", "⌘")] {
        // die Taste selbst kann ebenfalls so heißen, wenn sie die letzte ist; Modifikatoren stehen davor
        if parts[..parts.len() - 1].contains(&name) {
            out.push_str(symbol);
        }
    }
    out.push_str(match key.as_str() {
        "return" => "↩",
        "enter" => "⌅",
        "delete" => "⌫",
        "esc" | "escape" => "⎋",
        "tab" => "⇥",
        "space" => "␣",
        "left" => "←",
        "right" => "→",
        "up" => "↑",
        "down" => "↓",
        other => return format!("{out}{}", other.to_uppercase()),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beispiele() {
        assert_eq!(format_shortcut("cmd+shift+k"), "⇧⌘K");
        assert_eq!(format_shortcut("Shift+Cmd+K"), "⇧⌘K");
        assert_eq!(format_shortcut("ctrl+alt+delete"), "⌃⌥⌫");
        assert_eq!(format_shortcut("cmd+return"), "⌘↩");
        assert_eq!(format_shortcut("cmd+left"), "⌘←");
        assert_eq!(format_shortcut("escape"), "⎋");
        assert_eq!(format_shortcut("cmd+/"), "⌘/");
        assert_eq!(format_shortcut("ctrl+alt+shift+cmd+space"), "⌃⌥⇧⌘␣");
    }

    #[test]
    fn ungueltig() {
        assert_eq!(format_shortcut("cmd"), "cmd");
        assert_eq!(format_shortcut(""), "");
        assert_eq!(format_shortcut("a+b"), "a+b");
    }
}
