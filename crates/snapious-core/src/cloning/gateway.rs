//! Gateway cloning (model): BP and LR reaction via the cores of the att sites.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};

/// An att site made of left arm, core and right arm.
/// `attB = Bl + core + Br`, `attP = Pl + core + Pr`, `attL = Bl + core + Pr`, `attR = Pl + core + Br`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewaySite {
    pub name: String,
    pub left_arm_b: String,
    pub core: String,
    pub right_arm_b: String,
    pub left_arm_p: String,
    pub right_arm_p: String,
}

impl GatewaySite {
    pub fn att_b(&self) -> String {
        format!("{}{}{}", self.left_arm_b, self.core, self.right_arm_b)
    }
    pub fn att_p(&self) -> String {
        format!("{}{}{}", self.left_arm_p, self.core, self.right_arm_p)
    }
    pub fn att_l(&self) -> String {
        format!("{}{}{}", self.left_arm_b, self.core, self.right_arm_p)
    }
    pub fn att_r(&self) -> String {
        format!("{}{}{}", self.left_arm_p, self.core, self.right_arm_b)
    }
}

/// Two att sites (`att1` and `att2`) that recombine only with themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gateway {
    pub att1: GatewaySite,
    pub att2: GatewaySite,
}

/// Finds `site` (uppercase) exactly once in `bases` (uppercase) and returns the start.
/// Error `Error::InvalidInput` with `"{name} nicht gefunden"` (no hit) or `"{name} occurs multiple times"` (more than one hit, also
/// overlapping).
pub fn find_unique(bases: &str, site: &str, name: &str) -> Result<usize> {
    let hits: Vec<usize> = if site.is_empty() || site.len() > bases.len() {
        Vec::new()
    } else {
        (0..=bases.len() - site.len())
            .filter(|i| bases.get(*i..*i + site.len()) == Some(site))
            .collect()
    };
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(Error::InvalidInput(format!("{} not found", name))),
        _ => Err(Error::InvalidInput(format!("{} occurs multiple times", name))),
    }
}

/// Rotates a circular document so that it begins at `first`, and splits it into `(insert, rest)` between `first` and `second`.
/// `first_name`/`second_name` are used for the error messages. Returns `(insert, rest)`.
fn split_between(
    bases: &str,
    first: &str,
    second: &str,
    first_name: &str,
    second_name: &str,
) -> Result<(String, String)> {
    let upper = bases.to_ascii_uppercase();
    let start = find_unique(&upper, first, first_name)?;
    let rotated = format!(
        "{}{}",
        upper.get(start..).unwrap_or(""),
        upper.get(..start).unwrap_or("")
    );
    let after_first = first.len();
    let second_pos = find_unique(&rotated, second, second_name)?;
    if second_pos < after_first {
        return Err(Error::InvalidInput(format!(
            "{} is not downstream of {}",
            second_name, first_name
        )));
    }
    let insert = rotated
        .get(after_first..second_pos)
        .unwrap_or("")
        .to_string();
    let rest = rotated
        .get(second_pos + second.len()..)
        .unwrap_or("")
        .to_string();
    Ok((insert, rest))
}

fn new_construct(name: &str, bases: &str, source: &str) -> Result<SequenceDocument> {
    use crate::model::sequence::{Sequence, SequenceType};
    use crate::util::{new_id, now_secs};
    let seq = Sequence::new(name, name, bases, SequenceType::Dna)?;
    let metadata = [("source".to_string(), source.to_string())]
        .into_iter()
        .collect();
    Ok(SequenceDocument::new(
        &new_id(),
        name,
        seq,
        Topology::Circular,
        Vec::new(),
        metadata,
        now_secs(),
    ))
}

impl Gateway {
    pub fn new(att1: GatewaySite, att2: GatewaySite) -> Gateway {
        Gateway { att1, att2 }
    }

