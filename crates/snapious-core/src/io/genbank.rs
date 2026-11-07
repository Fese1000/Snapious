//! Read and write GenBank flat files, including features, locations and qualifiers.
//!
//! Positions in the file are 1-based and inclusive; internally 0-based with exclusive end.
//! Supported: `a..b`, `a`, `a^b`, `<a..b`, `a..>b`, `complement(...)`, `join(...)`, `order(...)`
//! and `complement(join(...))`. `join(complement(...),...)` is rejected with an error message.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::{Feature, Interval, Location, Strand};
use crate::model::sequence::{Sequence, SequenceType};
use crate::util::{new_id, now_secs};

/// Reads all records (separated by `//`) from a text.
pub fn parse(text: &str) -> Result<Vec<SequenceDocument>> {
    let mut docs = Vec::new();
    let mut record: Vec<(usize, &str)> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim() == "//" {
            if record.iter().any(|(_, l)| l.starts_with("LOCUS")) {
                docs.push(parse_record(&record)?);
            }
            record.clear();
        } else {
            record.push((i + 1, line));
        }
    }
    if record.iter().any(|(_, l)| l.starts_with("LOCUS")) {
        docs.push(parse_record(&record)?);
    }
    Ok(docs)
}

/// Reads a GenBank file.
pub fn read(path: &Path) -> Result<Vec<SequenceDocument>> {
    let text = std::fs::read_to_string(path)?;
    parse(&text)
}

/// Writes a document as a GenBank file.
pub fn write(doc: &SequenceDocument, path: &Path) -> Result<()> {
    std::fs::write(path, format(doc))?;
    Ok(())
}

struct RawFeature {
    line: usize,
    feature_type: String,
    location: String,
    /// Raw qualifier text without the leading slash, e.g. `gene="abc"`.
    qualifiers: Vec<String>,
}

#[derive(PartialEq, Clone, Copy)]
enum State {
    Header,
    Definition,
    Features,
    Origin,
}

fn parse_record(lines: &[(usize, &str)]) -> Result<SequenceDocument> {
    let mut name = String::new();
    let mut protein = false;
    let mut circular = false;
    let mut definition = String::new();
    let mut accession = String::new();
    let mut organism = String::new();
    let mut raws: Vec<RawFeature> = Vec::new();
    let mut origin: Vec<(usize, String)> = Vec::new();
    let mut state = State::Header;

    for (no, line) in lines {
        if line.starts_with("LOCUS") {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            name = tokens.get(1).unwrap_or(&"").to_string();
            protein = tokens.contains(&"aa");
            circular = tokens.iter().any(|t| t.eq_ignore_ascii_case("circular"));
            state = State::Header;
        } else if let Some(rest) = line.strip_prefix("DEFINITION") {
            definition = rest.trim().to_string();
            state = State::Definition;
        } else if let Some(rest) = line.strip_prefix("ACCESSION") {
            accession = rest.split_whitespace().next().unwrap_or("").to_string();
            state = State::Header;
        } else if line.starts_with("FEATURES") {
            state = State::Features;
        } else if line.starts_with("ORIGIN") {
            state = State::Origin;
        } else {
            match state {
                State::Definition => {
                    if line.starts_with(' ') {
                        definition.push(' ');
                        definition.push_str(line.trim());
                    } else {
                        state = State::Header;
                    }
                }
                State::Header => {
                    if let Some(rest) = line.trim_start().strip_prefix("ORGANISM") {
                        organism = rest.trim().to_string();
                    }
                }
                State::Features => parse_feature_line(*no, line, &mut raws),
                State::Origin => origin.push((*no, line.to_string())),
            }
        }
    }

    if name.is_empty() {
        return Err(Error::Io("LOCUS line without a name".to_string()));
    }

    let lowered: String = origin
        .iter()
        .flat_map(|(_, l)| l.chars())
        .filter(|c| !c.is_whitespace() && !c.is_ascii_digit())
        .collect::<String>()
        .to_ascii_lowercase();
    let seq_type = if protein {
        SequenceType::Protein
    } else if lowered.contains('u') && !lowered.contains('t') {
        SequenceType::Rna
    } else {
        SequenceType::Dna
    };

    let allowed = seq_type.allowed();
    for (no, l) in &origin {
        for c in l.chars() {
            if c.is_whitespace() || c.is_ascii_digit() {
                continue;
            }
            let u = c.to_ascii_uppercase();
            if !u.is_ascii() || !allowed.contains(&(u as u8)) {
                return Err(Error::Io(format!(
                    "Line {}: Invalid character '{}' in the sequence",
                    no, c
                )));
            }
        }
    }

    let mut sequence = Sequence::new(&name, &name, &lowered, seq_type)?;
    sequence.set_description(&definition);
    let seq_len = sequence.len();

    let mut features = Vec::with_capacity(raws.len());
    for raw in &raws {
        features.push(build_feature(raw, seq_len)?);
    }

    let mut metadata = BTreeMap::new();
    if !definition.is_empty() {
        metadata.insert("definition".to_string(), definition);
    }
    if !accession.is_empty() {
        metadata.insert("accession".to_string(), accession);
    }
    if !organism.is_empty() {
        metadata.insert("organism".to_string(), organism);
    }

    Ok(SequenceDocument::new(
        &new_id(),
        &name,
        sequence,
        if circular {
            Topology::Circular
        } else {
            Topology::Linear
        },
        features,
        metadata,
        now_secs(),
    ))
}

