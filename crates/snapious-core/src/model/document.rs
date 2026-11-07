//! Dokumente: Sequenzdokument mit Annotationen, Topologie und Metadaten.
//!
//! Alle Änderungen liefern ein NEUES Dokument (gleiche ID), das Original bleibt unverändert.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::feature::{Feature, Interval, Location};
use crate::model::sequence::Sequence;
use crate::util::{new_id, now_secs};

/// Linear oder zirkulär (Plasmid).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Topology {
    Linear,
    Circular,
}

/// Ein Sequenzdokument.
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceDocument {
    id: String,
    name: String,
    created: i64,
    sequence: Sequence,
    topology: Topology,
    features: Vec<Feature>,
    metadata: BTreeMap<String, String>,
}

impl SequenceDocument {
    pub fn new(
        id: &str,
        name: &str,
        sequence: Sequence,
        topology: Topology,
        features: Vec<Feature>,
        metadata: BTreeMap<String, String>,
        created: i64,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            created,
            sequence,
            topology,
            features,
            metadata,
        }
    }

    /// Neues lineares Dokument ohne Features mit neuer ID.
    pub fn of(name: &str, sequence: Sequence) -> Self {
        Self::new(
            &new_id(),
            name,
            sequence,
            Topology::Linear,
            Vec::new(),
            BTreeMap::new(),
            now_secs(),
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Erstellungszeit in Sekunden seit 1970 (UTC).
    pub fn created(&self) -> i64 {
        self.created
    }

    pub fn sequence(&self) -> &Sequence {
        &self.sequence
    }

    pub fn topology(&self) -> Topology {
        self.topology
    }

    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }

    pub fn len(&self) -> usize {
        self.sequence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sequence.is_empty()
    }

    pub fn bases(&self) -> &str {
        self.sequence.bases()
    }

    /// Kopie als neues Dokument: neue ID, neue Erstellungszeit, neuer Name.
    pub fn derive(&self, name: &str) -> Self {
        Self {
            id: new_id(),
            name: name.to_string(),
            created: now_secs(),
            ..self.clone()
        }
    }

    pub fn with_name(&self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self.clone()
        }
    }

    pub fn with_topology(&self, topology: Topology) -> Self {
        Self {
            topology,
            ..self.clone()
        }
    }

    pub fn with_features(&self, features: Vec<Feature>) -> Self {
        Self {
            features,
            ..self.clone()
        }
    }

    pub fn with_metadata(&self, metadata: BTreeMap<String, String>) -> Self {
        Self {
            metadata,
            ..self.clone()
        }
    }

    /// Fügt `bases` vor Position `position` ein. Features werden mitgeführt:
    /// Intervalle ab `position` wandern, Intervalle über `position` hinweg wachsen,
    /// Intervalle, die genau bei `position` enden, bleiben unverändert.
    pub fn insert(&self, position: usize, bases: &str) -> Result<Self> {
        if position > self.len() {
            return Err(Error::OutOfRange(format!(
                "Insert position {} with length {}",
                position,
                self.len()
            )));
        }
        let cleaned = self.sequence.with_bases(bases)?;
        let ins = cleaned.bases();
        let n = ins.len();
        let mut combined = String::with_capacity(self.len() + n);
        combined.push_str(&self.bases()[..position]);
        combined.push_str(ins);
        combined.push_str(&self.bases()[position..]);
        let sequence = self.sequence.with_bases(&combined)?;
        let features = self
            .features
            .iter()
            .map(|f| shift_for_insert(f, position, n))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            sequence,
            features,
            ..self.clone()
        })
    }

    /// Löscht `[start, end)`. Features werden gekürzt, verschoben oder entfernt.
    pub fn delete(&self, start: usize, end: usize) -> Result<Self> {
        self.check_range(start, end)?;
        let mut combined = String::with_capacity(self.len() - (end - start));
        combined.push_str(&self.bases()[..start]);
        combined.push_str(&self.bases()[end..]);
        let sequence = self.sequence.with_bases(&combined)?;
        let mut features = Vec::new();
        for f in &self.features {
            if let Some(g) = shift_for_delete(f, start, end)? {
                features.push(g);
            }
        }
        Ok(Self {
            sequence,
            features,
            ..self.clone()
        })
    }

    /// Ersetzt `[start, end)` durch `bases` (Löschen, dann Einfügen bei `start`).
    pub fn replace(&self, start: usize, end: usize, bases: &str) -> Result<Self> {
        self.delete(start, end)?.insert(start, bases)
    }

    /// Ausschnitt `[start, end)` mit allen vollständig darin liegenden Features (Positionen minus `start`).
    pub fn sub_document(&self, start: usize, end: usize) -> Result<Self> {
        self.check_range(start, end)?;
        let sequence = self.sequence.with_bases(&self.bases()[start..end])?;
        let mut features = Vec::new();
        for f in &self.features {
            let loc = f.location();
            if loc.start() >= start && loc.end() <= end {
                features.push(f.with_location(loc.shifted(-(start as isize))?));
            }
        }
        Ok(Self {
            sequence,
            features,
            ..self.clone()
        })
    }

    /// Reverse-Komplement: Features werden gespiegelt, der Strang gedreht.
    pub fn reverse_complement(&self) -> Result<Self> {
        let len = self.len();
        let sequence = self.sequence.reverse_complement()?;
        let sequence = sequence.with_bases(sequence.bases())?;
        let mut features = Vec::with_capacity(self.features.len());
        for f in &self.features {
            let loc = f.location();
            let mut intervals = Vec::with_capacity(loc.intervals().len());
            for iv in loc.intervals().iter().rev() {
                intervals.push(Interval::new(len - iv.end(), len - iv.start())?);
            }
            let mirrored = Location::new(
                intervals,
                loc.strand().flipped(),
                loc.partial_end(),
                loc.partial_start(),
            )?;
            features.push(f.with_location(mirrored));
        }
        Ok(Self {
            sequence,
            features,
            ..self.clone()
        })
    }

    fn check_range(&self, start: usize, end: usize) -> Result<()> {
        if end > self.len() {
            return Err(Error::OutOfRange(format!(
                "Range {}-{} with length {}",
                start,
                end,
                self.len()
            )));
        }
        if start > end {
            return Err(Error::InvalidInput(format!(
                "Invalid range {}-{}",
                start, end
            )));
        }
        Ok(())
    }
}

