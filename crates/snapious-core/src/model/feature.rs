//! Annotations: strand, intervals, locations and features (all 0-based, end exclusive).

use crate::error::{Error, Result};

/// Strand of a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Strand {
    Forward,
    Reverse,
    None,
}

impl Strand {
    /// Opposite strand (`None` stays `None`).
    pub fn flipped(self) -> Strand {
        match self {
            Strand::Forward => Strand::Reverse,
            Strand::Reverse => Strand::Forward,
            Strand::None => Strand::None,
        }
    }
}

/// Contiguous range `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    start: usize,
    end: usize,
}

impl Interval {
    /// New interval; `end < start` is invalid.
    pub fn new(start: usize, end: usize) -> Result<Self> {
        if end < start {
            return Err(Error::InvalidInput(format!(
                "Invalid interval {}..{}",
                start, end
            )));
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn length(&self) -> usize {
        self.end - self.start
    }

    /// Do two intervals overlap (at least one shared base)?
    pub fn overlaps(&self, other: &Interval) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Location of a feature: one or more intervals (join) on one strand.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Location {
    intervals: Vec<Interval>,
    strand: Strand,
    partial_start: bool,
    partial_end: bool,
}

impl Location {
    /// New location; at least one interval required.
    pub fn new(
        intervals: Vec<Interval>,
        strand: Strand,
        partial_start: bool,
        partial_end: bool,
    ) -> Result<Self> {
        if intervals.is_empty() {
            return Err(Error::InvalidInput(
                "Location needs at least one interval".to_string(),
            ));
        }
        Ok(Self {
            intervals,
            strand,
            partial_start,
            partial_end,
        })
    }

    /// Simple location `[start, end)`.
    pub fn of(start: usize, end: usize, strand: Strand) -> Result<Self> {
        Self::new(vec![Interval::new(start, end)?], strand, false, false)
    }

    pub fn intervals(&self) -> &[Interval] {
        &self.intervals
    }

    pub fn strand(&self) -> Strand {
        self.strand
    }

    pub fn partial_start(&self) -> bool {
        self.partial_start
    }

    pub fn partial_end(&self) -> bool {
        self.partial_end
    }

    /// Smallest start of all intervals.
    pub fn start(&self) -> usize {
        self.intervals.iter().map(|i| i.start).min().unwrap_or(0)
    }

    /// Largest end of all intervals.
    pub fn end(&self) -> usize {
        self.intervals.iter().map(|i| i.end).max().unwrap_or(0)
    }

    /// Sum of the interval lengths.
    pub fn length(&self) -> usize {
        self.intervals.iter().map(|i| i.length()).sum()
    }

    /// Is the position (base index) in one of the intervals?
    pub fn contains(&self, position: usize) -> bool {
        self.intervals
            .iter()
            .any(|i| position >= i.start && position < i.end)
    }

    /// Shift all intervals by `delta` (the result must not become negative).
    pub fn shifted(&self, delta: isize) -> Result<Location> {
        let mut out = Vec::with_capacity(self.intervals.len());
        for i in &self.intervals {
            let s = i.start as isize + delta;
            let e = i.end as isize + delta;
            if s < 0 || e < 0 {
                return Err(Error::OutOfRange(format!(
                    "Shift by {} results in a negative position",
                    delta
                )));
            }
            out.push(Interval::new(s as usize, e as usize)?);
        }
        Location::new(out, self.strand, self.partial_start, self.partial_end)
    }

    /// Copy with different intervals (strand and partial markers stay).
    pub fn with_intervals(&self, intervals: Vec<Interval>) -> Result<Location> {
        Location::new(intervals, self.strand, self.partial_start, self.partial_end)
    }

    /// Copy with a different strand and partial markers.
    pub fn with_strand_and_partials(
        &self,
        strand: Strand,
        partial_start: bool,
        partial_end: bool,
    ) -> Location {
        Location {
            intervals: self.intervals.clone(),
            strand,
            partial_start,
            partial_end,
        }
    }
}

/// An annotation (gene, CDS, promoter, …) with qualifiers in input order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Feature {
    feature_type: String,
    name: String,
    location: Location,
    qualifiers: Vec<(String, Vec<String>)>,
}

impl Feature {
    pub fn new(
        feature_type: &str,
        name: &str,
        location: Location,
        qualifiers: Vec<(String, Vec<String>)>,
    ) -> Self {
        Self {
            feature_type: feature_type.to_string(),
            name: name.to_string(),
            location,
            qualifiers,
        }
    }

    /// Type, e.g. `CDS`, `gene`, `promoter`.
    pub fn feature_type(&self) -> &str {
        &self.feature_type
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn qualifiers(&self) -> &[(String, Vec<String>)] {
        &self.qualifiers
    }

    /// First value of the qualifier `key`, if present.
    pub fn qualifier(&self, key: &str) -> Option<&str> {
        self.qualifiers
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.first())
            .map(|s| s.as_str())
    }

    /// Copy with a new location.
    pub fn with_location(&self, location: Location) -> Feature {
        Feature {
            location,
            ..self.clone()
        }
    }

    /// Copy with a different name.
    pub fn with_name(&self, name: &str) -> Feature {
        Feature {
            name: name.to_string(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn einfache_position() {
        let l = Location::of(10, 20, Strand::Forward).unwrap();
        assert_eq!(l.length(), 10);
        assert!(l.contains(10));
        assert!(!l.contains(20));
    }

    #[test]
    fn join_position() {
        let l = Location::new(
            vec![
                Interval::new(0, 10).unwrap(),
                Interval::new(20, 25).unwrap(),
            ],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        assert_eq!(l.length(), 15);
        assert_eq!(l.start(), 0);
        assert_eq!(l.end(), 25);
        assert!(!l.contains(15));
        assert!(l.contains(22));
    }

    #[test]
    fn verschieben() {
        let l = Location::of(10, 20, Strand::Reverse)
            .unwrap()
            .shifted(5)
            .unwrap();
        assert_eq!((l.start(), l.end()), (15, 25));
        assert_eq!(l.strand(), Strand::Reverse);
        assert!(Location::of(2, 4, Strand::None)
            .unwrap()
            .shifted(-3)
            .is_err());
    }

    #[test]
    fn ungueltiges_intervall() {
        assert!(Interval::new(5, 3).is_err());
        assert!(Interval::new(5, 5).is_ok());
        assert!(Location::new(vec![], Strand::None, false, false).is_err());
    }

    #[test]
    fn ueberlappung() {
        let a = Interval::new(0, 10).unwrap();
        assert!(a.overlaps(&Interval::new(9, 12).unwrap()));
        assert!(!a.overlaps(&Interval::new(10, 12).unwrap()));
    }

    #[test]
    fn qualifier_zugriff() {
        let f = Feature::new(
            "gene",
            "abc",
            Location::of(0, 3, Strand::Forward).unwrap(),
            vec![(
                "gene".to_string(),
                vec!["abc".to_string(), "zweiter".to_string()],
            )],
        );
        assert_eq!(f.qualifier("gene"), Some("abc"));
        assert_eq!(f.qualifier("fehlt"), None);
        assert_eq!(f.feature_type(), "gene");
    }

    #[test]
    fn mit_neuer_position() {
        let f = Feature::new(
            "CDS",
            "x",
            Location::of(0, 3, Strand::Forward).unwrap(),
            vec![],
        );
        let g = f.with_location(Location::of(5, 9, Strand::Reverse).unwrap());
        assert_eq!(g.location().start(), 5);
        assert_eq!(f.location().start(), 0);
    }

    #[test]
    fn strang_umdrehen() {
        assert_eq!(Strand::Forward.flipped(), Strand::Reverse);
        assert_eq!(Strand::None.flipped(), Strand::None);
    }
}
