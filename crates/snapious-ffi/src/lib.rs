//! UniFFI-Schnittstelle des Snapious-Kerns für die Swift-App.
//!
//! Leitlinien: kleine, stabile Oberfläche; einfache Typen (Zahlen, Text, Listen, Records); Daten in
//! Blöcken statt zeichenweise; Fehler als [`ApiError`] (in Swift `throws`). Keine Fachlogik hier:
//! alles Rechnen steckt in `snapious-core`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use snapious_core::bio::align_columns::{
    columns_by_gaps, columns_codon_position, columns_constant, remove_columns,
};
use snapious_core::bio::align_stats::{
    column_identity, consensus, identity_matrix_percent, sequence_logo, strip_columns,
};
use snapious_core::bio::annotation_compare::{compare_sets, MatchMode, Span};
use snapious_core::bio::annotation_extract::extract_sequences;
use snapious_core::bio::apply_variants::apply_variants;
use snapious_core::bio::assembly::{assemble_contigs, place_reads};
use snapious_core::bio::backends::{align_with, AlignBackend};
use snapious_core::bio::backtranslate::{back_translate_degenerate, back_translate_preferred};
use snapious_core::bio::blast::blast_local;
use snapious_core::bio::blast_remote::{run_remote, BlastTransport, Sleeper};
use snapious_core::bio::calc;
use snapious_core::bio::dotplot::{diagonals, dotplot_points};
use snapious_core::bio::feature_match::find_feature_matches;
use snapious_core::bio::liabilities::find_liabilities;
use snapious_core::bio::motif::find_motif;
use snapious_core::bio::nussinov::max_pairs;
use snapious_core::bio::protein::{
    aromaticity, charge_at_ph, extinction_coefficient_280, isoelectric_point, molecular_weight,
};
use snapious_core::bio::protein_digest::{digest_protein, Protease};
use snapious_core::bio::protein_extra::{aliphatic_index, gravy};
use snapious_core::bio::read_consensus::quality_consensus;
use snapious_core::bio::read_map::map_reads;
use snapious_core::bio::repeats::{find_inverted_repeats, find_tandem_repeats};
use snapious_core::bio::rna_fold::{fold_rna, pairs_from_dot_bracket};
use snapious_core::bio::seq_stats::{cpg_islands, gc_windows};
use snapious_core::bio::silent_mutations::{
    changes_creating_motif, changes_removing_motif, synonymous_changes,
};
use snapious_core::bio::similarity_graph::{column_identity as column_identity_graph, window_mean};
use snapious_core::bio::variants::{find_variants, VariantKind};
use snapious_core::cloning::concatenate::concatenate;
use snapious_core::cloning::cut_positions::{cut_positions_from_spans, fragments_from_cuts};
use snapious_core::cloning::enzyme_sets::{builtin_sets, parse_enzyme_list, EnzymeSet};
use snapious_core::cloning::enzymes::iupac_reverse_complement;
use snapious_core::cloning::enzymes::EnzymeDb;
use snapious_core::cloning::fragment::{digest, End, EndType};
use snapious_core::cloning::gateway::{Gateway, GatewaySite};
use snapious_core::cloning::gel::{GelSimulator, Ladder};
use snapious_core::cloning::golden_gate;
use snapious_core::cloning::ligation::restriction_candidates;
use snapious_core::cloning::overlap::OverlapAssembly;
use snapious_core::cloning::topo::{directional_topo, ta_cloning};
use snapious_core::coverage::depth_from_sam;
use snapious_core::crispr::{find_guides, PamSpec};
use snapious_core::db::cache::{CachingHttp, SystemClock};
use snapious_core::db::ncbi::{CurlClient, HttpGet, NcbiClient};
use snapious_core::db::uniprot::UniprotClient;
use snapious_core::db::uniprot_entry::{entry_url, parse_entry};
use snapious_core::export::table::{parse_delimited, to_delimited};
use snapious_core::export::text::{format_sequence_text, report_markdown};
use snapious_core::io;
use snapious_core::io::abif::parse_abif;
use snapious_core::io::alignment_formats::{write_clustal, write_phylip};
use snapious_core::io::bed::{bed_to_features, parse_bed};
use snapious_core::io::blast_table::parse_blast_table;
use snapious_core::io::cml::parse_cml;
use snapious_core::io::gff3::{gff3_to_features, parse_gff3};
use snapious_core::io::gff3_write::write_gff3;
use snapious_core::io::hin::parse_hin;
use snapious_core::io::hmmer::parse_domtblout;
use snapious_core::io::interpro::parse_interpro_tsv;
use snapious_core::io::mol::parse_mol;
use snapious_core::io::paste::{detect_kind, parse_pasted, Kind};
use snapious_core::io::pdb::parse_pdb;
use snapious_core::io::qual::write_qual;
use snapious_core::io::quality_trim_range;
use snapious_core::io::sdf::parse_sdf;
use snapious_core::io::table_import::sequences_from_table;
use snapious_core::io::xyz::parse_xyz;
use snapious_core::model::sequence::{Sequence, SequenceType};
use snapious_core::model::{AlignmentDocument, Document, SequenceDocument, Strand, Topology};
use snapious_core::model::{Interval, Location};
use snapious_core::ngs::bam::read_bam;
use snapious_core::ngs::barcode::assign_barcodes;
use snapious_core::ngs::coverage_stats::coverage_stats;
use snapious_core::ngs::duplicates::{mark_duplicates, ReadKey};
use snapious_core::ngs::expression::{rpkm, tpm};
use snapious_core::ngs::fastq_stream::open_fastq;
use snapious_core::ngs::histogram::histogram as core_histogram;
use snapious_core::ngs::insert_size::{insert_sizes, insert_stats};
use snapious_core::ngs::kmers::{count_kmers, top_kmers};
use snapious_core::ngs::mapper::{
    bwa_index_args, bwa_index_exists, bwa_mem_args, minimap2_args, run_to_file,
};
use snapious_core::ngs::pairs::pair_reads as core_pair_reads;
use snapious_core::ngs::pileup::{call_snvs, to_vcf, CallOptions};
use snapious_core::ngs::qc::analyze_reads;
use snapious_core::ngs::stats::{benjamini_hochberg, median_of_ratios};
use snapious_core::ngs::trim::{trim_all, TrimOptions};
use snapious_core::ops::record::workflow_from_history;
use snapious_core::ops::{
    HistoryEntry, NullContext, OperationRegistry, OperationRunner, Workflow, WorkflowRunner,
};
use snapious_core::plugin::{discover, register_plugins};
use snapious_core::primer::mutagenesis::design_mutagenesis;
use snapious_core::primer::picker::{pick_pairs, PrimerConstraints};
use snapious_core::project::search as project_search;
use snapious_core::seqcheck::{DifferenceType, SequencingCheck};
use snapious_core::settings::{load_settings, save_settings, Settings};
use snapious_core::store::portable_path::{resolve_portable, to_portable};
use snapious_core::store::ProjectStore;
use snapious_core::tools::ToolRegistry;
use snapious_core::util::{new_id, now_secs};
use snapious_core::view::command_search::{rank_commands, Command};
use snapious_core::view::edit_session::EditSession;
use snapious_core::view::enzyme_map::{EnzymeFilter, EnzymeMapModel};
use snapious_core::view::error_hint::friendly_error as core_friendly_error;
use snapious_core::view::feature_detect::{common_templates, detect_features, find_orf_features};
use snapious_core::view::feature_layout::assign_lanes;
use snapious_core::view::feature_table::{filter_features, sort_features, FeatureRow, SortColumn};
use snapious_core::view::map_svg::{circular_map_svg, linear_map_svg};
use snapious_core::view::recent_files::add_recent;
use snapious_core::view::selection_stats::{as_fasta, as_reverse_complement, selection_stats};
use snapious_core::view::shortcut_conflicts::{find_conflicts, Shortcut};
use snapious_core::view::shortcut_format::format_shortcut as core_format_shortcut;
use snapious_core::view::translation::{
    translate_feature, translate_frame as translate_frame_core,
};

uniffi::setup_scaffolding!();

/// Fehler für die Oberfläche (die Meldung ist deutsch und für Nutzer gedacht).
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ApiError {
    #[error("{message}")]
    InvalidInput { message: String },
    #[error("{message}")]
    NotFound { message: String },
    #[error("{message}")]
    Io { message: String },
    #[error("{message}")]
    Internal { message: String },
}

impl From<snapious_core::Error> for ApiError {
    fn from(e: snapious_core::Error) -> Self {
        use snapious_core::Error as E;
        let message = e.to_string();
        match e {
            E::InvalidInput(_) | E::OutOfRange(_) => ApiError::InvalidInput { message },
            E::Io(_) => ApiError::Io { message },
            E::InvalidState(_) | E::Operation(_) => ApiError::Internal { message },
        }
    }
}

/// Kurzinfo zu einem Dokument im Projekt.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct DocumentInfoDto {
    pub id: String,
    pub name: String,
    pub doc_type: String,
    /// Erstellungszeit, Sekunden seit 1970 (UTC).
    pub created: i64,
}

/// Eckdaten eines Sequenzdokuments.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct DocumentSummary {
    pub id: String,
    pub name: String,
    pub doc_type: String,
    pub length: u64,
    /// `LINEAR` oder `CIRCULAR`.
    pub topology: String,
    /// `DNA`, `RNA` oder `PROTEIN`.
    pub seq_type: String,
    pub feature_count: u64,
    /// GC-Anteil in Prozent.
    pub gc_percent: f64,
    pub description: String,
    pub accession: String,
    pub organism: String,
}

/// Ein Feature (Annotation) mit Gesamtbereich `[start, end)`.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct FeatureInfo {
    pub feature_type: String,
    pub name: String,
    pub start: u64,
    pub end: u64,
    /// `FORWARD`, `REVERSE` oder `NONE`.
    pub strand: String,
}

/// Ein Teilstück eines Features mit Spur (für die Annotationsleisten unter der Sequenz).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PlacedFeatureDto {
    /// Index in der Liste von `features`.
    pub feature_index: u64,
    pub feature_type: String,
    pub name: String,
    pub start: u64,
    pub end: u64,
    /// `FORWARD`, `REVERSE` oder `NONE`.
    pub strand: String,
    pub lane: u32,
}

/// Kennzahlen einer Auswahl.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SelectionStatsDto {
    pub length: u64,
    pub gc_percent: f64,
    pub tm_celsius: f64,
}

/// Ein Enzym mit seinen Schnittstellen.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct EnzymeEntryDto {
    pub enzyme: String,
    pub count: u64,
    pub positions: Vec<u64>,
}

/// Eine mögliche stille Änderung in einem CDS-Feature (Position im Dokument, 0-basiert).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SilentChangeDto {
    pub position: u64,
    pub from: String,
    pub to: String,
    /// Nummer des Codons im CDS (1-basiert).
    pub codon: u64,
}

/// Ein Enzymset.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct EnzymeSetDto {
    pub name: String,
    pub enzymes: Vec<String>,
    /// `true` bei den eingebauten Sets (nicht löschbar).
    pub builtin: bool,
}

/// Ergebnis des Vergleichs der Annotationen zweier Sequenzen (Indizes in der Feature-Liste des jeweiligen Dokuments).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AnnotationComparisonDto {
    pub only_a: Vec<u64>,
    pub only_b: Vec<u64>,
    pub common_a: Vec<u64>,
    pub common_b: Vec<u64>,
}

/// Eine Gruppe aus der Aufteilung nach Barcodes.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct BarcodeGroupDto {
    pub name: String,
    pub count: u64,
    /// Ausgabedatei (FASTQ).
    pub path: String,
}

/// Ein Read-Paar und die ungepaarten Reads.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PairReportDto {
    /// Je Paar `[Vorwärtsread, Rückwärtsread]`.
    pub pairs: Vec<Vec<String>>,
    pub unpaired: Vec<String>,
}

/// Expressionswerte einer Referenz.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ExpressionRowDto {
    pub reference: String,
    pub length: u64,
    pub count: u64,
    pub rpkm: f64,
    pub tpm: f64,
}

/// Eine Klasse eines Häufigkeitsdiagramms.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct HistogramBinDto {
    pub start: u64,
    pub count: u64,
}

/// Ein Treffer der Feature-Suche nach Ähnlichkeit.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct FeatureMatchDto {
    pub start: u64,
    pub end: u64,
    pub reverse: bool,
    pub mismatches: u64,
}

/// Ein Eintrag der Befehlspalette.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CommandItemDto {
    pub title: String,
    pub keywords: Vec<String>,
}

/// Titel und Lösungsvorschlag zu einer Fehlermeldung.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ErrorHelpDto {
    pub title: String,
    pub hint: String,
}

/// Eine Schwachstelle in einem Protein.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct LiabilityDto {
    pub position: u64,
    pub length: u64,
    pub kind: String,
    pub motif: String,
}

/// Kennzahlen der Abdeckung einer SAM/BAM-Datei gegen eine Referenz.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CoverageReportDto {
    pub mean: f64,
    pub median: f64,
    pub min: u64,
    pub max: u64,
    /// Anteil der Positionen mit Tiefe ≥ 1, ≥ 10 und ≥ 30 (in dieser Reihenfolge).
    pub fractions: Vec<f64>,
    /// Zugeordnete Reads, davon Duplikate (gleiche Referenz, Position und Richtung).
    pub mapped_reads: u64,
    pub duplicate_reads: u64,
}

/// Fragmentlängen gepaarter Reads.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct InsertReportDto {
    pub count: u64,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    /// Häufigkeit in Klassen der Breite 25 bp (Beginn der Klasse, Anzahl).
    pub bins: Vec<HistogramBinDto>,
}

/// Ein k-mer mit Häufigkeit.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct KmerDto {
    pub kmer: String,
    pub count: u64,
}

/// Ergebnis der einfachen Faltung.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SimpleFoldDto {
    pub pairs: u64,
    /// Punkt-Klammer-Schreibweise.
    pub structure: String,
}

/// Eine att-Stelle für Gateway: Name, Kern und die vier Arme (`attB = links_b + kern + rechts_b`, `attP = links_p + kern + rechts_p`).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GatewaySiteDto {
    pub name: String,
    pub left_arm_b: String,
    pub core: String,
    pub right_arm_b: String,
    pub left_arm_p: String,
    pub right_arm_p: String,
}

/// Eine Aminosäure unter der Sequenz.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AminoAcidDto {
    pub base_position: u64,
    pub amino_acid: String,
    pub is_start: bool,
    pub is_stop: bool,
}

/// Ein Fragment eines virtuellen Verdaus.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct FragmentDto {
    pub length: u64,
    /// Schnittpositionen in der Quelle; bei einem Fragment über den Nullpunkt ist `end < start`.
    pub start: u64,
    pub end: u64,
    /// Textform der Enden, z. B. `stumpf`, `5' AATT`, `3' TGCA`.
    pub left_end: String,
    pub right_end: String,
}

/// Ein Enzym der Standarddatenbank.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct EnzymeInfoDto {
    pub name: String,
    pub recognition: String,
    pub sticky: bool,
}

/// Eine Gelbande: `mobility` 0 = oben, 1 = unten; `intensity` 0 bis 1.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GelBandDto {
    pub size_bp: u64,
    pub mobility: f64,
    pub intensity: f64,
}

/// Eine gRNA-Stelle.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GuideDto {
    pub protospacer: String,
    pub pam: String,
    pub start: u64,
    pub end: u64,
    pub forward: bool,
    pub gc_percent: f64,
    pub score: f64,
    pub warnings: Vec<String>,
}

/// Ein mögliches Klonierungsprodukt (Restriktionsklonierung).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CandidateDto {
    pub construct_length: u64,
    pub directional: bool,
    pub insert_reversed: bool,
    pub vector_length: u64,
    pub insert_length: u64,
    pub warnings: Vec<String>,
}

/// Ein Primerpaar für ein Zielfragment.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PrimerPairDto {
    pub forward_sequence: String,
    pub forward_start: u64,
    pub forward_end: u64,
    pub forward_tm: f64,
    pub reverse_sequence: String,
    pub reverse_start: u64,
    pub reverse_end: u64,
    pub reverse_tm: f64,
    pub product_size: u64,
    pub penalty: f64,
}

/// Ein Alignment zur Anzeige.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AlignmentDto {
    pub id: String,
    pub name: String,
    pub seq_type: String,
    pub row_names: Vec<String>,
    pub rows: Vec<String>,
    /// Konsens (Schwelle 0.5) und Identität je Spalte (0 bis 1).
    pub consensus: String,
    pub identity: Vec<f64>,
}

/// Ergebnis eines Zusammenbaus (Gibson oder Golden Gate): das neue Dokument und Hinweise.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AssemblyDto {
    pub document: DocumentInfoDto,
    pub warnings: Vec<String>,
}

/// Ein NCBI-Treffer.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NcbiRecordDto {
    pub id: String,
    pub accession: String,
    pub title: String,
    pub organism: String,
    /// Länge in Basen, 0 wenn unbekannt.
    pub length: u64,
}

/// Ergebnis einer NCBI-Suche.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NcbiSearchDto {
    pub total: u64,
    pub records: Vec<NcbiRecordDto>,
}

/// Eigenschaften eines Proteins.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ProteinPropsDto {
    pub length: u64,
    pub molecular_weight: f64,
    pub isoelectric_point: f64,
    pub charge_ph7: f64,
    pub extinction_280: f64,
    pub aromaticity: f64,
    pub aliphatic_index: f64,
    /// Mittlere Hydrophobie nach Kyte-Doolittle (GRAVY).
    pub gravy: f64,
}

/// Ein Peptid aus einem Verdau.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PeptideDto {
    pub sequence: String,
    pub start: u64,
    pub end: u64,
    pub missed_cleavages: u64,
    pub mass: f64,
}

/// GC-Anteil eines Fensters.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GcPointDto {
    pub position: u64,
    pub percent: f64,
}

/// Ein Bereich `[start, end)`.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RegionDto {
    pub start: u64,
    pub end: u64,
}

/// Ein Farbkanal einer Sequenzierspur.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TraceChannelDto {
    /// Base des Kanals (`A`, `C`, `G`, `T`).
    pub base: String,
    pub values: Vec<i32>,
}

/// Eine gelesene Sequenzierspur (`.ab1`).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TraceDto {
    pub bases: String,
    pub quality: Vec<u8>,
    pub peak_positions: Vec<u64>,
    pub channels: Vec<TraceChannelDto>,
}

/// Ein Unterschied zwischen Read und Referenz.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct DifferenceDto {
    /// `MISMATCH`, `INSERTION`, `DELETION` oder `LOW_QUALITY`.
    pub kind: String,
    pub reference_position: u64,
    pub read_position: u64,
    pub expected: String,
    pub observed: String,
    pub quality: u8,
}

/// Ergebnis des Abgleichs eines Reads mit einer Referenz.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ReadCheckDto {
    pub read_name: String,
    pub reversed: bool,
    pub reference_start: u64,
    pub reference_end: u64,
    pub identity: f64,
    pub differences: Vec<DifferenceDto>,
    pub aligned_read: String,
    pub aligned_reference: String,
}

/// Ein BLAST-Treffer.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct BlastHitDto {
    pub subject: String,
    pub identity_percent: f64,
    pub alignment_length: u64,
    pub query_start: u64,
    pub query_end: u64,
    pub subject_start: u64,
    pub subject_end: u64,
    pub reverse_strand: bool,
    pub evalue: f64,
    pub bit_score: f64,
}

/// Ein Basenpaar einer RNA-Struktur (0-basiert, `first < second`).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct BasePairDto {
    pub first: u64,
    pub second: u64,
}

/// Ergebnis einer RNA-Faltung.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RnaFoldDto {
    pub sequence: String,
    pub structure: String,
    pub energy: f64,
    pub pairs: Vec<BasePairDto>,
}

/// Primerpaar für gezielte Mutagenese.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct MutagenesisDto {
    pub forward: String,
    pub reverse: String,
    pub left_flank: u64,
    pub right_flank: u64,
    pub tm: f64,
}

/// Eine Wiederholung (`kind`: `tandem` oder `inverted`); `detail` beschreibt Einheit/Arme.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RepeatDto {
    pub kind: String,
    pub start: u64,
    pub end: u64,
    pub detail: String,
}

/// Eine Variante aus einem Alignment.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct VariantDto {
    pub column: u64,
    pub reference_position: u64,
    pub row_name: String,
    pub reference_base: String,
    pub alt_base: String,
    pub kind: String,
}

/// Abbildung eines Reads auf eine Referenz.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ReadMapDto {
    pub name: String,
    pub mapped: bool,
    pub position: u64,
    pub reverse: bool,
    pub mismatches: u64,
}

/// Diagonale eines Dotplots.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct DiagonalDto {
    pub start_a: u64,
    pub start_b: u64,
    pub length: u64,
}

/// Ng → pmol für dsDNA.
#[uniffi::export]
pub fn calc_ng_to_pmol(ng: f64, bp: u64) -> Result<f64, ApiError> {
    Ok(calc::ng_to_pmol(ng, bp as usize)?)
}

/// Benötigte Insert-Menge in ng für ein molares Verhältnis.
#[uniffi::export]
pub fn calc_ligation_insert_ng(
    vector_ng: f64,
    vector_bp: u64,
    insert_bp: u64,
    molar_ratio: f64,
) -> Result<f64, ApiError> {
    Ok(calc::ligation_insert_ng(
        vector_ng,
        vector_bp as usize,
        insert_bp as usize,
        molar_ratio,
    )?)
}

/// Volumen der Stammlösung für eine Verdünnung.
#[uniffi::export]
pub fn calc_dilution_volume(c_stock: f64, c_final: f64, v_final: f64) -> Result<f64, ApiError> {
    Ok(calc::dilution_volume(c_stock, c_final, v_final)?)
}

/// Kopienzahl von dsDNA.
#[uniffi::export]
pub fn calc_copy_number(ng: f64, bp: u64) -> Result<f64, ApiError> {
    Ok(calc::copy_number(ng, bp as usize)?)
}

/// Paarweise Identität (Prozent) aller Zeilen eines Alignments; `values` zeilenweise (`names.len()²` Werte).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct IdentityMatrixDto {
    pub names: Vec<String>,
    pub values: Vec<f64>,
}

/// Eine Spalte des Sequenzlogos: Symbole aufsteigend nach Höhe, `heights` in Bit.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct LogoColumnDto {
    pub symbols: Vec<String>,
    pub heights: Vec<f64>,
}

/// Ein Fundort einer Motivsuche.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct MotifHitDto {
    pub start: u64,
    pub end: u64,
    pub reverse: bool,
}

/// Eine verfügbare Operation (für Workflows).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct OperationInfoDto {
    pub id: String,
    pub label: String,
}

/// Ein gespeicherter Workflow.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct WorkflowDto {
    pub name: String,
    pub json: String,
    /// Zahl der Eingabedokumente, die der Workflow braucht (0, wenn das JSON ungültig ist).
    pub input_count: u64,
    pub step_count: u64,
}

/// Ergebnis eines Workflow-Laufs.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct WorkflowRunDto {
    pub documents: Vec<DocumentInfoDto>,
    /// Fehlermeldungen je Lauf (leer bei Erfolg).
    pub errors: Vec<String>,
}

fn end_text(e: &End) -> String {
    match e.end_type {
        EndType::Blunt => "stumpf".to_string(),
        EndType::FivePrime => format!("5' {}", e.overhang),
        EndType::ThreePrime => format!("3' {}", e.overhang),
    }
}

fn strand_text(s: Strand) -> String {
    match s {
        Strand::Forward => "FORWARD",
        Strand::Reverse => "REVERSE",
        Strand::None => "NONE",
    }
    .to_string()
}

/// Eine geöffnete Projektdatei.
#[derive(uniffi::Object)]
pub struct Project {
    store: Mutex<ProjectStore>,
    path: String,
    /// Bearbeitungssitzungen (Undo/Redo) je Dokument-ID; jede Änderung wird sofort gespeichert.
    sessions: Mutex<BTreeMap<String, EditSession>>,
}

