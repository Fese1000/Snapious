//! SnapGene-Dateien (`.dna`) lesen (nur Lesen, nach der öffentlich beschriebenen Blockstruktur; noch ohne Prüfung an echten Dateien).
//!
//! Aufbau: Folge von Blöcken `[Typ: 1 Byte][Länge: u32 Big-Endian][Daten]`.
//! - Typ `0x09` (zuerst): 8 Bytes `SnapGene`, dann drei u16 (Dateityp, Exportversion, Importversion).
//! - Typ `0x00`: DNA. Erstes Datenbyte = Flags (Bit 0 gesetzt = zirkulär), der Rest die Sequenz als ASCII.
//! - Typ `0x0A`: Features als XML: `<Feature name=".." type=".." directionality="1"> <Segment range="396-452"/> <Q name="note"><V text="…"/></Q> </Feature>`.
//!   `directionality`: 1 = vorwärts, 2 = rückwärts, sonst ohne Richtung. `range` ist 1-basiert und schließt das Ende ein.
//!
//! Andere Blöcke werden übersprungen.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location, Strand};
use crate::model::sequence::{Sequence, SequenceType};
use crate::util::{new_id, now_secs};

/// Ist das der Anfang einer SnapGene-Datei? (`0x09`, Länge 14, `SnapGene`).
pub fn is_snapgene(bytes: &[u8]) -> bool {
    bytes.len() >= 13
        && bytes.first() == Some(&0x09)
        && bytes.get(5..13) == Some(b"SnapGene".as_slice())
}

/// Ersetzt `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` und Zeichenverweise `&#10;`/`&#x0A;` durch die Zeichen.
pub fn xml_unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(rest.get(..i).unwrap_or(""));
        let tail = rest.get(i..).unwrap_or("");
        let end = tail.find(';');
        let replacement = end.and_then(|e| {
            let name = tail.get(1..e)?;
            match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => {
                    let code = if let Some(hex) = name.strip_prefix("#x") {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        name.strip_prefix('#').and_then(|d| d.parse::<u32>().ok())
                    };
                    code.and_then(char::from_u32)
                }
            }
        });
        match (replacement, end) {
            (Some(c), Some(e)) => {
                out.push(c);
                rest = tail.get(e + 1..).unwrap_or("");
            }
            _ => {
                out.push('&');
                rest = tail.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

/// Wert eines Attributs `name="…"` in einem Tag-Text (entschlüsselt) oder `None`.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let pattern = format!(" {}=\"", name);
    let start = tag.find(&pattern)? + pattern.len();
    let len = tag.get(start..)?.find('"')?;
    tag.get(start..start + len).map(xml_unescape)
}

/// Teilt XML grob in Tags (`<…>`), Textstücke ignorierend. Jeder Eintrag ist der Tag-Text ohne die spitzen Klammern.
fn tags(xml: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        let after = rest.get(i + 1..).unwrap_or("");
        match after.find('>') {
            Some(j) => {
                out.push(after.get(..j).unwrap_or(""));
                rest = after.get(j + 1..).unwrap_or("");
            }
            None => break,
        }
    }
    out
}

/// Feature im Aufbau: Name, Typ, Strang, Intervalle, Qualifier.
type PartialFeature = (
    String,
    String,
    Strand,
    Vec<Interval>,
    Vec<(String, Vec<String>)>,
);

