//! Restriktionsenzyme: Datenbank, Suche auf beiden Strängen (IUPAC), Schnittstellen und Überhänge.
//!
//! Schnittstellen werden als Zahl der Basen links des Schnitts relativ zum Beginn der Erkennungssequenz
//! angegeben (`cut_top` im Oberstrang, `cut_bottom` im Unterstrang, beides in Oberstrang-Koordinaten).
//! Beispiel EcoRI `G^AATTC`: `cut_top = 1`, `cut_bottom = 5`, 5'-Überhang `AATT`.
//! Bei Typ-IIS-Enzymen kann der Schnitt hinter der Erkennungssequenz liegen.
//! Daten nach NEB/REBASE (Auswahl).

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::model::document::{SequenceDocument, Topology};

/// Art des Enzyms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnzymeType {
    TypeI,
    TypeII,
    /// Schneidet außerhalb der Erkennungssequenz (Golden Gate).
    TypeIIS,
    /// Palindromische Erkennungssequenz (klassische Restriktionsenzyme).
    TypeIIP,
}

/// Dam-/CpG-Methylierung kann diese Enzyme blockieren (laut NEB).
const METHYLATION_SENSITIVE: [&str; 9] = [
    "AscI", "BclI", "BssHII", "ClaI", "HpaII", "MboI", "NotI", "SmaI", "XbaI",
];

const IUPAC: &[u8] = b"ACGTRYKMSWBDHVN";
const IUPAC_COMPLEMENT: &[u8] = b"TGCAYRMKWSVHDBN";

/// Reverse-Komplement mit IUPAC-Codes (unbekannte Zeichen werden zu `N`).
pub fn iupac_reverse_complement(seq: &str) -> String {
    seq.bytes()
        .rev()
        .map(|b| {
            IUPAC
                .iter()
                .position(|c| *c == b.to_ascii_uppercase())
                .map(|i| IUPAC_COMPLEMENT[i] as char)
                .unwrap_or('N')
        })
        .collect()
}

fn pattern_mask(c: u8) -> u8 {
    match c {
        b'A' => 1,
        b'C' => 2,
        b'G' => 4,
        b'T' | b'U' => 8,
        b'R' => 1 | 4,
        b'Y' => 2 | 8,
        b'K' => 4 | 8,
        b'M' => 1 | 2,
        b'S' => 2 | 4,
        b'W' => 1 | 8,
        b'B' => 2 | 4 | 8,
        b'D' => 1 | 4 | 8,
        b'H' => 1 | 2 | 8,
        b'V' => 1 | 2 | 4,
        _ => 15, // N
    }
}

/// Maske einer Base in der Sequenz: nur eindeutige Basen passen auf Mehrdeutigkeitscodes des Musters.
#[inline]
fn base_mask(b: u8) -> u8 {
    match b {
        b'A' => 1,
        b'C' => 2,
        b'G' => 4,
        b'T' | b'U' => 8,
        _ => 0,
    }
}

/// Treffer einer Erkennungssequenz in einer Sequenz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteMatch {
    /// Beginn der Erkennungssequenz im Oberstrang (0-basiert).
    pub start: usize,
    /// `true`, wenn die Erkennungssequenz im Oberstrang steht, `false` bei Treffer auf dem Gegenstrang.
    pub forward: bool,
    /// Anzahl Basen links vom Schnitt im Oberstrang (kann außerhalb `0..=len` liegen).
    pub cut_top: i64,
    /// Anzahl Basen links vom Schnitt im Unterstrang, in Oberstrang-Koordinaten.
    pub cut_bottom: i64,
}

/// Ein Restriktionsenzym.
#[derive(Clone, Debug, PartialEq)]
pub struct RestrictionEnzyme {
    name: String,
    recognition: String,
    cut_top: usize,
    cut_bottom: usize,
    enzyme_type: EnzymeType,
    organism: String,
    supplier: String,
    temperature: String,
    buffer: String,
    palindromic: bool,
    forward_mask: Vec<u8>,
    reverse_mask: Vec<u8>,
}