fn parse_feature_line(no: usize, line: &str, raws: &mut Vec<RawFeature>) {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }
    if indent < 21 {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let feature_type = parts.next().unwrap_or("").to_string();
        let location = parts.next().unwrap_or("").trim().to_string();
        raws.push(RawFeature {
            line: no,
            feature_type,
            location,
            qualifiers: Vec::new(),
        });
    } else if let Some(current) = raws.last_mut() {
        if let Some(q) = trimmed.strip_prefix('/') {
            current.qualifiers.push(q.to_string());
        } else if let Some(last) = current.qualifiers.last_mut() {
            // Continuation line; for /translation append without a space
            if !last.starts_with("translation=") {
                last.push(' ');
            }
            last.push_str(trimmed);
        } else {
            current.location.push_str(trimmed);
        }
    }
}

fn build_feature(raw: &RawFeature, seq_len: usize) -> Result<Feature> {
    let location = parse_location(&raw.location, raw.line)?;
    if location.end() > seq_len {
        return Err(Error::Io(format!(
            "Line {}: Position {} lies beyond the end of the sequence ({})",
            raw.line,
            location.end(),
            seq_len
        )));
    }

    let mut qualifiers: Vec<(String, Vec<String>)> = Vec::new();
    for text in &raw.qualifiers {
        let (key, value) = match text.split_once('=') {
            Some((k, v)) => (k.trim().to_string(), clean_value(v)),
            None => (text.trim().to_string(), String::new()),
        };
        if let Some(entry) = qualifiers.iter_mut().find(|(k, _)| *k == key) {
            entry.1.push(value);
        } else {
            qualifiers.push((key, vec![value]));
        }
    }

    let name = ["label", "gene", "product", "note"]
        .iter()
        .find_map(|k| {
            qualifiers
                .iter()
                .find(|(key, _)| key == k)
                .and_then(|(_, v)| v.first())
                .filter(|s| !s.is_empty())
                .cloned()
        })
        .unwrap_or_else(|| raw.feature_type.clone());

    Ok(Feature::new(&raw.feature_type, &name, location, qualifiers))
}

fn clean_value(v: &str) -> String {
    let t = v.trim();
    let t = t.strip_prefix('"').unwrap_or(t);
    let t = t.strip_suffix('"').unwrap_or(t);
    t.replace("\"\"", "\"")
}

type Parsed = (Vec<(usize, usize)>, Strand, bool, bool);

fn parse_location(text: &str, line: usize) -> Result<Location> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let (mut ranges, strand, ps, pe) = parse_loc_inner(&compact, line)?;
    ranges.sort();
    let intervals = ranges
        .into_iter()
        .map(|(s, e)| Interval::new(s, e))
        .collect::<Result<Vec<_>>>()?;
    Location::new(intervals, strand, ps, pe)
}

