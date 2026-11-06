//! GFF3 schreiben: die Features eines Sequenzdokuments als Annotationsdatei.

use crate::model::document::SequenceDocument;

/// Prozentkodierung für GFF3-Attribute: `%` → `%25`, `;` → `%3B`, `=` → `%3D`, `&` → `%26`, `,` → `%2C`, Tabulator → `%09`,
/// Zeilenumbruch → `%0A`, Wagenrücklauf → `%0D`; alles andere bleibt (auch Umlaute).
pub fn percent_encode_attribute(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for c in text.chars() {
        match c {
            '%' => out.push_str("%25"),
            ';' => out.push_str("%3B"),
            '=' => out.push_str("%3D"),
            '&' => out.push_str("%26"),
            ',' => out.push_str("%2C"),
            '\t' => out.push_str("%09"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            _ => out.push(c),
        }
    }
    out
}

/// Schreibt `doc` als GFF3-Text.
/// - Erste Zeile `##gff-version 3\n`, zweite `##sequence-region {Dokumentname} 1 {Länge}\n` (Name des Dokuments, Leerzeichen durch `_` ersetzt).
/// - Danach je Feature und je Intervall eine Zeile mit neun durch Tabulator getrennten Spalten:
///   `seqid` (wie oben), `source` = `snapious`, Typ des Features, Start (`start + 1`), Ende (`end`), Score `.`, Strang (`+`, `-` oder `.`),
///   Phase `.`, Attribute.
/// - Attribute: `ID=f{N}` (N = 1-basierte Nummer des Features; bei mehreren Intervallen haben alle Zeilen dieselbe `ID`), `Name={Name kodiert}`
///   (nur wenn der Name nicht leer ist), dann alle Qualifier in Reihenfolge als `{Schlüssel}={Wert1},{Wert2}` (Schlüssel und Werte
///   mit `percent_encode_attribute`); Qualifier mit dem Schlüssel `Name` oder `ID` werden weggelassen.
/// - Jede Zeile endet mit `\n`. Ein Dokument ohne Features ergibt nur die beiden Kopfzeilen.
pub fn write_gff3(doc: &SequenceDocument) -> String {
    let mut out = String::new();
    out.push_str("##gff-version 3\n");
    let seq_name = doc.name().replace(' ', "_");
    out.push_str(&format!("##sequence-region {} 1 {}\n", seq_name, doc.len()));

    let features = doc.features();
    for (i, feature) in features.iter().enumerate() {
        let location = feature.location();
        for interval in location.intervals() {
            let start = interval.start() + 1;
            let end = interval.end();
            let strand_char = match location.strand() {
                crate::model::feature::Strand::Forward => "+",
                crate::model::feature::Strand::Reverse => "-",
                crate::model::feature::Strand::None => ".",
            };

            let mut attrs = Vec::new();
            attrs.push(format!("ID=f{}", i + 1));

            let name = feature.name();
            if !name.is_empty() {
                attrs.push(format!("Name={}", percent_encode_attribute(name)));
            }

            for (key, values) in feature.qualifiers() {
                if key == "Name" || key == "ID" {
                    continue;
                }
                let encoded_values: Vec<String> =
                    values.iter().map(|v| percent_encode_attribute(v)).collect();
                attrs.push(format!("{}={}", key, encoded_values.join(",")));
            }

            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t.\t{}\t.\t{}\n",
                seq_name,
                "snapious",
                feature.feature_type(),
                start,
                end,
                strand_char,
                attrs.join(";")
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::gff3::{gff3_to_features, parse_gff3};
    use crate::model::document::Topology;
    use crate::model::feature::{Feature, Interval, Location, Strand};
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(features: Vec<Feature>) -> SequenceDocument {
        let seq = Sequence::new("s", "mein plasmid", &"A".repeat(100), SequenceType::Dna).unwrap();
        SequenceDocument::of("mein plasmid", seq)
            .with_topology(Topology::Circular)
            .with_features(features)
    }

    fn feat(
        kind: &str,
        name: &str,
        ivs: &[(usize, usize)],
        strand: Strand,
        q: Vec<(&str, Vec<&str>)>,
    ) -> Feature {
        let loc = Location::new(
            ivs.iter()
                .map(|(a, b)| Interval::new(*a, *b).unwrap())
                .collect(),
            strand,
            false,
            false,
        )
        .unwrap();
        let qualifiers = q
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(str::to_string).collect()))
            .collect();
        Feature::new(kind, name, loc, qualifiers)
    }

    #[test]
    fn kodierung() {
        assert_eq!(
            percent_encode_attribute("a;b=c&d,e%f"),
            "a%3Bb%3Dc%26d%2Ce%25f"
        );
        assert_eq!(percent_encode_attribute("x\ty\nz\r"), "x%09y%0Az%0D");
        assert_eq!(percent_encode_attribute("Größe"), "Größe");
        assert_eq!(percent_encode_attribute(""), "");
    }

    #[test]
    fn nur_kopf() {
        assert_eq!(
            write_gff3(&doc(vec![])),
            "##gff-version 3\n##sequence-region mein_plasmid 1 100\n"
        );
    }

    #[test]
    fn features_als_zeilen() {
        let d = doc(vec![
            feat(
                "gene",
                "gfp",
                &[(9, 29)],
                Strand::Forward,
                vec![("note", vec!["a;b", "c"]), ("ID", vec!["x"])],
            ),
            feat("CDS", "", &[(0, 5), (10, 15)], Strand::Reverse, vec![]),
            feat("misc_feature", "frei", &[(50, 60)], Strand::None, vec![]),
        ]);
        let text = write_gff3(&d);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2 + 1 + 2 + 1);
        assert_eq!(
            lines[2],
            "mein_plasmid\tsnapious\tgene\t10\t29\t.\t+\t.\tID=f1;Name=gfp;note=a%3Bb,c"
        );
        assert_eq!(
            lines[3],
            "mein_plasmid\tsnapious\tCDS\t1\t5\t.\t-\t.\tID=f2"
        );
        assert_eq!(
            lines[4],
            "mein_plasmid\tsnapious\tCDS\t11\t15\t.\t-\t.\tID=f2"
        );
        assert_eq!(
            lines[5],
            "mein_plasmid\tsnapious\tmisc_feature\t51\t60\t.\t.\t.\tID=f3;Name=frei"
        );
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn rundreise_ueber_den_leser() {
        let d = doc(vec![feat(
            "gene",
            "gfp",
            &[(9, 29)],
            Strand::Forward,
            vec![("note", vec!["hallo"])],
        )]);
        let records = parse_gff3(&write_gff3(&d)).unwrap();
        assert_eq!(records.len(), 1);
        let f = gff3_to_features(&records, "mein_plasmid").unwrap();
        assert_eq!(f[0].name(), "gfp");
        assert_eq!((f[0].location().start(), f[0].location().end()), (9, 29));
        assert_eq!(f[0].location().strand(), Strand::Forward);
        assert_eq!(f[0].qualifier("note"), Some("hallo"));
    }
}
