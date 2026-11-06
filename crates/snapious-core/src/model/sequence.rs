//! Biologische Sequenz (DNA, RNA, Protein) mit Grundoperationen.
//!
//! Port von `com.snapious.core.Sequence` (Java-Referenz) mit den dort behobenen Fehlern:
//! vollständige Codon-Tabelle, korrekte ORF-Suche, Typerkennung beim FASTA-Import.

use std::collections::BTreeMap;

use crate::error::{Error, Result};

/// Art der Sequenz.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SequenceType {
    Dna,
    Rna,
    Protein,
}

impl SequenceType {
    /// Erlaubte Zeichen (Großbuchstaben). Nukleotide: A, C, G, T/U und IUPAC-Mehrdeutigkeitscodes.
    pub fn allowed(&self) -> &'static [u8] {
        match self {
            SequenceType::Dna => b"ACGTRYKMSWBDHVN",
            SequenceType::Rna => b"ACGURYKMSWBDHVN",
            SequenceType::Protein => b"ACDEFGHIKLMNPQRSTVWYX*",
        }
    }

    /// Ist die Sequenz ein Nukleotid (DNA oder RNA)?
    pub fn is_nucleotide(&self) -> bool {
        !matches!(self, SequenceType::Protein)
    }
}

/// Eine biologische Sequenz. Die Basen werden immer als ASCII-Großbuchstaben gehalten.
#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    id: String,
    name: String,
    description: String,
    seq_type: SequenceType,
    bases: String,
}

/// Aminosäure (ein Buchstabe) für ein Codon im DNA-Alphabet (T statt U). Stopp = `*`, Unbekanntes = `X`.
pub fn amino_acid_for(codon: &[u8]) -> u8 {
    match codon {
        b"TTT" | b"TTC" => b'F',
        b"TTA" | b"TTG" | b"CTT" | b"CTC" | b"CTA" | b"CTG" => b'L',
        b"ATT" | b"ATC" | b"ATA" => b'I',
        b"ATG" => b'M',
        b"GTT" | b"GTC" | b"GTA" | b"GTG" => b'V',
        b"TCT" | b"TCC" | b"TCA" | b"TCG" | b"AGT" | b"AGC" => b'S',
        b"CCT" | b"CCC" | b"CCA" | b"CCG" => b'P',
        b"ACT" | b"ACC" | b"ACA" | b"ACG" => b'T',
        b"GCT" | b"GCC" | b"GCA" | b"GCG" => b'A',
        b"TAT" | b"TAC" => b'Y',
        b"CAT" | b"CAC" => b'H',
        b"CAA" | b"CAG" => b'Q',
        b"AAT" | b"AAC" => b'N',
        b"AAA" | b"AAG" => b'K',
        b"GAT" | b"GAC" => b'D',
        b"GAA" | b"GAG" => b'E',
        b"TGT" | b"TGC" => b'C',
        b"TGG" => b'W',
        b"CGT" | b"CGC" | b"CGA" | b"CGG" | b"AGA" | b"AGG" => b'R',
        b"GGT" | b"GGC" | b"GGA" | b"GGG" => b'G',
        b"TAA" | b"TAG" | b"TGA" => b'*',
        _ => b'X',
    }
}

/// Komplement einer einzelnen Base (IUPAC); `rna` bestimmt, ob A zu U wird.
fn complement_base(b: u8, rna: bool) -> u8 {
    match b {
        b'A' => {
            if rna {
                b'U'
            } else {
                b'T'
            }
        }
        b'T' | b'U' => b'A',
        b'G' => b'C',
        b'C' => b'G',
        b'R' => b'Y',
        b'Y' => b'R',
        b'K' => b'M',
        b'M' => b'K',
        b'B' => b'V',
        b'V' => b'B',
        b'D' => b'H',
        b'H' => b'D',
        b'S' => b'S',
        b'W' => b'W',
        _ => b'N',
    }
}

impl Sequence {
    /// Neue Sequenz. Leerzeichen und (bei Nukleotiden) Ziffern werden entfernt, Buchstaben groß geschrieben.
    pub fn new(id: &str, name: &str, bases: &str, seq_type: SequenceType) -> Result<Self> {
        let cleaned = Self::validate(bases, seq_type)?;
        Ok(Self {
            id: id.to_string(),
            name: if name.is_empty() {
                "Unnamed".to_string()
            } else {
                name.to_string()
            },
            description: String::new(),
            seq_type,
            bases: cleaned,
        })
    }