/// Liest die Features aus dem XML-Block.
pub fn parse_features(xml: &str) -> Result<Vec<Feature>> {
    let mut features = Vec::new();
    // aktuelles Feature: (Name, Typ, Strang, Intervalle, Qualifier); aktueller Qualifier: (Schlüssel, Werte)
    let mut current: Option<PartialFeature> = None;
    let mut qualifier: Option<(String, Vec<String>)> = None;
    for tag in tags(xml) {
        let name = tag.split_whitespace().next().unwrap_or("");
        match name {
            "Feature" => {
                let strand = match attribute(tag, "directionality").as_deref() {
                    Some("1") => Strand::Forward,
                    Some("2") => Strand::Reverse,
                    _ => Strand::None,
                };
                current = Some((
                    attribute(tag, "name").unwrap_or_default(),
                    attribute(tag, "type").unwrap_or_else(|| "misc_feature".to_string()),
                    strand,
                    Vec::new(),
                    Vec::new(),
                ));
            }
            "/Feature" => {
                if let Some((n, t, strand, intervals, quals)) = current.take() {
                    if !intervals.is_empty() {
                        features.push(Feature::new(
                            &t,
                            &n,
                            Location::new(intervals, strand, false, false)?,
                            quals,
                        ));
                    }
                }
            }
            "Segment" => {
                if let (Some(cur), Some(range)) = (current.as_mut(), attribute(tag, "range")) {
                    let (a, b) = range.split_once('-').ok_or_else(|| {
                        Error::InvalidInput(format!("Invalid range '{}'", range))
                    })?;
                    let bad = |_| Error::InvalidInput(format!("Invalid range '{}'", range));
                    let (a, b): (usize, usize) = (
                        a.trim().parse().map_err(bad)?,
                        b.trim().parse().map_err(bad)?,
                    );
                    if a == 0 || b < a {
                        return Err(Error::InvalidInput(format!(
                            "Invalid range '{}'",
                            range
                        )));
                    }
                    cur.3.push(Interval::new(a - 1, b)?);
                }
            }
            "Q" => {
                if current.is_some() {
                    qualifier = Some((attribute(tag, "name").unwrap_or_default(), Vec::new()));
                }
            }
            "V" => {
                if let Some((_, values)) = qualifier.as_mut() {
                    if let Some(v) = attribute(tag, "text")
                        .or_else(|| attribute(tag, "int"))
                        .or_else(|| attribute(tag, "predef"))
                    {
                        values.push(v);
                    }
                }
            }
            "/Q" => {
                if let (Some(q), Some(cur)) = (qualifier.take(), current.as_mut()) {
                    cur.4.push(q);
                }
            }
            _ => {}
        }
    }
    Ok(features)
}

