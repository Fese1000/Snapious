//! Suche der Befehlspalette (⌘K): unscharfer Vergleich von Eingabe und Befehlstiteln.

/// Ein Eintrag der Befehlspalette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub keywords: Vec<String>,
}

/// Unscharfe Bewertung: Alle Zeichen von `query` müssen in `text` in dieser Reihenfolge vorkommen
/// (Groß-/Kleinschreibung egal, Leerzeichen in `query` werden ignoriert). Es wird gierig von links gesucht:
/// jedes Zeichen nimmt das erste passende Zeichen hinter dem vorigen Treffer.
///
/// Punkte je Treffer: +1; zusätzlich +5, wenn der Treffer direkt auf den vorigen Treffer folgt; zusätzlich +8,
/// wenn der Treffer am Wortanfang liegt (Index 0 oder das Zeichen davor ist Leerzeichen, `.`, `-` oder `_`).
/// Leere Eingabe → `Some(0)`; passt nicht alles → `None`.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    if query.trim().is_empty() {
        return Some(0);
    }

    let q_chars: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if q_chars.is_empty() {
        return Some(0);
    }

    let t_lower: Vec<char> = text.to_lowercase().chars().collect();
    let mut score: i32 = 0;
    let mut prev_idx: Option<usize> = None;

    for &qc in &q_chars {
        let start_idx = prev_idx.map_or(0, |p| p + 1);
        let offset = t_lower.iter().skip(start_idx).position(|&tc| tc == qc);
        let match_idx = start_idx + offset?;

        score += 1;

        if let Some(prev) = prev_idx {
            if match_idx == prev + 1 {
                score += 5;
            }
        }

        if match_idx == 0 || matches_word_boundary(&t_lower, match_idx) {
            score += 8;
        }

        prev_idx = Some(match_idx);
    }

    Some(score)
}

fn matches_word_boundary(text: &[char], idx: usize) -> bool {
    text[idx - 1] == ' ' || text[idx - 1] == '.' || text[idx - 1] == '-' || text[idx - 1] == '_'
}

/// Sortiert Befehle nach Treffer für `query` und gibt die Indizes in `commands` zurück.
/// Leere Eingabe (nur Leerraum) → alle Indizes in Originalreihenfolge. Sonst: Wert eines Befehls = Maximum aus
/// `fuzzy_score` des Titels **plus 10** und den `fuzzy_score`-Werten der Stichwörter (ohne Aufschlag); Befehle ohne
/// Treffer fehlen. Sortierung: Wert absteigend, bei Gleichstand kürzerer Titel (Zeichenzahl), dann kleinerer Index.
pub fn rank_commands(commands: &[Command], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..commands.len()).collect();
    }

    let mut scored: Vec<(usize, i32, usize)> = Vec::with_capacity(commands.len());

    for (i, cmd) in commands.iter().enumerate() {
        let title_score = fuzzy_score(query, &cmd.title).map(|s| s + 10);
        let keyword_scores: Vec<i32> = cmd
            .keywords
            .iter()
            .filter_map(|k| fuzzy_score(query, k))
            .collect();

        let max_s = title_score.into_iter().chain(keyword_scores).max();
        if let Some(s) = max_s {
            scored.push((i, s, cmd.title.chars().count()));
        }
    }

    scored.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.0.cmp(&b.0))
    });

    scored.into_iter().map(|(i, _, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(title: &str, keywords: &[&str]) -> Command {
        Command {
            id: title.to_lowercase(),
            title: title.to_string(),
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn bewertung() {
        assert_eq!(fuzzy_score("", "abc"), Some(0));
        assert_eq!(fuzzy_score("abc", "abc"), Some(21)); // 9 + 6 + 6
        assert_eq!(fuzzy_score("ABC", "abc"), Some(21));
        assert_eq!(fuzzy_score("ac", "abc"), Some(10)); // 9 + 1
        assert_eq!(fuzzy_score("a c", "abc"), Some(10));
        assert_eq!(fuzzy_score("pg", "primer gel"), Some(18)); // 9 + (1 + 8)
        assert_eq!(fuzzy_score("cba", "abc"), None);
        assert_eq!(fuzzy_score("xyz", "abc"), None);
    }

    #[test]
    fn rangfolge() {
        let cmds = vec![
            cmd("Primer entwerfen", &["pcr"]),
            cmd("Verdau simulieren", &[]),
            cmd("Primer prüfen", &[]),
        ];
        // beide Primer-Titel: 9 + 6 + 10 = 25, der kürzere Titel zuerst
        assert_eq!(rank_commands(&cmds, "pr"), vec![2, 0]);
        // nur das Stichwort "pcr" trifft (21), Titel ohne Aufschlag
        assert_eq!(rank_commands(&cmds, "pcr"), vec![0]);
        assert_eq!(rank_commands(&cmds, "  "), vec![0, 1, 2]);
        assert!(rank_commands(&cmds, "qqq").is_empty());
        assert!(rank_commands(&[], "a").is_empty());
    }
}