    fn validate(bases: &str, seq_type: SequenceType) -> Result<String> {
        let allowed = seq_type.allowed();
        let mut out = String::with_capacity(bases.len());
        for c in bases.chars() {
            if c.is_whitespace() || (seq_type.is_nucleotide() && c.is_ascii_digit()) {
                continue;
            }
            let u = c.to_ascii_uppercase();
            if !u.is_ascii() || !allowed.contains(&(u as u8)) {
                return Err(Error::InvalidInput(format!(
                    "Ungültiges Zeichen '{}' für {:?}",
                    c, seq_type
                )));
            }
            out.push(u);
        }
        Ok(out)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn seq_type(&self) -> SequenceType {
        self.seq_type
    }

    /// Basen als Text (ASCII-Großbuchstaben).
    pub fn bases(&self) -> &str {
        &self.bases
    }

    /// Basen als Bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.bases.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.bases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn set_id(&mut self, id: &str) {
        self.id = id.to_string();
    }

    pub fn set_description(&mut self, description: &str) {
        self.description = description.to_string();
    }

    /// Ersetzt die Basen (mit Prüfung). Bei Fehler bleibt die Sequenz unverändert.
    pub fn set_bases(&mut self, bases: &str) -> Result<()> {
        self.bases = Self::validate(bases, self.seq_type)?;
        Ok(())
    }

    /// Kopie mit neuen Basen (Id, Name, Beschreibung und Typ bleiben). Prüft die Basen.
    pub fn with_bases(&self, bases: &str) -> Result<Sequence> {
        let mut copy = self.clone();
        copy.set_bases(bases)?;
        Ok(copy)
    }

    /// Basis an Position (0-basiert).
    pub fn base_at(&self, index: usize) -> Result<char> {
        self.bases
            .as_bytes()
            .get(index)
            .map(|b| *b as char)
            .ok_or_else(|| {
                Error::OutOfRange(format!("Index {} bei Länge {}", index, self.bases.len()))
            })
    }

    /// Teilsequenz `[start, end)`. Außerhalb → `OutOfRange`, `start > end` → `InvalidInput`.
    pub fn subsequence(&self, start: usize, end: usize) -> Result<Sequence> {
        if end > self.bases.len() {
            return Err(Error::OutOfRange(format!(
                "Bereich {}-{} außerhalb 0-{}",
                start,
                end,
                self.bases.len()
            )));
        }
        if start > end {
            return Err(Error::InvalidInput(format!(
                "Ungültiger Bereich: {}-{}",
                start, end
            )));
        }
        Ok(Sequence {
            id: self.id.clone(),
            name: format!("{}[sub:{}-{}]", self.name, start, end),
            description: String::new(),
            seq_type: self.seq_type,
            bases: self.bases[start..end].to_string(),
        })
    }

    fn require_nucleotide(&self) -> Result<()> {
        if self.seq_type.is_nucleotide() {
            Ok(())
        } else {
            Err(Error::InvalidState(
                "Operation nur für DNA/RNA verfügbar".to_string(),
            ))
        }
    }

    fn derived(&self, suffix: &str, bases: String, seq_type: SequenceType) -> Sequence {
        Sequence {
            id: self.id.clone(),
            name: format!("{} {}", self.name, suffix),
            description: String::new(),
            seq_type,
            bases,
        }
    }

    /// Komplementäre Sequenz (gleiche Leserichtung).
    pub fn complement(&self) -> Result<Sequence> {
        self.require_nucleotide()?;
        let rna = self.seq_type == SequenceType::Rna;
        let bases: String = self
            .as_bytes()
            .iter()
            .map(|b| complement_base(*b, rna) as char)
            .collect();
        Ok(self.derived("(komplementär)", bases, self.seq_type))
    }

    /// Reverse-Komplement.
    pub fn reverse_complement(&self) -> Result<Sequence> {
        self.require_nucleotide()?;
        let rna = self.seq_type == SequenceType::Rna;
        let bases: String = self
            .as_bytes()
            .iter()
            .rev()
            .map(|b| complement_base(*b, rna) as char)
            .collect();
        Ok(self.derived("(rev-kom)", bases, self.seq_type))
    }