impl Project {
    fn lock(&self) -> Result<MutexGuard<'_, ProjectStore>, ApiError> {
        self.store.lock().map_err(|_| ApiError::Internal {
            message: "Projektspeicher ist gesperrt (vorheriger Fehler)".to_string(),
        })
    }

    fn candidates_for(
        &self,
        vector_id: &str,
        vector_enzymes: &[String],
        insert_id: &str,
        insert_enzymes: &[String],
    ) -> Result<Vec<snapious_core::cloning::ligation::CloningCandidate>, ApiError> {
        let v = self.sequence_doc(vector_id)?;
        let i = self.sequence_doc(insert_id)?;
        let ve: Vec<&str> = vector_enzymes.iter().map(String::as_str).collect();
        let ie: Vec<&str> = insert_enzymes.iter().map(String::as_str).collect();
        Ok(restriction_candidates(
            &v,
            &ve,
            &i,
            &ie,
            &EnzymeDb::standard(),
        )?)
    }

    /// Führt eine Änderung in der Bearbeitungssitzung des Dokuments aus und speichert das Ergebnis.
    fn edit<F>(&self, id: &str, change: F) -> Result<(), ApiError>
    where
        F: FnOnce(&mut EditSession) -> snapious_core::error::Result<()>,
    {
        let mut sessions = self.sessions.lock().map_err(|_| ApiError::Internal {
            message: "Bearbeitungssitzungen sind gesperrt (vorheriger Fehler)".to_string(),
        })?;
        if !sessions.contains_key(id) {
            let doc = self.sequence_doc(id)?;
            sessions.insert(id.to_string(), EditSession::new(doc));
        }
        let session = sessions.get_mut(id).ok_or_else(|| ApiError::Internal {
            message: "Bearbeitungssitzung fehlt".to_string(),
        })?;
        change(session)?;
        let doc = Document::Sequence(session.document().clone());
        let mut store = self.lock()?;
        // Eine Version höchstens alle 30 Sekunden, damit Tipp-Bearbeitungen den Speicher nicht füllen
        let recent = store
            .versions(id)?
            .first()
            .is_some_and(|v| now_secs() - v.time < 30);
        if recent {
            store.save(&doc)?;
        } else {
            store.save_versioned(&doc, "Bearbeitet")?;
        }
        Ok(())
    }

    /// Vergisst die Bearbeitungssitzung eines Dokuments (z. B. nach dem Wiederherstellen einer Version).
    fn edit_sessions_clear(&self, id: &str) {
        if let Ok(mut s) = self.sessions.lock() {
            s.remove(id);
        }
    }

    fn alignment_doc(&self, id: &str) -> Result<AlignmentDocument, ApiError> {
        match self.lock()?.load(id)? {
            Some(Document::Alignment(d)) => Ok(d),
            Some(_) => Err(ApiError::InvalidInput {
                message: "Das Dokument ist kein Alignment".to_string(),
            }),
            None => Err(ApiError::NotFound {
                message: format!("Dokument '{}' nicht gefunden", id),
            }),
        }
    }

    fn save_new(
        &self,
        document: Document,
        operation: &str,
        inputs: Vec<String>,
        summary: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        let mut store = self.lock()?;
        store.save(&document)?;
        store.add_history(&HistoryEntry {
            id: new_id(),
            operation_id: operation.to_string(),
            parameters: Default::default(),
            input_ids: inputs,
            output_ids: vec![document.id().to_string()],
            time: now_secs(),
            summary,
        })?;
        Ok(DocumentInfoDto {
            id: document.id().to_string(),
            name: document.name().to_string(),
            doc_type: document.doc_type().to_string(),
            created: document.created(),
        })
    }

    fn feature_infos(features: Vec<snapious_core::model::Feature>) -> Vec<FeatureInfo> {
        features
            .iter()
            .map(|f| FeatureInfo {
                feature_type: f.feature_type().to_string(),
                name: f.name().to_string(),
                start: f.location().start() as u64,
                end: f.location().end() as u64,
                strand: strand_text(f.location().strand()),
            })
            .collect()
    }

    /// Qualitätswerte einer Sequenz aus der verknüpften Sequenzierdatei (`.ab1`/`.scf`), auf denselben Bereich gekürzt wie beim Import;
    /// ohne passende Datei gilt für jede Base der Wert 20.
    fn read_qualities(&self, id: &str, len: usize) -> Vec<u8> {
        let files = match self.lock() {
            Ok(store) => store.external_files(id).unwrap_or_default(),
            Err(_) => return vec![20; len],
        };
        for f in files.iter().filter(|f| f.kind == "ab1" || f.kind == "scf") {
            let Ok(bytes) = std::fs::read(&f.path) else {
                continue;
            };
            let (bases_len, qual) = if bytes.starts_with(b".scf") {
                match snapious_core::io::scf::parse_scf(&bytes) {
                    Ok(r) => (r.bases.len(), r.qualities),
                    Err(_) => continue,
                }
            } else {
                match parse_abif(&bytes) {
                    Ok(c) => (c.bases.len(), c.quality),
                    Err(_) => continue,
                }
            };
            let (a, b) = quality_trim_range(bases_len, &qual, 10, 20);
            if qual.len() >= b && b - a == len {
                return qual[a..b].to_vec();
            }
        }
        vec![20; len]
    }

    /// Echter Pfad zu einer gespeicherten Dateiangabe: Dateien im Ordner `<Projekt>.dateien` wandern mit dem Projekt mit.
    fn resolve_stored_path(&self, stored: &str) -> String {
        resolve_portable(Path::new(&self.path), stored, &|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| stored.to_string())
    }

    fn sequence_docs(&self, ids: &[String]) -> Result<Vec<SequenceDocument>, ApiError> {
        ids.iter().map(|id| self.sequence_doc(id)).collect()
    }

    fn sequence_doc(&self, id: &str) -> Result<SequenceDocument, ApiError> {
        match self.lock()?.load(id)? {
            Some(Document::Sequence(d)) => Ok(d),
            Some(_) => Err(ApiError::InvalidInput {
                message: "Das Dokument ist keine Sequenz".to_string(),
            }),
            None => Err(ApiError::NotFound {
                message: format!("Dokument '{}' nicht gefunden", id),
            }),
        }
    }
}

