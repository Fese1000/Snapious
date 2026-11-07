//! Klonierungs-Operationen für Registry und Workflows: Restriktionsklonierung, PCR-Simulation, Codon-Optimierung.

use crate::bio::codon_optimize::{optimize_dna, CodonTable, OptimizationOptions};
use crate::bio::codon_usage::count_codons;
use crate::cloning::enzymes::EnzymeDb;
use crate::cloning::ligation::restriction_candidates;
use crate::cloning::pcr::{PcrSimulator, Primer};
use crate::error::{Error, Result};
use crate::model::document::{Document, SequenceDocument, Topology};
use crate::model::sequence::{Sequence, SequenceType};
use crate::ops::operation::{required, Operation, OperationContext, OperationResult, Params};

/// Alle Eingaben müssen Sequenzdokumente sein; Fehler Error::Operation.
fn sequences(inputs: &[Document]) -> Result<Vec<&SequenceDocument>> {
    inputs
        .iter()
        .map(|d| match d {
            Document::Sequence(s) => Ok(s),
            _ => Err(Error::Operation(
                "All inputs must be sequences".to_string(),
            )),
        })
        .collect()
}

/// Zerlegt eine Liste wie EcoRI, BamHI (Trenner Komma, Semikolon oder Leerzeichen) in Namen; leere Teile entfallen.
fn split_list(text: &str) -> Vec<String> {
    text.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

/// clone.restriction: Restriktionsklonierung. Eingaben: zuerst der Vektor, dann das Insert (zwei Sequenzen, sonst Error::Operation).
/// Parameter: vector_enzymes und insert_enzymes (Pflicht, Listen wie bei split_list), candidate (optional, 0-basierter Index in
/// der sortierten Kandidatenliste von restriction_candidates, Standard 0; ungültiger Wert oder Index außerhalb -> Error::Operation).
/// Ergebnis: ein Ausgabedokument, das construct des gewählten Kandidaten; summary = "Construct {len} bp ({directional|non-directional})".
/// Gibt es keine Kandidaten: Error::Operation("No matching ends: no construct results"). Fehler der Kernfunktion
/// werden als Error::Operation mit deren Text weitergegeben (EnzymeDb::standard() verwenden).
pub struct RestrictionCloneOperation;

impl Operation for RestrictionCloneOperation {
    fn id(&self) -> &str {
        "clone.restriction"
    }

    fn label(&self) -> &str {
        "Restriction cloning"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let seqs = sequences(inputs)?;
        if seqs.len() != 2 {
            return Err(Error::Operation(
                "Exactly two inputs expected: vector and insert".to_string(),
            ));
        }

        let vector = seqs[0];
        let insert = seqs[1];

        let vector_enzymes_str = required(params, "vector_enzymes")?;
        let insert_enzymes_str = required(params, "insert_enzymes")?;

        let vector_enzymes: Vec<String> = split_list(vector_enzymes_str);
        let insert_enzymes: Vec<String> = split_list(insert_enzymes_str);

        // Core function call
        let vector_enzymes_refs: Vec<&str> = vector_enzymes.iter().map(|s| s.as_str()).collect();
        let insert_enzymes_refs: Vec<&str> = insert_enzymes.iter().map(|s| s.as_str()).collect();

        let candidates = restriction_candidates(
            vector,
            &vector_enzymes_refs,
            insert,
            &insert_enzymes_refs,
            &EnzymeDb::standard(),
        )
        .map_err(|e| Error::Operation(e.to_string()))?;

        if candidates.is_empty() {
            return Err(Error::Operation(
                "No matching ends: no construct results".to_string(),
            ));
        }

        let candidate_idx: usize = match params.get("candidate") {
            Some(s) => s
                .parse::<usize>()
                .map_err(|_| Error::Operation("Invalid candidate index".to_string()))?,
            None => 0,
        };

        if candidate_idx >= candidates.len() {
            return Err(Error::Operation(
                "Candidate index outside the valid range".to_string(),
            ));
        }

        let selected = &candidates[candidate_idx];

        let construct_doc = selected.construct.clone();

        let summary = format!(
            "Construct {} bp ({})",
            construct_doc.bases().len(),
            if selected.directional {
                "directional"
            } else {
                "non-directional"
            }
        );

        Ok(OperationResult {
            outputs: vec![Document::Sequence(construct_doc)],
            summary,
        })
    }
}

/// pcr.simulate: PCR-Simulation. Eingabe: genau eine Vorlage (Sequenz). Parameter: forward und reverse (Primersequenzen, Pflicht),
/// optional max_mismatches (Standard 2), min_match (Standard 12), max_length (Standard 10000); Zahlen, die sich nicht lesen
/// lassen -> Error::Operation. PcrSimulator::new(min_match, max_mismatches, max_length), Primernamen forward und reverse,
/// zirkulär, wenn die Vorlage Topology::Circular hat. Jedes Produkt wird ein lineares DNA-Dokument namens PCR-Produkt {n}
/// (n ab 1, Reihenfolge wie die Kernfunktion sie liefert), Metadaten source = "pcr.simulate".
/// Kein Produkt -> Error::Operation("No PCR product: the primers do not bind suitably"). summary = "{n} PCR-Produkt(e)".
pub struct PcrOperation;

impl Operation for PcrOperation {
    fn id(&self) -> &str {
        "pcr.simulate"
    }

    fn label(&self) -> &str {
        "Simulate PCR"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let seqs = sequences(inputs)?;
        if seqs.len() != 1 {
            return Err(Error::Operation(
                "Exactly one input expected: template".to_string(),
            ));
        }

        let template = seqs[0];

        let forward_seq = required(params, "forward")?;
        let reverse_seq = required(params, "reverse")?;

        // Optional params with defaults and error handling for invalid values
        let max_mismatches: usize = match required(params, "max_mismatches") {
            Ok(s) => s
                .trim()
                .parse::<usize>()
                .map_err(|_| Error::Operation("Invalid value for max_mismatches".to_string()))?,
            Err(_) => 2,
        };

        let min_match: usize = match required(params, "min_match") {
            Ok(s) => s
                .trim()
                .parse::<usize>()
                .map_err(|_| Error::Operation("Invalid value for min_match".to_string()))?,
            Err(_) => 12,
        };

        let max_length: usize = match required(params, "max_length") {
            Ok(s) => s
                .trim()
                .parse::<usize>()
                .map_err(|_| Error::Operation("Invalid value for max_length".to_string()))?,
            Err(_) => 10000,
        };

        let simulator = PcrSimulator::new(min_match, max_mismatches, max_length);

        let primers = vec![
            Primer {
                name: "forward".to_string(),
                sequence: forward_seq.to_string(),
            },
            Primer {
                name: "reverse".to_string(),
                sequence: reverse_seq.to_string(),
            },
        ];

        let products = simulator.simulate(
            template.sequence().bases(),
            template.topology() == Topology::Circular,
            &primers,
        );

        if products.is_empty() {
            return Err(Error::Operation(
                "No PCR product: the primers do not bind suitably".to_string(),
            ));
        }

        let mut outputs = Vec::with_capacity(products.len());
        for (i, prod) in products.iter().enumerate() {
            let name = format!("PCR product {}", i + 1);
            // Assuming PcrProduct has 'sequence' field (String)
            let seq = Sequence::new(&name, &name, &prod.sequence, SequenceType::Dna)
                .map_err(|e| Error::Operation(e.to_string()))?; // Assuming Sequence::new returns Result
            let mut meta = Params::new();
            meta.insert("source".to_string(), "pcr.simulate".to_string());
            let doc = Document::Sequence(SequenceDocument::of(&name, seq).with_metadata(meta));
            outputs.push(doc);
        }

        Ok(OperationResult {
            outputs,
            summary: format!("{} PCR product(s)", products.len()),
        })
    }
}

/// codon.optimize: Codon-Optimierung. Eingaben: zuerst die codierende DNA, dann ein Referenzdokument, aus dessen Codon-Häufigkeiten
/// (count_codons auf bases() des Referenzdokuments, dann CodonTable::from_counts) die Tabelle entsteht (zwei Sequenzen, sonst
/// Error::Operation). Parameter (alle optional): avoid (Liste wie bei split_list, Uppercase, Erkennungsstellen),
/// max_homopolymer (Standard 6). Sonst OptimizationOptions::standard() (Felder avoid_sites, max_homopolymer setzen).
/// Ergebnis: ein lineares DNA-Dokument {Name} (optimiert), Metadaten source = "codon.optimize"; summary = "Codon-optimiert, {len} bp".
/// Fehler der Kernfunktion optimize_dna als Error::Operation mit deren Text.
pub struct CodonOptimizeOperation;

impl Operation for CodonOptimizeOperation {
    fn id(&self) -> &str {
        "codon.optimize"
    }

    fn label(&self) -> &str {
        "Optimize codons"
    }

    fn run(
        &self,
        inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let seqs = sequences(inputs)?;
        if seqs.len() != 2 {
            return Err(Error::Operation(
                "Exactly two inputs expected: DNA and reference".to_string(),
            ));
        }

        let coding_dna = seqs[0];
        let reference = seqs[1];

        // Parse avoid sites
        let avoid_sites: Vec<String> = params
            .get("avoid")
            .map(|s| split_list(s))
            .unwrap_or_default();

        // Parse max_homopolymer (default 6) with error handling
        let max_homopolymer: usize = match required(params, "max_homopolymer") {
            Ok(s) => s
                .trim()
                .parse::<usize>()
                .map_err(|_| Error::Operation("Invalid value for max_homopolymer".to_string()))?,
            Err(_) => 6,
        };

        // Create options
        let mut options = OptimizationOptions::standard();
        options.avoid_sites = avoid_sites;
        options.max_homopolymer = max_homopolymer;

        // Count codons from reference
        let counts = count_codons(reference.sequence().bases());
        let table = CodonTable::from_counts(&counts);

        // Optimize
        let optimized_seq = optimize_dna(&table, coding_dna.sequence().bases(), &options)
            .map_err(|e| Error::Operation(e.to_string()))?;

        let name = format!("{} (optimized)", coding_dna.name());
        let seq = Sequence::new(&name, &name, &optimized_seq, SequenceType::Dna)
            .map_err(|e| Error::Operation(e.to_string()))?;
        let mut meta = Params::new();
        meta.insert("source".to_string(), "codon.optimize".to_string());
        let doc = Document::Sequence(SequenceDocument::of(&name, seq).with_metadata(meta));

        Ok(OperationResult {
            outputs: vec![doc],
            summary: format!("Codon-optimized, {} bp", optimized_seq.len()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::operation::NullContext;

    fn doc(name: &str, bases: &str, topology: Topology) -> Document {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        Document::Sequence(SequenceDocument::of(name, seq).with_topology(topology))
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

    fn vector() -> Document {
        let b = format!(
            "{}GGATCC{}GAATTC{}",
            "A".repeat(10),
            "T".repeat(8),
            "C".repeat(10)
        );
        doc("vec", &b, Topology::Circular)
    }

    fn insert() -> Document {
        doc("ins", "CCGGATCCACGTACGTGAATTCGG", Topology::Linear)
    }

    fn only_sequence(r: &OperationResult) -> &SequenceDocument {
        match &r.outputs[0] {
            Document::Sequence(s) => s,
            _ => panic!("keine Sequenz"),
        }
    }

    #[test]
    fn liste_zerlegen() {
        assert_eq!(split_list("EcoRI, BamHI"), vec!["EcoRI", "BamHI"]);
        assert_eq!(
            split_list("EcoRI;BamHI HindIII,,"),
            vec!["EcoRI", "BamHI", "HindIII"]
        );
        assert!(split_list("  ").is_empty());
    }

    #[test]
    fn klonierung_gerichtet() {
        let p = params(&[
            ("vector_enzymes", "BamHI,EcoRI"),
            ("insert_enzymes", "BamHI,EcoRI"),
        ]);
        let r = run(&RestrictionCloneOperation, &[vector(), insert()], &p).unwrap();
        assert_eq!(r.outputs.len(), 1);
        let c = only_sequence(&r);
        assert_eq!(c.topology(), Topology::Circular);
        assert!(r.summary.starts_with("Construct "));
        assert!(r.summary.contains("(directional)"));
        let doubled = format!("{}{}", c.bases(), c.bases());
        assert!(doubled.contains("GGATCC") && doubled.contains("GAATTC"));
    }

    #[test]
    fn klonierung_fehler() {
        let p = params(&[("vector_enzymes", "BamHI"), ("insert_enzymes", "BamHI")]);
        assert!(run(&RestrictionCloneOperation, &[vector()], &p).is_err());
        assert!(run(
            &RestrictionCloneOperation,
            &[vector(), insert()],
            &Params::new()
        )
        .is_err());
        let bad = params(&[
            ("vector_enzymes", "BamHI"),
            ("insert_enzymes", "BamHI"),
            ("candidate", "99"),
        ]);
        assert!(run(&RestrictionCloneOperation, &[vector(), insert()], &bad).is_err());
        let x = params(&[("vector_enzymes", "EcoRI"), ("insert_enzymes", "BamHI")]);
        let e = run(&RestrictionCloneOperation, &[vector(), insert()], &x).unwrap_err();
        assert!(e.to_string().contains("No matching ends"));
    }

    const T: &str = "ATGCCGTAGCTAGCATCGATCGGATCCGATCGATCGTAGCTAGCTAGGCTAGC";

    #[test]
    fn pcr_ganzes_template() {
        let rev = crate::cloning::enzymes::iupac_reverse_complement(&T[T.len() - 14..]);
        let p = params(&[("forward", "ATGCCGTAGCTAGC"), ("reverse", &rev)]);
        let r = run(&PcrOperation, &[doc("t", T, Topology::Linear)], &p).unwrap();
        assert_eq!(r.outputs.len(), 1);
        let prod = only_sequence(&r);
        assert_eq!(prod.bases(), T);
        assert_eq!(prod.name(), "PCR product 1");
        assert_eq!(prod.topology(), Topology::Linear);
        assert_eq!(r.summary, "1 PCR product(s)");
    }

    #[test]
    fn pcr_fehler() {
        let t = doc("t", T, Topology::Linear);
        let p = params(&[("forward", "ATGCCGTAGCTAGC"), ("reverse", "TTTTTTTTTTTTTT")]);
        let e = run(&PcrOperation, std::slice::from_ref(&t), &p).unwrap_err();
        assert!(e.to_string().contains("No PCR product"));
        assert!(run(&PcrOperation, std::slice::from_ref(&t), &Params::new()).is_err());
        let bad = params(&[
            ("forward", "ATGCCGTAGCTAGC"),
            ("reverse", "AAA"),
            ("max_mismatches", "x"),
        ]);
        assert!(run(&PcrOperation, &[t], &bad).is_err());
    }

    #[test]
    fn codons_optimieren() {
        let reference = format!(
            "ATG{}{}{}",
            "AAG".repeat(3),
            "TTC".repeat(3),
            "GGT".repeat(3)
        );
        let inputs = [
            doc("gen", "ATGAAATTTGGC", Topology::Linear),
            doc("ref", &reference, Topology::Linear),
        ];
        let r = run(&CodonOptimizeOperation, &inputs, &Params::new()).unwrap();
        let o = only_sequence(&r);
        assert_eq!(o.bases(), "ATGAAGTTCGGT");
        assert_eq!(o.name(), "gen (optimized)");
        assert_eq!(r.summary, "Codon-optimized, 12 bp");
        // verbotene Stelle
        let p = params(&[("avoid", "AAGTTC")]);
        let r2 = run(&CodonOptimizeOperation, &inputs, &p).unwrap();
        assert!(!only_sequence(&r2).bases().contains("AAGTTC"));
        assert!(run(&CodonOptimizeOperation, &inputs[..1], &Params::new()).is_err());
    }
}