impl RestrictionEnzyme {
    /// Neues Enzym. `cut_top`/`cut_bottom` wie in der Moduldokumentation. `^` in der Erkennungssequenz wird entfernt.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: &str,
        recognition: &str,
        cut_top: usize,
        cut_bottom: usize,
        enzyme_type: EnzymeType,
        organism: &str,
        supplier: &str,
        temperature: &str,
        buffer: &str,
    ) -> Result<Self> {
        let site: String = recognition
            .to_ascii_uppercase()
            .chars()
            .filter(|c| *c != '^')
            .collect();
        if site.is_empty() || !site.bytes().all(|b| IUPAC.contains(&b)) {
            return Err(Error::InvalidInput(format!(
                "Invalid recognition sequence for {}: '{}'",
                name, recognition
            )));
        }
        let rc = iupac_reverse_complement(&site);
        Ok(Self {
            name: name.to_string(),
            palindromic: rc == site,
            forward_mask: site.bytes().map(pattern_mask).collect(),
            reverse_mask: rc.bytes().map(pattern_mask).collect(),
            recognition: site,
            cut_top,
            cut_bottom,
            enzyme_type,
            organism: organism.to_string(),
            supplier: supplier.to_string(),
            temperature: temperature.to_string(),
            buffer: buffer.to_string(),
        })
    }

    /// Kurzform für symmetrische Schnitte: Unterstrang an der gespiegelten Position
    /// (`Länge − cut`); `sticky = false` erzwingt einen stumpfen Schnitt.
    #[allow(clippy::too_many_arguments)]
    pub fn symmetric(
        name: &str,
        recognition: &str,
        cut: usize,
        enzyme_type: EnzymeType,
        sticky: bool,
        organism: &str,
        supplier: &str,
        temperature: &str,
        buffer: &str,
    ) -> Result<Self> {
        let len = recognition.chars().filter(|c| *c != '^').count();
        let bottom = if sticky { len.saturating_sub(cut) } else { cut };
        Self::new(
            name,
            recognition,
            cut,
            bottom,
            enzyme_type,
            organism,
            supplier,
            temperature,
            buffer,
        )
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Erkennungssequenz (IUPAC, Uppercase, ohne Schnittmarkierung).
    pub fn recognition(&self) -> &str {
        &self.recognition
    }

    /// Schnittstelle im Oberstrang relativ zum Beginn der Erkennungssequenz.
    pub fn cut_top(&self) -> usize {
        self.cut_top
    }

    /// Schnittstelle im Unterstrang (Oberstrang-Koordinaten).
    pub fn cut_bottom(&self) -> usize {
        self.cut_bottom
    }

    pub fn enzyme_type(&self) -> EnzymeType {
        self.enzyme_type
    }

    pub fn organism(&self) -> &str {
        &self.organism
    }

    pub fn supplier(&self) -> &str {
        &self.supplier
    }

    pub fn temperature(&self) -> &str {
        &self.temperature
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn is_palindromic(&self) -> bool {
        self.palindromic
    }

    pub fn is_blunt(&self) -> bool {
        self.cut_top == self.cut_bottom
    }

    pub fn is_sticky(&self) -> bool {
        !self.is_blunt()
    }

    /// Überhang: positiv = 5'-Überhang, negativ = 3'-Überhang, 0 = stumpf.
    pub fn overhang_length(&self) -> i64 {
        self.cut_bottom as i64 - self.cut_top as i64
    }

    pub fn is_methylation_sensitive(&self) -> bool {
        METHYLATION_SENSITIVE.contains(&self.name.as_str())
    }

    /// Überhangsequenz (Oberstrang-Schreibweise), wenn sie festliegt: nicht bei stumpfen Enden, Typ IIS
    /// (Schnitt außerhalb der Erkennungssequenz) oder Überhängen mit mehrdeutigen Basen.
    pub fn overhang_sequence(&self) -> Option<String> {
        if self.is_blunt() {
            return None;
        }
        let (from, to) = (
            self.cut_top.min(self.cut_bottom),
            self.cut_top.max(self.cut_bottom),
        );
        if to > self.recognition.len() {
            return None;
        }
        let o = &self.recognition[from..to];
        o.bytes()
            .all(|b| b"ACGT".contains(&b))
            .then(|| o.to_string())
    }

    /// Ligierbar mit `other`: beide stumpf, oder gleiche Überhangsart und komplementäre Überhänge.
    pub fn is_compatible_with(&self, other: &RestrictionEnzyme) -> bool {
        if self.is_blunt() || other.is_blunt() {
            return self.is_blunt() && other.is_blunt();
        }
        if self.overhang_length().signum() != other.overhang_length().signum() {
            return false;
        }
        match (self.overhang_sequence(), other.overhang_sequence()) {
            (Some(a), Some(b)) => iupac_reverse_complement(&a) == b,
            _ => false,
        }
    }

    fn matches_at(seq: &[u8], start: usize, mask: &[u8], pattern_is_n: &[bool]) -> bool {
        for k in 0..mask.len() {
            if !pattern_is_n[k] && base_mask(seq[start + k]) & mask[k] == 0 {
                return false;
            }
        }
        true
    }

    /// Alle Treffer in `sequence` (Uppercase, T statt U) auf beiden Strängen, nach `start` sortiert.
    pub fn find_matches(&self, sequence: &[u8]) -> Vec<SiteMatch> {
        let len = self.recognition.len();
        let mut out = Vec::new();
        if sequence.len() < len {
            return out;
        }
        let fwd_n: Vec<bool> = self.forward_mask.iter().map(|m| *m == 15).collect();
        let rev_n: Vec<bool> = self.reverse_mask.iter().map(|m| *m == 15).collect();
        for i in 0..=(sequence.len() - len) {
            if Self::matches_at(sequence, i, &self.forward_mask, &fwd_n) {
                out.push(SiteMatch {
                    start: i,
                    forward: true,
                    cut_top: (i + self.cut_top) as i64,
                    cut_bottom: (i + self.cut_bottom) as i64,
                });
            }
            if !self.palindromic && Self::matches_at(sequence, i, &self.reverse_mask, &rev_n) {
                out.push(SiteMatch {
                    start: i,
                    forward: false,
                    cut_top: (i + len) as i64 - self.cut_bottom as i64,
                    cut_bottom: (i + len) as i64 - self.cut_top as i64,
                });
            }
        }
        out
    }

    /// Schneidet eine lineare Sequenz an allen Erkennungsstellen (Oberstrang-Fragmente).
    /// Schnitte außerhalb von `1..len` erzeugen keine Fragmente; doppelte Schnittstellen zählen einmal.
    pub fn cut(&self, sequence: &str) -> Vec<String> {
        let upper = sequence.to_ascii_uppercase().replace('U', "T");
        let mut cuts: Vec<i64> = self
            .find_matches(upper.as_bytes())
            .iter()
            .map(|m| m.cut_top)
            .filter(|c| *c > 0 && (*c as usize) < upper.len())
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        let mut fragments = Vec::with_capacity(cuts.len() + 1);
        let mut last = 0usize;
        for c in cuts {
            fragments.push(upper[last..c as usize].to_string());
            last = c as usize;
        }
        fragments.push(upper[last..].to_string());
        fragments
    }
}

/// Eine Schnittstelle eines Enzyms in einer Sequenz.
#[derive(Clone, Debug, PartialEq)]
pub struct CutSite {
    pub enzyme: String,
    pub sequence_id: String,
    /// Beginn der Erkennungssequenz (0-basiert).
    pub position: usize,
    /// Anzahl Basen links vom Schnitt im Oberstrang.
    pub cut_top: i64,
    /// Anzahl Basen links vom Schnitt im Unterstrang (Oberstrang-Koordinaten).
    pub cut_bottom: i64,
    /// `true`, wenn die Erkennungssequenz im Oberstrang steht.
    pub forward: bool,
    /// Überhang in Oberstrang-Richtung; leer bei stumpfen Enden oder Schnitt außerhalb der Sequenz.
    pub overhang: String,
}

impl CutSite {
    pub fn is_blunt(&self) -> bool {
        self.cut_top == self.cut_bottom
    }

    /// Überhang oder `None` bei stumpfen Enden.
    pub fn overhang(&self) -> Option<&str> {
        (!self.overhang.is_empty()).then_some(self.overhang.as_str())
    }
}

impl std::fmt::Display for CutSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CutSite[{}@{} in {}]",
            self.enzyme, self.position, self.sequence_id
        )
    }
}