#[uniffi::export]
impl Project {
    /// Öffnet die Projektdatei oder legt sie neu an.
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, ApiError> {
        let store = ProjectStore::open(Path::new(&path))?;
        Ok(Arc::new(Self {
            store: Mutex::new(store),
            path,
            sessions: Mutex::new(BTreeMap::new()),
        }))
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Alle Dokumente, nach Erstellungszeit sortiert.
    pub fn documents(&self) -> Result<Vec<DocumentInfoDto>, ApiError> {
        Ok(self
            .lock()?
            .list()?
            .into_iter()
            .map(|i| DocumentInfoDto {
                id: i.id,
                name: i.name,
                doc_type: i.doc_type,
                created: i.created,
            })
            .collect())
    }

    /// Rückübersetzung eines Protein-Dokuments in ein neues DNA-Dokument. `mode`: `preferred` (häufigste E.-coli-Codons)
    /// oder `degenerate` (IUPAC-Mehrdeutigkeitscodes).
    pub fn back_translate_document(
        &self,
        id: String,
        mode: String,
        name: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        let d = self.sequence_doc(&id)?;
        if d.sequence().seq_type() != SequenceType::Protein {
            return Err(ApiError::InvalidInput {
                message: "Rückübersetzung geht nur von einem Protein aus".to_string(),
            });
        }
        let dna = back_translate(d.bases().to_string(), mode)?;
        let title = if name.trim().is_empty() {
            format!("{} (rückübersetzt)", d.name())
        } else {
            name.trim().to_string()
        };
        let seq = Sequence::new(&new_id(), &title, &dna, SequenceType::Dna)?;
        self.save_new(
            Document::Sequence(SequenceDocument::of(&title, seq)),
            "sequence.backtranslate",
            vec![id],
            "Rückübersetzung".to_string(),
        )
    }

    /// Mögliche stille Einzelbasenänderungen im CDS-Feature `feature_index` (nur Vorwärtsstrang). `site` ist leer (alle),
    /// eine DNA-Folge oder der Name eines Enzyms; `mode` ist `all`, `create` (erzeugt die Stelle) oder `remove` (zerstört sie).
    pub fn silent_changes(
        &self,
        id: String,
        feature_index: u64,
        site: String,
        mode: String,
    ) -> Result<Vec<SilentChangeDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let f = d
            .features()
            .get(feature_index as usize)
            .ok_or_else(|| ApiError::InvalidInput {
                message: format!("Feature {} gibt es nicht", feature_index),
            })?;
        if f.location().strand() == Strand::Reverse {
            return Err(ApiError::InvalidInput {
                message: "Stille Änderungen gibt es nur für CDS auf dem Vorwärtsstrang".to_string(),
            });
        }
        let start = f.location().start();
        let end = f.location().end().min(d.bases().len());
        let cds = d
            .bases()
            .get(start..end)
            .ok_or_else(|| ApiError::InvalidInput {
                message: "Der Bereich des Features liegt außerhalb der Sequenz".to_string(),
            })?;
        let motif = {
            let t = site.trim().to_uppercase();
            if t.is_empty() || t.chars().all(|c| "ACGT".contains(c)) {
                t
            } else {
                EnzymeDb::standard()
                    .get(site.trim())
                    .map(|e| e.recognition().to_uppercase())
                    .ok_or_else(|| ApiError::InvalidInput {
                        message: format!(
                            "'{}' ist weder eine DNA-Folge noch ein bekanntes Enzym",
                            site.trim()
                        ),
                    })?
            }
        };
        if mode != "all" && motif.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Für diese Auswahl ist eine Erkennungsstelle nötig".to_string(),
            });
        }
        let list = match mode.as_str() {
            "create" => changes_creating_motif(cds, &motif),
            "remove" => changes_removing_motif(cds, &motif),
            _ => synonymous_changes(cds),
        };
        Ok(list
            .into_iter()
            .map(|c| SilentChangeDto {
                position: (start + c.position) as u64,
                from: c.from.to_string(),
                to: c.to.to_string(),
                codon: (c.position / 3 + 1) as u64,
            })
            .collect())
    }

    /// Fügt Sequenzdokumente in der gegebenen Reihenfolge zu einer neuen Sequenz zusammen (Features wandern mit).
    pub fn concatenate_documents(
        &self,
        ids: Vec<String>,
        name: String,
        circular: bool,
    ) -> Result<DocumentInfoDto, ApiError> {
        let docs = self.sequence_docs(&ids)?;
        let result = concatenate(&docs, &name, circular)?;
        self.save_new(
            Document::Sequence(result),
            "sequence.concatenate",
            ids,
            "Sequenzen zusammengefügt".to_string(),
        )
    }

    /// Vergleicht die Features zweier Sequenzen. `mode` ist `exact` (Typ, Beginn und Ende gleich) oder `overlap`
    /// (gleicher Typ, überlappend).
    pub fn compare_annotations(
        &self,
        id_a: String,
        id_b: String,
        mode: String,
    ) -> Result<AnnotationComparisonDto, ApiError> {
        let spans = |d: &SequenceDocument| -> Vec<Span> {
            d.features()
                .iter()
                .map(|f| Span {
                    start: f.location().start(),
                    end: f.location().end(),
                    kind: f.feature_type().to_string(),
                })
                .collect()
        };
        let a = spans(&self.sequence_doc(&id_a)?);
        let b = spans(&self.sequence_doc(&id_b)?);
        let m = if mode == "overlap" {
            MatchMode::Overlap
        } else {
            MatchMode::Exact
        };
        let r = compare_sets(&a, &b, m);
        let conv = |v: Vec<usize>| v.into_iter().map(|i| i as u64).collect::<Vec<u64>>();
        Ok(AnnotationComparisonDto {
            only_a: conv(r.only_a),
            only_b: conv(r.only_b),
            common_a: conv(r.common_a),
            common_b: conv(r.common_b),
        })
    }

    /// Entfernt Spalten aus einem Alignment und legt das Ergebnis als neues Alignment an. `kind`: `gaps` (Lückenanteil
    /// mindestens `param`, 0–1), `constant` (Spalten ohne Lücke, in denen alle gleich sind) oder `codon` (Codonposition
    /// `param` = 1, 2 oder 3, ab Spalte `start`, 0-basiert).
    pub fn strip_alignment_columns(
        &self,
        id: String,
        kind: String,
        param: f64,
        start: u64,
        name: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        let a = self.alignment_doc(&id)?;
        let rows = a.rows();
        let len = rows.first().map(|r| r.len()).unwrap_or(0);
        let mask = match kind.as_str() {
            "gaps" => columns_by_gaps(rows, param),
            "constant" => columns_constant(rows),
            "codon" => {
                let pos = param as usize;
                if !(1..=3).contains(&pos) {
                    return Err(ApiError::InvalidInput {
                        message: "Die Codonposition muss 1, 2 oder 3 sein".to_string(),
                    });
                }
                columns_codon_position(len, start as usize, pos - 1)
            }
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannte Auswahl '{}'", other),
                })
            }
        };
        let removed = mask.iter().filter(|b| **b).count();
        if removed == len && len > 0 {
            return Err(ApiError::InvalidInput {
                message: "Es würden alle Spalten entfernt".to_string(),
            });
        }
        let new_rows = remove_columns(rows, &mask);
        let title = if name.trim().is_empty() {
            format!("{} (gekürzt)", a.name())
        } else {
            name.trim().to_string()
        };
        let doc = AlignmentDocument::of(&title, a.seq_type(), a.row_names().to_vec(), new_rows)?;
        self.save_new(
            Document::Alignment(doc),
            "alignment.strip_columns",
            vec![id],
            format!("{} Spalten entfernt", removed),
        )
    }

    /// Erzeugt aus einem Alignment eine neue Sequenz: die Referenzzeile (ohne Lücken) mit allen Varianten der Zeile `row`
    /// gegenüber der Zeile `reference`.
    pub fn sequence_with_variants(
        &self,
        id: String,
        reference: u64,
        row: u64,
        name: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        let a = self.alignment_doc(&id)?;
        let rows = a.rows();
        let ref_row = rows
            .get(reference as usize)
            .ok_or_else(|| ApiError::InvalidInput {
                message: "Die Referenzzeile gibt es nicht".to_string(),
            })?;
        let vars = find_variants(rows, reference as usize)?;
        let ungapped: String = ref_row
            .chars()
            .filter(|c| *c != '-' && *c != '.')
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let bases = apply_variants(&ungapped, &vars, row as usize)?;
        let title = if name.trim().is_empty() {
            format!(
                "{} mit Varianten von {}",
                a.row_names()
                    .get(reference as usize)
                    .cloned()
                    .unwrap_or_default(),
                a.row_names().get(row as usize).cloned().unwrap_or_default()
            )
        } else {
            name.trim().to_string()
        };
        let seq = Sequence::new(&new_id(), &title, &bases, a.seq_type())?;
        self.save_new(
            Document::Sequence(SequenceDocument::of(&title, seq)),
            "alignment.apply_variants",
            vec![id],
            "Varianten angewendet".to_string(),
        )
    }

    /// Mittlere paarweise Identität je Alignment-Spalte (0–1), mit einem zentrierten Fenster der Breite `window` geglättet
    /// (0 oder 1 = ungeglättet).
    pub fn alignment_similarity(&self, id: String, window: u64) -> Result<Vec<f64>, ApiError> {
        let a = self.alignment_doc(&id)?;
        Ok(window_mean(
            &column_identity_graph(a.rows()),
            window as usize,
        ))
    }

    /// Legt aus allen Features des Typs `feature_type` (leer = alle) je eine neue Sequenz an (mit `flank` Basen Rand, Gegenstrang
    /// revers-komplementiert). Höchstens 1000 Features.
    pub fn extract_features(
        &self,
        id: String,
        feature_type: String,
        flank: u64,
    ) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let picked: Vec<&snapious_core::model::feature::Feature> = d
            .features()
            .iter()
            .filter(|f| {
                feature_type.is_empty() || f.feature_type().eq_ignore_ascii_case(&feature_type)
            })
            .collect();
        if picked.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Keine passenden Features".to_string(),
            });
        }
        if picked.len() > 1000 {
            return Err(ApiError::InvalidInput {
                message: "Zu viele Features (höchstens 1000)".to_string(),
            });
        }
        let spans: Vec<(usize, usize, bool)> = picked
            .iter()
            .map(|f| {
                (
                    f.location().start(),
                    f.location().end(),
                    f.location().strand() == Strand::Reverse,
                )
            })
            .collect();
        let seqs = extract_sequences(d.bases(), &spans, flank as usize);
        let mut out = Vec::new();
        for (f, bases) in picked.iter().zip(seqs) {
            if bases.is_empty() {
                continue;
            }
            let title = format!("{} ({})", f.name(), d.name());
            let seq = Sequence::new(&new_id(), &title, &bases, d.sequence().seq_type())?;
            let info = self.save_new(
                Document::Sequence(SequenceDocument::of(&title, seq)),
                "sequence.extract",
                vec![id.clone()],
                format!("Feature {} extrahiert", f.name()),
            )?;
            out.push(info);
        }
        Ok(out)
    }

    /// Sucht die kurze Sequenz `reference` mit höchstens `max_mismatch` Unterschieden auf beiden Strängen (Feature-Finder
    /// nach Ähnlichkeit). Höchstens 5000 Treffer.
    pub fn find_similar_features(
        &self,
        id: String,
        reference: String,
        max_mismatch: u64,
    ) -> Result<Vec<FeatureMatchDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let n = reference.trim().len() as u64;
        Ok(
            find_feature_matches(d.bases(), reference.trim(), max_mismatch as usize)
                .into_iter()
                .take(5000)
                .map(|m| FeatureMatchDto {
                    start: m.start as u64,
                    end: m.start as u64 + n,
                    reverse: m.reverse,
                    mismatches: m.mismatches as u64,
                })
                .collect(),
        )
    }

    /// Schwachstellen eines Proteins (N-Glykosylierung, Deamidierung, Isomerisierung, Oxidation, freies Cystein).
    pub fn protein_liabilities(&self, id: String) -> Result<Vec<LiabilityDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        Ok(find_liabilities(d.bases())
            .into_iter()
            .map(|l| LiabilityDto {
                position: l.position as u64,
                length: l.length as u64,
                kind: l.kind,
                motif: l.motif,
            })
            .collect())
    }

    /// Liest eine Domänentabelle (`hmmscan --domtblout` oder InterProScan-TSV; `format` = `hmmer` oder `interpro`) und trägt die
    /// Treffer für dieses Protein als Features vom Typ `domain` ein. Gehört mehr als ein Name zur Datei, zählen nur Treffer, deren
    /// Name mit dem Dokumentnamen beginnt (sonst alle). Gibt die Zahl der eingetragenen Features zurück.
    pub fn import_domain_table(
        &self,
        id: String,
        path: String,
        format: String,
    ) -> Result<u64, ApiError> {
        let d = self.sequence_doc(&id)?;
        let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
            message: format!("{}: {}", path, e),
        })?;
        let mut hits: Vec<(String, String, usize, usize)> = if format == "interpro" {
            parse_interpro_tsv(&text)?
                .into_iter()
                .map(|h| {
                    (
                        h.protein,
                        format!("{} {}", h.signature, h.description)
                            .trim()
                            .to_string(),
                        h.start,
                        h.end,
                    )
                })
                .collect()
        } else {
            parse_domtblout(&text)?
                .into_iter()
                .map(|h| (h.query, h.domain, h.ali_from, h.ali_to))
                .collect()
        };
        let names: std::collections::BTreeSet<&str> = hits.iter().map(|h| h.0.as_str()).collect();
        if names.len() > 1 {
            let mine = d.name().to_string();
            let matching: Vec<(String, String, usize, usize)> = hits
                .iter()
                .filter(|h| mine.starts_with(h.0.as_str()) || h.0.starts_with(mine.as_str()))
                .cloned()
                .collect();
            if !matching.is_empty() {
                hits = matching;
            }
        }
        let len = d.len();
        let infos: Vec<FeatureInfo> = hits
            .into_iter()
            .filter(|h| h.2 >= 1 && h.3 >= h.2 && h.3 <= len)
            .map(|h| FeatureInfo {
                feature_type: "domain".to_string(),
                name: h.1,
                start: (h.2 - 1) as u64,
                end: h.3 as u64,
                strand: "NONE".to_string(),
            })
            .collect();
        let n = infos.len() as u64;
        if n == 0 {
            return Err(ApiError::InvalidInput {
                message: "Keine passenden Treffer in der Datei (Länge oder Name passen nicht)"
                    .to_string(),
            });
        }
        self.add_features(id, infos)?;
        Ok(n)
    }

    /// Einfache Faltung nach Nussinov für `[start, end)` (Ende 0 = bis zum Ende; höchstens 2000 Basen): größte Zahl
    /// nicht kreuzender Basenpaare, ohne Energiemodell. Auch für DNA.
    pub fn simple_fold(&self, id: String, start: u64, end: u64) -> Result<SimpleFoldDto, ApiError> {
        let d = self.sequence_doc(&id)?;
        let len = d.len();
        let e = if end == 0 {
            len
        } else {
            (end as usize).min(len)
        };
        let s0 = (start as usize).min(e);
        if e - s0 > 2000 {
            return Err(ApiError::InvalidInput {
                message: "Für die einfache Faltung höchstens 2000 Basen wählen".to_string(),
            });
        }
        let part = d.bases().get(s0..e).ok_or_else(|| ApiError::InvalidInput {
            message: "Bereich außerhalb der Sequenz".to_string(),
        })?;
        let (pairs, structure) = max_pairs(part, 3);
        Ok(SimpleFoldDto {
            pairs: pairs as u64,
            structure,
        })
    }

    /// Kennzahlen der Abdeckung und der Duplikate aus einer SAM/BAM-Datei gegen die gewählte Referenz.
    pub fn coverage_report(
        &self,
        reference_id: String,
        alignment_path: String,
    ) -> Result<CoverageReportDto, ApiError> {
        let d = self.sequence_doc(&reference_id)?;
        let path = std::path::Path::new(&alignment_path);
        let records = if alignment_path.to_lowercase().ends_with(".bam") {
            read_bam(path)?.records
        } else {
            io::sam::read(path)?.records
        };
        let name = records
            .iter()
            .find(|r| r.reference != "*")
            .map(|r| r.reference.clone())
            .unwrap_or_else(|| d.name().to_string());
        let depth = depth_from_sam(&records, &name, d.len(), false, 0, false)?;
        let c = coverage_stats(&depth, &[1, 10, 30]);
        let keys: Vec<ReadKey> = records
            .iter()
            .filter(|r| !r.is_unmapped() && !r.is_secondary() && r.reference != "*")
            .map(|r| ReadKey {
                reference: r.reference.clone(),
                position: r.position,
                reverse: r.is_reverse(),
                quality: r.quality.bytes().map(|b| b.saturating_sub(33) as u64).sum(),
            })
            .collect();
        let dups = mark_duplicates(&keys).iter().filter(|b| **b).count();
        Ok(CoverageReportDto {
            mean: c.mean,
            median: c.median,
            min: c.min as u64,
            max: c.max as u64,
            fractions: c.fractions,
            mapped_reads: keys.len() as u64,
            duplicate_reads: dups as u64,
        })
    }

    /// TA-Klonierung: setzt das PCR-Produkt in beide Orientierungen in den linearen T-Vektor (zwei neue zirkuläre Dokumente).
    pub fn clone_ta(
        &self,
        t_vector_id: String,
        pcr_product_id: String,
    ) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let v = self.sequence_doc(&t_vector_id)?;
        let p = self.sequence_doc(&pcr_product_id)?;
        let docs = ta_cloning(&v, &p)?;
        let mut out = Vec::new();
        for d in docs {
            out.push(self.save_new(
                Document::Sequence(d),
                "clone.ta",
                vec![t_vector_id.clone(), pcr_product_id.clone()],
                "TA-Klonierung".to_string(),
            )?);
        }
        Ok(out)
    }

    /// Gerichtetes TOPO-Klonieren: das Insert muss mit `CACC` beginnen und wird hinter den linearen TOPO-Vektor gesetzt.
    pub fn clone_topo(
        &self,
        vector_id: String,
        insert_id: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        let v = self.sequence_doc(&vector_id)?;
        let i = self.sequence_doc(&insert_id)?;
        let d = directional_topo(&v, &i)?;
        self.save_new(
            Document::Sequence(d),
            "clone.topo",
            vec![vector_id, insert_id],
            "Gerichtetes TOPO-Klonieren".to_string(),
        )
    }

    /// Gateway-Reaktion. `kind` ist `bp` (erstes Dokument: lineares Produkt mit attB1/attB2, zweites: zirkulärer Spendervektor mit
    /// attP1/attP2) oder `lr` (erstes: zirkulärer Entry-Klon mit attL1/attL2, zweites: zirkulärer Zielvektor mit attR1/attR2).
    /// Die att-Stellen kommen aus dem Handbuch des verwendeten Kits.
    pub fn clone_gateway(
        &self,
        kind: String,
        first_id: String,
        second_id: String,
        att1: GatewaySiteDto,
        att2: GatewaySiteDto,
    ) -> Result<DocumentInfoDto, ApiError> {
        let site = |s: GatewaySiteDto| GatewaySite {
            name: s.name,
            left_arm_b: s.left_arm_b.trim().to_uppercase(),
            core: s.core.trim().to_uppercase(),
            right_arm_b: s.right_arm_b.trim().to_uppercase(),
            left_arm_p: s.left_arm_p.trim().to_uppercase(),
            right_arm_p: s.right_arm_p.trim().to_uppercase(),
        };
        let (a1, a2) = (site(att1), site(att2));
        if a1.core.is_empty() || a2.core.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Für beide att-Stellen wird der Kern gebraucht".to_string(),
            });
        }
        let gw = Gateway::new(a1, a2);
        let first = self.sequence_doc(&first_id)?;
        let second = self.sequence_doc(&second_id)?;
        let (d, op) = match kind.as_str() {
            "bp" => (gw.bp_reaction(&first, &second)?, "clone.gateway.bp"),
            "lr" => (gw.lr_reaction(&first, &second)?, "clone.gateway.lr"),
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannte Reaktion '{}'", other),
                })
            }
        };
        self.save_new(
            Document::Sequence(d),
            op,
            vec![first_id, second_id],
            format!("Gateway {}", kind.to_uppercase()),
        )
    }

    /// Konsens aus mehreren Sequenzierungen (Sanger-Reads): Reads, die besser als Gegenstrang zum ersten passen (gemeinsame 8-mer), werden
    /// revers-komplementiert (samt Qualitäten), alle werden ausgerichtet; die Konsensbase je Spalte wählt die Summe der Qualitäten. Legt das
    /// Alignment der Reads und die Konsenssequenz als neue Dokumente an und gibt die Konsenssequenz zurück. `min_quality`: Basen darunter zählen nicht.
    pub fn consensus_from_reads(
        &self,
        ids: Vec<String>,
        min_quality: u8,
        name: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        if ids.len() < 2 {
            return Err(ApiError::InvalidInput {
                message: "Für einen Konsens mindestens zwei Reads wählen".to_string(),
            });
        }
        let docs = self.sequence_docs(&ids)?;
        let kmers = |s: &str| -> std::collections::BTreeSet<String> {
            s.as_bytes()
                .windows(8)
                .map(|w| String::from_utf8_lossy(w).to_string())
                .collect()
        };
        let first = docs[0].bases().to_ascii_uppercase();
        let first_k = kmers(&first);
        let mut seqs = Vec::new();
        let mut quals: Vec<Vec<u8>> = Vec::new();
        for (i, d) in docs.iter().enumerate() {
            let bases = d.bases().to_ascii_uppercase();
            let mut q = self.read_qualities(&ids[i], bases.len());
            let mut use_bases = bases.clone();
            if i > 0 {
                let rc = iupac_reverse_complement(&bases);
                let fwd = kmers(&bases).intersection(&first_k).count();
                let rev = kmers(&rc).intersection(&first_k).count();
                if rev > fwd {
                    use_bases = rc;
                    q.reverse();
                }
            }
            seqs.push(Sequence::new(
                &new_id(),
                d.name(),
                &use_bases,
                d.sequence().seq_type(),
            )?);
            quals.push(q);
        }
        let rows_named = align_with(AlignBackend::Builtin, &seqs)?;
        let rows: Vec<String> = rows_named.iter().map(|r| r.1.clone()).collect();
        let names: Vec<String> = rows_named.iter().map(|r| r.0.clone()).collect();
        // Qualitäten in der Reihenfolge der Zeilen (align_with behält die Reihenfolge der Eingabe)
        let cons = quality_consensus(&rows, &quals, min_quality);
        if cons.bases.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Aus den Reads ließ sich kein Konsens bilden".to_string(),
            });
        }
        let aln = AlignmentDocument::of(
            &format!("Reads ({} Sequenzierungen)", ids.len()),
            docs[0].sequence().seq_type(),
            names,
            rows,
        )?;
        self.save_new(
            Document::Alignment(aln),
            "sequence.read_alignment",
            ids.clone(),
            "Reads ausgerichtet".to_string(),
        )?;
        let title = if name.trim().is_empty() {
            format!("Konsens ({} Reads)", ids.len())
        } else {
            name.trim().to_string()
        };
        let seq = Sequence::new(
            &new_id(),
            &title,
            &cons.bases,
            docs[0].sequence().seq_type(),
        )?;
        let mean_q =
            cons.qualities.iter().map(|q| *q as f64).sum::<f64>() / cons.qualities.len() as f64;
        self.save_new(
            Document::Sequence(SequenceDocument::of(&title, seq)),
            "sequence.read_consensus",
            ids,
            format!("Konsens aus Reads, mittlere Qualität {:.0}", mean_q),
        )
    }

    /// Legt Sequenzen aus eingefügtem Text an (roher Text, FASTA oder mit Zeilennummern). `kind` ist `DNA`, `RNA`,
    /// `PROTEIN` oder `AUTO` (Erkennung). Ein leerer `name` nimmt den Namen aus der FASTA-Kopfzeile; bei mehreren Einträgen
    /// wird bei gegebenem Namen durchnummeriert. `circular` gilt nur für Nukleotide.
    pub fn create_sequence(
        &self,
        name: String,
        text: String,
        kind: String,
        circular: bool,
    ) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let records = parse_pasted(&text);
        if records.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Der Text enthält keine Sequenz".to_string(),
            });
        }
        let many = records.len() > 1;
        let mut docs = Vec::with_capacity(records.len());
        for (i, r) in records.iter().enumerate() {
            let seq_type = match kind.to_uppercase().as_str() {
                "DNA" => SequenceType::Dna,
                "RNA" => SequenceType::Rna,
                "PROTEIN" => SequenceType::Protein,
                _ => match detect_kind(&r.residues) {
                    Kind::Dna => SequenceType::Dna,
                    Kind::Rna => SequenceType::Rna,
                    Kind::Protein => SequenceType::Protein,
                },
            };
            let doc_name = if !name.trim().is_empty() {
                if many {
                    format!("{} {}", name.trim(), i + 1)
                } else {
                    name.trim().to_string()
                }
            } else if !r.name.is_empty() {
                r.name.clone()
            } else {
                "Neue Sequenz".to_string()
            };
            let seq = Sequence::new(&new_id(), &doc_name, &r.residues, seq_type)?;
            let mut doc = SequenceDocument::of(&doc_name, seq);
            if circular && seq_type != SequenceType::Protein {
                doc = doc.with_topology(Topology::Circular);
            }
            docs.push(doc);
        }
        let mut store = self.lock()?;
        let mut infos = Vec::with_capacity(docs.len());
        let mut ids = Vec::with_capacity(docs.len());
        for d in docs {
            let doc = Document::Sequence(d);
            store.save(&doc)?;
            ids.push(doc.id().to_string());
            infos.push(DocumentInfoDto {
                id: doc.id().to_string(),
                name: doc.name().to_string(),
                doc_type: doc.doc_type().to_string(),
                created: doc.created(),
            });
        }
        store.add_history(&HistoryEntry {
            id: new_id(),
            operation_id: "sequence.create".to_string(),
            parameters: BTreeMap::new(),
            input_ids: Vec::new(),
            output_ids: ids,
            time: now_secs(),
            summary: format!("{} Sequenz(en) angelegt", infos.len()),
        })?;
        Ok(infos)
    }

    /// Aminosäuren im Leserahmen `frame`: 1, 2, 3 auf dem Vorwärtsstrang, −1, −2, −3 auf dem Gegenstrang (0 gilt nicht).
    /// `base_position` ist immer die kleinste Koordinate des Codons im Dokument (auch auf dem Gegenstrang).
    pub fn translate_frame(&self, id: String, frame: i32) -> Result<Vec<AminoAcidDto>, ApiError> {
        if frame == 0 || frame.abs() > 3 {
            return Err(ApiError::InvalidInput {
                message: "Leserahmen muss 1–3 oder −1 bis −3 sein".to_string(),
            });
        }
        let d = self.sequence_doc(&id)?;
        let bases = d.bases();
        let len = bases.len();
        let offset = (frame.unsigned_abs() as usize) - 1;
        let cells = if frame > 0 {
            translate_frame_core(bases, offset, 0, len)
        } else {
            let rc = iupac_reverse_complement(bases);
            translate_frame_core(&rc, offset, 0, len)
        };
        Ok(cells
            .into_iter()
            .map(|c| AminoAcidDto {
                base_position: if frame > 0 {
                    c.base_position as u64
                } else {
                    (len - c.base_position - 3) as u64
                },
                amino_acid: c.amino_acid.to_string(),
                is_start: c.is_start,
                is_stop: c.is_stop,
            })
            .collect())
    }

    /// Importiert eine Datei und legt die Dokumente an: FASTA, FASTQ, GenBank, EMBL, SnapGene, PIR, GCG (Sequenzen) sowie
    /// MEGA, NEXUS, Clustal und PHYLIP (Alignments). Liefert die neu angelegten Dokumente.
    pub fn import_file(&self, path: String) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let docs = io::read_documents(Path::new(&path))?;
        if docs.is_empty() {
            return Err(ApiError::InvalidInput {
                message: format!("Die Datei '{}' enthält keine Sequenzen", path),
            });
        }
        let mut store = self.lock()?;
        let mut infos = Vec::with_capacity(docs.len());
        let mut ids = Vec::with_capacity(docs.len());
        for doc in docs {
            store.save(&doc)?;
            ids.push(doc.id().to_string());
            infos.push(DocumentInfoDto {
                id: doc.id().to_string(),
                name: doc.name().to_string(),
                doc_type: doc.doc_type().to_string(),
                created: doc.created(),
            });
        }
        // Sanger-Sequenzierung (.ab1, .scf): die Originaldatei wird im Ordner `<Projekt>.dateien` abgelegt und mit der Sequenz verknüpft
        let trace_kind = std::fs::read(&path).ok().and_then(|b| {
            if b.starts_with(b"ABIF") {
                Some("ab1")
            } else if b.starts_with(b".scf") {
                Some("scf")
            } else {
                None
            }
        });
        if let Some(kind) = trace_kind {
            let project_path = std::path::PathBuf::from(&self.path);
            let dir = project_path.with_extension("dateien");
            std::fs::create_dir_all(&dir).map_err(|e| ApiError::Io {
                message: format!("{}: {}", dir.display(), e),
            })?;
            let file_name = Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "sequenzierung.ab1".to_string());
            let mut target = dir.join(&file_name);
            let mut n = 1;
            while target.exists() {
                let stem = Path::new(&file_name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let ext = Path::new(&file_name)
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
                    .unwrap_or_default();
                target = dir.join(format!("{stem}-{n}{ext}"));
                n += 1;
            }
            std::fs::copy(&path, &target).map_err(|e| ApiError::Io {
                message: format!("{}: {}", target.display(), e),
            })?;
            for info in &infos {
                store.add_external_file(
                    &to_portable(&project_path, &target),
                    kind,
                    &info.id,
                    &file_name,
                )?;
            }
        }
        store.add_history(&HistoryEntry {
            id: new_id(),
            operation_id: "import.file".to_string(),
            parameters: [("path".to_string(), path)].into_iter().collect(),
            input_ids: Vec::new(),
            output_ids: ids,
            time: now_secs(),
            summary: format!("{} Dokument(e) importiert", infos.len()),
        })?;
        Ok(infos)
    }

    /// Legt Sequenzen aus einer Tabelle (CSV/TSV) an. `sep` ist `,`, `;` oder `\t`; `name_col` und `seq_col` zählen ab 0.
    #[allow(clippy::too_many_arguments)]
    pub fn import_table(
        &self,
        path: String,
        sep: String,
        name_col: u64,
        seq_col: u64,
        has_header: bool,
        kind: String,
        circular: bool,
    ) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
            message: format!("{}: {}", path, e),
        })?;
        let rows = parse_delimited(&text, sep.chars().next().unwrap_or(','));
        let seqs = sequences_from_table(&rows, name_col as usize, seq_col as usize, has_header);
        if seqs.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "In den gewählten Spalten steht keine Sequenz".to_string(),
            });
        }
        let fasta: String = seqs
            .iter()
            .map(|(n, s)| format!(">{}\n{}\n", n.replace(['\n', '\r'], " "), s))
            .collect();
        self.create_sequence(String::new(), fasta, kind, circular)
    }

    /// Länge der gewählten Sequenzen als Häufigkeitsverteilung (Klassenbreite `bin_width`, 0 gilt als 1).
    pub fn length_histogram(
        &self,
        ids: Vec<String>,
        bin_width: u64,
    ) -> Result<Vec<HistogramBinDto>, ApiError> {
        let lens: Vec<usize> = self.sequence_docs(&ids)?.iter().map(|d| d.len()).collect();
        Ok(core_histogram(&lens, bin_width as usize)
            .into_iter()
            .map(|(start, count)| HistogramBinDto {
                start: start as u64,
                count: count as u64,
            })
            .collect())
    }

    /// Ordnet die gewählten Reads anhand der Namen (`/1`, `/2` oder `1:`, `2:`) zu Paaren.
    pub fn pair_reads_report(&self, ids: Vec<String>) -> Result<PairReportDto, ApiError> {
        let docs = self.sequence_docs(&ids)?;
        let names: Vec<String> = docs.iter().map(|d| d.name().to_string()).collect();
        let r = core_pair_reads(&names);
        Ok(PairReportDto {
            pairs: r
                .pairs
                .iter()
                .map(|(a, b)| vec![names[*a].clone(), names[*b].clone()])
                .collect(),
            unpaired: r.unpaired.iter().map(|i| names[*i].clone()).collect(),
        })
    }

    /// Fragmente, wenn die Sequenz an den Anfängen und/oder Enden aller Features des Typs `feature_type` (leer = alle Typen)
    /// geschnitten wird. Bei zirkulären Dokumenten umläuft das letzte Fragment den Nullpunkt.
    pub fn fragments_by_annotation(
        &self,
        id: String,
        feature_type: String,
        at_start: bool,
        at_end: bool,
    ) -> Result<Vec<FragmentDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let len = d.len();
        let spans: Vec<(usize, usize)> = d
            .features()
            .iter()
            .filter(|f| {
                feature_type.is_empty() || f.feature_type().eq_ignore_ascii_case(&feature_type)
            })
            .map(|f| (f.location().start(), f.location().end()))
            .collect();
        let cuts = cut_positions_from_spans(&spans, at_start, at_end);
        let circular = d.topology() == Topology::Circular;
        Ok(fragments_from_cuts(len, &cuts, circular)
            .into_iter()
            .map(|(s, e)| FragmentDto {
                length: if e > s || (!circular) {
                    (e - s) as u64
                } else {
                    (len - s + e) as u64
                },
                start: s as u64,
                end: e as u64,
                left_end: "Annotation".to_string(),
                right_end: "Annotation".to_string(),
            })
            .collect())
    }

    /// Eckdaten eines Dokuments.
    pub fn summary(&self, id: String) -> Result<DocumentSummary, ApiError> {
        let d = self.sequence_doc(&id)?;
        let meta = |k: &str| d.metadata().get(k).cloned().unwrap_or_default();
        Ok(DocumentSummary {
            id: d.id().to_string(),
            name: d.name().to_string(),
            doc_type: "sequence".to_string(),
            length: d.len() as u64,
            topology: match d.topology() {
                Topology::Linear => "LINEAR",
                Topology::Circular => "CIRCULAR",
            }
            .to_string(),
            seq_type: format!("{:?}", d.sequence().seq_type()).to_uppercase(),
            feature_count: d.features().len() as u64,
            gc_percent: d.sequence().gc_content() * 100.0,
            description: d.sequence().description().to_string(),
            accession: meta("accession"),
            organism: meta("organism"),
        })
    }

    /// Alle Features eines Dokuments.
    pub fn features(&self, id: String) -> Result<Vec<FeatureInfo>, ApiError> {
        let d = self.sequence_doc(&id)?;
        Ok(d.features()
            .iter()
            .map(|f| FeatureInfo {
                feature_type: f.feature_type().to_string(),
                name: f.name().to_string(),
                start: f.location().start() as u64,
                end: f.location().end() as u64,
                strand: match f.location().strand() {
                    Strand::Forward => "FORWARD",
                    Strand::Reverse => "REVERSE",
                    Strand::None => "NONE",
                }
                .to_string(),
            })
            .collect())
    }

    /// Basen `[start, end)`; ein Ende hinter der Sequenz wird auf die Länge begrenzt.
    pub fn bases(&self, id: String, start: u64, end: u64) -> Result<String, ApiError> {
        let d = self.sequence_doc(&id)?;
        let (s, e) = (start as usize, (end as usize).min(d.len()));
        if s > e {
            return Err(ApiError::InvalidInput {
                message: format!("Ungültiger Bereich {}-{}", start, end),
            });
        }
        Ok(d.bases()[s..e].to_string())
    }

    /// Teilstücke aller Features mit Spurnummer.
    pub fn feature_lanes(&self, id: String) -> Result<Vec<PlacedFeatureDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let features = d.features();
        Ok(assign_lanes(features)
            .into_iter()
            .filter_map(|p| {
                let f = features.get(p.feature_index)?;
                Some(PlacedFeatureDto {
                    feature_index: p.feature_index as u64,
                    feature_type: f.feature_type().to_string(),
                    name: f.name().to_string(),
                    start: p.segment.start() as u64,
                    end: p.segment.end() as u64,
                    strand: strand_text(f.location().strand()),
                    lane: p.lane as u32,
                })
            })
            .collect())
    }

    /// Kennzahlen der Auswahl `[start, end)`.
    pub fn selection_stats(
        &self,
        id: String,
        start: u64,
        end: u64,
    ) -> Result<SelectionStatsDto, ApiError> {
        let b = self.bases(id, start, end)?;
        let s = selection_stats(&b);
        Ok(SelectionStatsDto {
            length: s.length as u64,
            gc_percent: s.gc_percent,
            tm_celsius: s.tm_celsius,
        })
    }

    /// Text der Auswahl für die Zwischenablage: `format` ist `bases`, `fasta` oder `reverse_complement`.
    pub fn selection_text(
        &self,
        id: String,
        start: u64,
        end: u64,
        format: String,
    ) -> Result<String, ApiError> {
        let name = self.sequence_doc(&id)?.name().to_string();
        let b = self.bases(id, start, end)?;
        match format.as_str() {
            "bases" => Ok(b),
            "fasta" => Ok(as_fasta(&name, &b)),
            "reverse_complement" => Ok(as_reverse_complement(&b)),
            other => Err(ApiError::InvalidInput {
                message: format!("Unbekanntes Textformat '{}'", other),
            }),
        }
    }

    /// Enzyme der Standarddatenbank; `filter` ist `all`, `unique`, `double` oder `none`.
    pub fn enzyme_sites(
        &self,
        id: String,
        filter: String,
    ) -> Result<Vec<EnzymeEntryDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let f = match filter.as_str() {
            "all" => EnzymeFilter::All,
            "unique" => EnzymeFilter::UniqueCutters,
            "double" => EnzymeFilter::DoubleCutters,
            "none" => EnzymeFilter::NonCutters,
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannter Enzymfilter '{}'", other),
                })
            }
        };
        let model = EnzymeMapModel::new(&EnzymeDb::standard(), &d, f);
        Ok(model
            .entries()
            .iter()
            .map(|e| EnzymeEntryDto {
                enzyme: e.enzyme.clone(),
                count: e.count as u64,
                positions: e.positions.iter().map(|p| *p as u64).collect(),
            })
            .collect())
    }

    /// Aminosäuren eines Features (z. B. CDS) in Leserichtung des Features.
    pub fn translate_feature(
        &self,
        id: String,
        feature_index: u64,
    ) -> Result<Vec<AminoAcidDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let f = d
            .features()
            .get(feature_index as usize)
            .ok_or_else(|| ApiError::InvalidInput {
                message: format!("Feature {} gibt es nicht", feature_index),
            })?;
        let cells = translate_feature(d.bases(), f)?;
        Ok(cells
            .into_iter()
            .map(|c| AminoAcidDto {
                base_position: c.base_position as u64,
                amino_acid: c.amino_acid.to_string(),
                is_start: c.is_start,
                is_stop: c.is_stop,
            })
            .collect())
    }

    /// Alle Enzyme der Standarddatenbank.
    pub fn enzyme_list(&self) -> Vec<EnzymeInfoDto> {
        EnzymeDb::standard()
            .all()
            .iter()
            .map(|e| EnzymeInfoDto {
                name: e.name().to_string(),
                recognition: e.recognition().to_string(),
                sticky: e.is_sticky(),
            })
            .collect()
    }

    /// Virtueller Verdau mit den genannten Enzymen.
    pub fn digest_document(
        &self,
        id: String,
        enzymes: Vec<String>,
    ) -> Result<Vec<FragmentDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let names: Vec<&str> = enzymes.iter().map(String::as_str).collect();
        let fragments = digest(&d, &EnzymeDb::standard(), &names)?;
        Ok(fragments
            .iter()
            .map(|f| FragmentDto {
                length: f.length() as u64,
                start: f.source_start as u64,
                end: f.source_end as u64,
                left_end: end_text(&f.left),
                right_end: end_text(&f.right),
            })
            .collect())
    }

    /// gRNA-Stellen auf beiden Strängen; `nuclease` ist `spcas9`, `sacas9` oder `cas12a`.
    pub fn find_guides(&self, id: String, nuclease: String) -> Result<Vec<GuideDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let spec = match nuclease.as_str() {
            "spcas9" => PamSpec::sp_cas9(),
            "sacas9" => PamSpec::sa_cas9(),
            "cas12a" => PamSpec::cas12a(),
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannte Nuklease '{}'", other),
                })
            }
        };
        let circular = d.topology() == Topology::Circular;
        Ok(find_guides(&spec, d.bases(), 0, d.len(), circular)
            .into_iter()
            .map(|g| GuideDto {
                protospacer: g.protospacer,
                pam: g.pam,
                start: g.start as u64,
                end: g.end as u64,
                forward: g.forward,
                gc_percent: g.gc_percent,
                score: g.score,
                warnings: g.warnings,
            })
            .collect())
    }

    /// Mögliche Produkte der Restriktionsklonierung von Insert in Vektor.
    pub fn restriction_candidates(
        &self,
        vector_id: String,
        vector_enzymes: Vec<String>,
        insert_id: String,
        insert_enzymes: Vec<String>,
    ) -> Result<Vec<CandidateDto>, ApiError> {
        Ok(self
            .candidates_for(&vector_id, &vector_enzymes, &insert_id, &insert_enzymes)?
            .iter()
            .map(|c| CandidateDto {
                construct_length: c.construct.len() as u64,
                directional: c.directional,
                insert_reversed: c.insert_reversed,
                vector_length: c.vector.length() as u64,
                insert_length: c.insert.length() as u64,
                warnings: c.warnings.clone(),
            })
            .collect())
    }

    /// Speichert das Produkt mit Nummer `index` (aus `restriction_candidates`) als neues Dokument.
    pub fn save_candidate(
        &self,
        vector_id: String,
        vector_enzymes: Vec<String>,
        insert_id: String,
        insert_enzymes: Vec<String>,
        index: u64,
        name: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        let list = self.candidates_for(&vector_id, &vector_enzymes, &insert_id, &insert_enzymes)?;
        let c = list
            .get(index as usize)
            .ok_or_else(|| ApiError::InvalidInput {
                message: format!("Produkt {} gibt es nicht", index),
            })?;
        let doc = if name.trim().is_empty() {
            c.construct.clone()
        } else {
            c.construct.with_name(name.trim())
        };
        let document = Document::Sequence(doc);
        let mut store = self.lock()?;
        store.save(&document)?;
        store.add_history(&HistoryEntry {
            id: new_id(),
            operation_id: "clone.restriction".to_string(),
            parameters: [
                ("vector_enzymes".to_string(), vector_enzymes.join(",")),
                ("insert_enzymes".to_string(), insert_enzymes.join(",")),
                ("index".to_string(), index.to_string()),
            ]
            .into_iter()
            .collect(),
            input_ids: vec![vector_id, insert_id],
            output_ids: vec![document.id().to_string()],
            time: now_secs(),
            summary: "Restriktionsklonierung".to_string(),
        })?;
        Ok(DocumentInfoDto {
            id: document.id().to_string(),
            name: document.name().to_string(),
            doc_type: document.doc_type().to_string(),
            created: document.created(),
        })
    }

    /// Bis zu `max_results` Primerpaare, deren Produkt `[target_start, target_end)` einschließt (Standardvorgaben).
    pub fn pick_primers(
        &self,
        id: String,
        target_start: u64,
        target_end: u64,
        max_results: u64,
    ) -> Result<Vec<PrimerPairDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let pairs = pick_pairs(
            d.bases(),
            target_start as usize,
            target_end as usize,
            &PrimerConstraints::standard(),
            max_results as usize,
        )?;
        Ok(pairs
            .into_iter()
            .map(|p| PrimerPairDto {
                forward_sequence: p.forward.sequence,
                forward_start: p.forward.start as u64,
                forward_end: p.forward.end as u64,
                forward_tm: p.forward.tm,
                reverse_sequence: p.reverse.sequence,
                reverse_start: p.reverse.start as u64,
                reverse_end: p.reverse.end as u64,
                reverse_tm: p.reverse.tm,
                product_size: p.product_size as u64,
                penalty: p.penalty,
            })
            .collect())
    }

    /// Fügt Basen vor Position `position` ein (Rückgängig möglich).
    pub fn edit_insert(&self, id: String, position: u64, bases: String) -> Result<(), ApiError> {
        self.edit(&id, |s| s.insert(position as usize, &bases))
    }

    /// Löscht `[start, end)` (Rückgängig möglich).
    pub fn edit_delete(&self, id: String, start: u64, end: u64) -> Result<(), ApiError> {
        self.edit(&id, |s| s.delete(start as usize, end as usize))
    }

    /// Ersetzt `[start, end)` durch `bases` (Rückgängig möglich).
    pub fn edit_replace(
        &self,
        id: String,
        start: u64,
        end: u64,
        bases: String,
    ) -> Result<(), ApiError> {
        self.edit(&id, |s| s.replace(start as usize, end as usize, &bases))
    }

    /// Macht die letzte Änderung rückgängig; `false`, wenn nichts zurückzunehmen ist.
    pub fn edit_undo(&self, id: String) -> Result<bool, ApiError> {
        let mut did = false;
        self.edit(&id, |s| {
            did = s.undo();
            Ok(())
        })?;
        Ok(did)
    }

    /// Stellt die zuletzt zurückgenommene Änderung wieder her.
    pub fn edit_redo(&self, id: String) -> Result<bool, ApiError> {
        let mut did = false;
        self.edit(&id, |s| {
            did = s.redo();
            Ok(())
        })?;
        Ok(did)
    }

    /// Zahl der möglichen Rückgängig- und Wiederholen-Schritte `(undo, redo)`.
    pub fn edit_state(&self, id: String) -> Result<Vec<u64>, ApiError> {
        let sessions = self.sessions.lock().map_err(|_| ApiError::Internal {
            message: "Bearbeitungssitzungen sind gesperrt (vorheriger Fehler)".to_string(),
        })?;
        Ok(match sessions.get(&id) {
            Some(s) => vec![s.undo_depth() as u64, u64::from(s.can_redo())],
            None => vec![0, 0],
        })
    }

    /// Richtet mindestens zwei Sequenzdokumente aus und speichert das Alignment. `backend` ist `builtin`, `mafft`, `muscle` oder `clustalo`.
    pub fn align_documents(
        &self,
        ids: Vec<String>,
        name: String,
        backend: String,
    ) -> Result<DocumentInfoDto, ApiError> {
        if ids.len() < 2 {
            return Err(ApiError::InvalidInput {
                message: "Für ein Alignment sind mindestens zwei Sequenzen nötig".to_string(),
            });
        }
        let engine = AlignBackend::from_name(&backend).ok_or_else(|| ApiError::InvalidInput {
            message: format!("Unbekanntes Alignment-Verfahren '{}'", backend),
        })?;
        let mut seqs = Vec::new();
        for id in &ids {
            seqs.push(self.sequence_doc(id)?.sequence().clone());
        }
        let seq_type = seqs
            .first()
            .map(|s| s.seq_type())
            .unwrap_or(snapious_core::model::SequenceType::Dna);
        let aligned = align_with(engine, &seqs)?;
        let names: Vec<String> = aligned.iter().map(|r| r.0.clone()).collect();
        let rows: Vec<String> = aligned.iter().map(|r| r.1.clone()).collect();
        let title = if name.trim().is_empty() {
            "Alignment".to_string()
        } else {
            name.trim().to_string()
        };
        let doc = AlignmentDocument::of(&title, seq_type, names, rows)?;
        let n = ids.len();
        self.save_new(
            Document::Alignment(doc),
            "align.multiple",
            ids,
            format!("{} Sequenzen ausgerichtet ({})", n, backend),
        )
    }

    /// Alignment zur Anzeige.
    pub fn alignment(&self, id: String) -> Result<AlignmentDto, ApiError> {
        let d = self.alignment_doc(&id)?;
        Ok(AlignmentDto {
            id: d.id().to_string(),
            name: d.name().to_string(),
            seq_type: format!("{:?}", d.seq_type()).to_uppercase(),
            row_names: d.row_names().to_vec(),
            rows: d.rows().to_vec(),
            consensus: consensus(d.rows(), 0.5, true)?,
            identity: column_identity(d.rows())?,
        })
    }

    /// Gibson-/Überlappungs-Zusammenbau der Sequenzdokumente in der gegebenen Reihenfolge.
    /// `method` ist `gibson`, `infusion` oder `slic`.
    pub fn assemble_overlap(
        &self,
        ids: Vec<String>,
        method: String,
        circular: bool,
    ) -> Result<AssemblyDto, ApiError> {
        let docs = self.sequence_docs(&ids)?;
        let assembly = match method.as_str() {
            "gibson" => OverlapAssembly::gibson(),
            "infusion" => OverlapAssembly::in_fusion(),
            "slic" => OverlapAssembly::slic(),
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannte Methode '{}'", other),
                })
            }
        };
        let result = assembly.assemble(&docs, circular)?;
        let info = self.save_new(
            Document::Sequence(result.construct),
            "clone.overlap",
            ids,
            format!("Zusammenbau ({})", method),
        )?;
        Ok(AssemblyDto {
            document: info,
            warnings: result.warnings,
        })
    }

    /// Golden-Gate-Zusammenbau mit dem Enzym `enzyme` (z. B. `BsaI`).
    pub fn assemble_golden_gate(
        &self,
        ids: Vec<String>,
        enzyme: String,
        circular: bool,
    ) -> Result<AssemblyDto, ApiError> {
        let docs = self.sequence_docs(&ids)?;
        let result = golden_gate::assemble(&EnzymeDb::standard(), &enzyme, &docs, circular)?;
        let info = self.save_new(
            Document::Sequence(result.construct),
            "clone.goldengate",
            ids,
            format!("Golden Gate ({})", enzyme),
        )?;
        Ok(AssemblyDto {
            document: info,
            warnings: result.warnings,
        })
    }

    /// Offene Leserahmen (ORFs) mit mindestens `min_codons` Codons auf beiden Strängen (noch nicht ins Dokument eingetragen).
    pub fn find_orfs(&self, id: String, min_codons: u64) -> Result<Vec<FeatureInfo>, ApiError> {
        let d = self.sequence_doc(&id)?;
        Ok(Self::feature_infos(find_orf_features(
            d.bases(),
            min_codons as usize,
        )?))
    }

    /// Bekannte Sequenzmotive (Promotoren, Primerstellen, Tags …) auf beiden Strängen (noch nicht ins Dokument eingetragen).
    pub fn detect_common_features(&self, id: String) -> Result<Vec<FeatureInfo>, ApiError> {
        let d = self.sequence_doc(&id)?;
        Ok(Self::feature_infos(detect_features(
            d.bases(),
            &common_templates(),
        )?))
    }

    /// Trägt Features ins Dokument ein (als ein Rückgängig-Schritt je Feature).
    pub fn add_features(&self, id: String, features: Vec<FeatureInfo>) -> Result<(), ApiError> {
        for f in features {
            let strand = match f.strand.as_str() {
                "FORWARD" => Strand::Forward,
                "REVERSE" => Strand::Reverse,
                _ => Strand::None,
            };
            let location = Location::new(
                vec![Interval::new(f.start as usize, f.end as usize)?],
                strand,
                false,
                false,
            )?;
            let feature =
                snapious_core::model::Feature::new(&f.feature_type, &f.name, location, Vec::new());
            self.edit(&id, |s| s.add_feature(feature))?;
        }
        Ok(())
    }

    /// Entfernt das Feature mit diesem Index (Rückgängig möglich).
    pub fn remove_feature(&self, id: String, index: u64) -> Result<(), ApiError> {
        self.edit(&id, |s| s.remove_feature(index as usize))
    }

    /// Sucht in NCBI Nucleotide (`nuccore`) und liefert Kurzinfos (braucht Internet und `curl`).
    pub fn ncbi_search(&self, term: String, max_results: u64) -> Result<NcbiSearchDto, ApiError> {
        let client = NcbiClient::new(cached_http());
        let (total, records) = client.search("nuccore", &term, max_results as usize)?;
        Ok(NcbiSearchDto {
            total: total as u64,
            records: records
                .into_iter()
                .map(|r| NcbiRecordDto {
                    id: r.id,
                    accession: r.accession,
                    title: r.title,
                    organism: r.organism,
                    length: r.length.unwrap_or(0),
                })
                .collect(),
        })
    }

    /// Lädt einen NCBI-Datensatz (Kennung aus `ncbi_search`) als GenBank und legt ihn im Projekt an.
    pub fn ncbi_import(&self, id: String) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let text = NcbiClient::new(cached_http()).fetch_genbank(&id)?;
        let docs = io::genbank::parse(&text)?;
        if docs.is_empty() {
            return Err(ApiError::InvalidInput {
                message: format!("NCBI lieferte keine Sequenz für {}", id),
            });
        }
        let mut store = self.lock()?;
        let mut infos = Vec::new();
        let mut ids = Vec::new();
        for d in docs {
            let doc = Document::Sequence(d);
            store.save(&doc)?;
            ids.push(doc.id().to_string());
            infos.push(DocumentInfoDto {
                id: doc.id().to_string(),
                name: doc.name().to_string(),
                doc_type: doc.doc_type().to_string(),
                created: doc.created(),
            });
        }
        store.add_history(&HistoryEntry {
            id: new_id(),
            operation_id: "import.ncbi".to_string(),
            parameters: [("id".to_string(), id)].into_iter().collect(),
            input_ids: Vec::new(),
            output_ids: ids,
            time: now_secs(),
            summary: format!("{} Sequenz(en) von NCBI", infos.len()),
        })?;
        Ok(infos)
    }

    /// Das Protein, das ein Feature (z. B. CDS) codiert; ein abschließendes Stopp-Zeichen wird weggelassen.
    pub fn protein_of_feature(&self, id: String, feature_index: u64) -> Result<String, ApiError> {
        let cells = self.translate_feature(id, feature_index)?;
        let mut protein: String = cells.iter().map(|c| c.amino_acid.clone()).collect();
        if protein.ends_with('*') {
            protein.pop();
        }
        Ok(protein)
    }

    /// Eigenschaften einer Proteinsequenz.
    pub fn protein_properties(&self, protein: String) -> ProteinPropsDto {
        ProteinPropsDto {
            length: protein.chars().filter(|c| c.is_ascii_alphabetic()).count() as u64,
            molecular_weight: molecular_weight(&protein),
            isoelectric_point: isoelectric_point(&protein),
            charge_ph7: charge_at_ph(&protein, 7.0),
            extinction_280: extinction_coefficient_280(&protein, false),
            aromaticity: aromaticity(&protein),
            aliphatic_index: aliphatic_index(&protein),
            gravy: gravy(&protein),
        }
    }

    /// Verdau eines Proteins; `protease` ist `trypsin`, `lysc`, `argc`, `gluc`, `chymotrypsin` oder `cnbr`.
    pub fn protein_digest(
        &self,
        protein: String,
        protease: String,
        missed_cleavages: u64,
    ) -> Result<Vec<PeptideDto>, ApiError> {
        let p = match protease.as_str() {
            "trypsin" => Protease::Trypsin,
            "lysc" => Protease::LysC,
            "argc" => Protease::ArgC,
            "gluc" => Protease::GluC,
            "chymotrypsin" => Protease::Chymotrypsin,
            "cnbr" => Protease::CyanogenBromide,
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannte Protease '{}'", other),
                })
            }
        };
        Ok(digest_protein(&protein, p, missed_cleavages as usize, 1)?
            .into_iter()
            .map(|x| PeptideDto {
                sequence: x.sequence,
                start: x.start as u64,
                end: x.end as u64,
                missed_cleavages: x.missed_cleavages as u64,
                mass: x.mass,
            })
            .collect())
    }

    /// GC-Verlauf in Prozent mit Fenster `window` und Schritt `step`.
    pub fn gc_profile(
        &self,
        id: String,
        window: u64,
        step: u64,
    ) -> Result<Vec<GcPointDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        Ok(gc_windows(d.bases(), window as usize, step as usize)?
            .into_iter()
            .map(|(p, v)| GcPointDto {
                position: p as u64,
                percent: v,
            })
            .collect())
    }

    /// CpG-Inseln nach Gardiner-Garden/Frommer (Fenster 200, GC ≥ 50 %, beobachtet/erwartet ≥ 0,6).
    pub fn cpg_island_regions(&self, id: String) -> Result<Vec<RegionDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        Ok(cpg_islands(d.bases(), 200, 0.5, 0.6)?
            .into_iter()
            .map(|r| RegionDto {
                start: r.start as u64,
                end: r.end as u64,
            })
            .collect())
    }

    /// Schreibt ein Dokument in eine Datei. `format`: Sequenz `genbank`, `fasta`, `gff3` oder `svg` (Karte); Alignment `clustal`, `phylip` oder `fasta`;
    /// Baum `newick` oder `svg`.
    pub fn export_document(
        &self,
        id: String,
        path: String,
        format: String,
    ) -> Result<(), ApiError> {
        let doc = self.lock()?.load(&id)?.ok_or_else(|| ApiError::NotFound {
            message: format!("Dokument '{}' nicht gefunden", id),
        })?;
        let bad = |what: &str| ApiError::InvalidInput {
            message: format!("{} kann nicht als '{}' gespeichert werden", what, format),
        };
        let text = match (&doc, format.as_str()) {
            (Document::Sequence(d), "genbank") => io::genbank::format(d),
            (Document::Sequence(d), "embl") => io::embl::format(d),
            (Document::Sequence(d), "gff3") => write_gff3(d),
            (Document::Sequence(d), "svg") => {
                // Einzelschneider als Enzymstellen
                let model =
                    EnzymeMapModel::new(&EnzymeDb::standard(), d, EnzymeFilter::UniqueCutters);
                let mut sites: Vec<(String, usize)> = model
                    .entries()
                    .iter()
                    .flat_map(|e| e.positions.iter().map(|p| (e.enzyme.clone(), *p)))
                    .collect();
                sites.sort_by_key(|s| s.1);
                if d.topology() == Topology::Circular {
                    circular_map_svg(d, &sites, 800.0)
                } else {
                    linear_map_svg(d, &sites, 1000.0, 260.0)
                }
            }
            (Document::Sequence(d), "fasta") => {
                io::fasta::format(std::slice::from_ref(d.sequence()))
            }
            (Document::Alignment(d), "clustal") => {
                let rows: Vec<(String, String)> = d
                    .row_names()
                    .iter()
                    .cloned()
                    .zip(d.rows().iter().cloned())
                    .collect();
                write_clustal(&rows)?
            }
            (Document::Alignment(d), "mega") => {
                let rows: Vec<(String, String)> = d
                    .row_names()
                    .iter()
                    .cloned()
                    .zip(d.rows().iter().cloned())
                    .collect();
                io::mega::write_mega(&rows)
            }
            (Document::Alignment(d), "nexus") => {
                let rows: Vec<(String, String)> = d
                    .row_names()
                    .iter()
                    .cloned()
                    .zip(d.rows().iter().cloned())
                    .collect();
                io::nexus::write_nexus(&rows)?
            }
            (Document::Alignment(d), "phylip") => {
                let rows: Vec<(String, String)> = d
                    .row_names()
                    .iter()
                    .cloned()
                    .zip(d.rows().iter().cloned())
                    .collect();
                write_phylip(&rows)?
            }
            (Document::Alignment(d), "fasta") => {
                let mut out = String::new();
                for (n, r) in d.row_names().iter().zip(d.rows()) {
                    out.push_str(&format!(">{}\n{}\n", n, r));
                }
                out
            }
            (Document::Sequence(_), _) => return Err(bad("Die Sequenz")),
            (Document::Alignment(_), _) => return Err(bad("Das Alignment")),
        };
        std::fs::write(&path, text).map_err(|e| ApiError::Io {
            message: format!("Datei '{}' konnte nicht geschrieben werden: {}", path, e),
        })?;
        Ok(())
    }

    /// Gleicht eine Sequenzierdatei (`.ab1`) mit der Sequenz des Dokuments ab (Qualität unter 20 wird gemeldet).
    pub fn check_read_file(&self, id: String, path: String) -> Result<ReadCheckDto, ApiError> {
        let d = self.sequence_doc(&id)?;
        let trace = read_trace(path.clone())?;
        let name = std::path::Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Read".to_string());
        let r = SequencingCheck::default().check(
            d.bases(),
            d.topology() == Topology::Circular,
            &name,
            &trace.bases,
            &trace.quality,
        )?;
        Ok(ReadCheckDto {
            read_name: r.read_name,
            reversed: r.reversed,
            reference_start: r.reference_start as u64,
            reference_end: r.reference_end as u64,
            identity: r.identity,
            differences: r
                .differences
                .into_iter()
                .map(|x| DifferenceDto {
                    kind: match x.kind {
                        DifferenceType::Mismatch => "MISMATCH",
                        DifferenceType::Insertion => "INSERTION",
                        DifferenceType::Deletion => "DELETION",
                        DifferenceType::LowQuality => "LOW_QUALITY",
                    }
                    .to_string(),
                    reference_position: x.reference_position as u64,
                    read_position: x.read_position as u64,
                    expected: x.expected,
                    observed: x.observed,
                    quality: x.quality,
                })
                .collect(),
            aligned_read: r.aligned_read,
            aligned_reference: r.aligned_reference,
        })
    }

    /// Sucht in UniProt (braucht Internet und `curl`); die Treffer kommen im gleichen Format wie bei NCBI (`accession` = UniProt-Zugang,
    /// `id` = dieselbe Zugangsnummer, `organism`, `title` = Proteinname, `length` in Aminosäuren).
    pub fn uniprot_search(
        &self,
        query: String,
        max_results: u64,
    ) -> Result<NcbiSearchDto, ApiError> {
        let hits = UniprotClient::new(cached_http()).search(&query, max_results as usize)?;
        let records: Vec<NcbiRecordDto> = hits
            .into_iter()
            .map(|r| NcbiRecordDto {
                id: r.accession.clone(),
                accession: r.accession,
                title: r.name,
                organism: r.organism,
                length: r.length,
            })
            .collect();
        Ok(NcbiSearchDto {
            total: records.len() as u64,
            records,
        })
    }

    /// Lädt einen UniProt-Eintrag als Proteinsequenz und legt ihn im Projekt an.
    pub fn uniprot_import(&self, accession: String) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let json = cached_http().get(&entry_url(&accession))?;
        let entry = parse_entry(&json)?;
        let doc = Document::Sequence(entry.to_document()?);
        let mut store = self.lock()?;
        store.save(&doc)?;
        let ids = vec![doc.id().to_string()];
        let infos = vec![DocumentInfoDto {
            id: doc.id().to_string(),
            name: doc.name().to_string(),
            doc_type: doc.doc_type().to_string(),
            created: doc.created(),
        }];
        store.add_history(&HistoryEntry {
            id: new_id(),
            operation_id: "import.uniprot".to_string(),
            parameters: [("accession".to_string(), accession)].into_iter().collect(),
            input_ids: Vec::new(),
            output_ids: ids,
            time: now_secs(),
            summary: format!("{} Protein(e) von UniProt", infos.len()),
        })?;
        Ok(infos)
    }

    /// Lokale BLAST-Suche: die Sequenz `query_id` gegen die Sequenzen `subject_ids`; `program` ist `blastn` oder `blastp`.
    pub fn blast_search(
        &self,
        query_id: String,
        subject_ids: Vec<String>,
        program: String,
        max_hits: u64,
    ) -> Result<Vec<BlastHitDto>, ApiError> {
        let query = self.sequence_doc(&query_id)?.sequence().clone();
        let mut subjects = Vec::new();
        for id in &subject_ids {
            subjects.push(self.sequence_doc(id)?.sequence().clone());
        }
        Ok(blast_local(&program, &query, &subjects, max_hits as usize)?
            .into_iter()
            .map(|h| BlastHitDto {
                reverse_strand: h.reverse_strand(),
                subject: h.subject,
                identity_percent: h.identity_percent,
                alignment_length: h.alignment_length as u64,
                query_start: h.query_start as u64,
                query_end: h.query_end as u64,
                subject_start: h.subject_start as u64,
                subject_end: h.subject_end as u64,
                evalue: h.evalue,
                bit_score: h.bit_score,
            })
            .collect())
    }

    /// BLAST über NCBI (`blastn`/`blastp`/…, Datenbank z. B. `nt`, `nr`). Dauert meist Minuten; blockiert bis zum Ergebnis.
    pub fn blast_remote(
        &self,
        query_id: String,
        program: String,
        database: String,
    ) -> Result<Vec<BlastHitDto>, ApiError> {
        let query = self.sequence_doc(&query_id)?.bases().to_string();
        let hits = run_remote(&CurlTransport, &ThreadSleeper, &program, &database, &query)?;
        Ok(hits
            .into_iter()
            .map(|h| BlastHitDto {
                reverse_strand: h.reverse_strand(),
                subject: h.subject,
                identity_percent: h.identity_percent,
                alignment_length: h.alignment_length as u64,
                query_start: h.query_start as u64,
                query_end: h.query_end as u64,
                subject_start: h.subject_start as u64,
                subject_end: h.subject_end as u64,
                evalue: h.evalue,
                bit_score: h.bit_score,
            })
            .collect())
    }

    /// Erkennt Einzelbasenvarianten: Reads aus einer SAM- oder BAM-Datei gegen die gewählte Sequenz (Standardwerte).
    /// Schreibt die Varianten zusätzlich als VCF nach `vcf_path` (leer = nicht schreiben).
    pub fn call_variants(
        &self,
        reference_id: String,
        alignment_path: String,
        vcf_path: String,
    ) -> Result<Vec<SnvDto>, ApiError> {
        let d = self.sequence_doc(&reference_id)?;
        let records = if alignment_path.to_lowercase().ends_with(".bam") {
            read_bam(std::path::Path::new(&alignment_path))?.records
        } else {
            io::sam::read(std::path::Path::new(&alignment_path))?.records
        };
        // Referenzname: der erste Datensatz, der auf eine Referenz zeigt, sonst der Dokumentname
        let name = records
            .iter()
            .find(|r| r.reference != "*")
            .map(|r| r.reference.clone())
            .unwrap_or_else(|| d.name().to_string());
        let snvs = call_snvs(&name, d.bases(), &records, &CallOptions::standard())?;
        if !vcf_path.is_empty() {
            io::vcf::write(&to_vcf(&name, &snvs), std::path::Path::new(&vcf_path))?;
        }
        Ok(snvs
            .into_iter()
            .map(|s| SnvDto {
                position: s.position as u64,
                ref_base: s.ref_base.to_string(),
                alt_base: s.alt_base.to_string(),
                depth: s.depth as u64,
                alt_fraction: s.alt_fraction,
                quality: s.quality,
            })
            .collect())
    }

    /// Verknüpfte externe Dateien (leere `doc_id` = alle).
    pub fn external_files(&self, doc_id: String) -> Result<Vec<ExternalFileDto>, ApiError> {
        let files = self.lock()?.external_files(&doc_id)?;
        Ok(files
            .into_iter()
            .map(|mut f| {
                f.path = self.resolve_stored_path(&f.path);
                external_dto(f)
            })
            .collect())
    }

    /// Verknüpft eine Datei mit dem Projekt (Pfad wird nur vermerkt, die Datei nicht kopiert).
    pub fn add_external_file(
        &self,
        path: String,
        kind: String,
        doc_id: String,
        label: String,
    ) -> Result<ExternalFileDto, ApiError> {
        Ok(external_dto(
            self.lock()?
                .add_external_file(&path, &kind, &doc_id, &label)?,
        ))
    }

    /// Entfernt eine Verknüpfung (die Datei bleibt).
    pub fn remove_external_file(&self, id: String) -> Result<(), ApiError> {
        Ok(self.lock()?.remove_external_file(&id)?)
    }

    /// Bildet Reads (FASTQ-Dateien, eine oder zwei) mit `minimap2` oder `bwa` auf die gewählte Sequenz ab. Die Referenz wird als FASTA
    /// in `output_dir` geschrieben, das Ergebnis als SAM daneben und mit dem Dokument verknüpft.
    pub fn map_reads_to_reference(
        &self,
        reference_id: String,
        read_paths: Vec<String>,
        mapper: String,
        preset: String,
        threads: u64,
        output_dir: String,
    ) -> Result<ExternalFileDto, ApiError> {
        let d = self.sequence_doc(&reference_id)?;
        let tools = ToolRegistry::defaults();
        let program = tools
            .executable(&mapper)
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Das Programm '{}' ist nicht installiert", mapper),
            })?;
        if read_paths.is_empty() || read_paths.len() > 2 {
            return Err(ApiError::InvalidInput {
                message: "Es werden eine oder zwei Read-Dateien erwartet".to_string(),
            });
        }
        let dir = std::path::PathBuf::from(&output_dir);
        std::fs::create_dir_all(&dir).map_err(|e| ApiError::Io {
            message: e.to_string(),
        })?;
        let safe: String = d
            .name()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let fasta = dir.join(format!("{}.fa", safe));
        std::fs::write(
            &fasta,
            io::fasta::format(std::slice::from_ref(d.sequence())),
        )
        .map_err(|e| ApiError::Io {
            message: e.to_string(),
        })?;
        let reference = fasta.to_string_lossy().to_string();
        let sam = dir.join(format!("{}.{}.sam", safe, mapper));
        let threads = threads.max(1) as usize;
        if mapper == "bwa" {
            if !bwa_index_exists(&reference) {
                run_to_file(
                    &program.to_string_lossy(),
                    &bwa_index_args(&reference),
                    &dir.join("bwa-index.log"),
                )?;
            }
            run_to_file(
                &program.to_string_lossy(),
                &bwa_mem_args(threads, &reference, &read_paths),
                &sam,
            )?;
        } else {
            let preset = if preset.is_empty() {
                "sr".to_string()
            } else {
                preset
            };
            run_to_file(
                &program.to_string_lossy(),
                &minimap2_args(&preset, threads, &reference, &read_paths),
                &sam,
            )?;
        }
        Ok(external_dto(self.lock()?.add_external_file(
            &sam.to_string_lossy(),
            "sam",
            &reference_id,
            &format!("{} ({})", d.name(), mapper),
        )?))
    }

    /// Lesetiefe entlang der gewählten Sequenz aus einer SAM- oder BAM-Datei, auf höchstens `bins` Werte gemittelt.
    pub fn coverage_depth(
        &self,
        reference_id: String,
        alignment_path: String,
        bins: u64,
    ) -> Result<Vec<f64>, ApiError> {
        let d = self.sequence_doc(&reference_id)?;
        let path = std::path::Path::new(&alignment_path);
        let records = if alignment_path.to_lowercase().ends_with(".bam") {
            read_bam(path)?.records
        } else {
            io::sam::read(path)?.records
        };
        let name = records
            .iter()
            .find(|r| r.reference != "*")
            .map(|r| r.reference.clone())
            .unwrap_or_else(|| d.name().to_string());
        let depth = depth_from_sam(&records, &name, d.len(), true, 0, false)?;
        let bins = (bins.max(1) as usize).min(depth.len().max(1));
        if depth.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity(bins);
        for b in 0..bins {
            let from = b * depth.len() / bins;
            let to = ((b + 1) * depth.len() / bins)
                .max(from + 1)
                .min(depth.len());
            let sum: u64 = depth[from..to].iter().map(|v| *v as u64).sum();
            out.push(sum as f64 / (to - from) as f64);
        }
        Ok(out)
    }

    /// Alle Ordner.
    pub fn folders(&self) -> Result<Vec<FolderDto>, ApiError> {
        Ok(self
            .lock()?
            .folders()?
            .into_iter()
            .map(|f| FolderDto {
                id: f.id,
                name: f.name,
                parent_id: f.parent_id,
            })
            .collect())
    }

    pub fn create_folder(&self, name: String, parent_id: String) -> Result<FolderDto, ApiError> {
        let f = self.lock()?.create_folder(&name, &parent_id)?;
        Ok(FolderDto {
            id: f.id,
            name: f.name,
            parent_id: f.parent_id,
        })
    }

    pub fn rename_folder(&self, id: String, name: String) -> Result<(), ApiError> {
        Ok(self.lock()?.rename_folder(&id, &name)?)
    }

    pub fn delete_folder(&self, id: String, delete_contents: bool) -> Result<(), ApiError> {
        Ok(self.lock()?.delete_folder(&id, delete_contents)?)
    }

    /// Verschiebt ein Dokument in einen Ordner (leer = Wurzel).
    pub fn move_document(&self, doc_id: String, folder_id: String) -> Result<(), ApiError> {
        Ok(self.lock()?.move_document(&doc_id, &folder_id)?)
    }

    pub fn folder_of(&self, doc_id: String) -> Result<String, ApiError> {
        Ok(self.lock()?.folder_of(&doc_id)?)
    }

    pub fn documents_in_folder(&self, folder_id: String) -> Result<Vec<DocumentInfoDto>, ApiError> {
        Ok(self
            .lock()?
            .documents_in_folder(&folder_id)?
            .into_iter()
            .map(info_dto)
            .collect())
    }

    pub fn set_note(&self, doc_id: String, text: String) -> Result<(), ApiError> {
        Ok(self.lock()?.set_note(&doc_id, &text)?)
    }

    pub fn note(&self, doc_id: String) -> Result<String, ApiError> {
        Ok(self.lock()?.note(&doc_id)?.unwrap_or_default())
    }

    pub fn set_favorite(&self, doc_id: String, favorite: bool) -> Result<(), ApiError> {
        Ok(self.lock()?.set_favorite(&doc_id, favorite)?)
    }

    pub fn favorites(&self) -> Result<Vec<DocumentInfoDto>, ApiError> {
        Ok(self
            .lock()?
            .favorites()?
            .into_iter()
            .map(info_dto)
            .collect())
    }

    /// Suche: `kind` ist `name`, `type`, `feature`, `metadata` (`text` = Schlüssel, `text2` = Wert) oder `motif`.
    pub fn search_documents(
        &self,
        kind: String,
        text: String,
        text2: String,
    ) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let store = self.lock()?;
        let found = match kind.as_str() {
            "name" => project_search::by_name(&store, &text)?,
            "type" => project_search::by_type(&store, &text)?,
            "feature" => project_search::by_feature_text(&store, &text)?,
            "metadata" => project_search::by_metadata(&store, &text, &text2)?,
            "motif" => project_search::by_sequence_motif(&store, &text)?,
            other => {
                return Err(ApiError::InvalidInput {
                    message: format!("Unbekannte Suchart '{}'", other),
                })
            }
        };
        Ok(found.into_iter().map(info_dto).collect())
    }

    /// Gruppen von Sequenzdokumenten mit gleicher Sequenz (Dubletten).
    pub fn find_duplicates(&self) -> Result<Vec<Vec<DocumentInfoDto>>, ApiError> {
        Ok(project_search::find_sequence_duplicates(&*self.lock()?)?
            .into_iter()
            .map(|g| g.into_iter().map(info_dto).collect())
            .collect())
    }

    /// Neue Namen nach einem Muster (`{name}`, `{n}`, `{n:03}`, `{type}`) für die gewählten Dokumente anwenden.
    pub fn batch_rename(&self, ids: Vec<String>, pattern: String) -> Result<u64, ApiError> {
        let mut store = self.lock()?;
        let infos: Vec<snapious_core::store::DocumentInfo> = store
            .list()?
            .into_iter()
            .filter(|d| ids.contains(&d.id))
            .collect();
        let plan = project_search::batch_rename_plan(&infos, &pattern);
        let mut n = 0;
        for (id, name) in plan {
            if let Some(doc) = store.load(&id)? {
                let renamed = match doc {
                    Document::Sequence(d) => Document::Sequence(d.with_name(&name)),
                    Document::Alignment(d) => Document::Alignment(d.with_name(&name)),
                };
                store.save(&renamed)?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Gespeicherte Versionen eines Dokuments, neueste zuerst.
    pub fn versions(&self, doc_id: String) -> Result<Vec<VersionDto>, ApiError> {
        Ok(self
            .lock()?
            .versions(&doc_id)?
            .into_iter()
            .map(|v| VersionDto {
                version: v.version,
                time: v.time,
                summary: v.summary,
            })
            .collect())
    }

    pub fn restore_version(&self, doc_id: String, version: i64) -> Result<(), ApiError> {
        self.edit_sessions_clear(&doc_id);
        Ok(self.lock()?.restore_version(&doc_id, version)?)
    }

    /// Zeichnet aus dem Verlauf eines Dokuments einen Workflow auf und speichert ihn unter `name`. Rückgabe: Zahl der Schritte.
    pub fn record_workflow(&self, doc_id: String, name: String) -> Result<u64, ApiError> {
        let mut store = self.lock()?;
        let history = store.history_for(&doc_id)?;
        let wf = workflow_from_history(&name, &history)?;
        wf.validate(&registry_with_plugins())?;
        store.save_workflow(&wf.name, &wf.to_json())?;
        Ok(wf.steps.len() as u64)
    }

    /// Text- und Berichtsexport einer Sequenz: `kind` ist `text` (Positionen, Features) oder `report` (Markdown).
    pub fn export_text(&self, id: String, kind: String) -> Result<String, ApiError> {
        let d = self.sequence_doc(&id)?;
        match kind.as_str() {
            "text" => Ok(format_sequence_text(&d, 60, true, true)),
            "report" => {
                let history = self.lock()?.history_for(&id)?;
                Ok(report_markdown(&d, &history))
            }
            "features_csv" | "features_tsv" => {
                let mut rows = vec![vec![
                    "Typ".to_string(),
                    "Name".to_string(),
                    "Start".to_string(),
                    "Ende".to_string(),
                    "Strang".to_string(),
                    "Länge".to_string(),
                ]];
                for f in d.features() {
                    let loc = f.location();
                    rows.push(vec![
                        f.feature_type().to_string(),
                        f.name().to_string(),
                        (loc.start() + 1).to_string(),
                        loc.end().to_string(),
                        match loc.strand() {
                            Strand::Forward => "+",
                            Strand::Reverse => "-",
                            _ => ".",
                        }
                        .to_string(),
                        loc.length().to_string(),
                    ]);
                }
                let sep = if kind == "features_tsv" { '\t' } else { ',' };
                Ok(to_delimited(&rows, sep))
            }
            other => Err(ApiError::InvalidInput {
                message: format!("Unbekannte Exportart '{}'", other),
            }),
        }
    }

    /// Faltet den Bereich `[start, end)` (Ende 0 = bis zum Ende) mit ViennaRNA (`RNAfold`); höchstens 5000 Basen.
    pub fn fold_region(&self, id: String, start: u64, end: u64) -> Result<RnaFoldDto, ApiError> {
        let d = self.sequence_doc(&id)?;
        let len = d.len();
        let (s, e) = (
            start as usize,
            if end == 0 {
                len
            } else {
                (end as usize).min(len)
            },
        );
        if s >= e {
            return Err(ApiError::InvalidInput {
                message: format!("Ungültiger Bereich {}-{}", start, end),
            });
        }
        if e - s > 5000 {
            return Err(ApiError::InvalidInput {
                message: "Für die RNA-Faltung sind höchstens 5000 Basen erlaubt".to_string(),
            });
        }
        let text = d.bases().get(s..e).unwrap_or("");
        let f = fold_rna(text)?;
        let pairs = pairs_from_dot_bracket(&f.structure)?
            .into_iter()
            .map(|(a, b)| BasePairDto {
                first: a as u64,
                second: b as u64,
            })
            .collect();
        Ok(RnaFoldDto {
            sequence: f.sequence,
            structure: f.structure,
            energy: f.energy,
            pairs,
        })
    }

    /// Sucht ein Motiv (IUPAC-Codes erlaubt) auf beiden Strängen; bei zirkulären Sequenzen auch über den Nullpunkt.
    pub fn find_motif_hits(&self, id: String, motif: String) -> Result<Vec<MotifHitDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let circular = d.topology() == Topology::Circular;
        Ok(find_motif(d.bases(), &motif, true, circular)?
            .into_iter()
            .map(|h| MotifHitDto {
                start: h.start as u64,
                end: h.end as u64,
                reverse: h.reverse,
            })
            .collect())
    }

    /// Entfernt Spalten mit Lückenanteil `>= threshold` (nur Spalten mit mindestens einer Lücke) und speichert das Ergebnis als neues Alignment.
    pub fn alignment_strip(&self, id: String, threshold: f64) -> Result<DocumentInfoDto, ApiError> {
        let a = self.alignment_doc(&id)?;
        let rows = strip_columns(a.rows(), threshold)?;
        let doc = AlignmentDocument::of(
            &format!("{} (bereinigt)", a.name()),
            a.seq_type(),
            a.row_names().to_vec(),
            rows,
        )?;
        self.save_new(
            Document::Alignment(doc),
            "align.strip",
            vec![id],
            format!("Spalten mit Lücken ≥ {:.0} % entfernt", threshold * 100.0),
        )
    }

    /// Mutagenese-Primer: ersetzt `[start, end)` durch `replacement` (leer = Deletion, `start == end` = Einfügung).
    #[allow(clippy::too_many_arguments)]
    pub fn mutagenesis_primers(
        &self,
        id: String,
        start: u64,
        end: u64,
        replacement: String,
        min_flank: u64,
        max_flank: u64,
        target_tm: f64,
    ) -> Result<MutagenesisDto, ApiError> {
        let d = self.sequence_doc(&id)?;
        let p = design_mutagenesis(
            d.bases(),
            start as usize,
            end as usize,
            &replacement,
            min_flank as usize,
            max_flank as usize,
            target_tm,
        )?;
        Ok(MutagenesisDto {
            forward: p.forward,
            reverse: p.reverse,
            left_flank: p.left_flank as u64,
            right_flank: p.right_flank as u64,
            tm: p.tm,
        })
    }

    /// Tandem- und invertierte Wiederholungen einer Sequenz.
    pub fn find_repeats(
        &self,
        id: String,
        min_unit: u64,
        max_unit: u64,
        min_copies: u64,
        min_arm: u64,
        max_spacer: u64,
    ) -> Result<Vec<RepeatDto>, ApiError> {
        let d = self.sequence_doc(&id)?;
        let bases = d.bases().to_ascii_uppercase();
        let mut out: Vec<RepeatDto> = find_tandem_repeats(
            &bases,
            min_unit as usize,
            max_unit as usize,
            min_copies as usize,
        )
        .into_iter()
        .map(|r| RepeatDto {
            kind: "tandem".to_string(),
            start: r.start as u64,
            end: r.end as u64,
            detail: format!("{} × {}", r.unit, r.copies),
        })
        .collect();
        out.extend(
            find_inverted_repeats(&bases, min_arm as usize, max_spacer as usize)
                .into_iter()
                .map(|r| RepeatDto {
                    kind: "inverted".to_string(),
                    start: r.start as u64,
                    end: r.end() as u64,
                    detail: format!("Arm {} bp, Spacer {} bp", r.arm, r.spacer),
                }),
        );
        Ok(out)
    }

    /// Varianten aller Alignment-Zeilen gegenüber der Zeile `reference`.
    pub fn alignment_variants(
        &self,
        id: String,
        reference: u64,
    ) -> Result<Vec<VariantDto>, ApiError> {
        let a = self.alignment_doc(&id)?;
        let vars = find_variants(a.rows(), reference as usize)?;
        Ok(vars
            .into_iter()
            .map(|v| VariantDto {
                column: v.column as u64,
                reference_position: v.reference_position as u64,
                row_name: a.row_names().get(v.row).cloned().unwrap_or_default(),
                reference_base: v.reference_base.to_string(),
                alt_base: v.alt_base.to_string(),
                kind: match v.kind {
                    VariantKind::Substitution => "Substitution",
                    VariantKind::Insertion => "Insertion",
                    VariantKind::Deletion => "Deletion",
                }
                .to_string(),
            })
            .collect())
    }

    /// Setzt Sequenzdokumente zu Contigs zusammen und speichert jeden Contig als neues Dokument.
    pub fn assemble_reads(
        &self,
        ids: Vec<String>,
        min_overlap: u64,
    ) -> Result<Vec<DocumentInfoDto>, ApiError> {
        let docs = self.sequence_docs(&ids)?;
        let reads: Vec<String> = docs
            .iter()
            .map(|d| d.bases().to_ascii_uppercase())
            .collect();
        let contigs = assemble_contigs(&reads, min_overlap as usize);
        let mut out = Vec::new();
        for (i, c) in contigs.iter().enumerate() {
            let placed = place_reads(c, &reads).iter().flatten().count();
            let seq = snapious_core::model::sequence::Sequence::new(
                &new_id(),
                &format!("Contig {}", i + 1),
                c,
                snapious_core::model::sequence::SequenceType::Dna,
            )?;
            out.push(self.save_new(
                Document::Sequence(SequenceDocument::of(&format!("Contig {}", i + 1), seq)),
                "assemble.reads",
                ids.clone(),
                format!("Contig {} aus {} Reads", i + 1, placed),
            )?);
        }
        Ok(out)
    }

    /// Bildet Reads (Sequenzdokumente) lückenlos auf die Referenz ab.
    pub fn map_reads_to(
        &self,
        reference_id: String,
        read_ids: Vec<String>,
        max_mismatches: u64,
    ) -> Result<Vec<ReadMapDto>, ApiError> {
        let r = self.sequence_doc(&reference_id)?;
        let docs = self.sequence_docs(&read_ids)?;
        let reads: Vec<String> = docs.iter().map(|d| d.bases().to_string()).collect();
        let maps = map_reads(r.bases(), &reads, max_mismatches as usize);
        Ok(docs
            .iter()
            .zip(maps)
            .map(|(d, m)| ReadMapDto {
                name: d.name().to_string(),
                mapped: m.is_some(),
                position: m.map_or(0, |x| x.position as u64),
                reverse: m.is_some_and(|x| x.reverse),
                mismatches: m.map_or(0, |x| x.mismatches as u64),
            })
            .collect())
    }

    /// Dotplot zweier Sequenzen als Diagonalen (Punkte werden zu Linien zusammengefasst).
    pub fn dotplot(
        &self,
        id_a: String,
        id_b: String,
        window: u64,
        min_matches: u64,
        reverse: bool,
    ) -> Result<Vec<DiagonalDto>, ApiError> {
        let a = self.sequence_doc(&id_a)?;
        let b = self.sequence_doc(&id_b)?;
        let pts = dotplot_points(
            a.bases(),
            b.bases(),
            window as usize,
            min_matches as usize,
            reverse,
        )?;
        Ok(diagonals(&pts)
            .into_iter()
            .map(|(i, j, l)| DiagonalDto {
                start_a: i as u64,
                start_b: j as u64,
                length: l as u64,
            })
            .collect())
    }

    /// Identitätsmatrix (Prozent) der Zeilen eines Alignments.
    pub fn alignment_identity_matrix(&self, id: String) -> Result<IdentityMatrixDto, ApiError> {
        let a = self.alignment_doc(&id)?;
        let m = identity_matrix_percent(a.rows())?;
        Ok(IdentityMatrixDto {
            names: a.row_names().to_vec(),
            values: m.into_iter().flatten().collect(),
        })
    }

    /// Sequenzlogo eines Alignments (Schneider & Stephens, mit Kleinprobenkorrektur).
    pub fn alignment_logo(&self, id: String) -> Result<Vec<LogoColumnDto>, ApiError> {
        let a = self.alignment_doc(&id)?;
        let protein = format!("{:?}", a.seq_type()) == "Protein";
        Ok(sequence_logo(a.rows(), protein, true)?
            .into_iter()
            .map(|c| LogoColumnDto {
                symbols: c.symbols.iter().map(|(ch, _)| ch.to_string()).collect(),
                heights: c.symbols.iter().map(|(_, h)| *h).collect(),
            })
            .collect())
    }

    /// Konsenssequenz eines Alignments (Schwelle 0,5, Lücken ohne Gaps) als neues Sequenzdokument.
    pub fn alignment_consensus(&self, id: String) -> Result<DocumentInfoDto, ApiError> {
        let a = self.alignment_doc(&id)?;
        let text: String = consensus(a.rows(), 0.5, true)?
            .chars()
            .filter(|c| *c != '-')
            .collect();
        if text.is_empty() {
            return Err(ApiError::InvalidInput {
                message: "Der Konsens ist leer".to_string(),
            });
        }
        let name = format!("{} Konsens", a.name());
        let seq = snapious_core::model::Sequence::new(&name, &name, &text, a.seq_type())?;
        self.save_new(
            Document::Sequence(SequenceDocument::of(&name, seq)),
            "align.consensus",
            vec![id],
            "Konsens".to_string(),
        )
    }

    /// Alle Operationen, die in Workflows vorkommen dürfen.
    pub fn operation_list(&self) -> Vec<OperationInfoDto> {
        registry_with_plugins()
            .all()
            .iter()
            .map(|o| OperationInfoDto {
                id: o.id().to_string(),
                label: o.label().to_string(),
            })
            .collect()
    }

    /// Alle gespeicherten Workflows.
    pub fn workflow_list(&self) -> Result<Vec<WorkflowDto>, ApiError> {
        Ok(self
            .lock()?
            .workflows()?
            .into_iter()
            .map(|(name, json)| {
                let parsed = Workflow::from_json(&json).ok();
                WorkflowDto {
                    name,
                    input_count: parsed.as_ref().map_or(0, |w| w.input_count as u64),
                    step_count: parsed.as_ref().map_or(0, |w| w.steps.len() as u64),
                    json,
                }
            })
            .collect())
    }

    /// Prüft und speichert einen Workflow (JSON); der Name kommt aus dem JSON.
    pub fn workflow_save(&self, json: String) -> Result<String, ApiError> {
        let wf = Workflow::from_json(&json)?;
        wf.validate(&registry_with_plugins())?;
        self.lock()?.save_workflow(&wf.name, &wf.to_json())?;
        Ok(wf.name)
    }

    pub fn workflow_delete(&self, name: String) -> Result<(), ApiError> {
        self.lock()?.delete_workflow(&name)?;
        Ok(())
    }

    /// Führt einen gespeicherten Workflow aus. Braucht der Workflow eine Eingabe, läuft er für jedes der `input_ids` einzeln
    /// (Stapel); sonst bilden alle `input_ids` zusammen die Eingaben eines einzigen Laufs. Ergebnisse werden als neue Dokumente gespeichert.
    pub fn workflow_run(
        &self,
        name: String,
        input_ids: Vec<String>,
    ) -> Result<WorkflowRunDto, ApiError> {
        let json = self
            .lock()?
            .workflows()?
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, j)| j)
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Workflow '{}' nicht gefunden", name),
            })?;
        let wf = Workflow::from_json(&json)?;
        let mut inputs = Vec::new();
        for id in &input_ids {
            inputs.push(self.lock()?.load(id)?.ok_or_else(|| ApiError::NotFound {
                message: format!("Dokument '{}' nicht gefunden", id),
            })?);
        }
        let registry = registry_with_plugins();
        let wr = WorkflowRunner::new(&registry);
        let mut runner = OperationRunner::new();
        let mut ctx = NullContext;
        let results = if wf.input_count == 1 {
            wr.run_batch(&wf, &inputs, &mut runner, &mut ctx)
        } else {
            vec![match wr.run(&wf, &inputs, &mut runner, &mut ctx) {
                Ok(r) => r,
                Err(e) => snapious_core::ops::WorkflowResult {
                    outputs: Vec::new(),
                    history: Vec::new(),
                    errors: vec![e.to_string()],
                },
            }]
        };
        let mut docs = Vec::new();
        let mut errors = Vec::new();
        let mut store = self.lock()?;
        for r in results {
            errors.extend(r.errors);
            for d in r.outputs {
                store.save(&d)?;
                docs.push(DocumentInfoDto {
                    id: d.id().to_string(),
                    name: d.name().to_string(),
                    doc_type: d.doc_type().to_string(),
                    created: d.created(),
                });
            }
            for h in r.history {
                store.add_history(&h)?;
            }
        }
        Ok(WorkflowRunDto {
            documents: docs,
            errors,
        })
    }

    /// Liest Annotationen (GFF3 oder BED, nach der Dateiendung `.gff`, `.gff3`, `.bed`) und trägt sie als Features ins Dokument ein.
    /// Die Kennung der Datei (`seqid`/`chrom`) muss dem Dokumentnamen (Leerzeichen als `_`) oder dem Accession-Metadatum entsprechen;
    /// gibt es nur eine Kennung in der Datei, wird sie genommen. Rückgabe: Zahl der eingetragenen Features.
    pub fn import_annotations(&self, id: String, path: String) -> Result<u64, ApiError> {
        let d = self.sequence_doc(&id)?;
        let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
            message: format!("Datei '{}' konnte nicht gelesen werden: {}", path, e),
        })?;
        let lower = path.to_lowercase();
        let wanted = [
            d.name().replace(' ', "_"),
            d.name().to_string(),
            d.metadata().get("accession").cloned().unwrap_or_default(),
        ];
        let pick = |ids: Vec<String>| -> Result<String, ApiError> {
            let mut unique = ids;
            unique.sort();
            unique.dedup();
            if let Some(m) = unique
                .iter()
                .find(|u| wanted.iter().any(|w| !w.is_empty() && w == *u))
            {
                return Ok(m.clone());
            }
            match unique.as_slice() {
                [one] => Ok(one.clone()),
                [] => Err(ApiError::InvalidInput {
                    message: "Die Datei enthält keine Annotationen".to_string(),
                }),
                many => Err(ApiError::InvalidInput {
                    message: format!(
                        "Keine Kennung passt zum Dokument; in der Datei: {}",
                        many.join(", ")
                    ),
                }),
            }
        };
        let features = if lower.ends_with(".bed") {
            let records = parse_bed(&text)?;
            let chrom = pick(records.iter().map(|r| r.chrom.clone()).collect())?;
            bed_to_features(&records, &chrom)?
        } else if lower.ends_with(".gff") || lower.ends_with(".gff3") {
            let records = parse_gff3(&text)?;
            let seqid = pick(records.iter().map(|r| r.seqid.clone()).collect())?;
            gff3_to_features(&records, &seqid)?
        } else {
            return Err(ApiError::InvalidInput {
                message: "Unbekannte Endung (erwartet .gff, .gff3 oder .bed)".to_string(),
            });
        };
        let count = features.len() as u64;
        for f in features {
            self.edit(&id, |s| s.add_feature(f))?;
        }
        Ok(count)
    }

    pub fn rename_document(&self, id: String, name: String) -> Result<(), ApiError> {
        let d = self.sequence_doc(&id)?;
        self.lock()?.save(&Document::Sequence(d.with_name(&name)))?;
        Ok(())
    }

    pub fn delete_document(&self, id: String) -> Result<(), ApiError> {
        self.lock()?.delete(&id)?;
        Ok(())
    }
}

