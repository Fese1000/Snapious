//! Verständliche Fehlermeldungen: übersetzt technische Meldungen in Titel und Lösungsvorschlag.

/// Titel und Hinweis für die Anzeige.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorHelp {
    pub title: String,
    pub hint: String,
}

/// Wählt anhand der Meldung (Groß-/Kleinschreibung egal, Teilstring-Suche, Regeln in dieser Reihenfolge, erste passende gilt):
/// 1. `database is locked` → Titel „Projekt gesperrt“, Hinweis „Das Projekt ist in einer anderen Instanz geöffnet. Schließen Sie sie und versuchen Sie es erneut.“
/// 2. `permission denied` → „Kein Zugriff“, „Die App darf diese Datei nicht lesen oder schreiben. Prüfen Sie die Berechtigungen.“
/// 3. `no such file` oder `nicht gefunden` → „Datei nicht gefunden“, „Prüfen Sie, ob die Datei verschoben oder gelöscht wurde.“
/// 4. `curl`, `network`, `timed out` oder `could not resolve` → „Keine Verbindung“, „Prüfen Sie die Internetverbindung und versuchen Sie es später erneut.“
/// 5. `invalid` oder `ungültig` → „Ungültige Eingabe“, „Prüfen Sie Ihre Eingaben auf Tippfehler und erlaubte Zeichen.“
/// 6. sonst Titel „Etwas ist schiefgelaufen“, Hinweis `"Details: "` + Meldung (getrimmt; länger als 200 Zeichen → die ersten 200 Zeichen + `…`).
pub fn friendly_error(message: &str) -> ErrorHelp {
    let lower = message.to_lowercase();

    if lower.contains("database is locked") {
        return ErrorHelp {
            title: "Projekt gesperrt".to_string(),
            hint: "Das Projekt ist in einer anderen Instanz geöffnet. Schließen Sie sie und versuchen Sie es erneut.".to_string(),
        };
    }
    if lower.contains("permission denied") {
        return ErrorHelp {
            title: "Kein Zugriff".to_string(),
            hint: "Die App darf diese Datei nicht lesen oder schreiben. Prüfen Sie die Berechtigungen.".to_string(),
        };
    }
    if lower.contains("no such file") || lower.contains("nicht gefunden") {
        return ErrorHelp {
            title: "Datei nicht gefunden".to_string(),
            hint: "Prüfen Sie, ob die Datei verschoben oder gelöscht wurde.".to_string(),
        };
    }
    if lower.contains("curl")
        || lower.contains("network")
        || lower.contains("timed out")
        || lower.contains("could not resolve")
    {
        return ErrorHelp {
            title: "Keine Verbindung".to_string(),
            hint: "Prüfen Sie die Internetverbindung und versuchen Sie es später erneut."
                .to_string(),
        };
    }
    if lower.contains("invalid") || lower.contains("ungültig") {
        return ErrorHelp {
            title: "Ungültige Eingabe".to_string(),
            hint: "Prüfen Sie Ihre Eingaben auf Tippfehler und erlaubte Zeichen.".to_string(),
        };
    }

    let trimmed = message.trim();
    let truncated: String = trimmed.chars().take(200).collect();
    let hint_suffix = if trimmed.chars().count() > 200 {
        "…"
    } else {
        ""
    };
    let hint = format!("Details: {}{}", truncated, hint_suffix);

    ErrorHelp {
        title: "Etwas ist schiefgelaufen".to_string(),
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regeln() {
        let h = friendly_error("Error: Database is LOCKED by another process");
        assert_eq!(h.title, "Projekt gesperrt");
        assert!(h
            .hint
            .starts_with("Das Projekt ist in einer anderen Instanz"));
        assert_eq!(
            friendly_error("open: Permission denied").title,
            "Kein Zugriff"
        );
        assert_eq!(
            friendly_error("x.fa: No such file or directory").title,
            "Datei nicht gefunden"
        );
        assert_eq!(
            friendly_error("curl: (6) Could not resolve host").title,
            "Keine Verbindung"
        );
        assert_eq!(friendly_error("Ungültige Basen").title, "Ungültige Eingabe");
        assert_eq!(
            friendly_error("invalid sequence").hint,
            "Prüfen Sie Ihre Eingaben auf Tippfehler und erlaubte Zeichen."
        );
    }

    #[test]
    fn reihenfolge_und_fallback() {
        // "permission denied" steht vor "no such file"
        assert_eq!(
            friendly_error("no such file: permission denied").title,
            "Kein Zugriff"
        );
        let h = friendly_error("  seltsam  ");
        assert_eq!(h.title, "Etwas ist schiefgelaufen");
        assert_eq!(h.hint, "Details: seltsam");
        let lang = "x".repeat(250);
        let h = friendly_error(&lang);
        assert_eq!(h.hint, format!("Details: {}…", "x".repeat(200)));
    }
}
