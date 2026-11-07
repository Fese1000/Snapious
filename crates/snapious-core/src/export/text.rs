//! Textdarstellung einer Sequenz und Markdown-Bericht.

use crate::model::document::{SequenceDocument, Topology};
use crate::model::feature::Strand;
use crate::ops::operation::HistoryEntry;

/// Textdarstellung. Die Basen werden in Zeilen zu `bases_per_line` Zeichen (mindestens 10; kleinere Werte gelten als 10) geschrieben,
/// innerhalb einer Zeile in Blöcken zu 10 Zeichen, durch ein Leerzeichen getrennt (Uppercase).
/// * `numbers`: jede Zeile beginnt mit der 1-basierten Startposition, rechtsbündig auf die Stellenzahl der Sequenzlänge aufgefüllt, danach zwei Leerzeichen.
///   Ohne `numbers` beginnt die Zeile direkt mit den Basen.
/// * `features`: nach der Sequenz eine Leerzeile, die Zeile `Features:` und je Feature eine Zeile `{Typ} {Name} {start+1}..{ende}` mit ` (-)` am Ende bei
///   `Strand::Reverse` (Start des ersten bis Ende des letzten Intervalls: `location().start()`/`end()`).
///
/// Jede Zeile endet mit `\n`.
pub fn format_sequence_text(
    doc: &SequenceDocument,
    bases_per_line: usize,
    numbers: bool,
    features: bool,
) -> String {
    let bpl = bases_per_line.max(10);
    let bases = doc.bases().to_ascii_uppercase();
    let total = bases.len();
    let width = total.to_string().len();
    let mut out = String::new();
    let bytes = bases.as_bytes();
    let mut start = 0;
    while start < total {
        let end = (start + bpl).min(total);
        if numbers {
            out.push_str(&format!("{:>width$}  ", start + 1, width = width));
        }
        let blocks: Vec<&str> = bytes[start..end]
            .chunks(10)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();
        out.push_str(&blocks.join(" "));
        out.push('\n');
        start = end;
    }
    if features {
        out.push_str("\nFeatures:\n");
        for f in doc.features() {
            let rev = if f.location().strand() == Strand::Reverse {
                " (-)"
            } else {
                ""
            };
            out.push_str(&format!(
                "{} {} {}..{}{}\n",
                f.feature_type(),
                f.name(),
                f.location().start() + 1,
                f.location().end(),
                rev
            ));
        }
    }
    out
}

