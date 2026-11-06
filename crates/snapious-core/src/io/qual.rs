//! QUAL-Dateien schreiben (Qualitätswerte zu FASTA-Sequenzen).

/// Schreibt Einträge `(Name, Qualitätswerte)`: je Eintrag die Zeile `>Name`, danach die Werte als Dezimalzahlen mit einem
/// Leerzeichen getrennt, 17 Werte je Zeile; jede Zeile endet mit `\n`. Ohne Werte steht nur die Kopfzeile.
pub fn write_qual(records: &[(String, Vec<u8>)]) -> String {
    let mut out = String::new();

    for (name, quals) in records {
        out.push('>');
        out.push_str(name);
        out.push('\n');

        if !quals.is_empty() {
            for chunk in quals.chunks(17) {
                let line: String = chunk
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                out.push_str(&line);
                out.push('\n');
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umbruch_bei_17() {
        let v: Vec<u8> = (1..=20).collect();
        let t = write_qual(&[("r1".to_string(), v)]);
        assert_eq!(
            t,
            ">r1\n1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17\n18 19 20\n"
        );
    }

    #[test]
    fn mehrere_und_leer() {
        let t = write_qual(&[
            ("a".to_string(), vec![40, 30]),
            ("b".to_string(), vec![]),
            ("c".to_string(), vec![5]),
        ]);
        assert_eq!(t, ">a\n40 30\n>b\n>c\n5\n");
        assert_eq!(write_qual(&[]), "");
        let exakt: Vec<u8> = vec![9; 17];
        assert_eq!(
            write_qual(&[("x".to_string(), exakt)]),
            ">x\n9 9 9 9 9 9 9 9 9 9 9 9 9 9 9 9 9\n"
        );
    }
}
