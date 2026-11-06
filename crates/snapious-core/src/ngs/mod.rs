//! NGS-Grundlagen: FASTQ streamen, FASTA-Index, Qualitätsbericht, Trimming, BAM lesen, Varianten, Mapping- und Assembly-Aufrufe.

pub mod bam;
pub mod barcode;
pub mod coverage_stats;
pub mod duplicates;
pub mod expression;
pub mod fai;
pub mod fastq_stream;
pub mod histogram;
pub mod insert_size;
pub mod kmers;
pub mod mapper;
pub mod pairs;
pub mod pileup;
pub mod qc;
pub mod spades;
pub mod stats;
pub mod trim;