fn shift_for_insert(f: &Feature, pos: usize, n: usize) -> Result<Feature> {
    let loc = f.location();
    let mut out = Vec::with_capacity(loc.intervals().len());
    for iv in loc.intervals() {
        let (s, e) = (iv.start(), iv.end());
        let (ns, ne) = if s >= pos {
            (s + n, e + n)
        } else if e > pos {
            (s, e + n)
        } else {
            (s, e)
        };
        out.push(Interval::new(ns, ne)?);
    }
    Ok(f.with_location(loc.with_intervals(out)?))
}

fn shift_for_delete(f: &Feature, start: usize, end: usize) -> Result<Option<Feature>> {
    let d = end - start;
    let map = |p: usize| -> usize {
        if p <= start {
            p
        } else if p >= end {
            p - d
        } else {
            start
        }
    };
    let loc = f.location();
    let mut out = Vec::new();
    for iv in loc.intervals() {
        let (ns, ne) = (map(iv.start()), map(iv.end()));
        if ns == ne && iv.length() > 0 {
            continue; // Intervall lag vollständig im gelöschten Bereich
        }
        out.push(Interval::new(ns, ne)?);
    }
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(f.with_location(loc.with_intervals(out)?)))
}

/// Ein Dokument des Projekts.
#[derive(Clone, Debug, PartialEq)]
pub enum Document {
    Sequence(SequenceDocument),
    Alignment(crate::model::alignment::AlignmentDocument),
}