/// Sammlung von Enzymen.
#[derive(Clone, Debug, Default)]
pub struct EnzymeDb {
    enzymes: Vec<RestrictionEnzyme>,
}

/// (Name, Erkennung, cut_top, cut_bottom, Typ, Organismus, Temperatur)
type Row = (
    &'static str,
    &'static str,
    usize,
    usize,
    EnzymeType,
    &'static str,
    &'static str,
);

const STANDARD: &[Row] = &[
    (
        "EcoRI",
        "GAATTC",
        1,
        5,
        EnzymeType::TypeIIP,
        "E. coli",
        "37°C",
    ),
    (
        "BamHI",
        "GGATCC",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus amyloliquefaciens",
        "37°C",
    ),
    (
        "HindIII",
        "AAGCTT",
        1,
        5,
        EnzymeType::TypeIIP,
        "Haemophilus influenzae",
        "37°C",
    ),
    (
        "NotI",
        "GCGGCCGC",
        2,
        6,
        EnzymeType::TypeIIP,
        "Nocardia otitidis-caviarum",
        "37°C",
    ),
    (
        "XhoI",
        "CTCGAG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Xanthomonas holcicola",
        "37°C",
    ),
    (
        "XbaI",
        "TCTAGA",
        1,
        5,
        EnzymeType::TypeIIP,
        "Xanthomonas badrii",
        "37°C",
    ),
    (
        "SalI",
        "GTCGAC",
        1,
        5,
        EnzymeType::TypeIIP,
        "Streptomyces albus",
        "37°C",
    ),
    (
        "PstI",
        "CTGCAG",
        5,
        1,
        EnzymeType::TypeIIP,
        "Providencia stuartii",
        "37°C",
    ),
    (
        "KpnI",
        "GGTACC",
        5,
        1,
        EnzymeType::TypeIIP,
        "Klebsiella pneumoniae",
        "37°C",
    ),
    (
        "SacI",
        "GAGCTC",
        5,
        1,
        EnzymeType::TypeIIP,
        "Streptomyces achromogenes",
        "37°C",
    ),
    (
        "SacII",
        "CCGCGG",
        4,
        2,
        EnzymeType::TypeIIP,
        "Streptomyces achromogenes",
        "37°C",
    ),
    (
        "SmaI",
        "CCCGGG",
        3,
        3,
        EnzymeType::TypeIIP,
        "Serratia marcescens",
        "25°C",
    ),
    (
        "XmaI",
        "CCCGGG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Xanthomonas malvacearum",
        "37°C",
    ),
    (
        "NcoI",
        "CCATGG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Nocardia corallina",
        "37°C",
    ),
    (
        "NdeI",
        "CATATG",
        2,
        4,
        EnzymeType::TypeIIP,
        "Neisseria denitrificans",
        "37°C",
    ),
    (
        "NheI",
        "GCTAGC",
        1,
        5,
        EnzymeType::TypeIIP,
        "Neisseria mucosa",
        "37°C",
    ),
    (
        "SpeI",
        "ACTAGT",
        1,
        5,
        EnzymeType::TypeIIP,
        "Sphaerotilus natans",
        "37°C",
    ),
    (
        "BglII",
        "AGATCT",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus globigii",
        "37°C",
    ),
    (
        "ClaI",
        "ATCGAT",
        2,
        4,
        EnzymeType::TypeIIP,
        "Caryophanon latum",
        "37°C",
    ),
    (
        "EcoRV",
        "GATATC",
        3,
        3,
        EnzymeType::TypeIIP,
        "E. coli",
        "37°C",
    ),
    (
        "SphI",
        "GCATGC",
        5,
        1,
        EnzymeType::TypeIIP,
        "Streptomyces phaeochromogenes",
        "37°C",
    ),
    (
        "ApaI",
        "GGGCCC",
        5,
        1,
        EnzymeType::TypeIIP,
        "Acetobacter pasteurianus",
        "25°C",
    ),
    (
        "PvuI",
        "CGATCG",
        4,
        2,
        EnzymeType::TypeIIP,
        "Proteus vulgaris",
        "37°C",
    ),
    (
        "PvuII",
        "CAGCTG",
        3,
        3,
        EnzymeType::TypeIIP,
        "Proteus vulgaris",
        "37°C",
    ),
    (
        "HpaI",
        "GTTAAC",
        3,
        3,
        EnzymeType::TypeIIP,
        "Haemophilus parainfluenzae",
        "37°C",
    ),
    (
        "StuI",
        "AGGCCT",
        3,
        3,
        EnzymeType::TypeIIP,
        "Streptomyces tubercidicus",
        "37°C",
    ),
    (
        "ScaI",
        "AGTACT",
        3,
        3,
        EnzymeType::TypeIIP,
        "Streptomyces caespitosus",
        "37°C",
    ),
    (
        "NruI",
        "TCGCGA",
        3,
        3,
        EnzymeType::TypeIIP,
        "Nocardia rubra",
        "37°C",
    ),
    (
        "SnaBI",
        "TACGTA",
        3,
        3,
        EnzymeType::TypeIIP,
        "Sphaerotilus natans",
        "37°C",
    ),
    (
        "DraI",
        "TTTAAA",
        3,
        3,
        EnzymeType::TypeIIP,
        "Deinococcus radiodurans",
        "37°C",
    ),
    (
        "PmeI",
        "GTTTAAAC",
        4,
        4,
        EnzymeType::TypeIIP,
        "Pseudomonas mendocina",
        "37°C",
    ),
    (
        "SwaI",
        "ATTTAAAT",
        4,
        4,
        EnzymeType::TypeIIP,
        "Staphylococcus warneri",
        "25°C",
    ),
    (
        "AscI",
        "GGCGCGCC",
        2,
        6,
        EnzymeType::TypeIIP,
        "Arthrobacter species",
        "37°C",
    ),
    (
        "PacI",
        "TTAATTAA",
        5,
        3,
        EnzymeType::TypeIIP,
        "Pseudomonas alcaligenes",
        "37°C",
    ),
    (
        "FseI",
        "GGCCGGCC",
        6,
        2,
        EnzymeType::TypeIIP,
        "Frankia species",
        "37°C",
    ),
    (
        "SbfI",
        "CCTGCAGG",
        6,
        2,
        EnzymeType::TypeIIP,
        "Streptomyces species",
        "37°C",
    ),
    (
        "AseI",
        "ATTAAT",
        2,
        4,
        EnzymeType::TypeIIP,
        "Aquaspirillum serpens",
        "37°C",
    ),
    (
        "NsiI",
        "ATGCAT",
        5,
        1,
        EnzymeType::TypeIIP,
        "Neisseria sicca",
        "37°C",
    ),
    (
        "MluI",
        "ACGCGT",
        1,
        5,
        EnzymeType::TypeIIP,
        "Micrococcus luteus",
        "37°C",
    ),
    (
        "BssHII",
        "GCGCGC",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus stearothermophilus",
        "50°C",
    ),
    (
        "AgeI",
        "ACCGGT",
        1,
        5,
        EnzymeType::TypeIIP,
        "Agrobacterium gelatinovorum",
        "37°C",
    ),
    (
        "BsrGI",
        "TGTACA",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus stearothermophilus",
        "37°C",
    ),
    (
        "BspEI",
        "TCCGGA",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus species",
        "37°C",
    ),
    (
        "BclI",
        "TGATCA",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus caldolyticus",
        "50°C",
    ),
    (
        "AatII",
        "GACGTC",
        5,
        1,
        EnzymeType::TypeIIP,
        "Acetobacter aceti",
        "37°C",
    ),
    (
        "EagI",
        "CGGCCG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Enterobacter agglomerans",
        "37°C",
    ),
    (
        "MfeI",
        "CAATTG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Mycoplasma fermentans",
        "37°C",
    ),
    (
        "BsiWI",
        "CGTACG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Bacillus stearothermophilus",
        "55°C",
    ),
    (
        "AluI",
        "AGCT",
        2,
        2,
        EnzymeType::TypeIIP,
        "Arthrobacter luteus",
        "37°C",
    ),
    (
        "HaeIII",
        "GGCC",
        2,
        2,
        EnzymeType::TypeIIP,
        "Haemophilus aegyptius",
        "37°C",
    ),
    (
        "MspI",
        "CCGG",
        1,
        3,
        EnzymeType::TypeIIP,
        "Moraxella species",
        "37°C",
    ),
    (
        "HpaII",
        "CCGG",
        1,
        3,
        EnzymeType::TypeIIP,
        "Haemophilus parainfluenzae",
        "37°C",
    ),
    (
        "TaqI",
        "TCGA",
        1,
        3,
        EnzymeType::TypeIIP,
        "Thermus aquaticus",
        "65°C",
    ),
    (
        "MboI",
        "GATC",
        0,
        4,
        EnzymeType::TypeIIP,
        "Moraxella bovis",
        "37°C",
    ),
    (
        "Sau3AI",
        "GATC",
        0,
        4,
        EnzymeType::TypeIIP,
        "Staphylococcus aureus",
        "37°C",
    ),
    (
        "DpnII",
        "GATC",
        0,
        4,
        EnzymeType::TypeIIP,
        "Diplococcus pneumoniae",
        "37°C",
    ),
    (
        "RsaI",
        "GTAC",
        2,
        2,
        EnzymeType::TypeIIP,
        "Rhodopseudomonas sphaeroides",
        "37°C",
    ),
    (
        "MseI",
        "TTAA",
        1,
        3,
        EnzymeType::TypeIIP,
        "Micrococcus species",
        "37°C",
    ),
    (
        "NlaIII",
        "CATG",
        4,
        0,
        EnzymeType::TypeIIP,
        "Neisseria lactamica",
        "37°C",
    ),
    (
        "HhaI",
        "GCGC",
        3,
        1,
        EnzymeType::TypeIIP,
        "Haemophilus haemolyticus",
        "37°C",
    ),
    (
        "CfoI",
        "GCGC",
        3,
        1,
        EnzymeType::TypeIIP,
        "Clostridium formicoaceticum",
        "37°C",
    ),
    (
        "AvaI",
        "CYCGRG",
        1,
        5,
        EnzymeType::TypeIIP,
        "Anabaena variabilis",
        "37°C",
    ),
    (
        "HincII",
        "GTYRAC",
        3,
        3,
        EnzymeType::TypeIIP,
        "Haemophilus influenzae",
        "37°C",
    ),
    (
        "HinfI",
        "GANTC",
        1,
        4,
        EnzymeType::TypeIIP,
        "Haemophilus influenzae",
        "37°C",
    ),
    (
        "BstNI",
        "CCWGG",
        2,
        3,
        EnzymeType::TypeIIP,
        "Bacillus stearothermophilus",
        "60°C",
    ),
    (
        "Sau96I",
        "GGNCC",
        1,
        4,
        EnzymeType::TypeIIP,
        "Staphylococcus aureus",
        "37°C",
    ),
    (
        "AlwNI",
        "CAGNNNCTG",
        6,
        3,
        EnzymeType::TypeIIP,
        "Acinetobacter lwoffii",
        "37°C",
    ),
    (
        "SfiI",
        "GGCCNNNNNGGCC",
        8,
        5,
        EnzymeType::TypeIIP,
        "Streptomyces fimbriatus",
        "50°C",
    ),
    (
        "BsaI",
        "GGTCTC",
        7,
        11,
        EnzymeType::TypeIIS,
        "Bacillus stearothermophilus",
        "37°C",
    ),
    (
        "BsmBI",
        "CGTCTC",
        7,
        11,
        EnzymeType::TypeIIS,
        "Bacillus stearothermophilus",
        "55°C",
    ),
    (
        "Esp3I",
        "CGTCTC",
        7,
        11,
        EnzymeType::TypeIIS,
        "Erwinia species",
        "37°C",
    ),
    (
        "BbsI",
        "GAAGAC",
        8,
        12,
        EnzymeType::TypeIIS,
        "Bacillus laterosporus",
        "37°C",
    ),
    (
        "BpiI",
        "GAAGAC",
        8,
        12,
        EnzymeType::TypeIIS,
        "Bacillus species",
        "37°C",
    ),
    (
        "SapI",
        "GCTCTTC",
        8,
        11,
        EnzymeType::TypeIIS,
        "Saccharopolyspora species",
        "37°C",
    ),
    (
        "BspQI",
        "GCTCTTC",
        8,
        11,
        EnzymeType::TypeIIS,
        "Bacillus species",
        "50°C",
    ),
    (
        "FokI",
        "GGATG",
        14,
        18,
        EnzymeType::TypeIIS,
        "Flavobacterium okeanokoites",
        "37°C",
    ),
    (
        "BsmAI",
        "GTCTC",
        6,
        10,
        EnzymeType::TypeIIS,
        "Bacillus stearothermophilus",
        "55°C",
    ),
    (
        "HgaI",
        "GACGC",
        10,
        15,
        EnzymeType::TypeIIS,
        "Haemophilus gallinarum",
        "37°C",
    ),
    (
        "AarI",
        "CACCTGC",
        11,
        15,
        EnzymeType::TypeIIS,
        "Arthrobacter aurescens",
        "37°C",
    ),
];

