//! Gateway-Klonierung (Modell): BP- und LR-Reaktion über die Kerne der att-Stellen.

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};

/// Eine att-Stelle aus linkem Arm, Kern und rechtem Arm.
/// `attB = Bl + Kern + Br`, `attP = Pl + Kern + Pr`, `attL = Bl + Kern + Pr`, `attR = Pl + Kern + Br`.
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

/// Zwei att-Stellen (`att1` und `att2`), die nur mit sich selbst rekombinieren.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gateway {
    pub att1: GatewaySite,
    pub att2: GatewaySite,
}

/// Findet `site` (Uppercase) genau einmal in `bases` (Uppercase) und gibt den Beginn zurück.
/// Fehler `Error::InvalidInput` mit `"{name} nicht gefunden"` (kein Treffer) oder `"{name} occurs multiple times"` (mehr als ein Treffer, auch
/// überlappend).
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

/// Dreht ein zirkuläres Dokument so, dass es bei `first` beginnt, und zerlegt es in `(Insert, Rest)` zwischen `first` und `second`.
/// `first_name`/`second_name` dienen den Fehlermeldungen. Gibt `(Insert, Rest)` zurück.
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

    /// BP-Reaktion `attB × attP → attL + attR`; Rückgabe ist der Entry-Klon.
    ///
    /// - `att_b_product` muss linear, `donor` zirkulär sein, sonst `Error::InvalidInput`.
    /// - Im Produkt (Uppercase) steht `attB1` vor `attB2` (je genau einmal, mit `find_unique` und den Namen `attB1`/`attB2`);
    ///   das Insert ist der Text dazwischen. Im Spender ist `attP1` + `attP2` zu finden (Namen `attP1`/`attP2`), zirkulär: der Spender
    ///   wird so gedreht, dass er bei `attP1` beginnt; `attP2` muss danach hinter `attP1` liegen (sonst `Error::InvalidInput`).
    ///   Der Rest ist der Text hinter `attP2` bis zum Ende des gedrehten Spenders.
    /// - Ergebnis: `attL1 + Insert + attL2 + Rest`, zirkulär, Name `"{Spender} + {Produkt} (BP)"`, DNA, neue ID, Metadaten
    ///   `source = "clone.gateway.bp"`. Features werden nicht übernommen.
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

    /// LR-Reaktion `attL × attR → attB + attP`; Rückgabe ist der Expressionsklon.
    ///
    /// - Beide Dokumente müssen zirkulär sein, sonst `Error::InvalidInput`.
    /// - Im Entry-Klon: `attL1` und `attL2` (Namen `attL1`/`attL2`), gedreht wie oben bei `attL1` beginnend, Insert = Text zwischen den beiden.
    ///   Im Zielvektor: `attR1`, `attR2`, gedreht bei `attR1` beginnend, Rest hinter `attR2`.
    /// - Ergebnis: `attB1 + Insert + attB2 + Rest`, zirkulär, Name `"{Zielvektor} + {Entry-Klon} (LR)"`, Metadaten `source = "clone.gateway.lr"`.
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
        // Spender gedreht: attP1 + SEL + attP2 + Rest ("TTTT" + "CCCC"); Ergebnis attL1 + Insert + attL2 + Rest
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
        // falsche Topologie
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
        // fehlende att-Stelle
        let ohne = doc("ohne", "GATCGATC", Topology::Linear);
        match g.bp_reaction(&ohne, &donor()) {
            Err(Error::InvalidInput(m)) => assert_eq!(m, "attB1 not found"),
            other => panic!("unerwartet: {:?}", other),
        }
        // falsche Reihenfolge der Stellen im Produkt
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