    /// Transkription DNA → RNA (T wird zu U).
    pub fn transcribe(&self) -> Result<Sequence> {
        if self.seq_type != SequenceType::Dna {
            return Err(Error::InvalidState("Nur DNA transkribierbar".to_string()));
        }
        Ok(self.derived(
            "(transkribiert)",
            self.bases.replace('T', "U"),
            SequenceType::Rna,
        ))
    }

    /// Rücktranskription RNA → DNA (U wird zu T).
    pub fn to_dna(&self) -> Result<Sequence> {
        if self.seq_type != SequenceType::Rna {
            return Err(Error::InvalidState(
                "Nur RNA rücktranskribierbar".to_string(),
            ));
        }
        Ok(self.derived("(cDNA)", self.bases.replace('U', "T"), SequenceType::Dna))
    }

    /// Translation im ersten Leserahmen. Stopcodons als `*`, unvollständiges Schlusscodon wird ignoriert.
    pub fn translate(&self) -> Result<Sequence> {
        self.require_nucleotide()?;
        let dna = self.bases.replace('U', "T");
        let (codons, _rest) = dna.as_bytes().as_chunks::<3>();
        let protein: String = codons.iter().map(|c| amino_acid_for(c) as char).collect();
        Ok(self.derived("(translatiert)", protein, SequenceType::Protein))
    }

    /// Offene Leserahmen (ATG bis Stopcodon oder Sequenzende) in den drei Vorwärts-Rahmen.
    /// Verschachtelte Startcodons innerhalb eines ORFs werden nicht separat gemeldet.
    /// `min_codons` = Mindestlänge in Codons (ohne Stopcodon).
    pub fn find_orfs(&self, min_codons: usize) -> Result<Vec<Sequence>> {
        self.require_nucleotide()?;
        let dna = self.bases.replace('U', "T");
        let b = dna.as_bytes();
        let mut orfs = Vec::new();
        for frame in 0..3usize {
            let mut i = frame;
            while i + 2 < b.len() {
                if &b[i..i + 3] != b"ATG" {
                    i += 3;
                    continue;
                }
                let mut j = i;
                while j + 2 < b.len() && amino_acid_for(&b[j..j + 3]) != b'*' {
                    j += 3;
                }
                let codons = (j - i) / 3;
                if codons >= min_codons {
                    let mut orf = dna[i..j].to_string();
                    if self.seq_type == SequenceType::Rna {
                        orf = orf.replace('T', "U");
                    }
                    orfs.push(Sequence {
                        id: format!("{}_ORF_frame{}_{}", self.id, frame + 1, i),
                        name: format!("{} ORF frame{}:{}", self.name, frame + 1, i),
                        description: String::new(),
                        seq_type: self.seq_type,
                        bases: orf,
                    });
                }
                i = j + 3;
            }
        }
        Ok(orfs)
    }

    /// Alle Startpositionen eines exakten Motivs (Groß-/Kleinschreibung egal), auch überlappend.
    pub fn find_motif(&self, motif: &str) -> Vec<usize> {
        let m = motif.to_ascii_uppercase();
        if m.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(pos) = self.bases[from..].find(&m) {
            out.push(from + pos);
            from += pos + 1;
            if from >= self.bases.len() {
                break;
            }
        }
        out
    }

    /// GC-Anteil (0.0 bis 1.0); 0.0 bei leerer Sequenz.
    pub fn gc_content(&self) -> f64 {
        if self.bases.is_empty() {
            return 0.0;
        }
        let gc = self
            .as_bytes()
            .iter()
            .filter(|b| matches!(**b, b'G' | b'C'))
            .count();
        gc as f64 / self.bases.len() as f64
    }

    /// AT-/AU-Anteil (0.0 bis 1.0).
    pub fn at_content(&self) -> f64 {
        if self.bases.is_empty() {
            return 0.0;
        }
        let at = self
            .as_bytes()
            .iter()
            .filter(|b| matches!(**b, b'A' | b'T' | b'U'))
            .count();
        at as f64 / self.bases.len() as f64
    }

    /// Relative Häufigkeit der Zeichen A, C, G, T, U, N.
    pub fn base_frequencies(&self) -> BTreeMap<char, f64> {
        let mut out = BTreeMap::new();
        for c in "ATCGUN".chars() {
            let count = self.as_bytes().iter().filter(|b| **b as char == c).count();
            let f = if self.bases.is_empty() {
                0.0
            } else {
                count as f64 / self.bases.len() as f64
            };
            out.insert(c, f);
        }
        out
    }

