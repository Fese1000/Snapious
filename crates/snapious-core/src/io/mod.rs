//! Dateiformate: FASTA, FASTQ, GenBank, SAM, VCF.

pub mod abif;
pub mod ace;
pub mod alignment_formats;
pub mod bed;
pub mod blast_table;
pub mod cml;
pub mod csfasta;
pub mod embl;
pub mod fasta;
pub mod fastq;
pub mod gcg;
pub mod genbank;
pub mod gff3;
pub mod gff3_write;
pub mod hin;
pub mod hmmer;
pub mod interpro;
pub mod mega;
pub mod mol;
pub mod nexus;
pub mod paste;
pub mod pdb;
pub mod pir;
pub mod qual;
pub mod quality;
pub mod sam;
pub mod scf;
pub mod sdf;
pub mod sff;
pub mod snapgene;
pub mod table_import;
pub mod vcf;
pub mod xyz;

use std::path::Path;

use crate::error::{Error, Result};
use crate::model::alignment::AlignmentDocument;
use crate::model::document::{Document, SequenceDocument};
use crate::model::sequence::{Sequence, SequenceType};

/// Erkennbare Dateiformate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Fasta,
    Fastq,
    GenBank,
    Embl,
    Sam,
    Vcf,
    Newick,
}

/// Erkennt das Format am Inhalt (erste nicht leere Zeile). `None`, wenn nichts passt.
pub fn detect_format(text: &str) -> Option<Format> {
    let first = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    if first.starts_with(">") {
        Some(Format::Fasta)
    } else if first.starts_with("LOCUS") {
        Some(Format::GenBank)
    } else if first.starts_with("ID   ") {
        Some(Format::Embl)
    } else if first.starts_with("##fileformat=VCF") || first.starts_with("#CHROM") {
        Some(Format::Vcf)
    } else if first.starts_with("@HD")
        || first.starts_with("@SQ")
        || first.starts_with("@PG")
        || first.starts_with("@CO")
    {
        Some(Format::Sam)
    } else if first.starts_with('@') {
        // FASTQ-Kopfzeilen haben keine Tabulatoren
        if first.contains('\t') && first.split('\t').count() >= 11 {
            Some(Format::Sam)
        } else {
            Some(Format::Fastq)
        }
    } else if first.starts_with('(') {
        Some(Format::Newick)
    } else {
        None
    }
}

/// Liest Sequenzdokumente aus einer FASTA-, FASTQ-, GenBank-, EMBL- oder SnapGene-Datei (Format wird am Inhalt erkannt).
pub fn read_sequence_documents(path: &Path) -> Result<Vec<SequenceDocument>> {
    let bytes = std::fs::read(path)?;
    if snapgene::is_snapgene(&bytes) {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "SnapGene".to_string());
        return Ok(vec![snapgene::parse_snapgene(&bytes, &name)?]);
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| Error::Io("Die Datei ist kein Text und keine SnapGene-Datei".to_string()))?;
    match detect_format(&text) {
        Some(Format::GenBank) => genbank::parse(&text),
        Some(Format::Embl) => embl::parse(&text),
        Some(Format::Fasta) => Ok(fasta::parse(&text)?
            .into_iter()
            .map(|s: Sequence| {
                let name = s.name().to_string();
                SequenceDocument::of(&name, s)
            })
            .collect()),
        Some(Format::Fastq) => {
            let mut docs = Vec::new();
            for r in fastq::parse(&text)? {
                let s = r.to_sequence()?;
                docs.push(SequenceDocument::of(&r.id, s));
            }
            Ok(docs)
        }
        other => Err(Error::Io(format!(
            "Das Format {:?} enthält keine Sequenzdokumente",
            other
        ))),
    }
}

/// Weitere Formate, die nur beim Import erkannt werden (Alignments und Sequenzen).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportFormat {
    Mega,
    Nexus,
    Clustal,
    Phylip,
    Pir,
    Gcg,
    Csfasta,
    Ace,
}

