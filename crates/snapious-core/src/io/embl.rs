//! Read and write EMBL format (sequence records with features).
//!
//! Layout of a record (identifier in the first two columns):
//! `ID   name; SV 1; circular; genomic DNA; STD; SYN; 60 BP.`, `AC   TEST0001;`, `DE   description.`,
//! `OS   organism`, features as `FT   gene            5..20` with qualifiers `FT                   /gene="abc"`,
//! then `SQ   Sequence 60 BP; ...` followed by sequence lines (5 spaces, bases in 6 groups of ten, position counter at the end of the line)
//! and `//` as the end. Lines `XX` and `FH` are placeholders and are ignored when reading.
//! Feature positions use the same notation as GenBank (`complement(30..50)`, `join(...)`).
//! Hint: the reader may convert the text to GenBank notation and call `crate::io::genbank::parse`; the writer
//! may use `crate::io::genbank::format` and convert its lines. Metadata keys as in GenBank:
//! `definition` (from `DE`), `accession` (from `AC`, without the trailing semicolon), `organism` (from `OS`).

use crate::error::Result;
use crate::io::genbank;
use crate::model::document::SequenceDocument;

/// Reads all records from EMBL text. Records without an `ID` line are skipped.
/// The name is the first word of the `ID` line (before the semicolon); the topology is circular if the word `circular` occurs,
/// otherwise linear. Errors as in `genbank::parse` (e.g. invalid positions), with `Error::InvalidInput`.
pub fn parse(text: &str) -> Result<Vec<SequenceDocument>> {
    let mut docs = Vec::new();
    let mut record: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim_end() == "//" {
            push_record(&record, &mut docs)?;
            record.clear();
        } else {
            record.push(line);
        }
    }
    push_record(&record, &mut docs)?;
    Ok(docs)
}

/// Converts a record (without `//`) and appends the document; records without an `ID` line are dropped.
fn push_record(lines: &[&str], docs: &mut Vec<SequenceDocument>) -> Result<()> {
    if !lines.iter().any(|l| l.starts_with("ID   ")) {
        return Ok(());
    }
    let gb = embl_to_genbank(lines);
    docs.extend(genbank::parse(&gb)?);
    Ok(())
}

/// Writes a document as EMBL text (ends with `//\n`). The result can be read back with `parse` and yields
/// name, bases, topology, features (type, name, position, qualifier) and metadata (`definition`, `accession`, `organism`) unchanged.
pub fn format(doc: &SequenceDocument) -> String {
    genbank_to_embl(&genbank::format(doc))
}

/// Converts the lines of an EMBL record into GenBank notation.
fn embl_to_genbank(lines: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_features = false;
    let mut in_sequence = false;
    let mut after_de = false;
    for line in lines {
        let code = line.get(..2).unwrap_or("");
        let rest = line.get(5..).unwrap_or("");
        if in_sequence {
            if !line.trim().is_empty() {
                out.push(format!("        1 {}", rest.trim()));
            }
            continue;
        }
        match code {
            "ID" => {
                let name = rest.split(';').next().unwrap_or("").trim();
                let circular = rest.to_ascii_lowercase().contains("circular");
                out.push(format!(
                    "LOCUS       {} 0 bp    DNA     {}",
                    name,
                    if circular { "circular" } else { "linear" }
                ));
            }
            "AC" => out.push(format!("ACCESSION   {}", rest.trim().trim_end_matches(';'))),
            "DE" => {
                if after_de {
                    out.push(format!("            {}", rest.trim()));
                } else {
                    out.push(format!("DEFINITION  {}", rest.trim()));
                }
                after_de = true;
                continue;
            }
            "OS" => out.push(format!("  ORGANISM  {}", rest.trim())),
            "FT" => {
                if !in_features {
                    out.push("FEATURES             Location/Qualifiers".to_string());
                    in_features = true;
                }
                out.push(format!("  {}", &line[2..]));
            }
            "SQ" => out.push("ORIGIN".to_string()),
            _ => {}
        }
        after_de = false;
        in_sequence = code == "SQ";
    }
    out.push("//".to_string());
    out.join("\n") + "\n"
}