/// Banden einer Gelspur für Fragmentgrößen.
#[uniffi::export]
pub fn gel_lane(sizes: Vec<u64>, agarose_percent: f64, mass_ng: f64) -> Vec<GelBandDto> {
    let sizes: Vec<usize> = sizes.iter().map(|s| *s as usize).collect();
    GelSimulator::new(agarose_percent)
        .lane("", &sizes, mass_ng)
        .bands
        .into_iter()
        .map(|b| GelBandDto {
            size_bp: b.size_bp as u64,
            mobility: b.mobility,
            intensity: b.intensity,
        })
        .collect()
}

/// Abruf mit Zwischenspeicher (24 Stunden) unter `~/.snapious/cache` (sonst im Temp-Ordner).
fn cached_http() -> CachingHttp<CurlClient, SystemClock> {
    let dir = std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".snapious").join("cache"))
        .unwrap_or_else(|| std::env::temp_dir().join("snapious-cache"));
    CachingHttp::new(CurlClient, dir, 24 * 3600, SystemClock)
}

/// BLAST-Transport über `curl`.
struct CurlTransport;

impl BlastTransport for CurlTransport {
    fn get(&self, url: &str) -> snapious_core::error::Result<String> {
        CurlClient.get(url)
    }

    fn post(&self, url: &str, body: &str) -> snapious_core::error::Result<String> {
        let out = std::process::Command::new("curl")
            .args([
                "-sS",
                "-L",
                "--max-time",
                "120",
                "--fail",
                "--data",
                body,
                url,
            ])
            .output()
            .map_err(|e| snapious_core::error::Error::Io(format!("curl: {}", e)))?;
        if !out.status.success() {
            return Err(snapious_core::error::Error::Io(format!(
                "Abruf fehlgeschlagen: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

/// Wartet wirklich.
struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep_secs(&self, secs: u64) {
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }
}

/// Eine Kette einer PDB-Struktur: Sequenz und C-alpha-Koordinaten (`ca` = x, y, z hintereinander).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PdbChainDto {
    pub id: String,
    pub sequence: String,
    pub ca: Vec<f64>,
}

/// Eine eingelesene PDB-Datei.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PdbDto {
    pub atom_count: u64,
    pub chains: Vec<PdbChainDto>,
    /// Alle Atome als x, y, z (nur bei Molekülen aus XYZ/MOL; bei PDB leer).
    pub atoms: Vec<f64>,
    /// Elementsymbole zu `atoms` (ein Eintrag je Atom).
    pub elements: Vec<String>,
    /// Bindungen als Paare von Atomindizes (0-basiert) in `atoms`.
    pub bonds: Vec<u64>,
}

/// Liest eine PDB-Datei (Ketten, Sequenzen, C-alpha-Spur) für die 3D-Ansicht.
#[uniffi::export]
pub fn pdb_open(path: String) -> Result<PdbDto, ApiError> {
    let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
        message: format!("{}: {}", path, e),
    })?;
    let pdb = parse_pdb(&text);
    let chains = pdb
        .chains()
        .into_iter()
        .map(|c| {
            let mut last = i32::MIN;
            let mut ca = Vec::new();
            for a in pdb
                .atoms
                .iter()
                .filter(|a| !a.hetero && a.chain == c && a.name == "CA")
            {
                if a.number != last {
                    ca.extend([a.x, a.y, a.z]);
                    last = a.number;
                }
            }
            PdbChainDto {
                id: c.to_string(),
                sequence: pdb.sequence(c),
                ca,
            }
        })
        .collect();
    Ok(PdbDto {
        atom_count: pdb.atoms.len() as u64,
        chains,
        atoms: Vec::new(),
        elements: Vec::new(),
        bonds: Vec::new(),
    })
}

/// Öffnet eine Struktur- oder Moleküldatei für die 3D-Ansicht: `.pdb`, `.xyz` oder `.mol`/`.sdf` (erstes Molekül).
#[uniffi::export]
pub fn structure_open(path: String) -> Result<PdbDto, ApiError> {
    let lower = path.to_lowercase();
    if !(lower.ends_with(".xyz")
        || lower.ends_with(".mol")
        || lower.ends_with(".sdf")
        || lower.ends_with(".cml")
        || lower.ends_with(".hin"))
    {
        return pdb_open(path);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
        message: format!("{}: {}", path, e),
    })?;
    let mut atoms = Vec::new();
    let mut elements = Vec::new();
    let mut bonds = Vec::new();
    if lower.ends_with(".xyz") {
        for a in parse_xyz(&text)? {
            atoms.extend([a.x, a.y, a.z]);
            elements.push(a.element);
        }
    } else if lower.ends_with(".hin") {
        let (a, b) = parse_hin(&text)?;
        for x in a {
            atoms.extend([x.x, x.y, x.z]);
            elements.push(x.element);
        }
        for x in b {
            bonds.extend([x.from as u64, x.to as u64]);
        }
    } else if lower.ends_with(".sdf") {
        let first = parse_sdf(&text)?
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InvalidInput {
                message: "Keine Moleküle in der SDF-Datei".to_string(),
            })?;
        for x in first.atoms {
            atoms.extend([x.x, x.y, x.z]);
            elements.push(x.element);
        }
        for x in first.bonds {
            bonds.extend([x.from as u64, x.to as u64]);
        }
    } else if lower.ends_with(".cml") {
        let (a, b) = parse_cml(&text)?;
        for x in a {
            atoms.extend([x.x, x.y, x.z]);
            elements.push(x.element);
        }
        for x in b {
            bonds.extend([x.from as u64, x.to as u64]);
        }
    } else {
        let (a, b) = parse_mol(&text)?;
        for x in a {
            atoms.extend([x.x, x.y, x.z]);
            elements.push(x.element);
        }
        for x in b {
            bonds.extend([x.from as u64, x.to as u64]);
        }
    }
    Ok(PdbDto {
        atom_count: elements.len() as u64,
        chains: Vec::new(),
        atoms,
        elements,
        bonds,
    })
}