/// `true`, wenn die Zeile wie eine SOLiD-Farbfolge aussieht: erste Base A, C, G oder T, danach mindestens eine Farbe `0`–`3` oder `.`.
fn is_color_line(line: &str) -> bool {
    let mut chars = line.trim().chars();
    matches!(
        chars.next().map(|c| c.to_ascii_uppercase()),
        Some('A' | 'C' | 'G' | 'T')
    ) && line.trim().len() >= 2
        && chars.all(|c| matches!(c, '0'..='3' | '.'))
}

/// Erkennt MEGA (`#mega`), NEXUS (`#NEXUS`), Clustal (`CLUSTAL`), PHYLIP (erste Zeile zwei ganze Zahlen), PIR (`>XX;`) und GCG
/// (innerhalb der ersten 30 Zeilen eine Zeile, die mit `..` endet), SOLiD csfasta (`>Name`, danach eine Farbfolge wie `T0123`, vorher
/// dürfen `#`-Kommentare stehen) und ACE (erste Zeile `AS <Zahl> <Zahl>`) am Inhalt. `None`, wenn nichts passt.
pub fn detect_import_format(text: &str) -> Option<ImportFormat> {
    let mut content = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    if let (Some(h), Some(next)) = (content.next(), content.next()) {
        if h.starts_with('>') && is_color_line(next) {
            return Some(ImportFormat::Csfasta);
        }
    }
    let first = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    let up = first.to_uppercase();
    {
        let t: Vec<&str> = first.split_whitespace().collect();
        if t.len() == 3
            && t[0] == "AS"
            && t[1].parse::<usize>().is_ok()
            && t[2].parse::<usize>().is_ok()
        {
            return Some(ImportFormat::Ace);
        }
    }
    if up.starts_with("#MEGA") {
        return Some(ImportFormat::Mega);
    }
    if up.starts_with("#NEXUS") {
        return Some(ImportFormat::Nexus);
    }
    if up.starts_with("CLUSTAL") {
        return Some(ImportFormat::Clustal);
    }
    let nums: Vec<&str> = first.split_whitespace().collect();
    if nums.len() == 2 && nums.iter().all(|n| n.parse::<usize>().is_ok()) {
        return Some(ImportFormat::Phylip);
    }
    if first.starts_with('>') {
        let head = first.trim_start_matches('>');
        if head.get(2..3) == Some(";") && head.is_char_boundary(2) {
            return Some(ImportFormat::Pir);
        }
        return None;
    }
    if text
        .lines()
        .take(30)
        .any(|l| l.trim().ends_with("..") && l.trim().len() > 2)
        && text.trim_start().starts_with("!!")
    {
        return Some(ImportFormat::Gcg);
    }
    None
}

fn alignment_from_rows(name: &str, rows: Vec<(String, String)>) -> Result<Document> {
    let joined: String = rows
        .iter()
        .map(|r| r.1.as_str())
        .collect::<Vec<_>>()
        .join("");
    let seq_type = match paste::detect_kind(&joined) {
        paste::Kind::Dna => SequenceType::Dna,
        paste::Kind::Rna => SequenceType::Rna,
        paste::Kind::Protein => SequenceType::Protein,
    };
    let (names, seqs): (Vec<String>, Vec<String>) = rows.into_iter().unzip();
    Ok(Document::Alignment(AlignmentDocument::of(
        name, seq_type, names, seqs,
    )?))
}

fn sequence_of(name: &str, residues: &str, protein: bool) -> Result<Document> {
    let t = if protein {
        SequenceType::Protein
    } else {
        match paste::detect_kind(residues) {
            paste::Kind::Rna => SequenceType::Rna,
            paste::Kind::Protein => SequenceType::Protein,
            paste::Kind::Dna => SequenceType::Dna,
        }
    };
    let seq = Sequence::new(&crate::util::new_id(), name, residues, t)?;
    Ok(Document::Sequence(SequenceDocument::of(name, seq)))
}

/// Basen ohne Zeichen, die für DNA nicht erlaubt sind (diese werden `N`).
fn dna_clean(text: &str) -> String {
    let allowed = SequenceType::Dna.allowed();
    text.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            let u = c.to_ascii_uppercase();
            if u.is_ascii() && allowed.contains(&(u as u8)) {
                u
            } else {
                'N'
            }
        })
        .collect()
}