    /// BP reaction `attB × attP → attL + attR`; returns the entry clone.
    ///
    /// - `att_b_product` must be linear, `donor` circular, otherwise `Error::InvalidInput`.
    /// - In the product (uppercase) `attB1` comes before `attB2` (each exactly once, via `find_unique` and the names `attB1`/`attB2`);
    ///   the insert is the text in between. In the donor `attP1` + `attP2` are to be found (names `attP1`/`attP2`), circular: the donor
    ///   is rotated so that it begins at `attP1`; `attP2` must then lie after `attP1` (otherwise `Error::InvalidInput`).
    ///   The rest is the text after `attP2` up to the end of the rotated donor.
    /// - Result: `attL1 + insert + attL2 + rest`, circular, name `"{donor} + {product} (BP)"`, DNA, new ID, metadata
    ///   `source = "clone.gateway.bp"`. Features are not carried over.
    pub fn bp_reaction(
        &self,
        att_b_product: &SequenceDocument,
        donor: &SequenceDocument,
    ) -> Result<SequenceDocument> {
        if att_b_product.topology() != Topology::Linear {
            return Err(Error::InvalidInput(
                "The product with the attB sites must be linear".to_string(),
            ));
        }
        if donor.topology() != Topology::Circular {
            return Err(Error::InvalidInput(
                "The donor vector must be circular".to_string(),
            ));
        }
        let product = att_b_product.bases().to_ascii_uppercase();
        let b1 = find_unique(&product, &self.att1.att_b().to_ascii_uppercase(), "attB1")?;
        let b2 = find_unique(&product, &self.att2.att_b().to_ascii_uppercase(), "attB2")?;
        let insert_start = b1 + self.att1.att_b().len();
        if b2 < insert_start {
            return Err(Error::InvalidInput(
                "attB2 is not downstream of attB1".to_string(),
            ));
        }
        let insert = product.get(insert_start..b2).unwrap_or("");
        let (_removed, rest) = split_between(
            donor.bases(),
            &self.att1.att_p().to_ascii_uppercase(),
            &self.att2.att_p().to_ascii_uppercase(),
            "attP1",
            "attP2",
        )?;
        let bases = format!(
            "{}{}{}{}",
            self.att1.att_l(),
            insert,
            self.att2.att_l(),
            rest
        )
        .to_ascii_uppercase();
        new_construct(
            &format!("{} + {} (BP)", donor.name(), att_b_product.name()),
            &bases,
            "clone.gateway.bp",
        )
    }

