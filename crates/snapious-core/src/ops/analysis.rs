//! Analyse-Operationen für Registry und Workflows: Primer entwerfen, CRISPR-Guides suchen, Sequenzierung prüfen.
//! Sie liefern keine neuen Dokumente, sondern einen Ergebnistext in `summary` (mehrere Zeilen, getrennt durch `\n`).

use crate::crispr::{find_guides, PamSpec};
use crate::error::{Error, Result};
use crate::model::document::{Document, SequenceDocument, Topology};
use crate::ops::operation::{required_usize, Operation, OperationContext, OperationResult, Params};
use crate::primer::picker::{pick_pairs, PrimerConstraints};
use crate::seqcheck::{DifferenceType, SequencingCheck};

/// Genau ein Sequenzdokument aus den Eingaben; sonst `Error::Operation("Es wird genau eine Sequenz erwartet")`.
fn single(inputs: &[Document]) -> Result<&SequenceDocument> {
    match inputs {
        [Document::Sequence(d)] => Ok(d),
        _ => Err(Error::Operation(
            "Es wird genau eine Sequenz erwartet".to_string(),
        )),
    }
}

/// `primer.pick`: Primerpaare für einen Zielbereich. Eingabe: eine Sequenz. Parameter: `start` und `end` (Pflicht, 0-basiert, `end` exklusiv, über
/// `required_usize`), optional `max_results` (Standard 3). Aufruf `pick_pairs(bases, start, end, &PrimerConstraints::standard(), max_results)`; Fehler als
/// `Error::Operation` mit dem Text. `outputs` leer. `summary`: ist die Liste leer, `"Keine passenden Primerpaare"`; sonst je Paar eine Zeile
/// `"{i}. {forward} / {reverse} · Produkt {product_size} bp"` (`i` ab 1; die Felder des Rückgabetyps `PickedPair` selbst nachsehen: Sequenzen beider Primer und Produktgröße).
pub struct PrimerPickOperation;

impl Operation for PrimerPickOperation {
    fn id(&self) -> &str {
        "primer.pick"
    }

