//! Integrationstest Phase 5: Reads simulieren → QC → Trimming → Variantenerkennung → VCF; dazu FASTA-Index.

use snapious_core::io::fastq::FastqRecord;
use snapious_core::io::sam::SamRecord;
use snapious_core::io::vcf;
use snapious_core::ngs::fai::{build_fai, fetch_from_bytes};
use snapious_core::ngs::pileup::{call_snvs, to_vcf, CallOptions};
use snapious_core::ngs::qc::analyze_reads;
use snapious_core::ngs::trim::{trim_all, TrimOptions};
use snapious_core::util::Rng;

const REF_LEN: usize = 5000;
const SNV_POS: usize = 2500;

fn reference() -> String {
    let mut rng = Rng::new(11);
    (0..REF_LEN)
        .map(|_| ['A', 'C', 'G', 'T'][rng.below(4)])
        .collect()
}

/// 1000 Reads à 100 bp (mittlere Tiefe 20); jeder zweite Read, der Position 2500 überdeckt, trägt dort eine Substitution.
fn simulate(reference: &str) -> (Vec<FastqRecord>, Vec<SamRecord>) {
    let mut rng = Rng::new(5);
    let bytes: Vec<char> = reference.chars().collect();
    let alt = |c: char| if c == 'G' { 'T' } else { 'G' };
    let mut fq = Vec::new();
    let mut sam = Vec::new();
    let mut covering = 0usize;
    for i in 0..1000 {
        let start = rng.below(REF_LEN - 100);
        let mut seq: Vec<char> = bytes[start..start + 100].to_vec();
        if (start..start + 100).contains(&SNV_POS) {
            covering += 1;
            if covering.is_multiple_of(2) {
                let k = SNV_POS - start;
                seq[k] = alt(seq[k]);
            }
        }
        let seq: String = seq.into_iter().collect();
        let qual = "I".repeat(100);
        let name = format!("read{}", i);
        fq.push(FastqRecord {
            id: name.clone(),
            comment: String::new(),
            sequence: seq.clone(),
            quality: qual.clone(),
        });
        sam.push(SamRecord {
            read_name: name,
            flag: 0,
            reference: "ref".to_string(),
            position: start + 1,
            mapq: 60,
            cigar: "100M".to_string(),
            mate_reference: "*".to_string(),
            mate_position: 0,
            template_length: 0,
            sequence: seq,
            quality: qual,
        });
    }
    (fq, sam)
}

#[test]
fn von_den_reads_bis_zur_variante() {
    let reference = reference();
    let (fq, sam) = simulate(&reference);

    // QC
    let qc = analyze_reads(fq.clone(), "AGATCGGAAGAGC", 100_000);
    assert_eq!(qc.reads, 1000);
    assert_eq!((qc.min_length, qc.max_length), (100, 100));
    assert!((qc.gc_percent - 50.0).abs() < 5.0, "{}", qc.gc_percent);
    assert!(qc.mean_quality_per_position.iter().all(|q| *q == 40.0));
    assert_eq!(qc.adapter_percent, 0.0);

    // Trimming ändert gute Reads kaum (zufällige Adapter-Treffer am Readende sind selten)
    let (kept, stats) = trim_all(fq, &TrimOptions::standard());
    assert!(kept.len() >= 990, "{}", kept.len());
    assert!(stats.bases_removed < 500, "{}", stats.bases_removed);

    // Varianten: genau die Substitution an Position 2500
    let snvs = call_snvs("ref", &reference, &sam, &CallOptions::standard()).unwrap();
    assert_eq!(snvs.len(), 1, "{:?}", snvs);
    assert_eq!(snvs[0].position, SNV_POS);
    assert!(snvs[0].alt_fraction > 0.3 && snvs[0].alt_fraction < 0.7);

    // VCF
    let calls = to_vcf("ref", &snvs);
    let text = vcf::format(&calls);
    let back = vcf::parse(&text).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].position, SNV_POS + 1);
    assert!(back[0].info.starts_with("DP="));
}

#[test]
fn fasta_index_liefert_dieselben_basen() {
    let reference = reference();
    let wrapped: Vec<String> = reference
        .as_bytes()
        .chunks(70)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect();
    let fasta = format!(">ref beschreibung\n{}\n", wrapped.join("\n"));
    let idx = build_fai(fasta.as_bytes()).unwrap();
    assert_eq!(idx[0].length, REF_LEN as u64);
    let got = fetch_from_bytes(fasta.as_bytes(), &idx[0], 2490, 2510).unwrap();
    assert_eq!(got, &reference[2490..2510]);
}
