package com.snapious.annotation;

import java.util.*;
import java.util.stream.*;

/**
 * Annotation parser for GFF3 and GenBank formats.
 * Parses annotation files and extracts feature tables with positions, types and metadata.
 */
public class AnnotationParser {

    // ======== FEATURE TYPES (ontology) ==========

    public enum FeatureType {
        // Gene elements
        GENE("gene"), TRANSCRIPT("transcript"), MRNA("mRNA"),

        // Exons/Introns
        EXON("exon"), INTRON("intron"), UTR("five_prime_UTR", "three_prime_UTR"),

        // CDS/Protein
        CDS("CDS"), START_CODON("start_codon"), STOP_CODON("stop_codon"),

        // Regulatory elements
        PROMOTER("promoter"), ENHANCER("enhancer"), SILENCER("silencer"),

        // Transcription factors
        TFBS("transcription_binding_site", "TF_binding_site"), RBS("ribosome_binding_site"),

        // Miscellaneous
        SIGNAL_PEPTIDE("signal_peptide"), TRANSMEMBRANE("transmembrane"),
        DOMAIN("domain"), MOTIF("motif"),

        // Markers/other
        REPETITIVE_ELEMENT("repetitive_element"), TRANSPOSON("transposon"),
        SNP("snp"), MUTATION("mutation"),

        // Anything that is not recognized
        OTHER("other");

        private final List<String> aliases;

        FeatureType(String... aliases) {
            this.aliases = Arrays.asList(aliases);
        }

        public List<String> getAliases() { return aliases; }

        /** Detect FeatureType from a string label */
        public static FeatureType fromString(String label) {
            if (label == null || label.isEmpty()) return null;

            for (FeatureType ft : values()) {
                if (ft.name().equalsIgnoreCase(label)) return ft;

                for (String alias : ft.aliases) {
                    if (alias.equalsIgnoreCase(label)) return ft;
                }
            }

            return FeatureType.OTHER;  // fallback for unknown types
        }
    }

    // ======== FEATURE (Hauptdatenstruktur) ==========

    /** Represents a genomic feature with all metadata */
    public static class GenomicFeature {
        private String seqId;           // chromosome/scaffold ID (e.g. "chr1", "NC_000001")
        private String source;          // source (e.g. "RefSeq", "Ensembl")
        private FeatureType type;       // type (e.g. CDS, exon)
        private int start;              // start position (1-based, inclusive)
        private int end;                // end position (1-based, inclusive)
        private double score;           // score (or -1 if not given)
        private char strand;            // '+' or '-'
        private String phase;           // CDS phase (0, 1, 2) or empty
        private Map<String, List<String>> attributes;  // free attributes (key → values)

        public GenomicFeature(String seqId, String source, FeatureType type,
                             int start, int end, double score, char strand,
                             String phase, Map<String, List<String>> attributes) {
            this.seqId = seqId;
            this.source = source;
            this.type = type;
            this.start = start;
            this.end = end;
            this.score = score;
            this.strand = strand;
            this.phase = phase;
            this.attributes = attributes != null ? attributes : Collections.emptyMap();
        }

        public String getSeqId() { return seqId; }
        public String getSource() { return source; }
        public FeatureType getType() { return type; }
        public int getStart() { return start; }
        public int getEnd() { return end; }
        public double getScore() { return score; }
        public char getStrand() { return strand; }
        public String getPhase() { return phase; }

        /** Length of the feature */
        public int length() { return end - start + 1; }

        /** Midpoint of the feature */
        public int midpoint() { return (start + end) / 2; }

        /** Check whether two features overlap */
        public boolean overlaps(GenomicFeature other) {
            return this.seqId.equals(other.seqId) &&
                this.start <= other.end && this.end >= other.start;
        }

        /** Get attribute value (first value) */
        public String getAttribute(String key) {
            List<String> values = attributes.get(key);
            return (values != null && !values.isEmpty()) ? values.get(0) : null;
        }

        /** Get ID (usually "ID=...") */
        public String getId() { return getAttribute("ID"); }

        /** Get parent ID (usually "Parent=...") */
        public String getParentId() { return getAttribute("Parent"); }

        /** Get name (usually "Name=...") */
        public String getName() { return getAttribute("Name"); }

        /** Get alias (usually "Alias=...") */
        public String getAlias() { return getAttribute("Alias"); }

        /** Get description (usually "note=..." or "description=...") */
        public String getDescription() {
            return getAttribute("note") != null ? getAttribute("note") : getAttribute("description");
        }

