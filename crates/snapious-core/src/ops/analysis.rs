//! Analysis operations for registry and workflows: design primers, search CRISPR guides, check sequencing.
//! They produce no new documents, but a result text in `summary` (several lines separated by `\n`).

use crate::crispr::{find_guides, PamSpec};
use crate::error::{Error, Result};
use crate::model::document::{Document, SequenceDocument, Topology};
use crate::ops::operation::{required_usize, Operation, OperationContext, OperationResult, Params};
use crate::primer::picker::{pick_pairs, PrimerConstraints};
use crate::seqcheck::{DifferenceType, SequencingCheck};

/// Exactly one sequence document from the inputs; otherwise `Error::Operation("Exactly one sequence expected")`.
fn single(inputs: &[Document]) -> Result<&SequenceDocument> {
    match inputs {
        [Document::Sequence(d)] => Ok(d),
        _ => Err(Error::Operation(
            "Exactly one sequence expected".to_string(),
        )),
    }
}

/// `primer.pick`: primer pairs for a target region. Input: one sequence. Parameters: `start` and `end` (required, 0-based, `end` exclusive, via
/// `required_usize`), optional `max_results` (default 3). Calls `pick_pairs(bases, start, end, &PrimerConstraints::standard(), max_results)`; errors as
/// `Error::Operation` with the text. `outputs` empty. `summary`: if the list is empty, `"No matching primer pairs"`; otherwise one line per pair
/// `"{i}. {forward} / {reverse} · Produkt {product_size} bp"` (`i` from 1; look up the fields of the return type `PickedPair` itself: sequences of both primers and product size).
pub struct PrimerPickOperation;

impl Operation for PrimerPickOperation {
    fn id(&self) -> &str {
        "primer.pick"
    }

    fn label(&self) -> &str {
        "Design primers"
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
                Error::Operation(format!("Parameter 'max_results' is not a number: {}", t))
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
            "No matching primer pairs".to_string()
        } else {
            pairs
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    format!(
                        "{}. {} / {} · product {} bp",
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

/// `crispr.find`: search guide RNAs. Input: one sequence. Parameters: optional `nuclease` (`spcas9` default, `sacas9`, `cas12a`; case-insensitive,
/// anything else → `Error::Operation`). Calls `find_guides(&spec, bases, 0, length, circular)` with `circular` from the topology. `outputs` empty.
/// `summary`: first line `"{n} Guides ({name of the nuclease})"` (`PamSpec::name`), then for the first 10 guides one line each
/// `"{start+1}–{end} {+|−} {protospacer} {pam} · Wertung {score:.0}"` (`+` when `forward`, `−` otherwise).
pub struct CrisprFindOperation;

impl Operation for CrisprFindOperation {
    fn id(&self) -> &str {
        "crispr.find"
    }

    fn label(&self) -> &str {
        "Find CRISPR guides"
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
                    "Unknown nuclease '{}' (spcas9, sacas9, cas12a)",
                    other
                )))
            }
        };
        let circular = d.topology() == Topology::Circular;
        let guides = find_guides(&spec, d.bases(), 0, d.len(), circular);
        let mut lines = vec![format!("{} guides ({})", guides.len(), spec.name)];
        for g in guides.iter().take(10) {
            lines.push(format!(
                "{}–{} {} {} {} · score {:.0}",
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

/// `seq.check`: check sequencing against a reference. Inputs: first the reference, then at least one read (sequence documents, quality unknown),
/// otherwise `Error::Operation`. Parameters: optional `min_quality` (default 20, integer 0 to 93, otherwise `Error::Operation`; only effective with qualities, empty here).
/// Per read `SequencingCheck::new(min_quality).check(reference_bases, circular, name, read_bases, &[])` (circular from the reference topology); errors as `Error::Operation`.
/// `outputs` empty. `summary`: one line per read `"{Name}: {identity:.1} % Identität, {k} Unterschied(e){, revers}"` (`, revers` only when `reversed`; `k` = number of
/// entries in `differences` without `LowQuality`; type `DifferenceType` from `crate::seqcheck`).
pub struct SeqCheckOperation;

impl Operation for SeqCheckOperation {
    fn id(&self) -> &str {
        "seq.check"
    }

    fn label(&self) -> &str {
        "Check sequencing"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let [Document::Sequence(reference), reads @ ..] = inputs else {
            return Err(Error::Operation(
                "One reference and at least one read expected".to_string(),
            ));
        };
        if reads.is_empty() {
            return Err(Error::Operation(
                "At least one read expected".to_string(),
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
                        "Parameter 'min_quality' is not a number from 0 to 93: {}",
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
                    "All inputs must be sequences".to_string(),
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
                "{}: {:.1} % identity, {} difference(s){}",
                read.name(),
                res.identity,
                diffs,
                if res.reversed { ", reverse" } else { "" }
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
        assert!(r.summary.contains(" · product "));
        let lines = r.summary.lines().count();
        assert!((1..=3).contains(&lines));
        let one = run(
            &PrimerPickOperation,
            std::slice::from_ref(&d),
            &params(&[("start", "150"), ("end", "250"), ("max_results", "1")]),
        )
        .unwrap();
        assert_eq!(one.summary.lines().count(), 1);
        // GC-rich: no pairs
        let gc = doc("g", &"GC".repeat(200), Topology::Linear);
        let none = run(
            &PrimerPickOperation,
            &[gc],
            &params(&[("start", "150"), ("end", "250")]),
        )
        .unwrap();
        assert_eq!(none.summary, "No matching primer pairs");
        // errors
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
            first.contains("guides (SpCas9") || first.contains("guides ("),
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
        assert!(cas12a.summary.lines().next().unwrap().contains("guides ("));
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
        assert_eq!(lines[0], "read1: 100.0 % identity, 0 difference(s)");
        assert!(
            lines[1].starts_with("read2: 98.3 % identity, 1 difference(s)"),
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
