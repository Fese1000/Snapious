//! Friendly error messages: translates technical messages into a title and a suggested fix.

/// Title and hint for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorHelp {
    pub title: String,
    pub hint: String,
}

/// Chooses by message (case-insensitive substring search, rules in this order, first match wins):
/// 1. `database is locked` → Titel „Project locked“, Hinweis „The project is open in another instance. Close it and try again.“
/// 2. `permission denied` → „No access“, „The app is not allowed to read or write this file. Check the permissions.“
/// 3. `no such file` or `nicht gefunden` → „File not found“, „Check whether the file was moved or deleted.“
/// 4. `curl`, `network`, `timed out` or `could not resolve` → „No connection“, „Check your internet connection and try again later.“
/// 5. `invalid` or `ungültig` → „Invalid input“, „Check your input for typos and allowed characters.“
/// 6. otherwise title „Something went wrong“, hint `"Details: "` + message (trimmed; longer than 200 characters → the first 200 characters + `…`).
pub fn friendly_error(message: &str) -> ErrorHelp {
    let lower = message.to_lowercase();

    if lower.contains("database is locked") {
        return ErrorHelp {
            title: "Project locked".to_string(),
            hint: "The project is open in another instance. Close it and try again.".to_string(),
        };
    }
    if lower.contains("permission denied") {
        return ErrorHelp {
            title: "No access".to_string(),
            hint: "The app is not allowed to read or write this file. Check the permissions.".to_string(),
        };
    }
    if lower.contains("no such file") || lower.contains("not found") {
        return ErrorHelp {
            title: "File not found".to_string(),
            hint: "Check whether the file was moved or deleted.".to_string(),
        };
    }
    if lower.contains("curl")
        || lower.contains("network")
        || lower.contains("timed out")
        || lower.contains("could not resolve")
    {
        return ErrorHelp {
            title: "No connection".to_string(),
            hint: "Check your internet connection and try again later."
                .to_string(),
        };
    }
    if lower.contains("invalid") {
        return ErrorHelp {
            title: "Invalid input".to_string(),
            hint: "Check your input for typos and allowed characters.".to_string(),
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
        title: "Something went wrong".to_string(),
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regeln() {
        let h = friendly_error("Error: Database is LOCKED by another process");
        assert_eq!(h.title, "Project locked");
        assert!(h
            .hint
            .starts_with("The project is open in another instance"));
        assert_eq!(
            friendly_error("open: Permission denied").title,
            "No access"
        );
        assert_eq!(
            friendly_error("x.fa: No such file or directory").title,
            "File not found"
        );
        assert_eq!(
            friendly_error("curl: (6) Could not resolve host").title,
            "No connection"
        );
        assert_eq!(friendly_error("Invalid bases").title, "Invalid input");
        assert_eq!(
            friendly_error("invalid sequence").hint,
            "Check your input for typos and allowed characters."
        );
    }

    #[test]
    fn reihenfolge_und_fallback() {
        // "permission denied" comes before "no such file"
        assert_eq!(
            friendly_error("no such file: permission denied").title,
            "No access"
        );
        let h = friendly_error("  seltsam  ");
        assert_eq!(h.title, "Something went wrong");
        assert_eq!(h.hint, "Details: seltsam");
        let lang = "x".repeat(250);
        let h = friendly_error(&lang);
        assert_eq!(h.hint, format!("Details: {}…", "x".repeat(200)));
    }
}
