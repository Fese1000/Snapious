//! CML-Moleküldateien (Chemical Markup Language) lesen: Atome und Bindungen.

use crate::error::{Error, Result};

/// Ein Atom.
#[derive(Clone, Debug, PartialEq)]
pub struct CmlAtom {
    /// Wert von `id`.
    pub id: String,
    /// Wert von `elementType`.
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Eine Bindung zwischen zwei Atomen (Indizes in der Atomliste, 0-basiert) mit Ordnung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CmlBond {
    pub from: usize,
    pub to: usize,
    pub order: u8,
}

/// Liest Atome und Bindungen. Gesucht werden Tags `<atom ... />` (oder `<atom ...>`) mit den Attributen `id`, `elementType`, `x3`, `y3`,
/// `z3` und Tags `<bond ... />` mit `atomRefs2="id1 id2"` und `order` (Zahl `1`–`3`; `S` = 1, `D` = 2, `T` = 3; fehlend = 1).
/// Attributwerte stehen in doppelten oder einfachen Anführungszeichen. Kein XML-Parser nötig: Tag für Tag nach `<atom` und `<bond`
/// suchen und die Attribute lesen. Fehler `Error::InvalidInput`: ein Atom ohne `id`, `elementType` oder lesbare Koordinaten `x3`/`y3`/`z3`,
/// eine Bindung, die auf unbekannte Atom-IDs verweist oder kein `atomRefs2` hat, oder keine Atome.
pub fn parse_cml(text: &str) -> Result<(Vec<CmlAtom>, Vec<CmlBond>)> {
    fn attr(tag: &str, name: &str) -> Option<String> {
        let mut from = 0;
        while let Some(i) = tag[from..].find(name) {
            let at = from + i;
            let before_ok = at == 0 || tag[..at].ends_with(char::is_whitespace);
            let rest = &tag[at + name.len()..];
            if before_ok {
                let rest = rest.trim_start();
                if let Some(r) = rest.strip_prefix('=') {
                    let r = r.trim_start();
                    let quote = r.chars().next()?;
                    if quote == '"' || quote == '\'' {
                        let inner = &r[1..];
                        let end = inner.find(quote)?;
                        return Some(inner[..end].to_string());
                    }
                }
            }
            from = at + name.len();
        }
        None
    }

    fn tags<'a>(text: &'a str, open: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(i) = text[from..].find(open) {
            let start = from + i;
            let after = start + open.len();
            // nur "<atom " / "<atom>" / "<atom/", nicht "<atomArray"
            let next = text[after..].chars().next();
            if next.is_some_and(|c| c.is_whitespace() || c == '>' || c == '/') {
                if let Some(end) = text[after..].find('>') {
                    out.push(&text[after..after + end]);
                    from = after + end;
                    continue;
                }
            }
            from = after;
        }
        out
    }

    let mut atoms = Vec::new();
    for t in tags(text, "<atom") {
        let get = |n: &str| {
            attr(t, n).ok_or_else(|| Error::InvalidInput(format!("Atom ohne Attribut '{n}'")))
        };
        let num = |n: &str| -> Result<f64> {
            get(n)?
                .trim()
                .parse::<f64>()
                .map_err(|_| Error::InvalidInput(format!("Ungültige Koordinate '{n}'")))
        };
        atoms.push(CmlAtom {
            id: get("id")?,
            element: get("elementType")?,
            x: num("x3")?,
            y: num("y3")?,
            z: num("z3")?,
        });
    }
    if atoms.is_empty() {
        return Err(Error::InvalidInput(
            "Keine Atome in der CML-Datei".to_string(),
        ));
    }
    let mut bonds = Vec::new();
    for t in tags(text, "<bond") {
        let refs = attr(t, "atomRefs2")
            .ok_or_else(|| Error::InvalidInput("Bindung ohne atomRefs2".to_string()))?;
        let mut ids = refs.split_whitespace();
        let (Some(a), Some(b)) = (ids.next(), ids.next()) else {
            return Err(Error::InvalidInput(
                "atomRefs2 braucht zwei Atome".to_string(),
            ));
        };
        let find = |id: &str| {
            atoms
                .iter()
                .position(|x| x.id == id)
                .ok_or_else(|| Error::InvalidInput(format!("Unbekanntes Atom '{id}'")))
        };
        let order = match attr(t, "order").as_deref().map(str::trim) {
            Some("2") | Some("D") => 2,
            Some("3") | Some("T") => 3,
            _ => 1,
        };
        bonds.push(CmlBond {
            from: find(a)?,
            to: find(b)?,
            order,
        });
    }
    Ok((atoms, bonds))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WASSER: &str = "<?xml version=\"1.0\"?>\n<molecule xmlns=\"http://www.xml-cml.org/schema\" id=\"m1\">\n <atomArray>\n  <atom id=\"a1\" elementType=\"O\" x3=\"0.0\" y3=\"0.0\" z3=\"0.0\"/>\n  <atom id=\"a2\" elementType=\"H\" x3=\"0.757\" y3=\"0.586\" z3=\"0.0\"/>\n  <atom id='a3' elementType='H' x3='-0.757' y3='0.586' z3='0.0'/>\n </atomArray>\n <bondArray>\n  <bond atomRefs2=\"a1 a2\" order=\"1\"/>\n  <bond atomRefs2=\"a1 a3\" order=\"S\"/>\n </bondArray>\n</molecule>\n";

    #[test]
    fn wasser() {
        let (a, b) = parse_cml(WASSER).unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!((a[0].id.as_str(), a[0].element.as_str()), ("a1", "O"));
        assert_eq!((a[1].x, a[1].y, a[1].z), (0.757, 0.586, 0.0));
        assert_eq!(a[2].x, -0.757);
        assert_eq!(
            b,
            vec![
                CmlBond {
                    from: 0,
                    to: 1,
                    order: 1
                },
                CmlBond {
                    from: 0,
                    to: 2,
                    order: 1
                }
            ]
        );
    }

    #[test]
    fn ordnung_und_fehler() {
        let doppelt = WASSER.replace("order=\"1\"", "order=\"D\"");
        assert_eq!(parse_cml(&doppelt).unwrap().1[0].order, 2);
        let ohne = WASSER.replace(" order=\"1\"", "");
        assert_eq!(parse_cml(&ohne).unwrap().1[0].order, 1);
        assert!(parse_cml("<molecule/>").is_err());
        assert!(parse_cml(&WASSER.replace("a1 a2", "a1 zz")).is_err());
        assert!(parse_cml(&WASSER.replace("x3=\"0.757\"", "x3=\"abc\"")).is_err());
        assert!(parse_cml(&WASSER.replace(" elementType=\"O\"", "")).is_err());
    }
}