    /// LR reaction `attL × attR → attB + attP`; returns the expression clone.
    ///
    /// - Both documents must be circular, otherwise `Error::InvalidInput`.
    /// - In the entry clone: `attL1` and `attL2` (names `attL1`/`attL2`), rotated as above to begin at `attL1`, insert = text between the two.
    ///   In the destination vector: `attR1`, `attR2`, rotated to begin at `attR1`, rest after `attR2`.
    /// - Result: `attB1 + insert + attB2 + rest`, circular, name `"{destination vector} + {entry clone} (LR)"`, metadata `source = "clone.gateway.lr"`.
    pub fn lr_reaction(
        &self,
        entry_clone: &SequenceDocument,
        destination: &SequenceDocument,
    ) -> Result<SequenceDocument> {
        if entry_clone.topology() != Topology::Circular
            || destination.topology() != Topology::Circular
        {
            return Err(Error::InvalidInput(
                "Entry clone and destination vector must be circular".to_string(),
            ));
        }
        let (insert, _) = split_between(
            entry_clone.bases(),
            &self.att1.att_l().to_ascii_uppercase(),
            &self.att2.att_l().to_ascii_uppercase(),
            "attL1",
            "attL2",
        )?;
        let (_removed, rest) = split_between(
            destination.bases(),
            &self.att1.att_r().to_ascii_uppercase(),
            &self.att2.att_r().to_ascii_uppercase(),
            "attR1",
            "attR2",
        )?;
        let bases = format!(
            "{}{}{}{}",
            self.att1.att_b(),
            insert,
            self.att2.att_b(),
            rest
        )
        .to_ascii_uppercase();
        new_construct(
            &format!("{} + {} (LR)", destination.name(), entry_clone.name()),
            &bases,
            "clone.gateway.lr",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::{Sequence, SequenceType};

    fn site(name: &str, bl: &str, core: &str, br: &str, pl: &str, pr: &str) -> GatewaySite {
        GatewaySite {
            name: name.to_string(),
            left_arm_b: bl.to_string(),
            core: core.to_string(),
            right_arm_b: br.to_string(),
            left_arm_p: pl.to_string(),
            right_arm_p: pr.to_string(),
        }
    }

    fn att1() -> GatewaySite {
        site("att1", "AAAA", "TTGT", "CCCC", "GGGG", "ATAT")
    }
    fn att2() -> GatewaySite {
        site("att2", "ACAC", "TTCT", "CGCG", "GTGT", "TATA")
    }

    fn doc(name: &str, bases: &str, topology: Topology) -> SequenceDocument {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        SequenceDocument::of(name, seq).with_topology(topology)
    }

    #[test]
    fn att_stellen() {
        let s = att1();
        assert_eq!(s.att_b(), "AAAATTGTCCCC");
        assert_eq!(s.att_p(), "GGGGTTGTATAT");
        assert_eq!(s.att_l(), "AAAATTGTATAT");
        assert_eq!(s.att_r(), "GGGGTTGTCCCC");
    }

    #[test]
    fn eindeutige_suche() {
        assert_eq!(find_unique("GGAAAATTGTGG", "AAAATTGT", "x").unwrap(), 2);
        match find_unique("GGGG", "AAAA", "attB1") {
            Err(Error::InvalidInput(m)) => assert_eq!(m, "attB1 not found"),
            other => panic!("unerwartet: {:?}", other),
        }
        match find_unique("AAAAAA", "AAAA", "attB2") {
            Err(Error::InvalidInput(m)) => assert_eq!(m, "attB2 occurs multiple times"),
            other => panic!("unerwartet: {:?}", other),
        }
    }

    const INS: &str = "ACGTACGTAC";

    fn product() -> SequenceDocument {
        let g = Gateway::new(att1(), att2());
        doc(
            "prod",
            &format!("GATC{}{}{}CTAG", g.att1.att_b(), INS, g.att2.att_b()),
            Topology::Linear,
        )
    }

    fn donor() -> SequenceDocument {
        let g = Gateway::new(att1(), att2());
        doc(
            "donor",
            &format!("DDDD{}SELSELSEL{}EEEE", g.att1.att_p(), g.att2.att_p())
                .replace('D', "C")
                .replace('S', "G")
                .replace('E', "T")
                .replace('L', "A"),
            Topology::Circular,
        )
    }

    #[test]
    fn bp_reaktion() {
        let g = Gateway::new(att1(), att2());
        let entry = g.bp_reaction(&product(), &donor()).unwrap();
        // Donor rotated: attP1 + SEL + attP2 + rest ("TTTT" + "CCCC"); result attL1 + insert + attL2 + rest
        let expected = format!("{}{}{}{}", att1().att_l(), INS, att2().att_l(), "TTTTCCCC");
        assert_eq!(entry.bases(), expected);
        assert_eq!(entry.topology(), Topology::Circular);
        assert_eq!(entry.name(), "donor + prod (BP)");
        assert!(!entry.bases().contains("GTAGTAGTA"));
        assert_eq!(
            entry.metadata().get("source").map(String::as_str),
            Some("clone.gateway.bp")
        );
    }

    #[test]
    fn lr_reaktion() {
        let g = Gateway::new(att1(), att2());
        let entry = g.bp_reaction(&product(), &donor()).unwrap();
        let dest = doc(
            "dest",
            &format!("FFFF{}CCDBCCDB{}GGGG", att1().att_r(), att2().att_r())
                .replace('F', "T")
                .replace('D', "A")
                .replace('B', "G"),
            Topology::Circular,
        );
        let expr = g.lr_reaction(&entry, &dest).unwrap();
        let expected = format!("{}{}{}{}", att1().att_b(), INS, att2().att_b(), "GGGGTTTT");
        assert_eq!(expr.bases(), expected);
        assert_eq!(expr.name(), "dest + donor + prod (BP) (LR)");
        assert_eq!(
            expr.metadata().get("source").map(String::as_str),
            Some("clone.gateway.lr")
        );
        assert_eq!(expr.topology(), Topology::Circular);
    }

    #[test]
    fn fehler() {
        let g = Gateway::new(att1(), att2());
        // wrong topology
        assert!(matches!(
            g.bp_reaction(&donor(), &donor()),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            g.bp_reaction(&product(), &product()),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            g.lr_reaction(&product(), &donor()),
            Err(Error::InvalidInput(_))
        ));
        // missing att site
        let ohne = doc("ohne", "GATCGATC", Topology::Linear);
        match g.bp_reaction(&ohne, &donor()) {
            Err(Error::InvalidInput(m)) => assert_eq!(m, "attB1 not found"),
            other => panic!("unerwartet: {:?}", other),
        }
        // wrong order of the sites in the product
        let falsch = doc(
            "falsch",
            &format!("{}{}{}", att2().att_b(), INS, att1().att_b()),
            Topology::Linear,
        );
        assert!(matches!(
            g.bp_reaction(&falsch, &donor()),
            Err(Error::InvalidInput(_))
        ));
    }
}