fn parse_loc_inner(s: &str, line: usize) -> Result<Parsed> {
    if let Some(inner) = s
        .strip_prefix("complement(")
        .and_then(|r| r.strip_suffix(')'))
    {
        let (r, strand, ps, pe) = parse_loc_inner(inner, line)?;
        if strand == Strand::Reverse {
            return Err(Error::Io(format!(
                "Line {}: not supported: nested complement",
                line
            )));
        }
        return Ok((r, Strand::Reverse, ps, pe));
    }
    for prefix in ["join(", "order("] {
        if let Some(inner) = s.strip_prefix(prefix).and_then(|r| r.strip_suffix(')')) {
            let mut ranges = Vec::new();
            let mut ps = false;
            let mut pe = false;
            let parts: Vec<&str> = inner.split(',').collect();
            for (i, part) in parts.iter().enumerate() {
                if part.starts_with("complement(") || part.starts_with("join(") {
                    return Err(Error::Io(format!(
                        "Line {}: not supported: {}",
                        line, s
                    )));
                }
                let (r, _, a, b) = parse_range(part, line)?;
                ranges.extend(r);
                if i == 0 {
                    ps = a;
                }
                if i == parts.len() - 1 {
                    pe = b;
                }
            }
            return Ok((ranges, Strand::Forward, ps, pe));
        }
    }
    parse_range(s, line)
}

fn parse_num(t: &str, line: usize) -> Result<usize> {
    t.parse::<usize>()
        .map_err(|_| Error::Io(format!("Line {}: Invalid position '{}'", line, t)))
}

fn parse_range(s: &str, line: usize) -> Result<Parsed> {
    let mut partial_start = false;
    let mut partial_end = false;
    let (a, b) = if let Some((a, b)) = s.split_once("..") {
        (a, Some(b))
    } else if let Some((a, b)) = s.split_once('^') {
        // Position between two bases: empty interval after base a
        let start = parse_num(a.trim_start_matches('<'), line)?;
        let _ = parse_num(b.trim_start_matches('>'), line)?;
        return Ok((vec![(start, start)], Strand::Forward, false, false));
    } else {
        (s, None)
    };
    let a = if let Some(rest) = a.strip_prefix('<') {
        partial_start = true;
        rest
    } else {
        a
    };
    let start = parse_num(a, line)?;
    let end = match b {
        Some(b) => {
            let b = if let Some(rest) = b.strip_prefix('>') {
                partial_end = true;
                rest
            } else {
                b
            };
            parse_num(b, line)?
        }
        None => start,
    };
    if start == 0 || end < start {
        return Err(Error::Io(format!(
            "Line {}: Invalid range '{}'",
            line, s
        )));
    }
    Ok((
        vec![(start - 1, end)],
        Strand::Forward,
        partial_start,
        partial_end,
    ))
}

// ---------------------------------------------------------------- Writing

/// Formats a document as GenBank text.
pub fn format(doc: &SequenceDocument) -> String {
    let seq = doc.sequence();
    let name = doc.name().replace(' ', "_");
    let (unit, mol) = match seq.seq_type() {
        SequenceType::Protein => ("aa", "PRT"),
        SequenceType::Rna => ("bp", "RNA"),
        SequenceType::Dna => ("bp", "DNA"),
    };
    let topo = if doc.topology() == Topology::Circular {
        "circular"
    } else {
        "linear"
    };
    let mut out = format!(
        "LOCUS       {:<16} {:>11} {}    {:<7} {:<8} SYN 01-JAN-2000\n",
        name,
        seq.len(),
        unit,
        mol,
        topo
    );
    if let Some(d) = doc.metadata().get("definition") {
        out.push_str(&format!("DEFINITION  {}\n", d));
    }
    if let Some(a) = doc.metadata().get("accession") {
        out.push_str(&format!("ACCESSION   {}\n", a));
    }
    if let Some(o) = doc.metadata().get("organism") {
        out.push_str(&format!("SOURCE      {}\n  ORGANISM  {}\n", o, o));
    }
    out.push_str("FEATURES             Location/Qualifiers\n");
    for f in doc.features() {
        out.push_str(&format!(
            "     {:<16}{}\n",
            f.feature_type(),
            format_location(f.location())
        ));
        for (key, values) in f.qualifiers() {
            for v in values {
                for l in format_qualifier(key, v) {
                    out.push_str("                     ");
                    out.push_str(&l);
                    out.push('\n');
                }
            }
        }
    }
    out.push_str("ORIGIN\n");
    let lower = seq.bases().to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for start in (0..bytes.len()).step_by(60) {
        out.push_str(&format!("{:>9}", start + 1));
        for block in 0..6 {
            let a = start + block * 10;
            if a >= bytes.len() {
                break;
            }
            let b = (a + 10).min(bytes.len());
            out.push(' ');
            out.push_str(&String::from_utf8_lossy(&bytes[a..b]));
        }
        out.push('\n');
    }
    out.push_str("//\n");
    out
}