    /// FASTA-Text: Kopfzeile `>id name [beschreibung]`, danach 60 Zeichen pro Zeile.
    pub fn to_fasta(&self) -> String {
        // Kopfzeile: Name, falls er schon mit der ID beginnt (FASTA-Import), sonst "ID Name"
        let header = if self.name == self.id || self.name.starts_with(&format!("{} ", self.id)) {
            self.name.clone()
        } else {
            format!("{} {}", self.id, self.name)
        };
        let mut s = format!(">{}", header);
        if !self.description.is_empty() {
            s.push(' ');
            s.push_str(&self.description);
        }
        s.push('\n');
        let b = self.bases.as_bytes();
        for (i, chunk) in b.chunks(60).enumerate() {
            if i > 0 {
                s.push('\n');
            }
            s.push_str(&String::from_utf8_lossy(chunk));
        }
        s
    }

    /// Sequenz aus FASTA-Zeilen. Typ: nur ACGTN → DNA, nur ACGUN → RNA, sonst Protein.
    pub fn from_fasta(id: &str, header: &str, sequence_lines: &str) -> Result<Sequence> {
        let bases: String = sequence_lines
            .lines()
            .map(|l| l.trim())
            .collect::<String>()
            .to_ascii_uppercase();
        let seq_type = if bases.bytes().all(|b| b"ACGTN".contains(&b)) {
            SequenceType::Dna
        } else if bases.bytes().all(|b| b"ACGUN".contains(&b)) {
            SequenceType::Rna
        } else {
            SequenceType::Protein
        };
        let name = header.strip_prefix('>').unwrap_or(header);
        Sequence::new(id, name, &bases, seq_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dna(s: &str) -> Sequence {
        Sequence::new("id", "Test", s, SequenceType::Dna).unwrap()
    }

    #[test]
    fn komplement_dna_und_rna() {
        assert_eq!(dna("ATCG").complement().unwrap().bases(), "TAGC");
        let rna = Sequence::new("r", "R", "AUCG", SequenceType::Rna).unwrap();
        assert_eq!(rna.complement().unwrap().bases(), "UAGC");
    }

    #[test]
    fn reverse_komplement() {
        assert_eq!(dna("ATCG").reverse_complement().unwrap().bases(), "CGAT");
        assert_eq!(
            dna("AACCGGTT").reverse_complement().unwrap().bases(),
            "AACCGGTT"
        );
    }

    #[test]
    fn komplement_von_protein_ist_fehler() {
        let p = Sequence::new("p", "P", "MDEK", SequenceType::Protein).unwrap();
        assert!(matches!(p.complement(), Err(Error::InvalidState(_))));
    }

    #[test]
    fn transkription_und_rueckweg() {
        let rna = dna("ATGCGT").transcribe().unwrap();
        assert_eq!(rna.bases(), "AUGCGU");
        assert_eq!(rna.seq_type(), SequenceType::Rna);
        assert_eq!(rna.to_dna().unwrap().bases(), "ATGCGT");
    }

    #[test]
    fn translation_dna_und_rna() {
        assert_eq!(dna("ATGTTTTAA").translate().unwrap().bases(), "MF*");
        let rna = Sequence::new("r", "R", "AUGUUUUAA", SequenceType::Rna).unwrap();
        assert_eq!(rna.translate().unwrap().bases(), "MF*");
    }

    #[test]
    fn translation_ignoriert_unvollstaendiges_codon() {
        assert_eq!(dna("ATGTT").translate().unwrap().bases(), "M");
    }

    #[test]
    fn codon_tabelle_stichproben() {
        assert_eq!(amino_acid_for(b"AGC"), b'S');
        assert_eq!(amino_acid_for(b"AGT"), b'S');
        assert_eq!(amino_acid_for(b"TGG"), b'W');
        assert_eq!(amino_acid_for(b"ACG"), b'T');
        assert_eq!(amino_acid_for(b"NNN"), b'X');
    }

    #[test]
    fn gc_gehalt_ist_anteil() {
        assert!((dna("ATCG").gc_content() - 0.5).abs() < 1e-9);
        assert!((dna("GCGC").gc_content() - 1.0).abs() < 1e-9);
        assert_eq!(dna("").gc_content(), 0.0);
    }

    #[test]
    fn basenhaeufigkeiten() {
        let f = dna("AATTCCGG").base_frequencies();
        for c in ['A', 'T', 'C', 'G'] {
            assert!((f[&c] - 0.25).abs() < 1e-9);
        }
    }

    #[test]
    fn teilsequenz_und_grenzen() {
        let s = dna("ATCGTAGC");
        assert_eq!(s.subsequence(2, 5).unwrap().bases(), "CGT");
        assert!(matches!(s.subsequence(2, 100), Err(Error::OutOfRange(_))));
        assert!(matches!(s.subsequence(5, 2), Err(Error::InvalidInput(_))));
    }

    #[test]
    fn ungueltige_base_wird_abgelehnt() {
        assert!(Sequence::new("a", "b", "ATXG", SequenceType::Dna).is_err());
        assert!(Sequence::new("a", "b", "AUG", SequenceType::Dna).is_err());
    }

    #[test]
    fn leerzeichen_ziffern_und_kleinbuchstaben() {
        let s = dna("atg c 12 gt");
        assert_eq!(s.bases(), "ATGCGT");
    }

    #[test]
    fn iupac_codes_sind_erlaubt() {
        let s = dna("GAATTCRY");
        assert_eq!(s.len(), 8);
        assert_eq!(s.reverse_complement().unwrap().bases(), "RYGAATTC");
    }

    #[test]
    fn protein_alphabet() {
        let p = Sequence::new("p", "P", "MDEK*", SequenceType::Protein).unwrap();
        assert_eq!(p.len(), 5);
    }

    #[test]
    fn orf_suche() {
        // ATG CCG TAG im Rahmen 3
        let s = dna("AAATGCCGTAGCTAA");
        let orfs = s.find_orfs(2).unwrap();
        assert_eq!(orfs.len(), 1);
        assert_eq!(orfs[0].bases(), "ATGCCG");
        assert!(s.find_orfs(3).unwrap().is_empty());
    }

    #[test]
    fn orf_ohne_stopcodon_laeuft_bis_zum_ende() {
        let s = dna("ATGAAACCC");
        let orfs = s.find_orfs(1).unwrap();
        assert_eq!(orfs[0].bases(), "ATGAAACCC");
    }

    #[test]
    fn motiv_suche_auch_ueberlappend() {
        assert_eq!(dna("AAAA").find_motif("AA"), vec![0, 1, 2]);
        assert_eq!(dna("GAATTCGAATTC").find_motif("gaattc"), vec![0, 6]);
        assert!(dna("ACGT").find_motif("").is_empty());
    }

    #[test]
    fn fasta_typ_erkennung() {
        assert_eq!(
            Sequence::from_fasta("a", ">x", "ACGGCT\nGGA")
                .unwrap()
                .seq_type(),
            SequenceType::Dna
        );
        assert_eq!(
            Sequence::from_fasta("a", ">x", "ACGUCU")
                .unwrap()
                .seq_type(),
            SequenceType::Rna
        );
        assert_eq!(
            Sequence::from_fasta("a", ">x", "MKVLAAGIV")
                .unwrap()
                .seq_type(),
            SequenceType::Protein
        );
        assert_eq!(
            Sequence::from_fasta("a", ">x y", "ACGT").unwrap().name(),
            "x y"
        );
    }

    #[test]
    fn fasta_kopfzeile_dupliziert_die_id_nicht() {
        let s = Sequence::from_fasta("a", ">a Beschreibung", "ACGT").unwrap();
        assert_eq!(s.to_fasta().lines().next().unwrap(), ">a Beschreibung");
        let t = Sequence::new("a", "a", "ACGT", SequenceType::Dna).unwrap();
        assert_eq!(t.to_fasta().lines().next().unwrap(), ">a");
    }

    #[test]
    fn fasta_ausgabe_bricht_bei_60_um() {
        let s = dna(&"A".repeat(70));
        let f = s.to_fasta();
        let lines: Vec<&str> = f.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ">id Test");
        assert_eq!(lines[1].len(), 60);
        assert_eq!(lines[2].len(), 10);
    }

    #[test]
    fn set_bases_prueft_und_laesst_bei_fehler_unveraendert() {
        let mut s = dna("ACGT");
        assert!(s.set_bases("ACXT").is_err());
        assert_eq!(s.bases(), "ACGT");
        s.set_bases("ggcc").unwrap();
        assert_eq!(s.bases(), "GGCC");
    }

    #[test]
    fn base_at_prueft_index() {
        let s = dna("ACGT");
        assert_eq!(s.base_at(2).unwrap(), 'G');
        assert!(s.base_at(4).is_err());
    }
}