/// Converts GenBank text (as produced by `genbank::format`) into EMBL notation.
fn genbank_to_embl(gb: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut bases = String::new();
    let mut in_features = false;
    let mut in_origin = false;
    let mut in_definition = false;
    let mut header = (String::new(), String::new(), String::new());
    for line in gb.lines() {
        if in_origin {
            if line.starts_with("//") {
                break;
            }
            bases.extend(line.chars().filter(|c| c.is_ascii_alphabetic()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("LOCUS") {
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            header.0 = tokens.first().unwrap_or(&"").to_string();
            header.1 = if tokens.iter().any(|t| t.eq_ignore_ascii_case("circular")) {
                "circular".to_string()
            } else {
                "linear".to_string()
            };
            continue;
        }
        if line.starts_with("FEATURES") {
            in_features = true;
            in_definition = false;
            out.push("XX".to_string());
            out.push("FH   Key             Location/Qualifiers".to_string());
            out.push("FH".to_string());
            continue;
        }
        if line.starts_with("ORIGIN") {
            in_origin = true;
            continue;
        }
        if in_features {
            out.push(format!("FT{}", line.get(2..).unwrap_or("")));
        } else if let Some(rest) = line.strip_prefix("DEFINITION") {
            out.push("XX".to_string());
            out.push(format!("DE   {}", rest.trim()));
            in_definition = true;
        } else if in_definition && line.starts_with(' ') {
            out.push(format!("DE   {}", line.trim()));
        } else if let Some(rest) = line.strip_prefix("ACCESSION") {
            in_definition = false;
            out.push("XX".to_string());
            out.push(format!("AC   {};", rest.trim()));
        } else if let Some(rest) = line.trim_start().strip_prefix("ORGANISM") {
            in_definition = false;
            out.push("XX".to_string());
            out.push(format!("OS   {}", rest.trim()));
        } else {
            in_definition = false;
        }
    }
    let upper = bases.to_ascii_uppercase();
    let count = |c: char| upper.chars().filter(|x| *x == c).count();
    let (a, c, g, t) = (count('A'), count('C'), count('G'), count('T'));
    let other = upper.len() - a - c - g - t;
    let id = format!(
        "ID   {}; SV 1; {}; genomic DNA; STD; SYN; {} BP.",
        header.0,
        header.1,
        upper.len()
    );
    let mut text = vec![id];
    text.extend(out);
    text.push("XX".to_string());
    text.push(format!(
        "SQ   Sequence {} BP; {} A; {} C; {} G; {} T; {} other;",
        upper.len(),
        a,
        c,
        g,
        t,
        other
    ));
    let lower = bases.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for start in (0..bytes.len()).step_by(60) {
        let end = (start + 60).min(bytes.len());
        let groups: Vec<&str> = bytes[start..end]
            .chunks(10)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();
        text.push(format!("     {:<66}{:>9}", groups.join(" "), end));
    }
    text.push("//".to_string());
    text.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::genbank;
    use crate::model::document::Topology;

    const MINI: &str = "ID   mini; SV 1; circular; genomic DNA; STD; SYN; 60 BP.
XX
AC   TEST0001;
XX
DE   Testplasmid.
XX
FH   Key             Location/Qualifiers
FH
FT   gene            5..20
FT                   /gene=\"abc\"
FT   CDS             complement(30..50)
FT                   /gene=\"xyz\"
SQ   Sequence 60 BP; 20 A; 14 C; 14 G; 12 T; 0 other;
     atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc        60
//
";

    #[test]
    fn mini_lesen() {
        let docs = parse(MINI).unwrap();
        assert_eq!(docs.len(), 1);
        let d = &docs[0];
        assert_eq!(d.name(), "mini");
        assert_eq!(d.len(), 60);
        assert_eq!(&d.bases()[..10], "ATGCATGCAT");
        assert_eq!(d.topology(), Topology::Circular);
        assert_eq!(
            d.metadata().get("accession").map(String::as_str),
            Some("TEST0001")
        );
        assert_eq!(
            d.metadata().get("definition").map(String::as_str),
            Some("Testplasmid.")
        );
        assert_eq!(d.features().len(), 2);
        let f = &d.features()[0];
        assert_eq!(f.feature_type(), "gene");
        assert_eq!((f.location().start(), f.location().end()), (4, 20));
        assert_eq!(f.qualifier("gene"), Some("abc"));
        let c = &d.features()[1];
        assert_eq!(c.feature_type(), "CDS");
        assert_eq!((c.location().start(), c.location().end()), (29, 50));
        assert_eq!(
            c.location().strand(),
            crate::model::feature::Strand::Reverse
        );
    }

    #[test]
    fn gleich_wie_genbank() {
        let gb = genbank::parse(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/mini.gb"))
                .unwrap(),
        )
        .unwrap();
        let em = parse(MINI).unwrap();
        assert_eq!(gb[0].bases(), em[0].bases());
        assert_eq!(gb[0].topology(), em[0].topology());
        assert_eq!(gb[0].features().len(), em[0].features().len());
        for (a, b) in gb[0].features().iter().zip(em[0].features()) {
            assert_eq!(a.feature_type(), b.feature_type());
            assert_eq!(a.location(), b.location());
        }
    }

    #[test]
    fn schreiben_und_wieder_lesen() {
        let d = &parse(MINI).unwrap()[0];
        let text = format(d);
        assert!(text.starts_with("ID   mini;"));
        assert!(text.contains("\nSQ   Sequence 60 BP;"));
        assert!(text.ends_with("//\n"));
        let e = &parse(&text).unwrap()[0];
        assert_eq!(e.name(), d.name());
        assert_eq!(e.bases(), d.bases());
        assert_eq!(e.topology(), d.topology());
        assert_eq!(e.metadata(), d.metadata());
        assert_eq!(e.features().len(), 2);
        for (a, b) in d.features().iter().zip(e.features()) {
            assert_eq!(a.feature_type(), b.feature_type());
            assert_eq!(a.location(), b.location());
            assert_eq!(a.qualifiers(), b.qualifiers());
        }
    }

    #[test]
    fn mehrere_und_leer() {
        let two = format!("{MINI}{}", MINI.replace("mini;", "zwei;"));
        let docs = parse(&two).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[1].name(), "zwei");
        assert!(parse("").unwrap().is_empty());
        assert!(parse("XX\n//\n").unwrap().is_empty());
    }

    #[test]
    fn linear_ohne_features() {
        let t = "ID   kurz; SV 1; linear; genomic DNA; STD; SYN; 8 BP.
SQ   Sequence 8 BP; 2 A; 2 C; 2 G; 2 T; 0 other;
     acgtacgt                                                                    8
//
";
        let d = &parse(t).unwrap()[0];
        assert_eq!(d.topology(), Topology::Linear);
        assert_eq!(d.bases(), "ACGTACGT");
        assert!(d.features().is_empty());
        let e = &parse(&format(d)).unwrap()[0];
        assert_eq!(e.bases(), "ACGTACGT");
    }
}