fn format_location(loc: &Location) -> String {
    let n = loc.intervals().len();
    let parts: Vec<String> = loc
        .intervals()
        .iter()
        .enumerate()
        .map(|(i, iv)| {
            let ps = if i == 0 && loc.partial_start() {
                "<"
            } else {
                ""
            };
            let pe = if i == n - 1 && loc.partial_end() {
                ">"
            } else {
                ""
            };
            if iv.length() == 0 {
                format!("{}^{}", iv.start(), iv.start() + 1)
            } else if iv.length() == 1 && ps.is_empty() && pe.is_empty() {
                format!("{}", iv.start() + 1)
            } else {
                format!("{}{}..{}{}", ps, iv.start() + 1, pe, iv.end())
            }
        })
        .collect();
    let body = if n > 1 {
        format!("join({})", parts.join(","))
    } else {
        parts.join(",")
    };
    if loc.strand() == Strand::Reverse {
        format!("complement({})", body)
    } else {
        body
    }
}

const QUALIFIER_WIDTH: usize = 58;

fn format_qualifier(key: &str, value: &str) -> Vec<String> {
    if value.is_empty() {
        return vec![format!("/{}", key)];
    }
    let unquoted = matches!(key, "codon_start" | "transl_table" | "number")
        && value.chars().all(|c| c.is_ascii_digit());
    let text = if unquoted {
        format!("/{}={}", key, value)
    } else {
        format!("/{}=\"{}\"", key, value.replace('"', "\"\""))
    };
    if key == "translation" {
        return text
            .as_bytes()
            .chunks(QUALIFIER_WIDTH)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= QUALIFIER_WIDTH {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = include_str!("../../testdata/mini.gb");

    #[test]
    fn mini_wird_gelesen() {
        let docs = parse(MINI).unwrap();
        assert_eq!(docs.len(), 1);
        let d = &docs[0];
        assert_eq!(d.len(), 60);
        assert_eq!(d.topology(), Topology::Circular);
        assert_eq!(
            d.metadata().get("accession").map(String::as_str),
            Some("TEST0001")
        );
        assert_eq!(d.name(), "mini");
        assert_eq!(d.bases(), "ATGC".repeat(15));
    }

    #[test]
    fn mini_features() {
        let d = &parse(MINI).unwrap()[0];
        let f = &d.features()[0];
        assert_eq!(f.feature_type(), "gene");
        assert_eq!(f.name(), "abc");
        assert_eq!((f.location().start(), f.location().end()), (4, 20));
        assert_eq!(f.location().strand(), Strand::Forward);
        let g = &d.features()[1];
        assert_eq!(g.feature_type(), "CDS");
        assert_eq!(g.name(), "xyz");
        assert_eq!((g.location().start(), g.location().end()), (29, 50));
        assert_eq!(g.location().strand(), Strand::Reverse);
        assert_eq!(g.qualifier("note"), Some("zwei Zeilen Fortsetzung"));
    }

    #[test]
    fn round_trip() {
        let d = &parse(MINI).unwrap()[0];
        let text = format(d);
        let e = &parse(&text).unwrap()[0];
        assert_eq!(e.bases(), d.bases());
        assert_eq!(e.topology(), d.topology());
        assert_eq!(e.features(), d.features());
        assert_eq!(e.metadata(), d.metadata());
    }

    #[test]
    fn join_und_komplement_von_join() {
        assert_eq!(
            parse_location("join(1..3,7..9)", 1).unwrap().intervals(),
            &[Interval::new(0, 3).unwrap(), Interval::new(6, 9).unwrap()]
        );
        let l = parse_location("complement(join(1..3,7..9))", 1).unwrap();
        assert_eq!(l.strand(), Strand::Reverse);
        assert_eq!(l.intervals().len(), 2);
    }

    #[test]
    fn positionsvarianten() {
        let l = parse_location("5", 1).unwrap();
        assert_eq!((l.start(), l.end()), (4, 5));
        let l = parse_location("<5..>10", 1).unwrap();
        assert!(l.partial_start() && l.partial_end());
        assert_eq!((l.start(), l.end()), (4, 10));
        let l = parse_location("5^6", 1).unwrap();
        assert_eq!((l.start(), l.end()), (5, 5));
    }

    #[test]
    fn join_mit_komplement_teilen_wird_abgelehnt() {
        assert!(parse_location("join(complement(1..3),complement(7..9))", 4).is_err());
    }

    #[test]
    fn ungueltige_positionen() {
        assert!(parse_location("abc", 1).is_err());
        assert!(parse_location("0..5", 1).is_err());
        assert!(parse_location("10..5", 1).is_err());
    }

    #[test]
    fn feature_hinter_dem_ende_ist_fehler() {
        let text = MINI.replace("complement(30..50)", "complement(30..80)");
        assert!(matches!(parse(&text), Err(Error::Io(_))));
    }

    #[test]
    fn ungueltiges_zeichen_in_origin() {
        let text = MINI.replace("atgcatgcat gcatgcatgc", "atgcatgxat gcatgcatgc");
        match parse(&text) {
            Err(Error::Io(m)) => assert!(m.contains("Line") && m.contains('x')),
            other => panic!("unerwartet: {:?}", other),
        }
    }

    #[test]
    fn zwei_eintraege() {
        let text = format!("{}{}", MINI, MINI.replace("mini", "zweite"));
        let docs = parse(&text).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[1].name(), "zweite");
    }

    #[test]
    fn lange_qualifier_werden_umbrochen_und_gelesen() {
        let seq = Sequence::new("s", "s", &"ACGT".repeat(20), SequenceType::Dna).unwrap();
        let note = "Dies ist ein sehr langer Notiztext mit vielen Wörtern damit der Umbruch auf mehrere Zeilen erfolgen muss und wieder korrekt zusammengesetzt wird";
        let f = Feature::new(
            "misc_feature",
            "x",
            Location::of(0, 10, Strand::Forward).unwrap(),
            vec![
                ("note".to_string(), vec![note.to_string()]),
                ("codon_start".to_string(), vec!["1".to_string()]),
                ("pseudo".to_string(), vec![String::new()]),
            ],
        );
        let d = SequenceDocument::of("lang", seq).with_features(vec![f]);
        let text = format(&d);
        assert!(text.lines().all(|l| l.len() <= 79 + 20));
        let e = &parse(&text).unwrap()[0];
        assert_eq!(e.features()[0].qualifier("note"), Some(note));
        assert_eq!(e.features()[0].qualifier("codon_start"), Some("1"));
        assert_eq!(e.features()[0].qualifier("pseudo"), Some(""));
    }

    #[test]
    fn translation_wird_ohne_leerzeichen_zusammengesetzt() {
        let seq = Sequence::new("s", "s", &"ACGT".repeat(20), SequenceType::Dna).unwrap();
        let tr = "M".repeat(150);
        let f = Feature::new(
            "CDS",
            "x",
            Location::of(0, 60, Strand::Forward).unwrap(),
            vec![("translation".to_string(), vec![tr.clone()])],
        );
        let d = SequenceDocument::of("t", seq).with_features(vec![f]);
        let e = &parse(&format(&d)).unwrap()[0];
        assert_eq!(e.features()[0].qualifier("translation"), Some(tr.as_str()));
    }

    #[test]
    fn mehrzeilige_position() {
        let text = MINI.replace(
            "complement(30..50)",
            "join(30..35,\n                     40..50)",
        );
        let d = &parse(&text).unwrap()[0];
        assert_eq!(d.features()[1].location().intervals().len(), 2);
    }

    #[test]
    fn datei_schreiben_und_lesen() {
        let d = &parse(MINI).unwrap()[0];
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.gb");
        write(d, &p).unwrap();
        let back = read(&p).unwrap();
        assert_eq!(back[0].features(), d.features());
        assert!(read(&dir.path().join("fehlt.gb")).is_err());
    }

    #[test]
    fn rna_und_protein_typen() {
        let rna = "LOCUS       r                         8 bp    RNA     linear   SYN 01-JAN-2000\nORIGIN\n        1 auggcuaa\n//\n";
        let d = &parse(rna).unwrap()[0];
        assert_eq!(d.sequence().seq_type(), SequenceType::Rna);
        let prot = "LOCUS       p                         4 aa            linear   SYN 01-JAN-2000\nORIGIN\n        1 mdek\n//\n";
        let p = &parse(prot).unwrap()[0];
        assert_eq!(p.sequence().seq_type(), SequenceType::Protein);
        assert_eq!(p.bases(), "MDEK");
    }
}