/// Ordner im Projekt.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct FolderDto {
    pub id: String,
    pub name: String,
    pub parent_id: String,
}

/// Eine gespeicherte Version eines Dokuments.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct VersionDto {
    pub version: i64,
    pub time: i64,
    pub summary: String,
}

/// Programmeinstellungen (Schlüssel wird in Meldungen nie ausgegeben).
#[derive(Clone, PartialEq, uniffi::Record)]
pub struct SettingsDto {
    pub ncbi_email: String,
    pub ncbi_api_key: String,
    pub default_organism: String,
    pub dark_mode: bool,
    pub undo_limit: u64,
}

fn info_dto(d: snapious_core::store::DocumentInfo) -> DocumentInfoDto {
    DocumentInfoDto {
        id: d.id,
        name: d.name,
        doc_type: d.doc_type,
        created: d.created,
    }
}

fn data_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".snapious"))
        .unwrap_or_else(|| std::env::temp_dir().join("snapious"))
}

/// Alle eingebauten Operationen plus die Operationen der Plugins unter `~/.snapious/plugins`.
fn registry_with_plugins() -> OperationRegistry {
    let mut r = OperationRegistry::with_builtins();
    let (plugins, _warnings) = discover(&data_dir().join("plugins"));
    let _ = register_plugins(&mut r, &plugins);
    r
}