/// Markdown-Bericht:
/// ```text
/// # {Name}
///
/// - Länge: {n} bp
/// - Topology: circular|linear
/// - GC-Gehalt: {x:.1} %
///
/// ## Features
///
/// | Type | Name | Range | Strand |
/// |---|---|---|---|
/// | gene | abc | 5..20 | + |
/// ```
/// Strang `+`, `-` oder `.` (kein Strang). Hat das Dokument keine Features, steht unter `## Features` der Satz `No features.` statt der Tabelle.
/// Ist `history` nicht leer, folgt nach einer Leerzeile `## History`, eine Leerzeile und je Eintrag eine Zeile `- {summary}` (Einträge in der übergebenen Reihenfolge).
/// Der GC-Gehalt ist der Anteil von G und C an allen Basen in Prozent (`doc.sequence().gc_content()` liefert den Anteil 0.0 bis 1.0; mal 100); bei leerer Sequenz 0.0.
pub fn report_markdown(doc: &SequenceDocument, history: &[HistoryEntry]) -> String {
    let mut out = format!("# {}\n\n", doc.name());
    out.push_str(&format!("- Length: {} bp\n", doc.len()));
    out.push_str(&format!(
        "- Topology: {}\n",
        if doc.topology() == Topology::Circular {
            "circular"
        } else {
            "linear"
        }
    ));
    let gc = if doc.is_empty() {
        0.0
    } else {
        doc.sequence().gc_content() * 100.0
    };
    out.push_str(&format!("- GC content: {:.1} %\n\n", gc));
    out.push_str("## Features\n\n");
    if doc.features().is_empty() {
        out.push_str("No features.\n");
    } else {
        out.push_str("| Type | Name | Range | Strand |\n|---|---|---|---|\n");
        for f in doc.features() {
            let strand = match f.location().strand() {
                Strand::Forward => "+",
                Strand::Reverse => "-",
                Strand::None => ".",
            };
            out.push_str(&format!(
                "| {} | {} | {}..{} | {} |\n",
                f.feature_type(),
                f.name(),
                f.location().start() + 1,
                f.location().end(),
                strand
            ));
        }
    }
    if !history.is_empty() {
        out.push_str("\n## History\n\n");
        for h in history {
            out.push_str(&format!("- {}\n", h.summary));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::{Feature, Location};
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(bases: &str, features: Vec<Feature>, topology: Topology) -> SequenceDocument {
        let seq = Sequence::new("s", "s", bases, SequenceType::Dna).unwrap();
        SequenceDocument::of("Testplasmid", seq)
            .with_topology(topology)
            .with_features(features)
    }

    fn feat(kind: &str, name: &str, a: usize, b: usize, strand: Strand) -> Feature {
        Feature::new(kind, name, Location::of(a, b, strand).unwrap(), Vec::new())
    }

    const SEQ: &str = "ACGTACGTACGTACGTACGTACGTA";

    #[test]
    fn text_mit_positionen() {
        let d = doc(SEQ, vec![], Topology::Linear);
        assert_eq!(
            format_sequence_text(&d, 10, true, false),
            " 1  ACGTACGTAC\n11  GTACGTACGT\n21  ACGTA\n"
        );
        assert_eq!(
            format_sequence_text(&d, 20, true, false),
            " 1  ACGTACGTAC GTACGTACGT\n21  ACGTA\n"
        );
        assert_eq!(
            format_sequence_text(&d, 20, false, false),
            "ACGTACGTAC GTACGTACGT\nACGTA\n"
        );
        // kleiner Wert gilt als 10
        assert_eq!(
            format_sequence_text(&d, 3, false, false),
            format_sequence_text(&d, 10, false, false)
        );
    }

    #[test]
    fn text_mit_features() {
        let d = doc(
            "ACGTACGTAC",
            vec![
                feat("gene", "abc", 4, 20, Strand::Forward),
                feat("CDS", "xyz", 29, 50, Strand::Reverse),
            ],
            Topology::Linear,
        );
        let t = format_sequence_text(&d, 10, false, true);
        assert_eq!(
            t,
            "ACGTACGTAC\n\nFeatures:\ngene abc 5..20\nCDS xyz 30..50 (-)\n"
        );
    }

    #[test]
    fn bericht() {
        let d = doc(
            "GGCCAATT",
            vec![
                feat("gene", "abc", 4, 20, Strand::Forward),
                feat("misc", "x", 0, 3, Strand::None),
            ],
            Topology::Circular,
        );
        let h = vec![
            HistoryEntry {
                id: "1".to_string(),
                operation_id: "edit.insert".to_string(),
                parameters: Default::default(),
                input_ids: vec![],
                output_ids: vec![],
                time: 0,
                summary: "Insert".to_string(),
            },
            HistoryEntry {
                id: "2".to_string(),
                operation_id: "clone.digest".to_string(),
                parameters: Default::default(),
                input_ids: vec![],
                output_ids: vec![],
                time: 0,
                summary: "Digest".to_string(),
            },
        ];
        let r = report_markdown(&d, &h);
        let expected = "# Testplasmid\n\n- Length: 8 bp\n- Topology: circular\n- GC content: 50.0 %\n\n## Features\n\n| Type | Name | Range | Strand |\n|---|---|---|---|\n| gene | abc | 5..20 | + |\n| misc | x | 1..3 | . |\n\n## History\n\n- Insert\n- Digest\n";
        assert_eq!(r, expected);
    }

    #[test]
    fn bericht_ohne_features() {
        let d = doc("ACGT", vec![], Topology::Linear);
        let r = report_markdown(&d, &[]);
        assert!(r.starts_with("# Testplasmid\n"));
        assert!(r.contains("- Topology: linear"));
        assert!(r.contains("## Features\n\nNo features.\n"));
        assert!(!r.contains("## History"));
    }
}
