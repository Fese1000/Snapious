//! PIR/NBRF-Sequenzdateien lesen.

use crate::error::{Error, Result};

/// Ein PIR-Eintrag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PirRecord {
    /// Zweibuchstabencode vor dem Semikolon, z. B. `P1` (Protein) oder `DL` (DNA linear).
    pub kind: String,
    pub id: String,
    /// Die Zeile nach der Kopfzeile (getrimmt, kann leer sein).
    pub description: String,
    /// Sequenz ohne Leerraum, Ziffern und `*`.
    pub sequence: String,
}

/// Liest alle Einträge. Eine Kopfzeile beginnt mit `>`, enthält ein Semikolon: `kind` steht zwischen `>` und `;`, `id` danach
/// (beide getrimmt). Die nächste Zeile ist die Beschreibung (auch wenn leer). Alle weiteren Zeilen bis zur nächsten Kopfzeile sind
/// Sequenz: Leerraum, Ziffern und `*` werden entfernt, die übrigen Zeichen bleiben. Zeilen vor der ersten Kopfzeile werden
/// ignoriert. Fehler `Error::InvalidInput`: eine Kopfzeile ohne Semikolon oder keine Einträge.
pub fn parse_pir(text: &str) -> Result<Vec<PirRecord>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut records = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if line.starts_with('>') {
            // Header Zeile i
            let semi_pos = match line.find(';') {
                Some(pos) => pos,
                None => return Err(Error::InvalidInput("Kopfzeile ohne Semikolon".into())),
            };

            let kind = line[1..semi_pos].trim().to_string();
            let id = line[semi_pos + 1..].trim().to_string();

            if kind.is_empty() || id.is_empty() {
                return Err(Error::InvalidInput("Kopfzeile ungültig".into()));
            }

            // Nächste Zeile (i+1) ist die Beschreibung, falls vorhanden
            let description = if i + 1 < lines.len() {
                lines[i + 1].trim().to_string()
            } else {
                String::new()
            };

            // Alle Zeilen ab i+2 bis zum nächsten Header sind Sequence
            let mut seq_builder = String::new();
            let mut j = i + 2;

            while j < lines.len() {
                let next_line = lines[j];
                // Prüfen, ob nächste Zeile ein neuer Header ist
                if next_line.starts_with('>') {
                    break;
                }

                // Sequence-Zeile verarbeiten: Leerraum, Ziffern und * entfernen
                for c in next_line.chars() {
                    if !c.is_whitespace() && !c.is_ascii_digit() && c != '*' {
                        seq_builder.push(c);
                    }
                }
                j += 1;
            }

            records.push(PirRecord {
                kind,
                id,
                description,
                sequence: seq_builder,
            });

            // Springe zum nächsten Header oder Ende
            i = j;
        } else {
            // Zeile vor dem ersten Header ignorieren
            i += 1;
        }
    }

    if records.is_empty() {
        return Err(Error::InvalidInput(
            "Keine gültigen Einträge gefunden".into(),
        ));
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zwei_eintraege() {
        let t = ">P1;SEQ1\nTest protein\nMKV LAA\nGG*\n>DL;SEQ2\n\nACGT\n";
        let r = parse_pir(t).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0],
            PirRecord {
                kind: "P1".into(),
                id: "SEQ1".into(),
                description: "Test protein".into(),
                sequence: "MKVLAAGG".into()
            }
        );
        assert_eq!(
            (
                r[1].kind.as_str(),
                r[1].id.as_str(),
                r[1].description.as_str(),
                r[1].sequence.as_str()
            ),
            ("DL", "SEQ2", "", "ACGT")
        );
    }

    #[test]
    fn fehler() {
        assert!(parse_pir(">ohne semikolon\nx\nACGT\n").is_err());
        assert!(parse_pir("nur text\n").is_err());
        assert!(parse_pir("").is_err());
        assert_eq!(
            parse_pir("vorspann\n>P1;a\nd\nAC12 GT\n").unwrap()[0].sequence,
            "ACGT"
        );
    }
}