/// Rückübersetzung eines Proteintextes: `preferred` (häufigste E.-coli-Codons) oder `degenerate` (IUPAC-Codes).
#[uniffi::export]
pub fn back_translate(protein: String, mode: String) -> Result<String, ApiError> {
    Ok(match mode.as_str() {
        "degenerate" => back_translate_degenerate(&protein)?,
        _ => back_translate_preferred(&protein)?,
    })
}

fn enzyme_sets_path() -> std::path::PathBuf {
    data_dir().join("enzyme_sets.json")
}

/// Alle Enzymsets: eingebaute und eigene (aus `~/.snapious/enzyme_sets.json`).
#[uniffi::export]
pub fn enzyme_sets() -> Vec<EnzymeSetDto> {
    let mut out: Vec<EnzymeSetDto> = builtin_sets()
        .into_iter()
        .map(|s| EnzymeSetDto {
            name: s.name,
            enzymes: s.enzymes,
            builtin: true,
        })
        .collect();
    if let Ok(text) = std::fs::read_to_string(enzyme_sets_path()) {
        if let Ok(map) = serde_json::from_str::<BTreeMap<String, Vec<String>>>(&text) {
            for (name, enzymes) in map {
                out.push(EnzymeSetDto {
                    name,
                    enzymes,
                    builtin: false,
                });
            }
        }
    }
    out
}

/// Speichert ein eigenes Enzymset (Name und Enzyme als Text, getrennt durch Komma, Semikolon oder Leerzeichen).
/// Ein vorhandenes eigenes Set gleichen Namens wird ersetzt; eingebaute Namen sind nicht erlaubt.
#[uniffi::export]
pub fn enzyme_set_save(name: String, enzymes: String) -> Result<(), ApiError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "Das Set braucht einen Namen".to_string(),
        });
    }
    if builtin_sets().iter().any(|s: &EnzymeSet| s.name == name) {
        return Err(ApiError::InvalidInput {
            message: "Dieser Name gehört zu einem eingebauten Set".to_string(),
        });
    }
    let list = parse_enzyme_list(&enzymes);
    if list.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "Das Set braucht mindestens ein Enzym".to_string(),
        });
    }
    let path = enzyme_sets_path();
    let mut map: BTreeMap<String, Vec<String>> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    map.insert(name, list);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| ApiError::Io {
            message: e.to_string(),
        })?;
    }
    let text = serde_json::to_string_pretty(&map).map_err(|e| ApiError::Internal {
        message: e.to_string(),
    })?;
    std::fs::write(&path, text).map_err(|e| ApiError::Io {
        message: e.to_string(),
    })?;
    Ok(())
}

/// Löscht ein eigenes Enzymset (eingebaute lassen sich nicht löschen).
#[uniffi::export]
pub fn enzyme_set_delete(name: String) -> Result<(), ApiError> {
    let path = enzyme_sets_path();
    let mut map: BTreeMap<String, Vec<String>> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    if map.remove(&name).is_some() {
        let text = serde_json::to_string_pretty(&map).map_err(|e| ApiError::Internal {
            message: e.to_string(),
        })?;
        std::fs::write(&path, text).map_err(|e| ApiError::Io {
            message: e.to_string(),
        })?;
    }
    Ok(())
}

/// Erste Zeilen einer Tabelle (CSV/TSV) zur Vorschau.
#[uniffi::export]
pub fn table_preview(
    path: String,
    sep: String,
    max_rows: u64,
) -> Result<Vec<Vec<String>>, ApiError> {
    let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
        message: format!("{}: {}", path, e),
    })?;
    Ok(parse_delimited(&text, sep.chars().next().unwrap_or(','))
        .into_iter()
        .take(max_rows as usize)
        .collect())
}

/// Teilt eine FASTQ-Datei nach Barcodes am Anfang der Reads. `barcodes` enthält je Zeile `Name,Sequenz` (oder mit Leerzeichen/Tab
/// getrennt; Zeilen mit `#` und leere Zeilen werden übersprungen). Je Barcode entsteht `<ordner>/<name>.fastq` (Read ohne
/// Barcode samt gekürzter Qualität), nicht zugeordnete Reads landen in `unassigned.fastq`.
#[uniffi::export]
pub fn split_fastq_by_barcode(
    path: String,
    barcodes: String,
    max_mismatch: u64,
    out_dir: String,
) -> Result<Vec<BarcodeGroupDto>, ApiError> {
    use std::io::Write;
    let mut list: Vec<(String, String)> = Vec::new();
    for line in barcodes.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split([',', '\t', ' ', ';']).filter(|p| !p.is_empty());
        match (parts.next(), parts.next()) {
            (Some(n), Some(s)) => {
                let safe: String = n
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                list.push((safe, s.to_uppercase()));
            }
            _ => {
                return Err(ApiError::InvalidInput {
                    message: format!(
                        "Barcode-Zeile nicht lesbar: '{}' (erwartet: Name,Sequenz)",
                        line
                    ),
                })
            }
        }
    }
    if list.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "Keine Barcodes angegeben".to_string(),
        });
    }
    std::fs::create_dir_all(&out_dir).map_err(|e| ApiError::Io {
        message: format!("{}: {}", out_dir, e),
    })?;
    let dir = std::path::PathBuf::from(&out_dir);
    let mut files: Vec<(
        String,
        std::path::PathBuf,
        std::io::BufWriter<std::fs::File>,
        u64,
    )> = Vec::new();
    for name in list
        .iter()
        .map(|b| b.0.clone())
        .chain(std::iter::once("unassigned".to_string()))
    {
        let p = dir.join(format!("{}.fastq", name));
        let f = std::fs::File::create(&p).map_err(|e| ApiError::Io {
            message: format!("{}: {}", p.display(), e),
        })?;
        files.push((name, p, std::io::BufWriter::new(f), 0));
    }
    let unassigned = files.len() - 1;
    for rec in open_fastq(Path::new(&path))? {
        let rec = rec?;
        let hit = assign_barcodes(
            std::slice::from_ref(&rec.sequence),
            &list,
            max_mismatch as usize,
        )
        .pop()
        .ok_or_else(|| ApiError::Internal {
            message: "Barcode-Zuordnung ohne Ergebnis".to_string(),
        })?;
        let (slot, seq, qual) = match hit.barcode {
            Some(i) => {
                let cut = list[i].1.len();
                (
                    i,
                    hit.trimmed,
                    rec.quality.get(cut..).unwrap_or("").to_string(),
                )
            }
            None => (unassigned, rec.sequence.clone(), rec.quality.clone()),
        };
        let header = if rec.comment.is_empty() {
            rec.id.clone()
        } else {
            format!("{} {}", rec.id, rec.comment)
        };
        let w = &mut files[slot].2;
        write!(w, "@{}\n{}\n+\n{}\n", header, seq, qual).map_err(|e| ApiError::Io {
            message: e.to_string(),
        })?;
        files[slot].3 += 1;
    }
    let mut out = Vec::new();
    for (name, p, mut w, count) in files {
        w.flush().map_err(|e| ApiError::Io {
            message: e.to_string(),
        })?;
        out.push(BarcodeGroupDto {
            name,
            count,
            path: p.to_string_lossy().to_string(),
        });
    }
    Ok(out)
}

/// Wandelt eine FASTQ-Datei in FASTA und QUAL (Phred+33) um. Gibt die Zahl der Reads zurück.
#[uniffi::export]
pub fn fastq_to_fasta_qual(
    path: String,
    fasta_path: String,
    qual_path: String,
) -> Result<u64, ApiError> {
    let mut fasta = String::new();
    let mut quals: Vec<(String, Vec<u8>)> = Vec::new();
    for rec in open_fastq(Path::new(&path))? {
        let rec = rec?;
        fasta.push_str(&format!(">{}\n{}\n", rec.id, rec.sequence));
        quals.push((
            rec.id.clone(),
            rec.quality.bytes().map(|b| b.saturating_sub(33)).collect(),
        ));
    }
    let io_err = |e: std::io::Error| ApiError::Io {
        message: e.to_string(),
    };
    std::fs::write(&fasta_path, fasta).map_err(io_err)?;
    std::fs::write(&qual_path, write_qual(&quals)).map_err(io_err)?;
    Ok(quals.len() as u64)
}

/// Expressionswerte aus einer SAM- oder BAM-Datei: Reads je Referenz (nur zugeordnete, keine sekundären), RPKM und TPM.
#[uniffi::export]
pub fn expression_from_alignment(path: String) -> Result<Vec<ExpressionRowDto>, ApiError> {
    let p = Path::new(&path);
    let (references, records): (Vec<(String, usize)>, Vec<snapious_core::io::sam::SamRecord>) =
        if path.to_lowercase().ends_with(".bam") {
            let b = read_bam(p)?;
            (
                b.references
                    .into_iter()
                    .map(|(n, l)| (n, l as usize))
                    .collect(),
                b.records,
            )
        } else {
            let s = io::sam::read(p)?;
            (s.references, s.records)
        };
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for r in records
        .iter()
        .filter(|r| !r.is_unmapped() && !r.is_secondary() && r.reference != "*")
    {
        *counts.entry(r.reference.clone()).or_insert(0) += 1;
    }
    let mut refs = references;
    for name in counts.keys() {
        if !refs.iter().any(|(n, _)| n == name) {
            refs.push((name.clone(), 0));
        }
    }
    let total: u64 = counts.values().sum();
    let count_of = |n: &str| counts.get(n).copied().unwrap_or(0);
    let counts_f: Vec<f64> = refs.iter().map(|(n, _)| count_of(n) as f64).collect();
    let lens_f: Vec<f64> = refs.iter().map(|(_, l)| *l as f64).collect();
    let t = tpm(&counts_f, &lens_f);
    Ok(refs
        .iter()
        .enumerate()
        .map(|(i, (n, l))| ExpressionRowDto {
            reference: n.clone(),
            length: *l as u64,
            count: count_of(n),
            rpkm: rpkm(count_of(n) as f64, *l as f64, total as f64),
            tpm: t[i],
        })
        .collect())
}

/// Sortiert die Befehle nach Treffer für `query` (unscharfe Suche über Titel und Stichwörter) und gibt die Indizes zurück.
#[uniffi::export]
pub fn command_rank(items: Vec<CommandItemDto>, query: String) -> Vec<u64> {
    let commands: Vec<Command> = items
        .into_iter()
        .map(|i| Command {
            id: i.title.clone(),
            title: i.title,
            keywords: i.keywords,
        })
        .collect();
    rank_commands(&commands, &query)
        .into_iter()
        .map(|i| i as u64)
        .collect()
}

/// Übersetzt eine technische Fehlermeldung in Titel und Lösungsvorschlag.
#[uniffi::export]
pub fn friendly_error(message: String) -> ErrorHelpDto {
    let h = core_friendly_error(&message);
    ErrorHelpDto {
        title: h.title,
        hint: h.hint,
    }
}

/// Setzt `path` an den Anfang der Liste der zuletzt geöffneten Projekte (höchstens `max` Einträge).
#[uniffi::export]
pub fn recent_add(list: Vec<String>, path: String, max: u64) -> Vec<String> {
    add_recent(&list, &path, max as usize)
}

/// Doppelt belegte Tastenkürzel: Paare von Indizes in `keys` (Tastenangaben wie `cmd+shift+k`).
#[uniffi::export]
pub fn shortcut_conflict_pairs(keys: Vec<String>) -> Vec<Vec<u64>> {
    let list: Vec<Shortcut> = keys
        .into_iter()
        .map(|k| Shortcut {
            id: k.clone(),
            keys: k,
        })
        .collect();
    find_conflicts(&list)
        .into_iter()
        .map(|(a, b)| vec![a as u64, b as u64])
        .collect()
}

/// Schwachstellen eines Proteintextes (z. B. aus einem CDS-Feature übersetzt).
#[uniffi::export]
pub fn protein_liabilities_text(protein: String) -> Vec<LiabilityDto> {
    find_liabilities(&protein)
        .into_iter()
        .map(|l| LiabilityDto {
            position: l.position as u64,
            length: l.length as u64,
            kind: l.kind,
            motif: l.motif,
        })
        .collect()
}

/// Wandelt `cmd+shift+k` in die Schreibweise `⇧⌘K` um.
#[uniffi::export]
pub fn format_shortcut(keys: String) -> String {
    core_format_shortcut(&keys)
}

/// Liest eine BLAST-Tabelle (`-outfmt 6`) für die Anzeige.
#[uniffi::export]
pub fn blast_table_open(path: String) -> Result<Vec<BlastHitDto>, ApiError> {
    let text = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
        message: format!("{}: {}", path, e),
    })?;
    Ok(parse_blast_table(&text)?
        .into_iter()
        .map(|r| BlastHitDto {
            subject: format!("{} → {}", r.query, r.subject),
            identity_percent: r.identity_percent,
            alignment_length: r.alignment_length as u64,
            query_start: r.query_start as u64,
            query_end: r.query_end as u64,
            subject_start: r.subject_start as u64,
            subject_end: r.subject_end as u64,
            reverse_strand: r.subject_end < r.subject_start,
            evalue: r.evalue,
            bit_score: r.bit_score,
        })
        .collect())
}

/// Angepasste p-Werte nach Benjamini-Hochberg (Mehrfachtest-Korrektur).
#[uniffi::export]
pub fn p_adjust_bh(p: Vec<f64>) -> Vec<f64> {
    benjamini_hochberg(&p)
}

/// Größenfaktoren (Median der Verhältnisse) für `counts[gen][probe]`.
#[uniffi::export]
pub fn size_factors(counts: Vec<Vec<f64>>) -> Vec<f64> {
    median_of_ratios(&counts)
}

/// Fragmentlängen gepaarter Reads aus einer SAM/BAM-Datei.
#[uniffi::export]
pub fn insert_size_report(path: String) -> Result<InsertReportDto, ApiError> {
    let p = Path::new(&path);
    let records = if path.to_lowercase().ends_with(".bam") {
        read_bam(p)?.records
    } else {
        io::sam::read(p)?.records
    };
    let sizes = insert_sizes(&records);
    let st = insert_stats(&sizes);
    let lens: Vec<usize> = sizes.iter().map(|s| *s as usize).collect();
    Ok(InsertReportDto {
        count: st.count as u64,
        mean: st.mean,
        median: st.median,
        std_dev: st.std_dev,
        bins: core_histogram(&lens, 25)
            .into_iter()
            .map(|(start, count)| HistogramBinDto {
                start: start as u64,
                count: count as u64,
            })
            .collect(),
    })
}

/// Häufigste k-mer der ersten `max_reads` Reads einer FASTQ-Datei (zum Erkennen von Adaptern und Verunreinigungen).
#[uniffi::export]
pub fn fastq_top_kmers(
    path: String,
    k: u64,
    top: u64,
    max_reads: u64,
) -> Result<Vec<KmerDto>, ApiError> {
    let mut seqs: Vec<String> = Vec::new();
    for rec in open_fastq(Path::new(&path))?.take(max_reads.max(1) as usize) {
        seqs.push(rec?.sequence);
    }
    let counts = count_kmers(&seqs, (k as usize).clamp(1, 32));
    Ok(top_kmers(&counts, top as usize)
        .into_iter()
        .map(|(kmer, count)| KmerDto {
            kmer,
            count: count as u64,
        })
        .collect())
}

/// Reihenfolge der Zeilen der Feature-Tabelle nach Filter und Sortierung. `type_filter` leer = alle Typen, `text` = Teil des Namens.
/// `column`: `type`, `name`, `start`, `end` oder `length`.
#[uniffi::export]
pub fn feature_table_order(
    features: Vec<FeatureInfo>,
    type_filter: String,
    text: String,
    column: String,
    ascending: bool,
) -> Vec<u64> {
    let rows: Vec<FeatureRow> = features
        .into_iter()
        .map(|f| FeatureRow {
            feature_type: f.feature_type,
            name: f.name,
            start: f.start as usize,
            end: f.end as usize,
        })
        .collect();
    let idx = filter_features(&rows, &type_filter, &text);
    let col = match column.as_str() {
        "type" => SortColumn::Type,
        "name" => SortColumn::Name,
        "end" => SortColumn::End,
        "length" => SortColumn::Length,
        _ => SortColumn::Start,
    };
    sort_features(&rows, &idx, col, ascending)
        .into_iter()
        .map(|i| i as u64)
        .collect()
}

/// Liest die Einstellungen aus `~/.snapious/settings.json`.
#[uniffi::export]
pub fn settings_load() -> SettingsDto {
    let s = load_settings(&data_dir().join("settings.json"));
    SettingsDto {
        ncbi_email: s.ncbi_email,
        ncbi_api_key: s.ncbi_api_key,
        default_organism: s.default_organism,
        dark_mode: s.dark_mode,
        undo_limit: s.undo_limit as u64,
    }
}

/// Speichert die Einstellungen (Datei nur für den Besitzer lesbar).
#[uniffi::export]
pub fn settings_save(s: SettingsDto) -> Result<(), ApiError> {
    let path = data_dir().join("settings.json");
    let mut current: Settings = load_settings(&path);
    current.ncbi_email = s.ncbi_email;
    current.ncbi_api_key = s.ncbi_api_key;
    current.default_organism = s.default_organism;
    current.dark_mode = s.dark_mode;
    current.undo_limit = s.undo_limit as usize;
    Ok(save_settings(&path, &current)?)
}

/// Namen der gefundenen Plugins mit ihren Operationen und die Warnungen des Ladens.
#[uniffi::export]
pub fn plugins_list() -> Vec<String> {
    let (plugins, warnings) = discover(&data_dir().join("plugins"));
    let mut out: Vec<String> = plugins
        .iter()
        .map(|p| {
            format!(
                "{} {} – {}",
                p.name,
                p.version,
                p.operations
                    .iter()
                    .map(|o| o.id.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    out.extend(warnings.into_iter().map(|w| format!("Warnung: {}", w)));
    out
}

/// Ordner der Plugins (wird bei Bedarf angelegt).
#[uniffi::export]
pub fn plugins_directory() -> String {
    let d = data_dir().join("plugins");
    let _ = std::fs::create_dir_all(&d);
    d.to_string_lossy().to_string()
}

/// Eine mit dem Projekt verknüpfte externe Datei.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ExternalFileDto {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub doc_id: String,
    pub label: String,
}

fn external_dto(f: snapious_core::store::ExternalFile) -> ExternalFileDto {
    ExternalFileDto {
        id: f.id,
        path: f.path,
        kind: f.kind,
        doc_id: f.doc_id,
        label: f.label,
    }
}

/// Ergebnis des Qualitätsberichts einer FASTQ-Datei.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct QcDto {
    pub reads: u64,
    pub min_length: u64,
    pub max_length: u64,
    pub mean_length: f64,
    pub gc_percent: f64,
    pub mean_quality: Vec<f64>,
    pub q25: Vec<f64>,
    pub median: Vec<f64>,
    pub q75: Vec<f64>,
    pub duplication_percent: f64,
    pub adapter_percent: f64,
    pub overrepresented: Vec<String>,
}

/// Statistik eines Trimming-Laufs.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TrimStatsDto {
    pub input_reads: u64,
    pub output_reads: u64,
    pub bases_removed: u64,
    pub adapters_found: u64,
}

/// Eine erkannte Einzelbasenvariante.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SnvDto {
    pub position: u64,
    pub ref_base: String,
    pub alt_base: String,
    pub depth: u64,
    pub alt_fraction: f64,
    pub quality: f64,
}

