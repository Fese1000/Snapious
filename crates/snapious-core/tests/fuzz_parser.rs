//! Random test: the file readers may report errors on arbitrary or corrupted content, but must never crash.

use std::panic::catch_unwind;

use snapious_core::export::table::{parse_delimited, to_delimited};
use snapious_core::io::{
    abif, ace, alignment_formats, bed, cml, csfasta, embl, fasta, fastq, gcg, genbank, gff3, mega,
    mol, nexus, paste, pdb, pir, sam, scf, sff, snapgene, vcf, xyz,
};
use snapious_core::ngs::bam::parse_bam;
use snapious_core::ngs::fai::{build_fai, parse_fai};
use snapious_core::ngs::fastq_stream::FastqReader;
use snapious_core::util::Rng;

/// Sample texts that are corrupted one after another.
fn seeds() -> Vec<String> {
    let gb = std::fs::read_to_string("testdata/mini.gb").unwrap();
    let puc = std::fs::read_to_string("testdata/puc19.gb").unwrap();
    vec![
        gb,
        puc[..puc.len().min(4000)].to_string(),
        ">a desc\nACGTACGT\n>b\nTTGGCC\n".to_string(),
        "@r1 x\nACGT\n+\nIIII\n@r2\nAC\n+\nII\n".to_string(),
        "ID   x; SV 1; circular; genomic DNA; STD; SYN; 8 BP.\nFT   gene            1..4\nFT                   /gene=\"a\"\nSQ   Sequence 8 BP;\n     acgtacgt        8\n//\n".to_string(),
        "#NEXUS\nBEGIN DATA;\nMATRIX\nA ACGT\nB AC-T\n;\nEND;\n".to_string(),
        "CLUSTAL W\n\nA ACGT\nB AC-T\n     * **\n".to_string(),
        " 2 4\nA ACGT\nB AC-T\n".to_string(),
        "chr1\t5\t20\tn\t0\t+\n".to_string(),
        "chr1\tsrc\tgene\t5\t20\t.\t+\t.\tID=g;Name=abc\n".to_string(),
        "@HD\tVN:1.6\n@SQ\tSN:r\tLN:100\nq\t0\tr\t5\t60\t3S4M2D1M\t*\t0\t0\tACGTACGT\tIIIIIIII\n".to_string(),
        "##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nr\t5\t.\tA\tG\t50\tPASS\tDP=10\n".to_string(),
        "ATOM      1 N    GLY A   1      10.000  11.000  12.000  1.00 20.00           N\nEND\n".to_string(),
        ">a\nACGTAC\nGT\n>b\nAC\n".to_string(),
        "#mega\n!Title T;\n#a\nACGT\n#b\nA-GT\n".to_string(),
        ">P1;S1\nbeschreibung\nMKV LAA*\n>DL;S2\n\nACGT\n".to_string(),
        "!!NA_SEQUENCE 1.0\n\n  seq1  Length: 8  Type: N  Check: 1  ..\n\n        1 ACGTACGT\n".to_string(),
        "3\nwater\nO 0.0 0.0 0.0\nH 0.757 0.586 0.0\nH -0.757 0.586 0.0\n".to_string(),
        "water\n  prog\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0\n    0.7570    0.5860    0.0000 H   0  0\n   -0.7570    0.5860    0.0000 H   0  0\n  1  2  1  0\n  1  3  1  0\nM  END\n".to_string(),
        "a,\"b,c\"\n\"x\"\"y\",z\n".to_string(),
        ">r1\nT0123\n>r2\nA01.3\n".to_string(),
        "AS 1 2\n\nCO c1 8 2 0 U\nACGTAC*G\n\nBQ\n 30 30\n\nAF r1 U 1\nAF r2 C -2\n\nRD r1 5 0 0\nACGTA\n\nRD r2 6 0 0\nGT*CGT\n\n".to_string(),
        "<molecule><atomArray><atom id=\"a1\" elementType=\"O\" x3=\"0\" y3=\"0\" z3=\"0\"/><atom id=\"a2\" elementType=\"H\" x3=\"1\" y3=\"0\" z3=\"0\"/></atomArray><bondArray><bond atomRefs2=\"a1 a2\" order=\"1\"/></bondArray></molecule>".to_string(),
    ]
}