fn dna_document(name: &str, bases: &str) -> Result<Document> {
    let seq = Sequence::new(&crate::util::new_id(), name, bases, SequenceType::Dna)?;
    Ok(Document::Sequence(SequenceDocument::of(name, seq)))
}

/// Position in einem Konsens mit Lücken (`*`) → Position ohne Lücken.
fn unpadded(consensus: &str, padded: usize) -> usize {
    consensus.chars().take(padded).filter(|c| *c != '*').count()
}

/// ACE-Contigs als Sequenzen (Konsens ohne `*`) mit je einem Feature `read` je Read (Strang nach `AF`).
fn ace_documents(text: &str) -> Result<Vec<Document>> {
    use crate::model::feature::{Feature, Location, Strand};
    let mut out = Vec::new();
    for c in ace::parse_ace(text)? {
        let bases: String = c.consensus.chars().filter(|ch| *ch != '*').collect();
        let bases = dna_clean(&bases);
        let seq = Sequence::new(&crate::util::new_id(), &c.name, &bases, SequenceType::Dna)?;
        let mut features = Vec::new();
        for r in &c.reads {
            let from = (r.offset - 1).max(0) as usize;
            let to = from
                .saturating_add(r.sequence.chars().count())
                .min(c.consensus.chars().count());
            let (s, e) = (unpadded(&c.consensus, from), unpadded(&c.consensus, to));
            if e > s {
                let strand = if r.complement {
                    Strand::Reverse
                } else {
                    Strand::Forward
                };
                features.push(Feature::new(
                    "read",
                    &r.name,
                    Location::of(s, e, strand)?,
                    Vec::new(),
                ));
            }
        }
        out.push(Document::Sequence(
            SequenceDocument::of(&c.name, seq).with_features(features),
        ));
    }
    Ok(out)
}

fn scf_documents(name: &str, bytes: &[u8]) -> Result<Vec<Document>> {
    let r = scf::parse_scf(bytes)?;
    let (a, b) = quality_trim_range(r.bases.len(), &r.qualities, 10, 20);
    let part = r.bases.get(a..b).unwrap_or("");
    if part.is_empty() {
        return Err(Error::InvalidInput(
            "Die Sequenzierung enthält keine Basen guter Qualität".to_string(),
        ));
    }
    Ok(vec![dna_document(name, &dna_clean(part))?])
}

/// SFF-Reads als Sequenzen, auf den Qualitäts-Clip gekürzt (1-basiert; 0 = kein Clip).
fn sff_documents(bytes: &[u8]) -> Result<Vec<Document>> {
    let f = sff::parse_sff(bytes)?;
    let mut out = Vec::new();
    for r in f.reads {
        let len = r.bases.len();
        let start = (r.clip_left as usize).saturating_sub(1).min(len);
        let end = if r.clip_right == 0 {
            len
        } else {
            (r.clip_right as usize).min(len)
        };
        let part = if end > start {
            r.bases.get(start..end).unwrap_or("")
        } else {
            ""
        };
        if part.is_empty() {
            continue;
        }
        out.push(dna_document(&r.name, &dna_clean(part))?);
    }
    if out.is_empty() {
        return Err(Error::InvalidInput(
            "Die SFF-Datei enthält keine Reads".to_string(),
        ));
    }
    Ok(out)
}

fn csfasta_documents(text: &str) -> Result<Vec<Document>> {
    csfasta::parse_csfasta(text)?
        .iter()
        .map(|(name, colors)| dna_document(name, &csfasta::decode_color_space(colors)?))
        .collect()
}