        /** All attributes */
        public Map<String, List<String>> getAttributes() { return attributes; }

        /** Formatted output */
        public String format() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("%s\t%s\t%s\t%d\t%d\t%.1f\t%c",
                seqId, source, type.name(), start, end, score, strand));

            if (!phase.isEmpty()) {
                sb.append(String.format("\t%s", phase));
            } else {
                sb.append("\t.");
            }

            // Attribute
            if (!attributes.isEmpty()) {
                sb.append("\t");
                attributes.forEach((key, values) -> {
                    if (!values.isEmpty()) {
                        sb.append(key).append("=").append(values.get(0));
                    }
                });
            }

            return sb.toString();
        }

        @Override
        public String toString() { return format(); }
    }

    // ======== GENE STRUCTURE ==========

    /** Represents a gene with all its transcripts and features */
    public static class Gene {
        private String id;           // gene ID (e.g. "ENSG00000139618")
        private String name;         // gene name/symbol (e.g. "BRCA1")
        private String seqId;        // chromosome (e.g. "chr17")
        private char strand;         // strand (+/-)
        private int start;           // gene start (across all transcripts)
        private int end;             // gene end (across all transcripts)
        private List<Transcript> transcripts;  // transcripts of this gene

        public Gene(String id, String name, String seqId, char strand,
                   int start, int end, List<Transcript> transcripts) {
            this.id = id;
            this.name = name;
            this.seqId = seqId;
            this.strand = strand;
            this.start = start;
            this.end = end;
            this.transcripts = transcripts != null ? transcripts : Collections.emptyList();
        }

        public String getId() { return id; }
        public String getName() { return name; }
        public String getSeqId() { return seqId; }
        public char getStrand() { return strand; }
        public int getStart() { return start; }
        public int getEnd() { return end; }

        /** Gene length */
        public int length() { return end - start + 1; }

        /** Number of transcripts */
        public int transcriptCount() { return transcripts.size(); }

        /** Longest transcript (LTT) */
        public Transcript getLongestTranscript() {
            return transcripts.stream()
                .max(Comparator.comparingInt(t -> t.cdsLength()))
                .orElse(null);
        }

        /** Generate gene summary */
        public String summary() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("Gen: %s (%s)", name, id));
            sb.append(String.format("\nChromosome: %s", seqId));
            sb.append(String.format("Strand: %c", strand));
            sb.append(String.format("Length: %d bp", length()));
                    sb.append(String.format("Transcripts: %d", transcripts.size()));

            // CDS length of the longest transcript
            Transcript ltt = getLongestTranscript();

            if (ltt != null) {
                sb.append(String.format("\nCDS length (LTT): %d bp", ltt.cdsLength()));
                sb.append(String.format("Exons (LTT): %d", ltt.exonCount()));
            }

            return sb.toString();
        }

        @Override
        public String toString() { return summary(); }
    }

    /** Represents a transcript with exons and CDS */
    public static class Transcript {
        private String id;           // transcript ID (e.g. "ENST00000261843")
        private String geneId;       // parent gene ID
        private String name;         // transcript symbol
        private List<GenomicFeature> exons;       // exon features
        private List<GenomicFeature> cdsFeatures;  // CDS features

        public Transcript(String id, String geneId, String name,
                         List<GenomicFeature> exons, List<GenomicFeature> cdsFeatures) {
            this.id = id;
            this.geneId = geneId;
            this.name = name;
            this.exons = exons != null ? exons : Collections.emptyList();
            this.cdsFeatures = cdsFeatures != null ? cdsFeatures : Collections.emptyList();
        }

        public String getId() { return id; }
        public String getGeneId() { return geneId; }

        /**Total number of exons */
        public int exonCount() { return exons.size(); }

        /** CDS length (summed over all exons) */
        public int cdsLength() {
            return cdsFeatures.stream().mapToInt(GenomicFeature::length).sum();
        }

        /** Transcript length (over all exons, without introns) */
        public int transcriptLength() {
            return exons.stream().mapToInt(GenomicFeature::length).sum();
        }

        /** Start position of the longest exon */
        public int transcriptionStart() {
            return exons.stream().mapToInt(GenomicFeature::getStart).min().orElse(0);
        }

        /** End position of the longest exon */
        public int transcriptionEnd() {
            return exons.stream().mapToInt(GenomicFeature::getEnd).max().orElse(0);
        }

        /** Generate transcript summary */
        public String summary() {
            return String.format("Transcript: %s (Gene: %s)\n" +
                "  Exons: %d, CDS length: %d bp, Transcript length: %d bp",
                id, geneId, exonCount(), cdsLength(), transcriptLength());
        }

        @Override
        public String toString() { return summary(); }
    }

    // ======== GFF3 PARSER ==========

    /**
     * Parses the GFF3 format (Generic Feature Format Version 3).
     * Standard: 9 columns, semicolon-separated attributes.
     */
    public static List<GenomicFeature> parseGFF3(String gff3Content) {
        List<GenomicFeature> features = new ArrayList<>();

        if (gff3Content == null || gff3Content.isEmpty()) {
            return features;
        }

        String[] lines = gff3Content.split("\n");

        for (int lineNum = 0; lineNum < lines.length; lineNum++) {
            String line = lines[lineNum].trim();

            // Skip empty lines and headers
            if (line.isEmpty() || line.startsWith("##") ) {
                continue;
            }

            // Skip feature lines without a correct structure
            if (!line.startsWith("#")) {
                try {
                    GenomicFeature feature = parseGFF3Line(line, lineNum + 1);

                    if (feature != null) {
                        features.add(feature);
                    }

                } catch (Exception e) {
                    // Ignore malformed lines, but log a warning
                }
            }
        }

        return features;
    }

    /** Parse a single GFF3 line (9 columns) */
    private static GenomicFeature parseGFF3Line(String line, int lineNum) {
        String[] columns = line.split("\t");

        if (columns.length < 9) {
            throw new IllegalArgumentException("Invalid GFF3 line (line " + lineNum
                + ": At least 9 columns required, found: " + columns.length);
        }

        String seqId = columns[0].trim();
        String source = columns[1].trim();
        String typeStr = columns[2].trim();
        FeatureType type = FeatureType.fromString(typeStr);

        int start = Integer.parseInt(columns[3].trim());
        int end = Integer.parseInt(columns[4].trim());

        double score;

        try {
            String scoreStr = columns[5].trim();

            if (".".equals(scoreStr)) {
                score = -1;  // not given
            } else {
                score = Double.parseDouble(scoreStr);
            }

        } catch (NumberFormatException e) {
            score = -1;  // fallback for invalid score
        }

        char strand = columns[6].trim().charAt(0);

        if (strand != '+' && strand != '-') {
            strand = '.';  // unknown strand
        }

        String phase = columns[7].trim();  // "0", "1", "2" or "."

        // Parse attributes (column 8, semicolon-separated key=value pairs)
        Map<String, List<String>> attributes = parseGFF3Attributes(columns[8]);

        return new GenomicFeature(seqId, source, type, start, end, score, strand, phase, attributes);
    }

    /** Parse the GFF3 attribute column (semicolon-separated key=value pairs) */
    private static Map<String, List<String>> parseGFF3Attributes(String attrString) {
        Map<String, List<String>> attributes = new HashMap<>();

        if (attrString == null || attrString.trim().isEmpty() || ".".equals(attrString.trim())) {
            return attributes;
        }

        String[] pairs = attrString.split(";");

        for (String pair : pairs) {
            pair = pair.trim();

            if (pair.isEmpty()) continue;

            int eqPos = pair.indexOf('=');

            if (eqPos > 0) {
                String key = pair.substring(0, eqPos).trim();
                String value = pair.substring(eqPos + 1).trim();

                attributes.computeIfAbsent(key, k -> new ArrayList<>()).add(value);
            }
        }

        return attributes;
    }

    // ======== GENBANK PARSER ==========

    /**
     * Parses the GenBank format (simplified).
     * Extracts features from the FEATURES table.
     */
    public static List<GenomicFeature> parseGenBank(String genbankContent) {
        List<GenomicFeature> features = new ArrayList<>();

        if (genbankContent == null || genbankContent.isEmpty()) {
            return features;
        }

        String[] lines = genbankContent.split("\n");
        boolean inFeaturesSection = false;

        for (String line : lines) {
            // Detect the FEATURES section
            if (line.trim().startsWith("features")) {
                inFeaturesSection = true;
                continue;
            }

            // End of the features section (next top-level section)
            if (inFeaturesSection && line.length() <= 60 && Character.isLetterOrDigit(line.charAt(0))) {
                String topKey = line.substring(0, Math.min(20, line.length())).trim();

                if (topKey.matches("(ORIGIN|REFERENCE|COMMENTS|source)\\b")) {
                    inFeaturesSection = false;
                    continue;
                }
            }

            if (inFeaturesSection) {
                // Feature lines start with 6 spaces, followed by position and type
                if (line.trim().startsWith("///") || line.trim().isEmpty()) {
                    continue;
                }

                try {
                    GenomicFeature feature = parseGenBankFeature(line);

                    if (feature != null) {
                        features.add(feature);
                    }

                } catch (Exception e) {
                    // Ignore malformed features
                }
            }
        }

        return features;
    }

    /** Parse a single GenBank feature line */
    private static GenomicFeature parseGenBankFeature(String line) {
        String trimmed = line.trim();

        // Skip empty lines and "locus_tag" / "product" etc. (description lines)
        if (trimmed.startsWith("locus_tag") || trimmed.startsWith("product")
            || trimmed.startsWith("note")) {
            return null;  // not a new feature line, but descriptive info
        }

        // GenBank position format: e.g. "100..200", "complement(100..200)", "<100"
        int featureStartPos = trimmed.indexOf('<') >= 0 ? trimmed.indexOf('<') :
                             trimmed.indexOf('^') >= 0 ? trimmed.indexOf('^') : 0;

        String posString = "";
        char strand = '+';

        // Check for complement() to determine the strand
        if (trimmed.startsWith("complement(")) {
            strand = '-';
            posString = trimmed.substring(13, trimmed.indexOf(')'));

        } else {
            posString = trimmed.substring(featureStartPos, trimmed.indexOf(' ', featureStartPos));
        }

        // Parse position (format: "100..200" or "<100" or ">300")
        int start, end;

        if (posString.contains("..")) {
            String[] parts = posString.split("\\.\\.");

            start = parseGenBankPosition(parts[0]);
            end = parseGenBankPosition(parts[1]);

        } else {
            // Single position (e.g. site, binding)
            start = parseGenBankPosition(posString);
            end = start;  // single position
        }

        // Feature type (e.g. "CDS", "exon", "gene")
        String typeStr = trimmed.substring(0, featureStartPos).trim();

        if (typeStr.isEmpty()) {
            return null;  // not a valid feature line
        }

        FeatureType type = FeatureType.fromString(typeStr);

        // Extract attributes from parentheses (e.g. "gene=BRCA1", "product=helicase")
        Map<String, List<String>> attributes = new HashMap<>();

        int parenStart = trimmed.indexOf('(');

        if (parenStart > 0) {
            String attrString = trimmed.substring(parenStart + 1, trimmed.indexOf(')', parenStart));
            String[] attrPairs = attrString.split(",");

            for (String pair : attrPairs) {
                pair = pair.trim();
                int eqPos = pair.indexOf('=');

                if (eqPos > 0) {
                    String key = pair.substring(0, eqPos).trim();
                    String value = pair.substring(eqPos + 1).trim().replaceAll("\"", "");

                    attributes.computeIfAbsent(key, k -> new ArrayList<>()).add(value);
                }
            }
        }

        // Default values for GenBank
        return new GenomicFeature(
            "genbank",  // seqId is extracted from the header (not in features)
            "GenBank",  // source
            type,       // type
            start,      // start position (1-based)
            end,        // end position (1-based)
            -1,         // score (GenBank has no score in features)
            strand,     // strand
            "",         // phase (GenBank has no CDS phase in features)
            attributes  // attributes
        );
    }

    /** Parse a GenBank position with < and > (approximate positions) */
    private static int parseGenBankPosition(String posString) {
        String clean = posString.replaceAll("[<>]", "");

        try {
            return Integer.parseInt(clean);

        } catch (NumberFormatException e) {
            return 0;  // fallback for invalid position
        }
    }

    // ======== ANNOTATION SUMMARY ==========

    /** Generate an annotation summary from features */
    public static String generateAnnotationSummary(List<GenomicFeature> features) {
        if (features.isEmpty()) return "No annotations present.";

        StringBuilder sb = new StringBuilder();

        sb.append("=== Annotation Summary ===\n\n");

        // Count feature types
        Map<FeatureType, Integer> typeCounts = new LinkedHashMap<>();

        for (GenomicFeature feature : features) {
            typeCounts.merge(feature.getType(), 1, Integer::sum);
        }

        // Sort by frequency
        List<Map.Entry<FeatureType, Integer>> sortedTypes = typeCounts.entrySet().stream()
            .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
            .toList();

        sb.append("Feature types:\n");

        for (Map.Entry<FeatureType, Integer> entry : sortedTypes) {
            sb.append(String.format("  %-25s: %d\n", entry.getKey().name(), entry.getValue()));
        }

        // Chromosomes/sequences
        Set<String> seqIds = features.stream().map(GenomicFeature::getSeqId).collect(Collectors.toSet());

        sb.append(String.format("\nSequences/chromosomes: %s", seqIds));

        // Strand statistics
        long forwardStrand = features.stream().filter(f -> f.getStrand() == '+').count();
        long reverseStrand = features.stream().filter(f -> f.getStrand() == '-').count();

        sb.append(String.format("\nForward strand: %d\n", forwardStrand));
        sb.append(String.format("Reverse strand: %d\n", reverseStrand));

        // Feature sizes
        sb.append("\nFeature sizes (bp):\n");

        List<GenomicFeature> sizedFeatures = features.stream()
            .filter(f -> f.length() > 0)
            .collect(Collectors.toList());

        if (!sizedFeatures.isEmpty()) {
            int[] sizes = sizedFeatures.stream().mapToInt(GenomicFeature::length).toArray();

            Arrays.sort(sizes);

            sb.append(String.format("  Minimum size: %d bp\n", sizes[0]));
            sb.append(String.format("  Maximum size: %d bp\n", sizes[sizes.length - 1]));

            double avgSize = Arrays.stream(sizes).average().orElse(0);
                    sb.append(String.format("  Average: %.0f bp\n", avgSize));
        }

        // Extract genes (features of type GENE)
        List<GenomicFeature> genes = features.stream()
            .filter(f -> f.getType() == FeatureType.GENE)
            .collect(Collectors.toList());

        if (!genes.isEmpty()) {
            sb.append(String.format("\nGene: %d\n", genes.size()));

            for (GenomicFeature gene : genes.subList(0, Math.min(5, genes.size()))) {
                String geneName = gene.getName() != null ? gene.getName() : gene.getId();

                sb.append(String.format("  %s (%s): %d bp on %c\n",
                    geneName, gene.getId(), gene.length(), gene.getStrand()));
            }

            if (genes.size() > 5) {
                sb.append(String.format("  ... and %d more genes\n", genes.size() - 5));
            }
        }

        return sb.toString();
    }

    /** Extract genes with all their transcripts */
    public static List<Gene> extractGenes(List<GenomicFeature> features) {
        Map<String, List<GenomicFeature>> geneGroups = features.stream()
            .filter(f -> f.getType() == FeatureType.GENE)
            .collect(Collectors.groupingBy(GenomicFeature::getId));

        List<Gene> genes = new ArrayList<>();

        for (Map.Entry<String, List<GenomicFeature>> entry : geneGroups.entrySet()) {
            String geneId = entry.getKey();

            // Find the gene feature
            GenomicFeature geneFeature = entry.getValue().get(0);

            // Find all transcripts (Parent = gene ID)
            List<GenomicFeature> transcriptFeatures = features.stream()
                .filter(f -> "transcript".equalsIgnoreCase(f.getType().name())
                    && geneId.equals(f.getParentId()))
                .collect(Collectors.toList());

            // Build the gene object with all transcripts
            List<Transcript> transcripts = new ArrayList<>();

            for (GenomicFeature tf : transcriptFeatures) {
                // Find exons and CDS for this transcript
                List<GenomicFeature> exons = features.stream()
                    .filter(f -> "exon".equalsIgnoreCase(f.getType().name())
                        && tf.getId().equals(f.getParentId()))
                    .sorted(Comparator.comparingInt(GenomicFeature::getStart))
                    .collect(Collectors.toList());

                List<GenomicFeature> cds = features.stream()
                    .filter(f -> "CDS".equalsIgnoreCase(f.getType().name())
                        && tf.getId().equals(f.getParentId()))
                    .sorted(Comparator.comparingInt(GenomicFeature::getStart))
                    .collect(Collectors.toList());

                transcripts.add(new Transcript(
                    tf.getId(), geneId, tf.getName(), exons, cds));
            }

            // Compute the gene region (across all transcripts)
            int geneStart = transcriptFeatures.stream()
                .mapToInt(GenomicFeature::getStart).min().orElse(geneFeature.getStart());

            int geneEnd = transcriptFeatures.stream()
                .mapToInt(GenomicFeature::getEnd).max().orElse(geneFeature.getEnd());

            genes.add(new Gene(
                geneId, geneFeature.getName(), geneFeature.getSeqId(),
                geneFeature.getStrand(), geneStart, geneEnd, transcripts));
        }

        return genes;
    }

    /** Generate the full annotation report */
    public static String generateFullAnnotationReport(List<GenomicFeature> features) {
        StringBuilder sb = new StringBuilder();

        // Summary
        sb.append(generateAnnotationSummary(features));

        // Gene list
        List<Gene> genes = extractGenes(features);

        if (!genes.isEmpty()) {
            sb.append("\n=== Gene ===\n");

            for (Gene gene : genes) {
                sb.append(gene.toString()).append("\n");

                // List transcripts
                if (!gene.transcripts.isEmpty()) {
                    sb.append("  Transcripts:\n");

                    for (Transcript transcript : gene.transcripts) {
                        sb.append("    ").append(transcript.toString()).append("\n");
                    }

                    // LTT details
                    Transcript ltt = gene.getLongestTranscript();

                    if (ltt != null) {
                        sb.append(String.format("  LTT: %s\n", ltt.getId()));
                    }
                }

                sb.append("\n");  // blank line after each gene
            }
        }

        return sb.toString();
    }

    /** Export features as a GFF3 string */
    public static String exportToGFF3(List<GenomicFeature> features) {
        StringBuilder sb = new StringBuilder();

        // Header (mandatory for GFF3)
        sb.append("##gff-version 3\n");

        for (GenomicFeature feature : features) {
            sb.append(feature.toString()).append("\n");
        }

        return sb.toString();
    }

    /** Export features as GenBank features (simplified) */
    public static String exportToGenBank(List<GenomicFeature> features) {
        StringBuilder sb = new StringBuilder();

        sb.append("FEATURES       Location/Qualifiers\n");

        for (GenomicFeature feature : features) {
            // Position
            String position;

                    if (feature.getType() == FeatureType.CDS) {
                if (feature.getStrand() == '-') {
                    position = String.format("complement(<%d..%d>", feature.start, feature.end);
                } else {
                    position = String.format("%d..%d", feature.start, feature.end);
                }

            } else {
                position = String.format("%d..%d", feature.start, feature.end);
            }

            // Feature type (left-aligned)
            sb.append(String.format("    %-30s %s\n", feature.getType().name(), position));

            // Qualifiers (in parentheses)
                    if (!feature.getAttributes().isEmpty()) {
                sb.append("            /");

                List<String> qualifiers = new ArrayList<>();

                feature.getAttributes().forEach((key, values) -> {
                    if (!values.isEmpty()) {
                        qualifiers.add(String.format("%s=\"%s\"", key, values.get(0)));
                    }
                });

                sb.append(String.join(", ", qualifiers)).append("\n");
            }
        }

        return sb.toString();
    }

    /** Find all features of a given type */
    public static List<GenomicFeature> filterByType(List<GenomicFeature> features, FeatureType type) {
        return features.stream()
            .filter(f -> f.getType() == type)
            .collect(Collectors.toList());
    }

    /** Find all features on a given chromosome */
    public static List<GenomicFeature> filterByChromosome(List<GenomicFeature> features, String seqId) {
        return features.stream()
            .filter(f -> f.getSeqId().equals(seqId))
            .collect(Collectors.toList());
    }

    /** Find overlapping features (conflicting annotations) */
    public static List<List<GenomicFeature>> findOverlappingFeatures(List<GenomicFeature> features) {
        List<List<GenomicFeature>> overlappingGroups = new ArrayList<>();

        for (int i = 0; i < features.size(); i++) {
            for (int j = i + 1; j < features.size(); j++) {
                if (features.get(i).overlaps(features.get(j))) {
                    List<GenomicFeature> group = new ArrayList<>();

                    group.add(features.get(i));
                    group.add(features.get(j));

                    // Find all further overlapping features (transitive)
                    for (int k = 0; k < features.size(); k++) {
                        if (k != i && k != j) {
                            for (GenomicFeature existing : group) {
                                if (existing.overlaps(features.get(k))) {
                                    group.add(features.get(k));
                                    break;  // no further scan for this k
                                }
                            }
                        }
                    }

                    if (group.size() > 2) {
                        overlappingGroups.add(group);
                    } else {
                        overlappingGroups.add(group);
                    }
                }
            }
        }

        return overlappingGroups;
    }
}