fn mutate(s: &str, rng: &mut Rng) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let n = 1 + rng.below(6);
    for _ in 0..n {
        if chars.is_empty() {
            chars.push('x');
            continue;
        }
        let i = rng.below(chars.len());
        match rng.below(5) {
            0 => {
                chars.remove(i);
            }
            1 => chars.insert(
                i,
                [
                    '\n', '\t', ' ', '(', ')', '"', '>', '@', '+', '-', '0', '9', 'Ä', '€', '\0',
                ][rng.below(15)],
            ),
            2 => chars.truncate(i),
            3 => {
                let c = chars[i];
                chars.insert(i, c);
            }
            _ => chars[i] = ['A', 'C', 'G', 'T', 'N', '-', '*', '/', ':', ';', ','][rng.below(11)],
        }
    }
    chars.into_iter().collect()
}

fn run_all(text: &str) -> Vec<&'static str> {
    let mut panics = Vec::new();
    macro_rules! chk {
        ($name:expr, $e:expr) => {
            if catch_unwind(|| {
                let _ = $e;
            })
            .is_err()
            {
                panics.push($name);
            }
        };
    }
    chk!("genbank", genbank::parse(text));
    chk!("embl", embl::parse(text));
    chk!("fasta", fasta::parse(text));
    chk!("fastq", fastq::parse(text));
    chk!(
        "fastq_stream",
        FastqReader::new(std::io::Cursor::new(text.as_bytes().to_vec())).count()
    );
    chk!("nexus", nexus::parse_nexus(text));
    chk!("clustal", alignment_formats::parse_clustal(text));
    chk!("phylip", alignment_formats::parse_phylip(text));
    chk!("bed", bed::parse_bed(text));
    chk!("gff3", gff3::parse_gff3(text));
    chk!("sam", sam::parse(text));
    chk!("cigar", sam::parse_cigar(text));
    chk!("vcf", vcf::parse(text));
    chk!("pdb", pdb::parse_pdb(text));
    chk!("csfasta", csfasta::parse_csfasta(text));
    chk!("csfasta_farben", csfasta::decode_color_space(text));
    chk!("solid_qual", csfasta::parse_solid_qual(text));
    chk!("ace", ace::parse_ace(text));
    chk!("cml", cml::parse_cml(text));
    chk!("scf", scf::parse_scf(text.as_bytes()));
    chk!("sff", sff::parse_sff(text.as_bytes()));
    chk!("mega", mega::parse_mega(text));
    chk!("pir", pir::parse_pir(text));
    chk!("gcg", gcg::parse_gcg(text));
    chk!("xyz", xyz::parse_xyz(text));
    chk!("mol", mol::parse_mol(text));
    chk!("paste", paste::parse_pasted(text));
    chk!("tabelle", parse_delimited(text, ','));
    chk!(
        "tabelle_schreiben",
        to_delimited(&parse_delimited(text, ';'), '\t')
    );
    chk!("fai", build_fai(text.as_bytes()));
    chk!("fai_parse", parse_fai(text));
    chk!("snapgene_xml", snapgene::parse_features(text));
    chk!("snapgene", snapgene::parse_snapgene(text.as_bytes(), "x"));
    chk!("abif", abif::parse_abif(text.as_bytes()));
    chk!("bam", parse_bam(text.as_bytes()));
    panics
}

#[test]
fn beschaedigte_dateien_stuerzen_nicht_ab() {
    let mut rng = Rng::new(2026);
    let seeds = seeds();
    let mut failures: Vec<(String, String)> = Vec::new();
    for round in 0..4000 {
        let base = &seeds[round % seeds.len()];
        let text = mutate(base, &mut rng);
        for name in run_all(&text) {
            failures.push((name.to_string(), text.chars().take(120).collect()));
        }
    }
    // pure random bytes
    for _ in 0..2000 {
        let len = rng.below(300);
        let bytes: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let text = String::from_utf8_lossy(&bytes).to_string();
        for name in run_all(&text) {
            failures.push((name.to_string(), text.chars().take(60).collect()));
        }
    }
    failures.sort();
    failures.dedup_by(|a, b| a.0 == b.0);
    assert!(failures.is_empty(), "Crashes: {:#?}", failures);
}
