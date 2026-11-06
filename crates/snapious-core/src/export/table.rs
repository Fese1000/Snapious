//! Tabellen als CSV/TSV schreiben und lesen (mit Anführungszeichen).

/// Schreibt Zeilen mit dem Trennzeichen `sep`. Ein Feld wird in `"` eingeschlossen, wenn es `sep`, `"`, `\n` oder `\r` enthält;
/// `"` im Feld wird verdoppelt. Jede Zeile endet mit `\n`. Keine Zeilen → "".
pub fn to_delimited(rows: &[Vec<String>], sep: char) -> String {
    let mut out = String::new();
    for row in rows {
        let fields: Vec<String> = row
            .iter()
            .map(|f| {
                if f.contains(sep) || f.contains('"') || f.contains('\n') || f.contains('\r') {
                    format!("\"{}\"", f.replace('"', "\"\""))
                } else {
                    f.clone()
                }
            })
            .collect();
        out.push_str(&fields.join(&sep.to_string()));
        out.push('\n');
    }
    out
}

/// Liest Text mit Trennzeichen `sep` in Zeilen und Felder. Felder in `"` dürfen `sep` und Zeilenumbrüche enthalten, `""` darin
/// ist ein `"`. Zeilenenden `\n`, `\r\n` und `\r`. Leere Zeilen (ohne Zeichen) werden übersprungen; ein leeres Feld bleibt ein
/// leerer Text (`a,,c` → `["a", "", "c"]`). Fehler gibt es nicht: ein fehlendes schließendes `"` nimmt den Rest als Feld.
pub fn parse_delimited(text: &str, sep: char) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut line_has_content = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => {
                in_quotes = true;
                line_has_content = true;
            }
            c if c == sep => {
                row.push(std::mem::take(&mut field));
                line_has_content = true;
            }
            '\n' | '\r' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                if line_has_content || !field.is_empty() {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                line_has_content = false;
            }
            _ => {
                field.push(c);
                line_has_content = true;
            }
        }
    }
    if line_has_content || !field.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|x| x.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[test]
    fn schreiben_und_zurueck() {
        let rows = r(&[&["a", "b,c"], &["x\"y", "z\nw"]]);
        let t = to_delimited(&rows, ',');
        assert_eq!(t, "a,\"b,c\"\n\"x\"\"y\",\"z\nw\"\n");
        assert_eq!(parse_delimited(&t, ','), rows);
        assert_eq!(to_delimited(&[], ','), "");
        assert_eq!(to_delimited(&r(&[&["a;b", "c"]]), '\t'), "a;b\tc\n");
    }

    #[test]
    fn lesen_varianten() {
        assert_eq!(
            parse_delimited("a;b\r\nc;d\r\n", ';'),
            r(&[&["a", "b"], &["c", "d"]])
        );
        assert_eq!(parse_delimited("a,,c\n", ','), r(&[&["a", "", "c"]]));
        assert_eq!(parse_delimited("\n\nx\n\ny", ','), r(&[&["x"], &["y"]]));
        assert_eq!(parse_delimited("\"a,b\",c", ','), r(&[&["a,b", "c"]]));
        assert!(parse_delimited("", ',').is_empty());
    }
}