/// Schneidet schlechte Enden einer Sequenzierung ab: Der Beginn ist die erste Base mit Qualität ≥ `min_quality`, deren Fenster der Breite
/// `window` eine mittlere Qualität ≥ `min_quality` hat; das Ende die letzte solche Base am Ende eines guten Fensters. Gibt `(Beginn, Ende)` zurück
/// (Ende exklusiv). Ist `quality` leer oder kürzer als `bases`, bleibt alles; gibt es kein gutes Fenster, ist das Ergebnis `(0, 0)`.
pub fn quality_trim_range(
    bases_len: usize,
    quality: &[u8],
    window: usize,
    min_quality: u8,
) -> (usize, usize) {
    if quality.is_empty() || quality.len() < bases_len || window == 0 || bases_len < window {
        return (0, bases_len);
    }
    let good = |i: usize| {
        let sum: usize = quality[i..i + window].iter().map(|q| *q as usize).sum();
        sum >= min_quality as usize * window
    };
    // Beginn: erste Base mit guter Qualität, deren Fenster gut ist; Ende: letzte solche Base am Fensterende
    let start = (0..=bases_len - window).find(|&i| quality[i] >= min_quality && good(i));
    let end = (window..=bases_len)
        .rev()
        .find(|&j| quality[j - 1] >= min_quality && good(j - window));
    match (start, end) {
        (Some(a), Some(b)) if b > a => (a, b),
        _ => (0, 0),
    }
}

/// Sanger-Sequenzierung (`.ab1`) als Sequenz: Basen ohne schlechte Enden (Fenster 10, mittlere Qualität ≥ 20), Name = Dateiname.
fn abif_documents(name: &str, bytes: &[u8]) -> Result<Vec<Document>> {
    let c = abif::parse_abif(bytes)?;
    let (a, b) = quality_trim_range(c.bases.len(), &c.quality, 10, 20);
    let part = c.bases.get(a..b).unwrap_or("");
    if part.is_empty() {
        return Err(Error::InvalidInput(
            "Die Sequenzierung enthält keine Basen guter Qualität".to_string(),
        ));
    }
    Ok(vec![dna_document(name, &dna_clean(part))?])
}