/// Qualitätsbericht einer FASTQ-Datei (auch `.gz`). Liest höchstens `max_reads` Reads (0 = alle).
#[uniffi::export]
pub fn qc_fastq(path: String, max_reads: u64) -> Result<QcDto, ApiError> {
    let reader = open_fastq(std::path::Path::new(&path))?;
    let mut reads = Vec::new();
    for r in reader {
        reads.push(r?);
        if max_reads > 0 && reads.len() as u64 >= max_reads {
            break;
        }
    }
    let q = analyze_reads(reads, "AGATCGGAAGAGC", 100_000);
    Ok(QcDto {
        reads: q.reads,
        min_length: q.min_length as u64,
        max_length: q.max_length as u64,
        mean_length: q.mean_length,
        gc_percent: q.gc_percent,
        mean_quality: q.mean_quality_per_position,
        q25: q.q25_per_position,
        median: q.median_per_position,
        q75: q.q75_per_position,
        duplication_percent: q.duplication_percent,
        adapter_percent: q.adapter_percent,
        overrepresented: q
            .overrepresented
            .into_iter()
            .map(|(s, n)| format!("{} ({}×)", s, n))
            .collect(),
    })
}

/// Trimmt eine FASTQ-Datei (Standardeinstellungen) und schreibt das Ergebnis nach `output` (Klartext-FASTQ).
#[uniffi::export]
pub fn trim_fastq(input: String, output: String) -> Result<TrimStatsDto, ApiError> {
    let mut reads = Vec::new();
    for r in open_fastq(std::path::Path::new(&input))? {
        reads.push(r?);
    }
    let (kept, stats) = trim_all(reads, &TrimOptions::standard());
    io::fastq::write(&kept, std::path::Path::new(&output))?;
    Ok(TrimStatsDto {
        input_reads: stats.input_reads,
        output_reads: stats.output_reads,
        bases_removed: stats.bases_removed,
        adapters_found: stats.adapters_found,
    })
}

/// Banden eines Größenstandards: `generic_1kb` oder `generic_100bp`.
#[uniffi::export]
pub fn gel_ladder(name: String, agarose_percent: f64) -> Result<Vec<GelBandDto>, ApiError> {
    let ladder = match name.as_str() {
        "generic_1kb" => Ladder::generic_1kb(),
        "generic_100bp" => Ladder::generic_100bp(),
        other => {
            return Err(ApiError::InvalidInput {
                message: format!("Unbekannter Größenstandard '{}'", other),
            })
        }
    };
    Ok(GelSimulator::new(agarose_percent)
        .ladder_lane(&ladder)
        .bands
        .into_iter()
        .map(|b| GelBandDto {
            size_bp: b.size_bp as u64,
            mobility: b.mobility,
            intensity: b.intensity,
        })
        .collect())
}

/// Liest eine Sequenzierdatei (`.ab1` im ABIF-Format oder `.scf` Version 3): Basen, Qualitäten, Peakpositionen und Spuren.
#[uniffi::export]
pub fn read_trace(path: String) -> Result<TraceDto, ApiError> {
    let bytes = std::fs::read(&path).map_err(|e| ApiError::Io {
        message: format!("Datei '{}' konnte nicht gelesen werden: {}", path, e),
    })?;
    if bytes.starts_with(b".scf") {
        let r = snapious_core::io::scf::parse_scf(&bytes)?;
        let samples = snapious_core::io::scf::parse_scf_samples(&bytes).unwrap_or_default();
        return Ok(TraceDto {
            bases: r.bases,
            quality: r.qualities,
            peak_positions: r.peaks.iter().map(|p| *p as u64).collect(),
            channels: samples
                .into_iter()
                .zip(["A", "C", "G", "T"])
                .map(|(v, b)| TraceChannelDto {
                    base: b.to_string(),
                    values: v.into_iter().map(i32::from).collect(),
                })
                .collect(),
        });
    }
    let c = parse_abif(&bytes)?;
    Ok(TraceDto {
        bases: c.bases,
        quality: c.quality,
        peak_positions: c.peak_positions.iter().map(|p| *p as u64).collect(),
        channels: c
            .channels
            .into_iter()
            .map(|(b, v)| TraceChannelDto {
                base: b.to_string(),
                values: v.into_iter().map(i32::from).collect(),
            })
            .collect(),
    })
}