impl EnzymeDb {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Standarddatenbank mit 79 gängigen Enzymen.
    pub fn standard() -> Self {
        let enzymes = STANDARD
            .iter()
            .filter_map(|(n, s, t, b, ty, org, temp)| {
                RestrictionEnzyme::new(n, s, *t, *b, *ty, org, "NEB", temp, "siehe Hersteller").ok()
            })
            .collect();
        Self { enzymes }
    }

    pub fn add(&mut self, enzyme: RestrictionEnzyme) {
        self.enzymes.push(enzyme);
    }

    pub fn all(&self) -> &[RestrictionEnzyme] {
        &self.enzymes
    }

    pub fn len(&self) -> usize {
        self.enzymes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.enzymes.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&RestrictionEnzyme> {
        self.enzymes.iter().find(|e| e.name == name)
    }

    /// Enzyme, deren Name den Suchtext enthält (Groß-/Kleinschreibung egal).
    pub fn search_by_name(&self, text: &str) -> Vec<&RestrictionEnzyme> {
        let t = text.to_lowercase();
        self.enzymes
            .iter()
            .filter(|e| e.name.to_lowercase().contains(&t))
            .collect()
    }

    pub fn search_by_recognition(&self, site: &str) -> Vec<&RestrictionEnzyme> {
        let s = site.to_ascii_uppercase();
        self.enzymes.iter().filter(|e| e.recognition == s).collect()
    }

    pub fn find_by_cut_position(&self, cut_top: usize) -> Vec<&RestrictionEnzyme> {
        self.enzymes
            .iter()
            .filter(|e| e.cut_top == cut_top)
            .collect()
    }

    pub fn find_by_type(&self, t: EnzymeType) -> Vec<&RestrictionEnzyme> {
        self.enzymes.iter().filter(|e| e.enzyme_type == t).collect()
    }

    pub fn blunt(&self) -> Vec<&RestrictionEnzyme> {
        self.enzymes.iter().filter(|e| e.is_blunt()).collect()
    }

    pub fn sticky(&self) -> Vec<&RestrictionEnzyme> {
        self.enzymes.iter().filter(|e| e.is_sticky()).collect()
    }

    /// Isoschizomere: andere Enzyme mit derselben Erkennungssequenz.
    pub fn isoschizomers(&self, enzyme: &RestrictionEnzyme) -> Vec<&RestrictionEnzyme> {
        self.enzymes
            .iter()
            .filter(|e| e.recognition == enzyme.recognition && e.name != enzyme.name)
            .collect()
    }

    /// Andere Enzyme mit ligierbaren Enden.
    pub fn compatible_with(&self, enzyme: &RestrictionEnzyme) -> Vec<&RestrictionEnzyme> {
        self.enzymes
            .iter()
            .filter(|e| e.name != enzyme.name && e.is_compatible_with(enzyme))
            .collect()
    }

    pub fn methylation_sensitive(&self) -> Vec<&RestrictionEnzyme> {
        self.enzymes
            .iter()
            .filter(|e| e.is_methylation_sensitive())
            .collect()
    }

    /// Alle Schnittstellen aller Enzyme in einer Sequenz, nach Position und Name sortiert.
    /// Zirkuläre Sequenzen werden über den Nullpunkt hinweg durchsucht; die Positionen liegen dann in `0..len`.
    pub fn find_all_cut_sites(
        &self,
        bases: &str,
        circular: bool,
        sequence_id: &str,
    ) -> Vec<CutSite> {
        let upper = bases.to_ascii_uppercase().replace('U', "T");
        let len = upper.len();
        let max_site = self
            .enzymes
            .iter()
            .map(|e| e.recognition.len())
            .max()
            .unwrap_or(0);
        let work: String = if circular && len > 0 && max_site > 1 {
            let extra = (max_site - 1).min(len);
            // Bei sehr kurzen Sequenzen ggf. mehrfach umlaufen
            let mut s = upper.clone();
            while s.len() < len + max_site - 1 {
                s.push_str(&upper[..extra.min(upper.len())]);
            }
            s.truncate(len + max_site - 1);
            s
        } else {
            upper.clone()
        };
        let bytes = work.as_bytes();
        let mut sites: Vec<CutSite> = self
            .enzymes
            .par_iter()
            .flat_map_iter(|enzyme| {
                enzyme
                    .find_matches(bytes)
                    .into_iter()
                    .filter(|m| !circular || m.start < len)
                    .map(|m| {
                        let wrap = |v: i64| {
                            if circular && len > 0 {
                                v.rem_euclid(len as i64)
                            } else {
                                v
                            }
                        };
                        let (ct, cb) = (wrap(m.cut_top), wrap(m.cut_bottom));
                        let overhang = overhang_text(&upper, m.cut_top, m.cut_bottom, circular);
                        CutSite {
                            enzyme: enzyme.name.clone(),
                            sequence_id: sequence_id.to_string(),
                            position: m.start,
                            cut_top: ct,
                            cut_bottom: cb,
                            forward: m.forward,
                            overhang,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        sites.sort_by(|a, b| {
            a.position
                .cmp(&b.position)
                .then_with(|| a.enzyme.cmp(&b.enzyme))
                .then_with(|| b.forward.cmp(&a.forward))
        });
        sites.dedup();
        sites
    }

    /// Schnittstellen in einem Sequenzdokument (berücksichtigt die Topologie).
    pub fn find_in_document(&self, doc: &SequenceDocument) -> Vec<CutSite> {
        self.find_all_cut_sites(
            doc.bases(),
            doc.topology() == Topology::Circular,
            doc.sequence().id(),
        )
    }

    /// Anzahl der Schnittstellen je Enzym (nur Enzyme mit mindestens einer).
    pub fn site_counts(&self, bases: &str, circular: bool) -> Vec<(String, usize)> {
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for s in self.find_all_cut_sites(bases, circular, "") {
            *counts.entry(s.enzyme).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }
}

/// Überhang in Oberstrang-Schreibweise aus den Schnittpositionen (leer bei stumpf/außerhalb).
fn overhang_text(upper: &str, cut_top: i64, cut_bottom: i64, circular: bool) -> String {
    if cut_top == cut_bottom {
        return String::new();
    }
    let len = upper.len() as i64;
    let (from, to) = (cut_top.min(cut_bottom), cut_top.max(cut_bottom));
    if circular && len > 0 {
        return (from..to)
            .map(|p| upper.as_bytes()[p.rem_euclid(len) as usize] as char)
            .collect();
    }
    if from < 0 || to > len {
        return String::new();
    }
    upper[from as usize..to as usize].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::genbank;

    fn db() -> EnzymeDb {
        EnzymeDb::standard()
    }

    fn site<'a>(sites: &'a [CutSite], name: &str) -> Option<&'a CutSite> {
        sites.iter().find(|s| s.enzyme == name)
    }

    #[test]
    fn datenbank_hat_79_enzyme_und_mehrere_typen() {
        let d = db();
        assert_eq!(d.len(), 79);
        assert!(d.all().len() > 50);
        assert!(!d.find_by_type(EnzymeType::TypeIIS).is_empty());
        assert!(!d.find_by_type(EnzymeType::TypeIIP).is_empty());
    }

    #[test]
    fn suchen() {
        let d = db();
        assert!(d.search_by_name("eco").iter().any(|e| e.name() == "EcoRI"));
        assert!(d
            .search_by_recognition("gaattc")
            .iter()
            .any(|e| e.name() == "EcoRI"));
        assert!(d.get("EcoRI").is_some() && d.get("gibt-es-nicht").is_none());
        assert!(!d.find_by_cut_position(1).is_empty());
    }

    #[test]
    fn stumpf_und_klebrig() {
        let d = db();
        assert!(!d.blunt().is_empty() && d.blunt().iter().all(|e| e.is_blunt()));
        assert!(!d.sticky().is_empty() && d.sticky().iter().all(|e| e.is_sticky()));
        assert!(d.get("SmaI").unwrap().is_blunt());
        assert!(d.get("PmeI").unwrap().is_blunt());
        assert!(d.get("EcoRV").unwrap().is_blunt());
        assert!(d.get("EcoRI").unwrap().is_sticky());
    }

    #[test]
    fn ecori_eigenschaften() {
        let e = db().get("EcoRI").unwrap().clone();
        assert_eq!(e.recognition(), "GAATTC");
        assert_eq!((e.cut_top(), e.cut_bottom()), (1, 5));
        assert_eq!(e.overhang_length(), 4);
        assert_eq!(e.overhang_sequence().as_deref(), Some("AATT"));
        assert!(e.is_palindromic());
        assert_eq!(e.organism(), "E. coli");
    }

    #[test]
    fn pstl_hat_3_strich_ueberhang() {
        let e = db().get("PstI").unwrap().clone();
        assert_eq!(e.overhang_length(), -4);
        assert_eq!(e.overhang_sequence().as_deref(), Some("TGCA"));
    }

    #[test]
    fn schnittstelle_ecori() {
        let sites = db().find_all_cut_sites("AAGGAATTCGATTAGCTAGC", false, "t");
        let s = site(&sites, "EcoRI").unwrap();
        assert_eq!(s.position, 3);
        assert_eq!((s.cut_top, s.cut_bottom), (4, 8));
        assert_eq!(s.overhang(), Some("AATT"));
        assert!(!s.is_blunt());
        assert_eq!(s.to_string(), "CutSite[EcoRI@3 in t]");
    }

    #[test]
    fn mehrere_ecori_stellen() {
        let sites = db().find_all_cut_sites(&"GAATTC".repeat(5), false, "m");
        assert_eq!(sites.iter().filter(|s| s.enzyme == "EcoRI").count(), 5);
    }

    #[test]
    fn stumpfe_schnittstelle() {
        let sites = db().find_all_cut_sites("AAACCCGGGTTT", false, "x");
        let s = site(&sites, "SmaI").unwrap();
        assert!(s.is_blunt());
        assert_eq!(s.overhang(), None);
        assert_eq!(s.cut_top, 6);
    }

    #[test]
    fn leere_und_kurze_sequenz() {
        assert!(db().find_all_cut_sites("", false, "e").is_empty());
        assert!(db().find_all_cut_sites("ATCG", false, "k").is_empty());
        assert!(db().find_all_cut_sites("", true, "e").is_empty());
    }

    #[test]
    fn schneiden_in_fragmente() {
        let e = db().get("EcoRI").unwrap().clone();
        assert_eq!(
            e.cut("AAGGAATTCGATTAGCTAGC"),
            vec!["AAGG", "AATTCGATTAGCTAGC"]
        );
        assert_eq!(e.cut("AAAA"), vec!["AAAA"]);
        let smai = db().get("SmaI").unwrap().clone();
        assert_eq!(smai.cut("AAACCCGGGTTT"), vec!["AAACCC", "GGGTTT"]);
    }

    #[test]
    fn schnitt_am_beginn_der_erkennung_terminiert() {
        let mbo = db().get("MboI").unwrap().clone();
        assert_eq!(mbo.cut("AAGATCTTGATCAA"), vec!["AA", "GATCTT", "GATCAA"]);
        assert_eq!(mbo.cut("GATC"), vec!["GATC"]);
    }

    #[test]
    fn nicht_palindromisch_beide_straenge() {
        let bsa = db().get("BsaI").unwrap().clone();
        assert!(!bsa.is_palindromic());
        let fwd = bsa.find_matches(b"AAAGGTCTCAAAAAAAAAA");
        assert_eq!(fwd.len(), 1);
        assert!(fwd[0].forward);
        assert_eq!((fwd[0].cut_top, fwd[0].cut_bottom), (3 + 7, 3 + 11));
        let rev = bsa.find_matches(b"AAAGAGACCAAAAAAAAAA");
        assert_eq!(rev.len(), 1);
        assert!(!rev[0].forward);
        assert_eq!((rev[0].cut_top, rev[0].cut_bottom), (3 + 6 - 11, 3 + 6 - 7));
    }

    #[test]
    fn mehrdeutige_erkennungssequenz() {
        let ava = db().get("AvaI").unwrap().clone(); // CYCGRG
        assert_eq!(ava.find_matches(b"AACCCGGGAA").len(), 1);
        assert_eq!(ava.find_matches(b"AACACGGGAA").len(), 0);
        let hinf = db().get("HinfI").unwrap().clone(); // GANTC
        assert_eq!(hinf.find_matches(b"GAATC").len(), 1);
        assert_eq!(hinf.find_matches(b"GACTC").len(), 1);
        assert_eq!(hinf.find_matches(b"GANTC").len(), 1); // N passt auf N
        assert_eq!(hinf.find_matches(b"GAATG").len(), 0);
        // N in der Sequenz passt nicht auf eindeutige Muster
        assert!(db()
            .get("EcoRI")
            .unwrap()
            .find_matches(b"GANTTC")
            .is_empty());
    }

    #[test]
    fn zirkulaer_ueber_den_nullpunkt() {
        let circ = db().find_all_cut_sites("TTCAAAAAAGAA", true, "c");
        let s = site(&circ, "EcoRI").unwrap();
        assert_eq!(s.position, 9);
        assert_eq!(s.cut_top, 10);
        assert_eq!(s.cut_bottom, 2); // (9 + 5) mod 12
        assert_eq!(s.overhang(), Some("AATT"));
        let lin = db().find_all_cut_sites("TTCAAAAAAGAA", false, "c");
        assert!(site(&lin, "EcoRI").is_none());
    }

    #[test]
    fn kompatible_enden() {
        let d = db();
        let bam = d.get("BamHI").unwrap();
        let bgl = d.get("BglII").unwrap();
        let eco = d.get("EcoRI").unwrap();
        assert!(bam.is_compatible_with(bgl));
        assert!(!bam.is_compatible_with(eco));
        assert!(d.compatible_with(bam).iter().any(|e| e.name() == "BglII"));
        // stumpfe Enden sind untereinander kompatibel, aber nicht mit klebrigen
        assert!(d
            .get("SmaI")
            .unwrap()
            .is_compatible_with(d.get("EcoRV").unwrap()));
        assert!(!d.get("SmaI").unwrap().is_compatible_with(eco));
        // 5'- und 3'-Überhang passen nicht zusammen
        assert!(!d.get("PstI").unwrap().is_compatible_with(eco));
    }

    #[test]
    fn isoschizomere() {
        let d = db();
        let sma = d.get("SmaI").unwrap();
        assert!(d.isoschizomers(sma).iter().any(|e| e.name() == "XmaI"));
        let all_cccggg = d.search_by_recognition("CCCGGG");
        assert!(all_cccggg.len() >= 2);
    }

    #[test]
    fn methylierung_und_temperaturen() {
        let d = db();
        assert!(d.get("NotI").unwrap().is_methylation_sensitive());
        assert!(!d.get("EcoRI").unwrap().is_methylation_sensitive());
        assert!(d.methylation_sensitive().iter().any(|e| e.name() == "ClaI"));
        assert_eq!(d.get("TaqI").unwrap().temperature(), "65°C");
        assert!(d.all().iter().any(|e| e.temperature() != "37°C"));
    }

    #[test]
    fn eigenes_enzym_hinzufuegen() {
        let mut d = db();
        let custom = RestrictionEnzyme::symmetric(
            "CustomI",
            "AATTCC",
            2,
            EnzymeType::TypeII,
            true,
            "Test",
            "Test",
            "37°C",
            "None",
        )
        .unwrap();
        assert_eq!(custom.cut_bottom(), 4);
        d.add(custom);
        assert_eq!(d.search_by_name("CustomI").len(), 1);
        assert!(RestrictionEnzyme::new("X", "", 0, 0, EnzymeType::TypeII, "", "", "", "").is_err());
        assert!(
            RestrictionEnzyme::new("X", "GAXTC", 1, 4, EnzymeType::TypeII, "", "", "", "").is_err()
        );
        let blunt = RestrictionEnzyme::symmetric(
            "B",
            "GG^CC",
            2,
            EnzymeType::TypeII,
            false,
            "",
            "",
            "37°C",
            "",
        )
        .unwrap();
        assert!(blunt.is_blunt());
    }

    #[test]
    fn grosse_sequenz_ist_schnell() {
        let seq = "ATCG".repeat(250_000); // 1 Mb
        let t = std::time::Instant::now();
        let sites = db().find_all_cut_sites(&seq, false, "g");
        assert!(t.elapsed().as_secs_f64() < 2.0, "{:?}", t.elapsed());
        assert!(sites.iter().any(|s| s.enzyme == "ClaI")); // ATCGAT
    }

    #[test]
    fn puc19_einzelschneider_im_mcs() {
        let doc = genbank::read(std::path::Path::new("testdata/puc19.gb"))
            .unwrap()
            .remove(0);
        let sites = db().find_in_document(&doc);
        for (name, pos) in [
            ("EcoRI", 395),
            ("BamHI", 416),
            ("HindIII", 446),
            ("SacI", 401),
            ("KpnI", 407),
            ("PstI", 434),
            ("XbaI", 422),
            ("SalI", 428),
        ] {
            let found: Vec<&CutSite> = sites.iter().filter(|s| s.enzyme == name).collect();
            assert_eq!(found.len(), 1, "{}", name);
            assert_eq!(found[0].position, pos, "{}", name);
        }
        let counts = db().site_counts(doc.bases(), true);
        assert!(counts.iter().any(|(n, c)| n == "EcoRI" && *c == 1));
    }

    #[test]
    fn sortierung_ist_stabil_und_ohne_duplikate() {
        let sites = db().find_all_cut_sites("GGATCCGAATTC", false, "s");
        let pos: Vec<usize> = sites.iter().map(|s| s.position).collect();
        let mut sorted = pos.clone();
        sorted.sort();
        assert_eq!(pos, sorted);
        let mut uniq = sites.clone();
        uniq.dedup();
        assert_eq!(uniq.len(), sites.len());
    }
}