/// Liest eine Datei als Dokumente: Alignments (MEGA, NEXUS, Clustal, PHYLIP), PIR-, GCG-, csfasta-, SCF- und SFF-Sequenzen,
/// ACE-Contigs (mit `read`-Features) und sonst wie
/// `read_sequence_documents` (FASTA, FASTQ, GenBank, EMBL, SnapGene). Der Name von Alignments ist der Dateiname ohne Endung.
pub fn read_documents(path: &Path) -> Result<Vec<Document>> {
    let bytes = std::fs::read(path)?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Alignment".to_string());
    if bytes.starts_with(b"ABIF") {
        return abif_documents(&stem, &bytes);
    }
    if bytes.starts_with(b".scf") {
        return scf_documents(&stem, &bytes);
    }
    if bytes.starts_with(b".sff") {
        return sff_documents(&bytes);
    }
    if !snapgene::is_snapgene(&bytes) {
        if let Ok(text) = String::from_utf8(bytes) {
            match detect_import_format(&text) {
                Some(ImportFormat::Csfasta) => return csfasta_documents(&text),
                Some(ImportFormat::Ace) => return ace_documents(&text),
                Some(ImportFormat::Mega) => {
                    return Ok(vec![alignment_from_rows(&stem, mega::parse_mega(&text)?)?])
                }
                Some(ImportFormat::Nexus) => {
                    return Ok(vec![alignment_from_rows(
                        &stem,
                        nexus::parse_nexus(&text)?,
                    )?])
                }
                Some(ImportFormat::Clustal) => {
                    return Ok(vec![alignment_from_rows(
                        &stem,
                        alignment_formats::parse_clustal(&text)?,
                    )?])
                }
                Some(ImportFormat::Phylip) => {
                    return Ok(vec![alignment_from_rows(
                        &stem,
                        alignment_formats::parse_phylip(&text)?,
                    )?])
                }
                Some(ImportFormat::Pir) => {
                    return pir::parse_pir(&text)?
                        .iter()
                        .map(|r| {
                            let name = if r.description.is_empty() {
                                r.id.clone()
                            } else {
                                format!("{} {}", r.id, r.description)
                            };
                            sequence_of(&name, &r.sequence, r.kind.starts_with('P'))
                        })
                        .collect();
                }
                Some(ImportFormat::Gcg) => {
                    let (name, seq) = gcg::parse_gcg(&text)?;
                    return Ok(vec![sequence_of(&name, &seq, false)?]);
                }
                None => {}
            }
        }
    }
    Ok(read_sequence_documents(path)?
        .into_iter()
        .map(Document::Sequence)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formate_werden_erkannt() {
        assert_eq!(detect_format(">a\nACGT\n"), Some(Format::Fasta));
        assert_eq!(
            detect_format("\n\nLOCUS       x 10 bp\n"),
            Some(Format::GenBank)
        );
        assert_eq!(detect_format("@r1\nACGT\n+\nIIII\n"), Some(Format::Fastq));
        assert_eq!(
            detect_format("ID   x; SV 1; linear; genomic DNA; STD; SYN; 4 BP.\n"),
            Some(Format::Embl)
        );
        assert_eq!(detect_format("@SQ\tSN:a\tLN:1\n"), Some(Format::Sam));
        assert_eq!(detect_format("@HD\tVN:1.6\n"), Some(Format::Sam));
        assert_eq!(detect_format("##fileformat=VCFv4.2\n"), Some(Format::Vcf));
        assert_eq!(detect_format("((A,B),C);"), Some(Format::Newick));
        assert_eq!(detect_format("irgendwas"), None);
        assert_eq!(detect_format("   \n\n"), None);
    }

    #[test]
    fn weitere_formate_und_dokumente() {
        assert_eq!(
            detect_import_format("#mega\n#a\nAC\n"),
            Some(ImportFormat::Mega)
        );
        assert_eq!(detect_import_format("#NEXUS\n"), Some(ImportFormat::Nexus));
        assert_eq!(
            detect_import_format("CLUSTAL W\n"),
            Some(ImportFormat::Clustal)
        );
        assert_eq!(
            detect_import_format(" 2 4\nA ACGT\n"),
            Some(ImportFormat::Phylip)
        );
        assert_eq!(
            detect_import_format(">P1;x\nd\nMK*\n"),
            Some(ImportFormat::Pir)
        );
        assert_eq!(detect_import_format(">normal\nACGT\n"), None);
        assert_eq!(
            detect_import_format("!!NA_SEQUENCE 1.0\n\n x ..\n 1 ACGT\n"),
            Some(ImportFormat::Gcg)
        );
        assert_eq!(detect_import_format("LOCUS x\n"), None);

        let dir = tempfile::tempdir().unwrap();
        let m = dir.path().join("a.meg");
        std::fs::write(&m, "#mega\n!Title t;\n#a\nACGT\n#b\nAC-T\n").unwrap();
        let docs = read_documents(&m).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].doc_type(), "alignment");
        let p = dir.path().join("p.pir");
        std::fs::write(&p, ">P1;S1\nein Protein\nMKVL*\n>DL;S2\n\nACGT\n").unwrap();
        let docs = read_documents(&p).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].name(), "S1 ein Protein");
        let g = dir.path().join("g.gcg");
        std::fs::write(
            &g,
            "!!NA_SEQUENCE 1.0\n\n  seq1  Length: 8  Type: N  Check: 1  ..\n\n        1 ACGTACGT\n",
        )
        .unwrap();
        assert_eq!(read_documents(&g).unwrap()[0].name(), "seq1");
        let f = dir.path().join("f.fa");
        std::fs::write(&f, ">x\nACGT\n").unwrap();
        assert_eq!(read_documents(&f).unwrap()[0].doc_type(), "sequence");
    }

    #[test]
    fn csfasta_ace_scf_sff_import() {
        assert_eq!(
            detect_import_format("# Titel\n>r1\nT0123\n"),
            Some(ImportFormat::Csfasta)
        );
        assert_eq!(detect_import_format(">r1\nACGT\n"), None);
        assert_eq!(detect_import_format("AS 1 2\n"), Some(ImportFormat::Ace));
        let dir = tempfile::tempdir().unwrap();
        let c = dir.path().join("a.csfasta");
        std::fs::write(&c, "# x\n>r1\nT0123\n").unwrap();
        let d = read_documents(&c).unwrap();
        assert_eq!(d.len(), 1);
        if let Document::Sequence(s) = &d[0] {
            assert_eq!(s.bases(), "TTGAT");
        } else {
            panic!("keine Sequenz");
        }
        // ACE: Konsens mit Lücke, zwei Reads
        let a = dir.path().join("a.ace");
        std::fs::write(
            &a,
            "AS 1 2\n\nCO contig1 8 2 0 U\nACGTAC*G\n\nBQ\n 30 30 30 30 30 30 30\n\nAF r1 U 1\nAF r2 C 3\n\nRD r1 5 0 0\nACGTA\n\nRD r2 6 0 0\nGTAC*G\n\n",
        )
        .unwrap();
        let d = read_documents(&a).unwrap();
        if let Document::Sequence(s) = &d[0] {
            assert_eq!(s.bases(), "ACGTACG");
            let f = s.features();
            assert_eq!(f.len(), 2);
            assert_eq!((f[0].location().start(), f[0].location().end()), (0, 5));
            assert_eq!((f[1].location().start(), f[1].location().end()), (2, 7));
            assert_eq!(
                f[1].location().strand(),
                crate::model::feature::Strand::Reverse
            );
        } else {
            panic!("keine Sequenz");
        }
        // SCF und SFF: aus den Testdaten der Leser nachgebaut
        let mut scf = Vec::new();
        scf.extend_from_slice(b".scf");
        for v in [0u32, 128, 3, 0, 0, 128, 0, 164] {
            scf.extend_from_slice(&v.to_be_bytes());
        }
        scf.extend_from_slice(b"3.00");
        scf.resize(128, 0);
        for p in [10u32, 20, 30] {
            scf.extend_from_slice(&p.to_be_bytes());
        }
        scf.extend_from_slice(&[40, 2, 1, 1, 30, 2, 1, 2, 20, 1, 1, 1]);
        scf.extend_from_slice(b"ACG");
        scf.extend_from_slice(&[0; 9]);
        let sp = dir.path().join("t.scf");
        std::fs::write(&sp, &scf).unwrap();
        assert_eq!(read_documents(&sp).unwrap()[0].name(), "t");
    }

    #[test]
    fn qualitaetsschnitt() {
        // schlechtes Ende vorn (5 Basen) und hinten (5 Basen), 20 gute dazwischen
        let mut q = vec![5u8; 5];
        q.extend(vec![40u8; 20]);
        q.extend(vec![5u8; 5]);
        assert_eq!(quality_trim_range(30, &q, 10, 20), (5, 25));
        assert_eq!(quality_trim_range(30, &[], 10, 20), (0, 30));
        assert_eq!(quality_trim_range(30, &[3u8; 30], 10, 20), (0, 0));
        assert_eq!(quality_trim_range(5, &[40; 5], 10, 20), (0, 5));
    }

    #[test]
    fn sequenzdokumente_aus_dateien() {
        let dir = tempfile::tempdir().unwrap();
        let fa = dir.path().join("a.fa");
        std::fs::write(&fa, ">s1\nACGT\n>s2\nGGGG\n").unwrap();
        let docs = read_sequence_documents(&fa).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[1].bases(), "GGGG");

        let fq = dir.path().join("a.fq");
        std::fs::write(&fq, "@r1\nACGT\n+\nIIII\n").unwrap();
        assert_eq!(read_sequence_documents(&fq).unwrap()[0].name(), "r1");

        let gb = dir.path().join("a.gb");
        std::fs::write(&gb, include_str!("../../testdata/mini.gb")).unwrap();
        assert_eq!(read_sequence_documents(&gb).unwrap()[0].len(), 60);

        let vcf = dir.path().join("a.vcf");
        std::fs::write(&vcf, "##fileformat=VCFv4.2\n").unwrap();
        assert!(read_sequence_documents(&vcf).is_err());
        assert!(read_sequence_documents(&dir.path().join("fehlt")).is_err());
    }
}