/// Version des Kerns (zum Prüfen, ob die Bibliothek geladen ist).
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mini_path() -> String {
        format!(
            "{}/../snapious-core/testdata/mini.gb",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    fn projekt() -> (tempfile::TempDir, Arc<Project>) {
        let dir = tempfile::tempdir().unwrap();
        let p = Project::open(dir.path().join("t.snapious").to_string_lossy().to_string()).unwrap();
        (dir, p)
    }

    #[test]
    fn version() {
        assert_eq!(core_version(), "0.1.0");
    }

    #[test]
    fn import_liste_und_zusammenfassung() {
        let (_d, p) = projekt();
        assert!(p.documents().unwrap().is_empty());
        let neu = p.import_file(mini_path()).unwrap();
        assert_eq!(neu.len(), 1);
        assert_eq!(neu[0].name, "mini");
        let liste = p.documents().unwrap();
        assert_eq!(liste, neu);
        let s = p.summary(neu[0].id.clone()).unwrap();
        assert_eq!(s.length, 60);
        assert_eq!(s.topology, "CIRCULAR");
        assert_eq!(s.seq_type, "DNA");
        assert_eq!(s.feature_count, 2);
        assert_eq!(s.accession, "TEST0001");
        assert!((s.gc_percent - 50.0).abs() < 1e-9);
    }

    #[test]
    fn features_und_basen() {
        let (_d, p) = projekt();
        let id = p.import_file(mini_path()).unwrap().remove(0).id;
        let f = p.features(id.clone()).unwrap();
        assert_eq!(
            (f[0].feature_type.as_str(), f[0].start, f[0].end),
            ("gene", 4, 20)
        );
        assert_eq!(f[1].strand, "REVERSE");
        assert_eq!(p.bases(id.clone(), 0, 8).unwrap(), "ATGCATGC");
        assert_eq!(p.bases(id.clone(), 56, 1000).unwrap(), "ATGC");
        assert!(p.bases(id, 10, 5).is_err());
    }

    #[test]
    fn umbenennen_und_loeschen() {
        let (_d, p) = projekt();
        let id = p.import_file(mini_path()).unwrap().remove(0).id;
        p.rename_document(id.clone(), "neu".to_string()).unwrap();
        assert_eq!(p.documents().unwrap()[0].name, "neu");
        assert_eq!(p.summary(id.clone()).unwrap().length, 60);
        p.delete_document(id.clone()).unwrap();
        assert!(p.documents().unwrap().is_empty());
        assert!(matches!(p.summary(id), Err(ApiError::NotFound { .. })));
    }

    #[test]
    fn fehlerfaelle() {
        let (d, p) = projekt();
        assert!(matches!(
            p.import_file("/gibt/es/nicht".to_string()),
            Err(ApiError::Io { .. })
        ));
        let leer = d.path().join("leer.fa");
        std::fs::write(&leer, "kein fasta").unwrap();
        assert!(p.import_file(leer.to_string_lossy().to_string()).is_err());
        assert!(matches!(
            p.features("x".to_string()),
            Err(ApiError::NotFound { .. })
        ));
        assert!(Project::open("/gibt/es/nicht/p.snapious".to_string()).is_err());
    }

    #[test]
    fn projekt_bleibt_nach_neu_oeffnen_erhalten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.snapious").to_string_lossy().to_string();
        {
            let p = Project::open(path.clone()).unwrap();
            p.import_file(mini_path()).unwrap();
        }
        let p = Project::open(path.clone()).unwrap();
        assert_eq!(p.documents().unwrap().len(), 1);
        assert_eq!(p.path(), path);
    }

    #[test]
    fn import_schreibt_verlauf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.snapious");
        let p = Project::open(path.to_string_lossy().to_string()).unwrap();
        p.import_file(mini_path()).unwrap();
        drop(p);
        let store = ProjectStore::open(&path).unwrap();
        let h = store.history().unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].operation_id, "import.file");
        assert_eq!(h[0].output_ids.len(), 1);
    }

    #[test]
    fn fasta_import_mit_mehreren_sequenzen() {
        let (d, p) = projekt();
        let fa = d.path().join("x.fa");
        std::fs::write(&fa, ">a\nACGT\n>b\nGGGGCCCC\n").unwrap();
        let neu = p.import_file(fa.to_string_lossy().to_string()).unwrap();
        assert_eq!(neu.len(), 2);
        assert_eq!(p.summary(neu[1].id.clone()).unwrap().gc_percent, 100.0);
    }

    #[test]
    fn ansichtsdaten() {
        let (_d, p) = projekt();
        let id = p.import_file(mini_path()).unwrap().remove(0).id;
        let lanes = p.feature_lanes(id.clone()).unwrap();
        assert!(!lanes.is_empty());
        assert!(lanes.iter().all(|l| l.end > l.start));
        let s = p.selection_stats(id.clone(), 0, 8).unwrap();
        assert_eq!(s.length, 8);
        assert!((s.gc_percent - 50.0).abs() < 1e-9);
        assert_eq!(
            p.selection_text(id.clone(), 0, 4, "bases".to_string())
                .unwrap(),
            "ATGC"
        );
        assert_eq!(
            p.selection_text(id.clone(), 0, 4, "reverse_complement".to_string())
                .unwrap(),
            "GCAT"
        );
        assert!(p
            .selection_text(id.clone(), 0, 4, "fasta".to_string())
            .unwrap()
            .starts_with('>'));
        assert!(p.selection_text(id.clone(), 0, 4, "x".to_string()).is_err());
        assert!(p.enzyme_sites(id.clone(), "all".to_string()).is_ok());
        assert!(p.enzyme_sites(id.clone(), "x".to_string()).is_err());
        assert!(p.translate_feature(id.clone(), 999).is_err());
        assert!(p.enzyme_list().len() >= 79);
        let f = p
            .digest_document(id.clone(), vec!["EcoRI".to_string()])
            .unwrap();
        assert!(!f.is_empty());
        assert!(p.find_guides(id.clone(), "spcas9".to_string()).is_ok());
        assert!(p.find_guides(id.clone(), "x".to_string()).is_err());
        assert!(p
            .restriction_candidates(
                id.clone(),
                vec!["EcoRI".to_string()],
                id.clone(),
                vec!["EcoRI".to_string()]
            )
            .is_ok());
        assert!(p
            .save_candidate(
                id.clone(),
                vec!["Nix".to_string()],
                id.clone(),
                vec![],
                0,
                String::new()
            )
            .is_err());
        assert!(p.pick_primers(id.clone(), 10, 5, 3).is_err());
        assert!(!p
            .find_motif_hits(id.clone(), "ATGC".to_string())
            .unwrap()
            .is_empty());
        assert!(p.find_motif_hits(id.clone(), "".to_string()).is_err());
        let orfs = p.find_orfs(id.clone(), 3).unwrap();
        let _ = p.detect_common_features(id.clone()).unwrap();
        let vorher = p.features(id.clone()).unwrap().len();
        p.add_features(
            id.clone(),
            vec![FeatureInfo {
                feature_type: "misc_feature".to_string(),
                name: "x".to_string(),
                start: 1,
                end: 5,
                strand: "FORWARD".to_string(),
            }],
        )
        .unwrap();
        assert_eq!(p.features(id.clone()).unwrap().len(), vorher + 1);
        p.remove_feature(id.clone(), vorher as u64).unwrap();
        assert_eq!(p.features(id.clone()).unwrap().len(), vorher);
        let _ = orfs;
        assert!(p
            .assemble_overlap(vec![id.clone()], "x".to_string(), false)
            .is_err());
        assert!(p
            .assemble_golden_gate(vec![id.clone()], "Nix".to_string(), false)
            .is_err());
        let before = p.summary(id.clone()).unwrap().length;
        p.edit_insert(id.clone(), 0, "AAA".to_string()).unwrap();
        assert_eq!(p.summary(id.clone()).unwrap().length, before + 3);
        assert_eq!(p.edit_state(id.clone()).unwrap(), vec![3, 0]);
        p.edit_delete(id.clone(), 0, 3).unwrap();
        assert_eq!(p.summary(id.clone()).unwrap().length, before);
        assert!(p.edit_undo(id.clone()).unwrap());
        assert_eq!(p.summary(id.clone()).unwrap().length, before + 3);
        assert!(p.edit_redo(id.clone()).unwrap());
        assert!(p.edit_undo(id.clone()).unwrap() && p.edit_undo(id.clone()).unwrap());
        while p.edit_undo(id.clone()).unwrap() {}
        assert_eq!(p.summary(id.clone()).unwrap().length, before);
        assert!(p.edit_delete(id.clone(), 5, 99999).is_err());
        assert!(p.digest_document(id, vec!["Nix".to_string()]).is_err());
        let g = gel_lane(vec![1000, 2000], 1.0, 100.0);
        assert_eq!(g.len(), 2);
        assert!((g[0].intensity - 1.0).abs() < 1e-9);
        assert_eq!(
            gel_ladder("generic_1kb".to_string(), 1.0).unwrap().len(),
            13
        );
        assert!(gel_ladder("x".to_string(), 1.0).is_err());
    }

    #[test]
    fn alignment_und_baum() {
        let (d, p) = projekt();
        let fa = d.path().join("x.fa");
        std::fs::write(
            &fa,
            ">a\nACGTACGTACGTAAAA\n>b\nACGTACGTACGTAAAT\n>c\nACGAACGTACCTAAAA\n",
        )
        .unwrap();
        let ids: Vec<String> = p
            .import_file(fa.to_string_lossy().to_string())
            .unwrap()
            .into_iter()
            .map(|i| i.id)
            .collect();
        assert!(p
            .align_documents(ids[..1].to_vec(), String::new(), "builtin".to_string())
            .is_err());
        let aln = p
            .align_documents(ids.clone(), "Test".to_string(), "builtin".to_string())
            .unwrap();
        assert_eq!(aln.doc_type, "alignment");
        let view = p.alignment(aln.id.clone()).unwrap();
        assert_eq!(view.rows.len(), 3);
        assert_eq!(view.identity.len(), view.rows[0].chars().count());
        assert!(p.summary(aln.id.clone()).is_err());
        let stripped = p.alignment_strip(aln.id.clone(), 1.0).unwrap();
        assert_eq!(stripped.doc_type, "alignment");
        let cons = p.alignment_consensus(aln.id.clone()).unwrap();
        assert_eq!(cons.doc_type, "sequence");
        let out = d.path().join("aus.aln");
        p.export_document(
            aln.id.clone(),
            out.to_string_lossy().to_string(),
            "clustal".to_string(),
        )
        .unwrap();
        assert!(std::fs::read_to_string(&out)
            .unwrap()
            .starts_with("CLUSTAL"));
        let map = d.path().join("karte.svg");
        p.export_document(
            ids[0].clone(),
            map.to_string_lossy().to_string(),
            "svg".to_string(),
        )
        .unwrap();
        assert!(std::fs::read_to_string(&map).unwrap().contains("<svg"));
        assert!(p.alignment(ids[0].clone()).is_err());
    }

    #[test]
    fn workflows() {
        let (_d, p) = projekt();
        let id = p.import_file(mini_path()).unwrap().remove(0).id;
        assert!(p.operation_list().len() >= 14);
        let json = r#"{"name":"rc+ins","steps":[{"operation":"edit.reverseComplement","inputs":["$in:0"]},
            {"operation":"edit.insert","parameters":{"position":1,"bases":"GG"},"inputs":["$step:0:0"]}],"outputs":["$step:1:0"]}"#;
        assert_eq!(p.workflow_save(json.to_string()).unwrap(), "rc+ins");
        assert!(p.workflow_save("kaputt".to_string()).is_err());
        assert!(p.workflow_save(r#"{"name":"x","steps":[{"operation":"nix","inputs":["$in:0"]}],"outputs":["$step:0:0"]}"#.to_string()).is_err());
        let list = p.workflow_list().unwrap();
        assert_eq!(
            (list.len(), list[0].input_count, list[0].step_count),
            (1, 1, 2)
        );
        let r = p
            .workflow_run("rc+ins".to_string(), vec![id.clone()])
            .unwrap();
        assert!(r.errors.is_empty());
        assert_eq!(r.documents.len(), 1);
        assert_eq!(p.summary(r.documents[0].id.clone()).unwrap().length, 62);
        assert!(p
            .workflow_run("gibtsnicht".to_string(), vec![id.clone()])
            .is_err());
        p.workflow_delete("rc+ins".to_string()).unwrap();
        assert!(p.workflow_list().unwrap().is_empty());
    }

    #[test]
    fn annotationen_importieren() {
        let (d, p) = projekt();
        let id = p.import_file(mini_path()).unwrap().remove(0).id;
        let vorher = p.features(id.clone()).unwrap().len();
        let bed = d.path().join("x.bed");
        std::fs::write(&bed, "chrA\t5\t15\tmotiv\t10\t+\n").unwrap();
        assert_eq!(
            p.import_annotations(id.clone(), bed.to_string_lossy().to_string())
                .unwrap(),
            1
        );
        let gff = d.path().join("x.gff3");
        std::fs::write(
            &gff,
            "##gff-version 3\nA\t.\tgene\t1\t9\t.\t+\t.\tID=g\nB\t.\tgene\t1\t9\t.\t+\t.\tID=h\n",
        )
        .unwrap();
        assert!(p
            .import_annotations(id.clone(), gff.to_string_lossy().to_string())
            .is_err());
        assert_eq!(p.features(id.clone()).unwrap().len(), vorher + 1);
        let gff_aus = d.path().join("aus.gff3");
        p.export_document(
            id.clone(),
            gff_aus.to_string_lossy().to_string(),
            "gff3".to_string(),
        )
        .unwrap();
        assert!(std::fs::read_to_string(&gff_aus)
            .unwrap()
            .starts_with("##gff-version 3"));
        assert!(p.import_annotations(id, "x.txt".to_string()).is_err());
    }

    #[test]
    fn rueckuebersetzung_zusammenfuegen_und_mehr() {
        let (_d, p) = projekt();
        assert_eq!(
            back_translate("MKV".into(), "degenerate".into()).unwrap(),
            "ATGAARGTN"
        );
        assert!(back_translate("MKX".into(), "preferred".into()).is_err());
        let prot = p
            .create_sequence("prot".into(), "MKV".into(), "PROTEIN".into(), false)
            .unwrap();
        let dna = p
            .back_translate_document(prot[0].id.clone(), "preferred".into(), String::new())
            .unwrap();
        assert_eq!(p.bases(dna.id.clone(), 0, 100).unwrap(), "ATGAAAGTG");
        assert!(p
            .back_translate_document(dna.id.clone(), "preferred".into(), String::new())
            .is_err());

        let a = p
            .create_sequence("a".into(), "ACGT".into(), "DNA".into(), false)
            .unwrap();
        let b = p
            .create_sequence("b".into(), "TTGCA".into(), "DNA".into(), false)
            .unwrap();
        let c = p
            .concatenate_documents(vec![a[0].id.clone(), b[0].id.clone()], "ab".into(), true)
            .unwrap();
        assert_eq!(p.bases(c.id.clone(), 0, 100).unwrap(), "ACGTTTGCA");
        assert!(p
            .concatenate_documents(vec![a[0].id.clone(), prot[0].id.clone()], "x".into(), false)
            .is_err());

        // Stille Änderungen im CDS CTGAAA (Leu Lys)
        let s = p
            .create_sequence("cds".into(), "GAGTTC".into(), "DNA".into(), false)
            .unwrap();
        p.add_features(
            s[0].id.clone(),
            vec![FeatureInfo {
                feature_type: "CDS".into(),
                name: "x".into(),
                start: 0,
                end: 6,
                strand: "FORWARD".into(),
            }],
        )
        .unwrap();
        let create = p
            .silent_changes(s[0].id.clone(), 0, "GAATTC".into(), "create".into())
            .unwrap();
        assert_eq!(create.len(), 1);
        assert_eq!(
            (
                create[0].position,
                create[0].from.as_str(),
                create[0].to.as_str(),
                create[0].codon
            ),
            (2, "G", "A", 1)
        );
        let by_enzyme = p
            .silent_changes(s[0].id.clone(), 0, "EcoRI".into(), "create".into())
            .unwrap();
        assert_eq!(by_enzyme, create);
        assert!(p
            .silent_changes(s[0].id.clone(), 0, "gibtsnicht".into(), "create".into())
            .is_err());
        assert!(p
            .silent_changes(s[0].id.clone(), 0, String::new(), "remove".into())
            .is_err());
        assert!(!p
            .silent_changes(s[0].id.clone(), 0, String::new(), "all".into())
            .unwrap()
            .is_empty());

        // Annotationen vergleichen
        p.add_features(
            a[0].id.clone(),
            vec![FeatureInfo {
                feature_type: "gene".into(),
                name: "g".into(),
                start: 0,
                end: 3,
                strand: "FORWARD".into(),
            }],
        )
        .unwrap();
        p.add_features(
            b[0].id.clone(),
            vec![FeatureInfo {
                feature_type: "gene".into(),
                name: "h".into(),
                start: 2,
                end: 5,
                strand: "FORWARD".into(),
            }],
        )
        .unwrap();
        let ex = p
            .compare_annotations(a[0].id.clone(), b[0].id.clone(), "exact".into())
            .unwrap();
        assert_eq!(
            (ex.only_a.len(), ex.only_b.len(), ex.common_a.len()),
            (1, 1, 0)
        );
        let ov = p
            .compare_annotations(a[0].id.clone(), b[0].id.clone(), "overlap".into())
            .unwrap();
        assert_eq!((ov.common_a, ov.common_b), (vec![0], vec![0]));

        // Alignment: Spalten entfernen und Varianten anwenden
        let x = p
            .create_sequence("x".into(), "ACGTAC".into(), "DNA".into(), false)
            .unwrap();
        let y = p
            .create_sequence("y".into(), "ACCTAC".into(), "DNA".into(), false)
            .unwrap();
        let aln = p
            .align_documents(
                vec![x[0].id.clone(), y[0].id.clone()],
                "aln".into(),
                "builtin".into(),
            )
            .unwrap();
        let cut = p
            .strip_alignment_columns(aln.id.clone(), "codon".into(), 3.0, 0, String::new())
            .unwrap();
        assert_eq!(p.alignment(cut.id.clone()).unwrap().rows[0].len(), 4);
        assert!(p
            .strip_alignment_columns(aln.id.clone(), "codon".into(), 4.0, 0, String::new())
            .is_err());
        assert!(p
            .strip_alignment_columns(aln.id.clone(), "gaps".into(), 0.0, 0, String::new())
            .is_err());
        let v = p
            .sequence_with_variants(aln.id.clone(), 0, 1, String::new())
            .unwrap();
        assert_eq!(p.bases(v.id, 0, 100).unwrap(), "ACCTAC");
    }

    #[test]
    fn formate_tabellen_und_ngs() {
        let (dir, p) = projekt();
        let d = dir.path();
        // MEGA und PIR importieren
        std::fs::write(d.join("a.meg"), "#mega\n!Title t;\n#a\nACGT\n#b\nAC-T\n").unwrap();
        let al = p
            .import_file(d.join("a.meg").to_string_lossy().to_string())
            .unwrap();
        assert_eq!(al[0].doc_type, "alignment");
        let exp = d.join("a_out.meg");
        p.export_document(
            al[0].id.clone(),
            exp.to_string_lossy().to_string(),
            "mega".into(),
        )
        .unwrap();
        assert!(std::fs::read_to_string(&exp).unwrap().starts_with("#mega"));
        std::fs::write(d.join("p.pir"), ">P1;S1\nEin Protein\nMKVL*\n").unwrap();
        let pi = p
            .import_file(d.join("p.pir").to_string_lossy().to_string())
            .unwrap();
        assert_eq!(p.summary(pi[0].id.clone()).unwrap().seq_type, "PROTEIN");

        // Tabelle
        std::fs::write(d.join("t.csv"), "Name,Seq\na,ACGT\n\"b c\",ttgg\n").unwrap();
        let prev =
            table_preview(d.join("t.csv").to_string_lossy().to_string(), ",".into(), 2).unwrap();
        assert_eq!(prev.len(), 2);
        let t = p
            .import_table(
                d.join("t.csv").to_string_lossy().to_string(),
                ",".into(),
                0,
                1,
                true,
                "DNA".into(),
                false,
            )
            .unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[1].name, "b c");
        assert!(p
            .import_table(
                d.join("t.csv").to_string_lossy().to_string(),
                ",".into(),
                0,
                7,
                true,
                "DNA".into(),
                false
            )
            .is_err());
        let hist = p
            .length_histogram(vec![t[0].id.clone(), t[1].id.clone()], 2)
            .unwrap();
        assert_eq!(
            hist,
            vec![
                HistogramBinDto { start: 0, count: 0 },
                HistogramBinDto { start: 2, count: 0 },
                HistogramBinDto { start: 4, count: 2 }
            ]
        );

        // Features als CSV
        p.add_features(
            t[0].id.clone(),
            vec![FeatureInfo {
                feature_type: "gene".into(),
                name: "g".into(),
                start: 0,
                end: 3,
                strand: "FORWARD".into(),
            }],
        )
        .unwrap();
        assert_eq!(
            p.export_text(t[0].id.clone(), "features_csv".into())
                .unwrap(),
            "Typ,Name,Start,Ende,Strang,Länge\ngene,g,1,3,+,3\n"
        );
        // Schnitt nach Annotation
        let fr = p
            .fragments_by_annotation(t[0].id.clone(), "gene".into(), true, true)
            .unwrap();
        assert_eq!(fr.iter().map(|f| f.length).collect::<Vec<_>>(), vec![3, 1]);

        // Reads paaren
        let r1 = p
            .create_sequence("r1/1".into(), "ACGT".into(), "DNA".into(), false)
            .unwrap();
        let r2 = p
            .create_sequence("r1/2".into(), "TTGG".into(), "DNA".into(), false)
            .unwrap();
        let rep = p
            .pair_reads_report(vec![r1[0].id.clone(), r2[0].id.clone(), t[0].id.clone()])
            .unwrap();
        assert_eq!(
            rep.pairs,
            vec![vec!["r1/1".to_string(), "r1/2".to_string()]]
        );
        assert_eq!(rep.unpaired, vec!["a".to_string()]);

        // Barcodes und QUAL
        let fq = d.join("r.fastq");
        std::fs::write(
            &fq,
            "@x1\nACGTGGGG\n+\nIIIIJJJJ\n@x2\nTTGACC\n+\nIIIIJJ\n@x3\nGGGGGG\n+\nIIIIII\n",
        )
        .unwrap();
        let out = d.join("out").to_string_lossy().to_string();
        let g = split_fastq_by_barcode(
            fq.to_string_lossy().to_string(),
            "b1,ACGT\nb2 TTGA\n# Kommentar\n".into(),
            0,
            out.clone(),
        )
        .unwrap();
        assert_eq!(
            g.iter()
                .map(|x| (x.name.as_str(), x.count))
                .collect::<Vec<_>>(),
            vec![("b1", 1), ("b2", 1), ("unassigned", 1)]
        );
        assert_eq!(
            std::fs::read_to_string(&g[0].path).unwrap(),
            "@x1\nGGGG\n+\nJJJJ\n"
        );
        assert!(
            split_fastq_by_barcode(fq.to_string_lossy().to_string(), "nurname".into(), 0, out)
                .is_err()
        );
        let n = fastq_to_fasta_qual(
            fq.to_string_lossy().to_string(),
            d.join("r.fa").to_string_lossy().to_string(),
            d.join("r.qual").to_string_lossy().to_string(),
        )
        .unwrap();
        assert_eq!(n, 3);
        assert!(std::fs::read_to_string(d.join("r.qual"))
            .unwrap()
            .starts_with(">x1\n40 40 40 40 41 41 41 41\n"));

        // Expression
        std::fs::write(d.join("e.sam"), "@SQ\tSN:g1\tLN:1000\n@SQ\tSN:g2\tLN:2000\nq1\t0\tg1\t1\t60\t4M\t*\t0\t0\tACGT\tIIII\nq2\t0\tg2\t1\t60\t4M\t*\t0\t0\tACGT\tIIII\nq3\t0\tg2\t5\t60\t4M\t*\t0\t0\tACGT\tIIII\nq4\t4\t*\t0\t0\t*\t*\t0\t0\tACGT\tIIII\n").unwrap();
        let ex = expression_from_alignment(d.join("e.sam").to_string_lossy().to_string()).unwrap();
        assert_eq!((ex[0].count, ex[1].count), (1, 2));
        assert!((ex[0].tpm + ex[1].tpm - 1e6).abs() < 1e-3);

        // Struktur
        std::fs::write(
            d.join("w.xyz"),
            "3\nwater\nO 0 0 0\nH 0.757 0.586 0\nH -0.757 0.586 0\n",
        )
        .unwrap();
        let st = structure_open(d.join("w.xyz").to_string_lossy().to_string()).unwrap();
        assert_eq!(
            (st.atom_count, st.atoms.len(), st.elements.len()),
            (3, 9, 3)
        );
    }

    #[test]
    fn auswertungen_und_neue_formate() {
        let (dir, p) = projekt();
        let d = dir.path();
        // Import: csfasta
        std::fs::write(d.join("a.csfasta"), "# x\n>r1\nT0123\n").unwrap();
        let c = p
            .import_file(d.join("a.csfasta").to_string_lossy().to_string())
            .unwrap();
        assert_eq!(p.bases(c[0].id.clone(), 0, 100).unwrap(), "TTGAT");
        // CML
        std::fs::write(d.join("w.cml"), "<molecule><atomArray><atom id=\"a1\" elementType=\"O\" x3=\"0\" y3=\"0\" z3=\"0\"/><atom id=\"a2\" elementType=\"H\" x3=\"1\" y3=\"0\" z3=\"0\"/></atomArray><bondArray><bond atomRefs2=\"a1 a2\"/></bondArray></molecule>").unwrap();
        let st = structure_open(d.join("w.cml").to_string_lossy().to_string()).unwrap();
        assert_eq!((st.elements.len(), st.bonds.len()), (2, 2));

        // Alignment: Ähnlichkeit und Chimäre
        let eltern_a = p
            .create_sequence("A".into(), "AAAAAAAA".into(), "DNA".into(), false)
            .unwrap();
        let eltern_b = p
            .create_sequence("B".into(), "CCCCCCCC".into(), "DNA".into(), false)
            .unwrap();
        let chim = p
            .create_sequence("Q".into(), "AAAACCCC".into(), "DNA".into(), false)
            .unwrap();
        let aln = p
            .align_documents(
                vec![
                    eltern_a[0].id.clone(),
                    eltern_b[0].id.clone(),
                    chim[0].id.clone(),
                ],
                "al".into(),
                "builtin".into(),
            )
            .unwrap();
        let ident = p.alignment_similarity(aln.id.clone(), 1).unwrap();
        assert_eq!(ident.len(), 8);

        // Extrahieren und Feature-Suche
        let s = p
            .create_sequence("s".into(), "AACCGGTTAACC".into(), "DNA".into(), false)
            .unwrap();
        p.add_features(
            s[0].id.clone(),
            vec![FeatureInfo {
                feature_type: "gene".into(),
                name: "g".into(),
                start: 2,
                end: 4,
                strand: "REVERSE".into(),
            }],
        )
        .unwrap();
        let ex = p
            .extract_features(s[0].id.clone(), "gene".into(), 0)
            .unwrap();
        assert_eq!(p.bases(ex[0].id.clone(), 0, 100).unwrap(), "GG");
        assert!(p
            .extract_features(s[0].id.clone(), "cds".into(), 0)
            .is_err());
        let m = p
            .find_similar_features(s[0].id.clone(), "AACC".into(), 0)
            .unwrap();
        assert_eq!(
            m.iter().map(|x| (x.start, x.reverse)).collect::<Vec<_>>(),
            vec![(0, false), (4, true), (8, false)]
        );
    }

    #[test]
    fn runde_vier_funktionen() {
        let (dir, p) = projekt();
        let d = dir.path();
        // Schwachstellen und Proteinwerte
        let pr = p
            .create_sequence("P1".into(), "ANSTGDSMWC".into(), "PROTEIN".into(), false)
            .unwrap();
        let l = p.protein_liabilities(pr[0].id.clone()).unwrap();
        assert_eq!(l.len(), 6);
        assert_eq!(
            (l[0].kind.as_str(), l[0].motif.as_str()),
            ("N-Glykosylierung", "NST")
        );
        let props = p.protein_properties("AVIL".into());
        assert!((props.aliphatic_index - 292.5).abs() < 1e-9);
        // Domänentabelle
        std::fs::write(
            d.join("h.tbl"),
            "# x\nPkinase PF00069.28 264 P1 - 10 1.2e-50 170.5 0.1 1 1 2.3e-54 4.5e-50 160.2 0.0 1 262 2 8 1 10 0.95 Protein kinase domain\n",
        )
        .unwrap();
        assert_eq!(
            p.import_domain_table(
                pr[0].id.clone(),
                d.join("h.tbl").to_string_lossy().to_string(),
                "hmmer".into()
            )
            .unwrap(),
            1
        );
        assert_eq!(
            p.features(pr[0].id.clone()).unwrap()[0].feature_type,
            "domain"
        );
        assert!(p
            .import_domain_table(
                pr[0].id.clone(),
                d.join("h.tbl").to_string_lossy().to_string(),
                "interpro".into()
            )
            .is_err());
        // einfache Faltung
        let rna = p
            .create_sequence("r".into(), "GGGAAACCC".into(), "DNA".into(), false)
            .unwrap();
        let f = p.simple_fold(rna[0].id.clone(), 0, 0).unwrap();
        assert_eq!((f.pairs, f.structure.as_str()), (3, "(((...)))"));
        // BLAST-Tabelle, Kürzel, Statistik
        std::fs::write(
            d.join("b.tsv"),
            "q1\ts1\t99.5\t200\t1\t0\t1\t200\t1000\t1199\t1e-100\t350.5\n",
        )
        .unwrap();
        let b = blast_table_open(d.join("b.tsv").to_string_lossy().to_string()).unwrap();
        assert_eq!((b.len(), b[0].subject.as_str()), (1, "q1 → s1"));
        assert_eq!(format_shortcut("cmd+shift+k".into()), "⇧⌘K");
        assert_eq!(
            p_adjust_bh(vec![0.01, 0.04, 0.03, 0.005]),
            vec![0.02, 0.04, 0.04, 0.02]
        );
        let sf = size_factors(vec![vec![10.0, 20.0], vec![100.0, 200.0]]);
        assert!((sf[0] - 0.5_f64.sqrt()).abs() < 1e-9);
        // Abdeckung, Duplikate, Fragmentlängen
        let refseq = p
            .create_sequence("chr".into(), "ACGTACGTAC".repeat(3), "DNA".into(), false)
            .unwrap();
        std::fs::write(
            d.join("c.sam"),
            "@SQ\tSN:chr\tLN:30\nq1\t99\tchr\t1\t60\t10M\t=\t11\t300\tACGTACGTAC\tIIIIIIIIII\nq2\t147\tchr\t11\t60\t10M\t=\t1\t-300\tACGTACGTAC\tIIIIIIIIII\nq3\t99\tchr\t1\t60\t10M\t=\t11\t320\tACGTACGTAC\tIIIIIIIIII\n",
        )
        .unwrap();
        let cr = p
            .coverage_report(
                refseq[0].id.clone(),
                d.join("c.sam").to_string_lossy().to_string(),
            )
            .unwrap();
        assert_eq!((cr.mapped_reads, cr.duplicate_reads), (3, 1));
        assert!(cr.fractions[0] > 0.6 && cr.max == 2);
        let ir = insert_size_report(d.join("c.sam").to_string_lossy().to_string()).unwrap();
        assert_eq!((ir.count, ir.mean), (2, 310.0));
        // k-mer
        std::fs::write(
            d.join("k.fastq"),
            "@a\nACGTACGT\n+\nIIIIIIII\n@b\nACGTAAAA\n+\nIIIIIIII\n",
        )
        .unwrap();
        let km =
            fastq_top_kmers(d.join("k.fastq").to_string_lossy().to_string(), 4, 1, 100).unwrap();
        assert_eq!((km[0].kmer.as_str(), km[0].count), ("ACGT", 3));
        // Feature-Tabelle
        let feats = vec![
            FeatureInfo {
                feature_type: "gene".into(),
                name: "lacZ".into(),
                start: 100,
                end: 400,
                strand: "FORWARD".into(),
            },
            FeatureInfo {
                feature_type: "CDS".into(),
                name: "AmpR".into(),
                start: 50,
                end: 950,
                strand: "FORWARD".into(),
            },
        ];
        assert_eq!(
            feature_table_order(
                feats.clone(),
                String::new(),
                String::new(),
                "name".into(),
                true
            ),
            vec![1, 0]
        );
        assert_eq!(
            feature_table_order(feats, "gene".into(), String::new(), "start".into(), true),
            vec![0]
        );
        // Struktur aus HIN
        std::fs::write(d.join("w.hin"), "forcefield mm+\nmol 1\natom 1 - O ** - 0.0 0.0 0.0 2 2 s 3 s\natom 2 - H ** - 0.757 0.586 0.0 1 1 s\natom 3 - H ** - -0.757 0.586 0.0 1 1 s\nendmol 1\n").unwrap();
        let st = structure_open(d.join("w.hin").to_string_lossy().to_string()).unwrap();
        assert_eq!((st.elements.len(), st.bonds.len()), (3, 4));
    }

    #[test]
    fn ta_topo_gateway() {
        let (_d, p) = projekt();
        let v = p
            .create_sequence("T".into(), "GGGGTTTT".into(), "DNA".into(), false)
            .unwrap();
        let ins = p
            .create_sequence("Ins".into(), "ACGTAC".into(), "DNA".into(), false)
            .unwrap();
        let ta = p.clone_ta(v[0].id.clone(), ins[0].id.clone()).unwrap();
        assert_eq!(ta.len(), 2);
        assert_eq!(p.bases(ta[0].id.clone(), 0, 100).unwrap(), "GGGGTTTTACGTAC");
        assert_eq!(p.summary(ta[0].id.clone()).unwrap().topology, "CIRCULAR");
        assert!(p.clone_topo(v[0].id.clone(), ins[0].id.clone()).is_err());
        let cacc = p
            .create_sequence("C".into(), "CACCATG".into(), "DNA".into(), false)
            .unwrap();
        let tp = p.clone_topo(v[0].id.clone(), cacc[0].id.clone()).unwrap();
        assert_eq!(p.bases(tp.id, 0, 100).unwrap(), "GGGGTTTTCACCATG");

        // Gateway BP mit frei erfundenen Teststellen (nur zum Prüfen des Ablaufs)
        let site = |n: &str, core: &str| GatewaySiteDto {
            name: n.into(),
            left_arm_b: "AAA".into(),
            core: core.into(),
            right_arm_b: "TTT".into(),
            left_arm_p: "CCC".into(),
            right_arm_p: "GGG".into(),
        };
        let prod = p
            .create_sequence(
                "P".into(),
                "GGAAAGATCTCTTTXXXXXAAAGATCCCTTTGG".replace('X', "A"),
                "DNA".into(),
                false,
            )
            .unwrap();
        let donor = p
            .create_sequence(
                "D".into(),
                "CCCGATCTCGGGAAAACCCGATCCCGGGTT".into(),
                "DNA".into(),
                true,
            )
            .unwrap();
        let bp = p
            .clone_gateway(
                "bp".into(),
                prod[0].id.clone(),
                donor[0].id.clone(),
                site("1", "GATCTC"),
                site("2", "GATCCC"),
            )
            .unwrap();
        assert!(p.bases(bp.id, 0, 200).unwrap().contains("AAAGATCTCGGG"));
        assert!(p
            .clone_gateway(
                "xx".into(),
                prod[0].id.clone(),
                donor[0].id.clone(),
                site("1", "GATCTC"),
                site("2", "GATCCC")
            )
            .is_err());
        assert!(p
            .clone_gateway(
                "bp".into(),
                prod[0].id.clone(),
                donor[0].id.clone(),
                site("1", ""),
                site("2", "GATCCC")
            )
            .is_err());
    }

    #[test]
    fn ab1_wird_als_eigene_datei_abgelegt() {
        let (dir, p) = projekt();
        // minimale ABIF-Datei mit PBAS und PCON, aufgebaut wie in den Kerntests
        let bases = b"ACGTACGTACGTACGTACGTACGTACGTAC";
        let qual = vec![40u8; bases.len()];
        let mut file = vec![0u8; 128];
        file[..4].copy_from_slice(b"ABIF");
        file[4..6].copy_from_slice(&101u16.to_be_bytes());
        let mut entries: Vec<([u8; 4], i32, Vec<u8>)> =
            vec![(*b"PBAS", 1, bases.to_vec()), (*b"PCON", 1, qual)];
        let mut dir_bytes = Vec::new();
        for (tag, num, data) in entries.drain(..) {
            let offset = file.len() as i32;
            file.extend_from_slice(&data);
            dir_bytes.extend_from_slice(&tag);
            dir_bytes.extend_from_slice(&num.to_be_bytes());
            dir_bytes.extend_from_slice(&2i16.to_be_bytes());
            dir_bytes.extend_from_slice(&1i16.to_be_bytes());
            dir_bytes.extend_from_slice(&(data.len() as i32).to_be_bytes());
            dir_bytes.extend_from_slice(&(data.len() as i32).to_be_bytes());
            dir_bytes.extend_from_slice(&offset.to_be_bytes());
            dir_bytes.extend_from_slice(&0i32.to_be_bytes());
        }
        let dir_offset = file.len() as i32;
        file.extend_from_slice(&dir_bytes);
        // Verzeichniseintrag des Verzeichnisses (Bytes 6..34)
        let mut head = Vec::new();
        head.extend_from_slice(b"tdir");
        head.extend_from_slice(&1i32.to_be_bytes());
        head.extend_from_slice(&1023i16.to_be_bytes());
        head.extend_from_slice(&28i16.to_be_bytes());
        head.extend_from_slice(&2i32.to_be_bytes());
        head.extend_from_slice(&(dir_bytes.len() as i32).to_be_bytes());
        head.extend_from_slice(&dir_offset.to_be_bytes());
        head.extend_from_slice(&0i32.to_be_bytes());
        file[6..34].copy_from_slice(&head);
        std::fs::write(dir.path().join("read1.ab1"), &file).unwrap();
        let docs = p
            .import_file(dir.path().join("read1.ab1").to_string_lossy().to_string())
            .unwrap();
        assert_eq!(
            p.bases(docs[0].id.clone(), 0, 100).unwrap(),
            String::from_utf8_lossy(bases)
        );
        let files = p.external_files(docs[0].id.clone()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].kind, "ab1");
        assert!(files[0].path.contains(".dateien"));
        assert!(std::path::Path::new(&files[0].path).exists());
        assert!(read_trace(files[0].path.clone()).is_ok());

        // Projekt mit Ordner an einen anderen Ort kopieren: die Spur wird dort gefunden
        let neu = dir.path().join("neu");
        std::fs::create_dir_all(&neu).unwrap();
        let alt_projekt = std::path::PathBuf::from(p.path());
        std::fs::copy(&alt_projekt, neu.join("t.snapious")).unwrap();
        std::fs::create_dir_all(neu.join("t.dateien")).unwrap();
        std::fs::copy(&files[0].path, neu.join("t.dateien").join("read1.ab1")).unwrap();
        let p2 = Project::open(neu.join("t.snapious").to_string_lossy().to_string()).unwrap();
        let f2 = p2.external_files(docs[0].id.clone()).unwrap();
        assert_eq!(f2.len(), 1);
        assert!(
            f2[0].path.starts_with(&*neu.to_string_lossy()),
            "{}",
            f2[0].path
        );
        assert!(read_trace(f2[0].path.clone()).is_ok());
    }

    #[test]
    fn konsens_aus_reads() {
        let (_d, p) = projekt();
        let reference = "ATGGCTAGCAAAGGAGAAGAACTTTTCACTGGAGTTGTCCCAATTCTTGTTGAATTAGATGGTGATGTTAATGGGCACAAATTTTCTGTC";
        let rc = |s: &str| -> String {
            s.chars()
                .rev()
                .map(|c| match c {
                    'A' => 'T',
                    'C' => 'G',
                    'G' => 'C',
                    _ => 'A',
                })
                .collect()
        };
        let r1 = p
            .create_sequence("r1".into(), reference[0..60].into(), "DNA".into(), false)
            .unwrap();
        let r2 = p
            .create_sequence("r2".into(), rc(&reference[30..90]), "DNA".into(), false)
            .unwrap();
        let r3 = p
            .create_sequence("r3".into(), reference[15..75].into(), "DNA".into(), false)
            .unwrap();
        let c = p
            .consensus_from_reads(
                vec![r1[0].id.clone(), r2[0].id.clone(), r3[0].id.clone()],
                10,
                String::new(),
            )
            .unwrap();
        let bases = p.bases(c.id.clone(), 0, 500).unwrap();
        assert!(bases.contains(&reference[30..60]), "{bases}");
        assert!(bases.len() >= 80, "{bases}");
        assert!(p
            .consensus_from_reads(vec![r1[0].id.clone()], 10, String::new())
            .is_err());
    }

    #[test]
    fn enzymsets() {
        let sets = enzyme_sets();
        assert!(sets.len() >= 3 && sets[0].builtin);
        assert!(enzyme_set_save("Häufige Klonierung".into(), "EcoRI".into()).is_err());
        assert!(enzyme_set_save(" ".into(), "EcoRI".into()).is_err());
        assert!(enzyme_set_save("x".into(), " , ".into()).is_err());
    }

    #[test]
    fn extreme_werte_stuerzen_nicht_ab() {
        let (_dir, p) = projekt();
        let id = p.import_file(mini_path()).unwrap()[0].id.clone();
        let big = u64::MAX;
        // Bereiche und Positionen
        let _ = p.bases(id.clone(), 0, big);
        let _ = p.bases(id.clone(), big, 0);
        let _ = p.bases(id.clone(), 10, 5);
        let _ = p.edit_insert(id.clone(), big, "ACGT".to_string());
        let _ = p.edit_insert(id.clone(), 0, "XYZ".to_string());
        let _ = p.edit_delete(id.clone(), 5, big);
        let _ = p.edit_delete(id.clone(), 9, 3);
        let _ = p.remove_feature(id.clone(), big);
        let _ = p.protein_of_feature(id.clone(), big);
        let _ = p.find_orfs(id.clone(), 0);
        let _ = p.find_orfs(id.clone(), big);
        let _ = p.fold_region(id.clone(), big, 0);
        let _ = p.fold_region(id.clone(), 0, big);
        let _ = p.mutagenesis_primers(id.clone(), big, big, "A".to_string(), 5, 10, 60.0);
        let _ = p.mutagenesis_primers(id.clone(), 0, 0, String::new(), 0, 0, f64::NAN);
        let _ = p.mutagenesis_primers(id.clone(), 5, 6, "Ä".to_string(), 5, big, 60.0);
        let _ = p.find_repeats(id.clone(), 0, 0, 0, 0, 0);
        let _ = p.find_repeats(id.clone(), big, big, big, big, big);
        let _ = p.dotplot(id.clone(), id.clone(), 0, 0, false);
        let _ = p.dotplot(id.clone(), id.clone(), big, big, true);
        let _ = p.assemble_reads(vec![id.clone()], 0);
        let _ = p.assemble_reads(vec![], big);
        let _ = p.map_reads_to(id.clone(), vec![id.clone()], big);
        let _ = p.coverage_depth(id.clone(), "/gibt/es/nicht.sam".to_string(), 0);
        let _ = p.call_variants(id.clone(), "/gibt/es/nicht.bam".to_string(), String::new());
        let _ = p.pick_primers(id.clone(), big, big, 3);
        let _ = p.find_guides(id.clone(), "gibtsnicht".to_string());
        let _ = p.find_motif_hits(id.clone(), "ZZZ".to_string());
        let _ = p.find_motif_hits(id.clone(), String::new());
        let _ = p.search_documents("motif".to_string(), "!!".to_string(), String::new());
        let _ = p.search_documents("unbekannt".to_string(), String::new(), String::new());
        let _ = p.batch_rename(vec![id.clone()], "{n:99999999999999999999}".to_string());
        let _ = p.batch_rename(vec![id.clone()], "{".to_string());
        let _ = p.restore_version(id.clone(), -1);
        let _ = p.move_document(id.clone(), "x".to_string());
        let _ = p.delete_folder("x".to_string(), true);
        let _ = p.export_text(id.clone(), "gibtsnicht".to_string());
        let _ = p.record_workflow(id.clone(), String::new());
        let _ = p.external_files(String::new());
        let _ = p.remove_external_file("x".to_string());
        // freie Funktionen
        let _ = calc_ng_to_pmol(f64::NAN, 0);
        let _ = calc_ng_to_pmol(f64::INFINITY, u64::MAX);
        let _ = calc_ligation_insert_ng(1.0, 0, 0, 0.0);
        let _ = calc_dilution_volume(0.0, 0.0, 0.0);
        let _ = calc_copy_number(-1.0, 1);
        let _ = gel_lane(vec![0, 1, u64::MAX], 0.0, f64::NAN);
        let _ = gel_ladder("gibtsnicht".to_string(), -1.0);
        let _ = pdb_open("/gibt/es/nicht.pdb".to_string());
        let _ = qc_fastq("/gibt/es/nicht.fastq".to_string(), 0);
        let _ = trim_fastq("/gibt/es/nicht.fastq".to_string(), "/tmp/x".to_string());
        // das Projekt ist danach noch benutzbar
        assert!(p.summary(id).is_ok());
    }
}