    fn label(&self) -> &str {
        "Primer entwerfen"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let d = single(inputs)?;
        let start = required_usize(params, "start")?;
        let end = required_usize(params, "end")?;
        let max_results = match params.get("max_results") {
            Some(t) => t.trim().parse::<usize>().map_err(|_| {
                Error::Operation(format!("Parameter 'max_results' ist keine Zahl: {}", t))
            })?,
            None => 3,
        };
        let pairs = pick_pairs(
            d.bases(),
            start,
            end,
            &PrimerConstraints::standard(),
            max_results,
        )
        .map_err(|e| Error::Operation(e.to_string()))?;
        let summary = if pairs.is_empty() {
            "Keine passenden Primerpaare".to_string()
        } else {
            pairs
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    format!(
                        "{}. {} / {} · Produkt {} bp",
                        i + 1,
                        p.forward.sequence,
                        p.reverse.sequence,
                        p.product_size
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        Ok(OperationResult {
            outputs: Vec::new(),
            summary,
        })
    }
}

/// `crispr.find`: Guide-RNAs suchen. Eingabe: eine Sequenz. Parameter: optional `nuclease` (`spcas9` Standard, `sacas9`, `cas12a`; Groß-/Kleinschreibung egal,
/// anderes → `Error::Operation`). Aufruf `find_guides(&spec, bases, 0, Länge, circular)` mit `circular` aus der Topologie. `outputs` leer.
/// `summary`: erste Zeile `"{n} Guides ({Name der Nuklease})"` (`PamSpec::name`), danach für die ersten 10 Guides je Zeile
/// `"{start+1}–{end} {+|−} {protospacer} {pam} · Wertung {score:.0}"` (`+` bei `forward`, `−` sonst).
pub struct CrisprFindOperation;

impl Operation for CrisprFindOperation {
    fn id(&self) -> &str {
        "crispr.find"
    }

    fn label(&self) -> &str {
        "CRISPR-Guides suchen"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let d = single(inputs)?;
        let spec = match params
            .get("nuclease")
            .map(|s| s.trim().to_lowercase())
            .as_deref()
        {
            None | Some("spcas9") => PamSpec::sp_cas9(),
            Some("sacas9") => PamSpec::sa_cas9(),
            Some("cas12a") => PamSpec::cas12a(),
            Some(other) => {
                return Err(Error::Operation(format!(
                    "Unbekannte Nuklease '{}' (spcas9, sacas9, cas12a)",
                    other
                )))
            }
        };
        let circular = d.topology() == Topology::Circular;
        let guides = find_guides(&spec, d.bases(), 0, d.len(), circular);
        let mut lines = vec![format!("{} Guides ({})", guides.len(), spec.name)];
        for g in guides.iter().take(10) {
            lines.push(format!(
                "{}–{} {} {} {} · Wertung {:.0}",
                g.start + 1,
                g.end,
                if g.forward { "+" } else { "−" },
                g.protospacer,
                g.pam,
                g.score
            ));
        }
        Ok(OperationResult {
            outputs: Vec::new(),
            summary: lines.join("\n"),
        })
    }
}

/// `seq.check`: Sequenzierung gegen eine Referenz prüfen. Eingaben: zuerst die Referenz, dann mindestens ein Read (Sequenzdokumente, Qualität unbekannt),
/// sonst `Error::Operation`. Parameter: optional `min_quality` (Standard 20, ganze Zahl 0 bis 93, sonst `Error::Operation`; wirkt nur mit Qualitäten, hier leer).
/// Je Read `SequencingCheck::new(min_quality).check(referenz_bases, circular, name, read_bases, &[])` (circular aus der Referenz-Topologie); Fehler als `Error::Operation`.
/// `outputs` leer. `summary`: je Read eine Zeile `"{Name}: {identity:.1} % Identität, {k} Unterschied(e){, revers}"` (`, revers` nur bei `reversed`; `k` = Zahl der
/// Einträge in `differences` ohne `LowQuality`; Typ `DifferenceType` aus `crate::seqcheck`).
pub struct SeqCheckOperation;

impl Operation for SeqCheckOperation {
    fn id(&self) -> &str {
        "seq.check"
    }

    fn label(&self) -> &str {
        "Sequenzierung prüfen"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let [Document::Sequence(reference), reads @ ..] = inputs else {
            return Err(Error::Operation(
                "Es werden eine Referenz und mindestens ein Read erwartet".to_string(),
            ));
        };
        if reads.is_empty() {
            return Err(Error::Operation(
                "Es wird mindestens ein Read erwartet".to_string(),
            ));
        }
        let min_quality = match params.get("min_quality") {
            Some(t) => t
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|q| *q <= 93)
                .ok_or_else(|| {
                    Error::Operation(format!(
                        "Parameter 'min_quality' ist keine Zahl von 0 bis 93: {}",
                        t
                    ))
                })?,
            None => 20,
        };
        let check = SequencingCheck::new(min_quality);
        let circular = reference.topology() == Topology::Circular;
        let mut lines = Vec::new();
        for r in reads {
            let Document::Sequence(read) = r else {
                return Err(Error::Operation(
                    "Alle Eingaben müssen Sequenzen sein".to_string(),
                ));
            };
            let res = check
                .check(reference.bases(), circular, read.name(), read.bases(), &[])
                .map_err(|e| Error::Operation(e.to_string()))?;
            let diffs = res
                .differences
                .iter()
                .filter(|d| d.kind != DifferenceType::LowQuality)
                .count();
            lines.push(format!(
                "{}: {:.1} % Identität, {} Unterschied(e){}",
                read.name(),
                res.identity,
                diffs,
                if res.reversed { ", revers" } else { "" }
            ));
        }
        Ok(OperationResult {
            outputs: Vec::new(),
            summary: lines.join("\n"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::{Sequence, SequenceType};
    use crate::ops::operation::NullContext;

    fn doc(name: &str, bases: &str, topology: Topology) -> Document {
        let s = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of(name, s).with_topology(topology))
    }

    fn params(items: &[(&str, &str)]) -> Params {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn run(op: &dyn Operation, inputs: &[Document], p: &Params) -> Result<OperationResult> {
        op.run(inputs, p, &mut NullContext)
    }

    fn random(len: usize, seed: u64) -> String {
        let mut rng = crate::util::Rng::new(seed);
        (0..len)
            .map(|_| ['A', 'C', 'G', 'T'][rng.below(4)])
            .collect()
    }

    #[test]
    fn primer() {
        let t = random(400, 42);
        let d = doc("t", &t, Topology::Linear);
        let r = run(
            &PrimerPickOperation,
            std::slice::from_ref(&d),
            &params(&[("start", "150"), ("end", "250")]),
        )
        .unwrap();
        assert!(r.outputs.is_empty());
        assert!(r.summary.starts_with("1. "), "{}", r.summary);
        assert!(r.summary.contains(" · Produkt "));
        let lines = r.summary.lines().count();
        assert!((1..=3).contains(&lines));
        let one = run(
            &PrimerPickOperation,
            std::slice::from_ref(&d),
            &params(&[("start", "150"), ("end", "250"), ("max_results", "1")]),
        )
        .unwrap();
        assert_eq!(one.summary.lines().count(), 1);
        // GC-reich: keine Paare
        let gc = doc("g", &"GC".repeat(200), Topology::Linear);
        let none = run(
            &PrimerPickOperation,
            &[gc],
            &params(&[("start", "150"), ("end", "250")]),
        )
        .unwrap();
        assert_eq!(none.summary, "Keine passenden Primerpaare");
        // Fehler
        assert!(run(
            &PrimerPickOperation,
            std::slice::from_ref(&d),
            &Params::new()
        )
        .is_err());
        assert!(run(
            &PrimerPickOperation,
            std::slice::from_ref(&d),
            &params(&[("start", "250"), ("end", "150")])
        )
        .is_err());
        assert!(run(
            &PrimerPickOperation,
            &[],
            &params(&[("start", "1"), ("end", "2")])
        )
        .is_err());
    }

    #[test]
    fn crispr() {
        let spacer = "ACGTACGTTGCAGCTAGCTA";
        let d = doc("c", &format!("TTTT{}TGGAAAA", spacer), Topology::Linear);
        let r = run(
            &CrisprFindOperation,
            std::slice::from_ref(&d),
            &Params::new(),
        )
        .unwrap();
        assert!(r.outputs.is_empty());
        let first = r.summary.lines().next().unwrap();
        assert!(
            first.contains("Guides (SpCas9") || first.contains("Guides ("),
            "{}",
            first
        );
        assert!(r.summary.contains(spacer));
        assert!(r.summary.contains(" TGG "));
        let cas12a = run(
            &CrisprFindOperation,
            std::slice::from_ref(&d),
            &params(&[("nuclease", "CAS12A")]),
        )
        .unwrap();
        assert!(cas12a.summary.lines().next().unwrap().contains("Guides ("));
        assert!(run(
            &CrisprFindOperation,
            std::slice::from_ref(&d),
            &params(&[("nuclease", "xyz")])
        )
        .is_err());
        assert!(run(&CrisprFindOperation, &[], &Params::new()).is_err());
    }

    #[test]
    fn sequenzierung() {
        let reference = random(120, 7);
        let read = reference[20..80].to_string();
        let mut changed: Vec<char> = read.chars().collect();
        changed[30] = if changed[30] == 'A' { 'C' } else { 'A' };
        let changed: String = changed.into_iter().collect();
        let inputs = [
            doc("ref", &reference, Topology::Linear),
            doc("read1", &read, Topology::Linear),
            doc("read2", &changed, Topology::Linear),
        ];
        let r = run(&SeqCheckOperation, &inputs, &Params::new()).unwrap();
        assert!(r.outputs.is_empty());
        let lines: Vec<&str> = r.summary.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "read1: 100.0 % Identität, 0 Unterschied(e)");
        assert!(
            lines[1].starts_with("read2: 98.3 % Identität, 1 Unterschied(e)"),
            "{}",
            lines[1]
        );
        assert!(run(&SeqCheckOperation, &inputs[..1], &Params::new()).is_err());
        assert!(run(
            &SeqCheckOperation,
            &inputs,
            &params(&[("min_quality", "x")])
        )
        .is_err());
    }
}