/// Liest eine SnapGene-Datei. `name` wird der Dokumentname. Fehler `Error::InvalidInput`: keine SnapGene-Datei, abgeschnittener Block,
/// kein DNA-Block, Sequenz mit ungültigen Zeichen. Ein Feature hinter dem Sequenzende ist ebenfalls ein Fehler.
pub fn parse_snapgene(bytes: &[u8], name: &str) -> Result<SequenceDocument> {
    if !is_snapgene(bytes) {
        return Err(Error::InvalidInput("Not a SnapGene file".to_string()));
    }
    let mut pos = 0usize;
    let mut dna: Option<(bool, String)> = None;
    let mut features: Vec<Feature> = Vec::new();
    while pos < bytes.len() {
        let kind = bytes.get(pos).copied().unwrap_or(0);
        let len_bytes = bytes
            .get(pos + 1..pos + 5)
            .ok_or_else(|| Error::InvalidInput("Truncated SnapGene file".to_string()))?;
        let len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        let data = bytes.get(pos + 5..pos + 5 + len).ok_or_else(|| {
            Error::InvalidInput("Truncated block in the SnapGene file".to_string())
        })?;
        match kind {
            0x00 => {
                let flags = data.first().copied().unwrap_or(0);
                let text: String = String::from_utf8_lossy(data.get(1..).unwrap_or(&[]))
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                dna = Some((flags & 1 == 1, text.to_ascii_uppercase()));
            }
            0x0A => features = parse_features(&String::from_utf8_lossy(data))?,
            _ => {}
        }
        pos += 5 + len;
    }
    let (circular, bases) = dna.ok_or_else(|| {
        Error::InvalidInput("The SnapGene file contains no sequence".to_string())
    })?;
    if let Some(f) = features.iter().find(|f| f.location().end() > bases.len()) {
        return Err(Error::InvalidInput(format!(
            "The feature '{}' lies beyond the end of the sequence",
            f.name()
        )));
    }
    let seq = Sequence::new(name, name, &bases, SequenceType::Dna)?;
    let topology = if circular {
        Topology::Circular
    } else {
        Topology::Linear
    };
    Ok(SequenceDocument::new(
        &new_id(),
        name,
        seq,
        topology,
        features,
        Default::default(),
        now_secs(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(kind: u8, data: &[u8]) -> Vec<u8> {
        let mut v = vec![kind];
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(data);
        v
    }

    fn file(circular: bool, bases: &str, xml: Option<&str>) -> Vec<u8> {
        let mut cookie = b"SnapGene".to_vec();
        cookie.extend_from_slice(&[0, 1, 0, 15, 0, 15]);
        let mut out = block(0x09, &cookie);
        let mut dna = vec![if circular { 3 } else { 2 }];
        dna.extend_from_slice(bases.as_bytes());
        out.extend(block(0x00, &dna));
        out.extend(block(0x08, b"<AdditionalSequenceProperties/>"));
        if let Some(x) = xml {
            out.extend(block(0x0A, x.as_bytes()));
        }
        out
    }

    const XML: &str = r##"<Features nextValidID="3">
        <Feature recentID="0" name="gfp &amp; Co" directionality="1" type="CDS" allowSegmentOverlaps="0">
            <Segment range="1-6" color="#00ff00" type="standard"/>
            <Q name="note"><V text="Grün &lt;fluoreszierend&gt;"/></Q>
            <Q name="codon_start"><V int="1"/></Q>
        </Feature>
        <Feature recentID="1" name="rev" directionality="2" type="promoter">
            <Segment range="7-9"/><Segment range="11-12"/>
        </Feature>
        <Feature recentID="2" name="ohne" type="misc_feature"><Segment range="3-3"/></Feature>
    </Features>"##;

    #[test]
    fn erkennung() {
        let f = file(true, "ATGC", None);
        assert!(is_snapgene(&f));
        assert!(!is_snapgene(b"LOCUS       x"));
        assert!(!is_snapgene(&[]));
        assert!(!is_snapgene(&f[..8]));
    }

    #[test]
    fn xml_zeichen() {
        assert_eq!(
            xml_unescape("a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos;"),
            "a & b <c> \"d\" 'e'"
        );
        assert_eq!(xml_unescape("x&#10;y&#x41;"), "x\nyA");
        assert_eq!(xml_unescape("a & b &unbekannt; c"), "a & b &unbekannt; c");
        assert_eq!(xml_unescape("ende &"), "ende &");
    }

    #[test]
    fn sequenz_und_topologie() {
        let d = parse_snapgene(&file(true, "atgcatgcatgc", None), "plasmid").unwrap();
        assert_eq!(d.bases(), "ATGCATGCATGC");
        assert_eq!(d.topology(), Topology::Circular);
        assert_eq!(d.name(), "plasmid");
        assert!(d.features().is_empty());
        let l = parse_snapgene(&file(false, "ATGC", None), "x").unwrap();
        assert_eq!(l.topology(), Topology::Linear);
    }

    #[test]
    fn features() {
        let d = parse_snapgene(&file(false, "ATGCATGCATGCATGC", Some(XML)), "x").unwrap();
        let f = d.features();
        assert_eq!(f.len(), 3);
        assert_eq!(f[0].name(), "gfp & Co");
        assert_eq!(f[0].feature_type(), "CDS");
        assert_eq!((f[0].location().start(), f[0].location().end()), (0, 6));
        assert_eq!(f[0].location().strand(), Strand::Forward);
        assert_eq!(f[0].qualifier("note"), Some("Grün <fluoreszierend>"));
        assert_eq!(f[0].qualifier("codon_start"), Some("1"));
        assert_eq!(f[1].location().strand(), Strand::Reverse);
        assert_eq!(f[1].location().intervals().len(), 2);
        assert_eq!(
            (
                f[1].location().intervals()[0].start(),
                f[1].location().intervals()[0].end()
            ),
            (6, 9)
        );
        assert_eq!(
            (
                f[1].location().intervals()[1].start(),
                f[1].location().intervals()[1].end()
            ),
            (10, 12)
        );
        assert_eq!(f[2].location().strand(), Strand::None);
        assert_eq!((f[2].location().start(), f[2].location().end()), (2, 3));
    }

    #[test]
    fn fehler() {
        assert!(matches!(
            parse_snapgene(b"kein snapgene", "x"),
            Err(Error::InvalidInput(_))
        ));
        let mut abgeschnitten = file(true, "ATGCATGC", None);
        abgeschnitten.truncate(abgeschnitten.len() - 3);
        assert!(matches!(
            parse_snapgene(&abgeschnitten, "x"),
            Err(Error::InvalidInput(_))
        ));
        // ohne DNA-Block
        let mut cookie = b"SnapGene".to_vec();
        cookie.extend_from_slice(&[0, 1, 0, 15, 0, 15]);
        assert!(matches!(
            parse_snapgene(&block(0x09, &cookie), "x"),
            Err(Error::InvalidInput(_))
        ));
        // Bereich hinter dem Ende
        let schlecht = r#"<Features><Feature name="a" type="gene"><Segment range="1-99"/></Feature></Features>"#;
        assert!(parse_snapgene(&file(false, "ATGC", Some(schlecht)), "x").is_err());
        assert!(parse_features(r#"<Feature name="a"><Segment range="5-2"/></Feature>"#).is_err());
        assert!(parse_features(r#"<Feature name="a"><Segment range="x"/></Feature>"#).is_err());
    }
}
