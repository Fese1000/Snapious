//! Write keyboard shortcuts for display in macOS style.

use crate::view::shortcut_conflicts::normalize_keys;

/// Converts `"cmd+shift+k"` into `"⇧⌘K"`. First `normalize_keys` (for `None`, `keys` is returned unchanged). Order of the
/// symbols as on macOS: Control `⌃`, Option `⌥`, Shift `⇧`, Command `⌘`, then the key. The key is capitalized;
/// special keys: `return` → `↩`, `enter` → `⌅`, `delete` → `⌫`, `esc` or `escape` → `⎋`, `tab` → `⇥`, `space` → `␣`,
/// `left` → `←`, `right` → `→`, `up` → `↑`, `down` → `↓`. Example `ctrl+alt+delete` → `⌃⌥⌫`.
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
        // the key itself may also have such a name if it is the last one; modifiers come before it
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