impl Document {
    pub fn id(&self) -> &str {
        match self {
            Document::Sequence(d) => d.id(),
            Document::Alignment(d) => d.id(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Document::Sequence(d) => d.name(),
            Document::Alignment(d) => d.name(),
        }
    }

    pub fn created(&self) -> i64 {
        match self {
            Document::Sequence(d) => d.created(),
            Document::Alignment(d) => d.created(),
        }
    }

    /// Dokumenttyp als Text für die Datenbank (`sequence` oder `alignment`).
    pub fn doc_type(&self) -> &'static str {
        match self {
            Document::Sequence(_) => "sequence",
            Document::Alignment(_) => "alignment",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::feature::Strand;
    use crate::model::sequence::SequenceType;

    fn doc_with_gene() -> SequenceDocument {
        let seq = Sequence::new("s", "S", "ACGTACGTAC", SequenceType::Dna).unwrap();
        let f = Feature::new(
            "gene",
            "g",
            Location::of(2, 6, Strand::Forward).unwrap(),
            vec![],
        );
        SequenceDocument::of("d", seq).with_features(vec![f])
    }

    fn span(d: &SequenceDocument) -> (usize, usize) {
        (
            d.features()[0].location().start(),
            d.features()[0].location().end(),
        )
    }

    #[test]
    fn einfuegen_vor_dem_feature() {
        let d = doc_with_gene().insert(0, "TT").unwrap();
        assert_eq!(d.bases(), "TTACGTACGTAC");
        assert_eq!(span(&d), (4, 8));
    }

    #[test]
    fn einfuegen_im_feature_verlaengert() {
        let d = doc_with_gene().insert(4, "TT").unwrap();
        assert_eq!(span(&d), (2, 8));
    }

    #[test]
    fn einfuegen_direkt_hinter_dem_feature_aendert_nichts() {
        let d = doc_with_gene().insert(6, "TT").unwrap();
        assert_eq!(span(&d), (2, 6));
    }

    #[test]
    fn einfuegen_am_ende_und_ausserhalb() {
        let d = doc_with_gene();
        assert_eq!(d.insert(10, "A").unwrap().len(), 11);
        assert!(d.insert(11, "A").is_err());
        assert!(d.insert(0, "X").is_err());
    }

    #[test]
    fn loeschen_variationen() {
        let d = doc_with_gene();
        let a = d.delete(0, 3).unwrap();
        assert_eq!(a.bases(), "TACGTAC");
        assert_eq!(span(&a), (0, 3));
        assert!(d.delete(2, 6).unwrap().features().is_empty());
        assert_eq!(span(&d.delete(4, 8).unwrap()), (2, 4));
        assert_eq!(span(&d.delete(8, 10).unwrap()), (2, 6));
    }

    #[test]
    fn loeschen_ungueltig() {
        let d = doc_with_gene();
        assert!(d.delete(5, 3).is_err());
        assert!(d.delete(0, 11).is_err());
    }

    #[test]
    fn ersetzen() {
        let d = doc_with_gene().replace(0, 2, "GGG").unwrap();
        assert_eq!(d.bases(), "GGGGTACGTAC");
        assert_eq!(d.len(), 11);
        assert_eq!(span(&d), (3, 7));
    }

    #[test]
    fn reverse_komplement_spiegelt_features() {
        let seq = Sequence::new("s", "S", "AACCGGTT", SequenceType::Dna).unwrap();
        let f = Feature::new(
            "gene",
            "g",
            Location::of(0, 2, Strand::Forward).unwrap(),
            vec![],
        );
        let d = SequenceDocument::of("d", seq).with_features(vec![f]);
        let r = d.reverse_complement().unwrap();
        assert_eq!(r.bases(), "AACCGGTT");
        assert_eq!(span(&r), (6, 8));
        assert_eq!(r.features()[0].location().strand(), Strand::Reverse);
        assert_eq!(r.reverse_complement().unwrap().features(), d.features());
    }

    #[test]
    fn reverse_komplement_kehrt_join_reihenfolge_um() {
        let seq = Sequence::new("s", "S", "ACGTACGTAC", SequenceType::Dna).unwrap();
        let loc = Location::new(
            vec![Interval::new(0, 2).unwrap(), Interval::new(5, 8).unwrap()],
            Strand::Forward,
            true,
            false,
        )
        .unwrap();
        let d = SequenceDocument::of("d", seq).with_features(vec![Feature::new(
            "CDS",
            "c",
            loc,
            vec![],
        )]);
        let l = d.reverse_complement().unwrap().features()[0]
            .location()
            .clone();
        assert_eq!(l.intervals()[0], Interval::new(2, 5).unwrap());
        assert_eq!(l.intervals()[1], Interval::new(8, 10).unwrap());
        assert!(!l.partial_start() && l.partial_end());
    }

    #[test]
    fn ausschnitt_mit_vollstaendigen_features() {
        let d = doc_with_gene().sub_document(2, 8).unwrap();
        assert_eq!(d.bases(), "GTACGT");
        assert_eq!(span(&d), (0, 4));
        assert!(doc_with_gene()
            .sub_document(3, 8)
            .unwrap()
            .features()
            .is_empty());
    }

    #[test]
    fn original_bleibt_unveraendert() {
        let d = doc_with_gene();
        let _ = d.insert(0, "TT").unwrap();
        let _ = d.delete(0, 5).unwrap();
        assert_eq!(d.bases(), "ACGTACGTAC");
        assert_eq!(span(&d), (2, 6));
    }

    #[test]
    fn aenderungen_behalten_die_id() {
        let d = doc_with_gene();
        let e = d
            .insert(0, "A")
            .unwrap()
            .with_name("neu")
            .with_topology(Topology::Circular);
        assert_eq!(e.id(), d.id());
        assert_eq!(e.topology(), Topology::Circular);
        assert_eq!(e.name(), "neu");
    }

    #[test]
    fn dokument_enum_liefert_grunddaten() {
        let d = Document::Sequence(doc_with_gene());
        assert_eq!(d.doc_type(), "sequence");
        assert_eq!(d.name(), "d");
    }
}
