package com.snapious.annotation;

import java.util.*;
import java.util.stream.*;

/**
 * Annotation Parser für GFF3 und GenBank Format.
 * Parst Annotations-Dateien und extrahiert Feature-Tabellen mit Positionen, Typen und Metadaten.
 */
public class AnnotationParser {

    // ======== FEATURE TYPEN (Ontologie) ==========

    public enum FeatureType {
        // Gen-Elemente
        GENE("gene"), TRANSCRIPT("transcript"), MRNA("mRNA"),

        // Exons/Introns
        EXON("exon"), INTRON("intron"), UTR("five_prime_UTR", "three_prime_UTR"),

        // CDS/Protein
        CDS("CDS"), START_CODON("start_codon"), STOP_CODON("stop_codon"),

        // Reguläre Elemente
        PROMOTER("promoter"), ENHANCER("enhancer"), SILENCER("silencer"),

        // Transkriptions-Faktoren
        TFBS("transcription_binding_site", "TF_binding_site"), RBS("ribosome_binding_site"),

        // Sonstige
        SIGNAL_PEPTIDE("signal_peptide"), TRANSMEMBRANE("transmembrane"),
        DOMAIN("domain"), MOTIF("motif"),

        // Marker/Other
        REPETITIVE_ELEMENT("repetitive_element"), TRANSPOSON("transposon"),
        SNP("snp"), MUTATION("mutation"),

        // Alles, was nicht erkannt wird
        OTHER("other");

        private final List<String> aliases;

        FeatureType(String... aliases) {
            this.aliases = Arrays.asList(aliases);
        }

        public List<String> getAliases() { return aliases; }

        /** Erkenne FeatureType aus String-Label */
        public static FeatureType fromString(String label) {
            if (label == null || label.isEmpty()) return null;

            for (FeatureType ft : values()) {
                if (ft.name().equalsIgnoreCase(label)) return ft;

                for (String alias : ft.aliases) {
                    if (alias.equalsIgnoreCase(label)) return ft;
                }
            }

            return FeatureType.OTHER;  // Fallback für unbekannte Typen
        }
    }

    // ======== FEATURE (Hauptdatenstruktur) ==========

    /** Repräsentiert ein Genomisches Feature mit allen Metadaten */
    public static class GenomicFeature {
        private String seqId;           // Chromosom/Scaffold ID (z.B. "chr1", "NC_000001")
        private String source;          // Quelle (z.B. "RefSeq", "Ensembl")
        private FeatureType type;       // Typ (z.B. CDS, Exon)
        private int start;              // Start-Position (1-basiert, inklusiv)
        private int end;                // End-Position (1-basiert, inklusiv)
        private double score;           // Score (oder -1 für keine Angabe)
        private char strand;            // '+' oder '-'
        private String phase;           // CDS-Phase (0, 1, 2) oder leer
        private Map<String, List<String>> attributes;  // Freie Attribute (key → values)

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

        /** Länge des Features */
        public int length() { return end - start + 1; }

        /** Mitte des Features */
        public int midpoint() { return (start + end) / 2; }

        /** Prüfe ob sich zwei Features überlappen */
        public boolean overlaps(GenomicFeature other) {
            return this.seqId.equals(other.seqId) &&
                this.start <= other.end && this.end >= other.start;
        }

        /** Hole Attribut-Wert (erster Wert) */
        public String getAttribute(String key) {
            List<String> values = attributes.get(key);
            return (values != null && !values.isEmpty()) ? values.get(0) : null;
        }

        /** Hole ID (meist "ID=...") */
        public String getId() { return getAttribute("ID"); }

        /** Hole Parent-ID (meist "Parent=...") */
        public String getParentId() { return getAttribute("Parent"); }

        /** Hole Name (meist "Name=...") */
        public String getName() { return getAttribute("Name"); }

        /** Hole Alias (meist "Alias=...") */
        public String getAlias() { return getAttribute("Alias"); }

        /** Hole Beschreibung (meist "note=..." oder "description=...") */
        public String getDescription() {
            return getAttribute("note") != null ? getAttribute("note") : getAttribute("description");
        }

        /** Alle Attribute */
        public Map<String, List<String>> getAttributes() { return attributes; }

        /** Formatierter Output */
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

    // ======== GEN STRUKTUR ==========

    /** Repräsentiert ein Gen mit allen seinen Transkripten und Features */
    public static class Gene {
        private String id;           // Gen-ID (z.B. "ENSG00000139618")
        private String name;         // Gen-Name/Symbol (z.B. "BRCA1")
        private String seqId;        // Chromosom (z.B. "chr17")
        private char strand;         // Strang (+/-)
        private int start;           // Gen-Start (über alle Transkripte)
        private int end;             // Gen-Ende (über alle Transkripte)
        private List<Transcript> transcripts;  // Transkripte dieses Gens

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

        /** Gen-Länge */
        public int length() { return end - start + 1; }

        /** Anzahl der Transkripte */
        public int transcriptCount() { return transcripts.size(); }

        /** Longest Transcript (LTT) */
        public Transcript getLongestTranscript() {
            return transcripts.stream()
                .max(Comparator.comparingInt(t -> t.cdsLength()))
                .orElse(null);
        }

        /** Generiere Gen-Zusammenfassung */
        public String summary() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("Gen: %s (%s)", name, id));
            sb.append(String.format("\nChromosom: %s", seqId));
            sb.append(String.format("Strang: %c", strand));
            sb.append(String.format("Länge: %d bp", length()));
                    sb.append(String.format("Transkripte: %d", transcripts.size()));

            // CDS-Länge des längsten Transkripts
            Transcript ltt = getLongestTranscript();

            if (ltt != null) {
                sb.append(String.format("\nCDS-Länge (LTT): %d bp", ltt.cdsLength()));
                sb.append(String.format("Exons (LTT): %d", ltt.exonCount()));
            }

            return sb.toString();
        }

        @Override
        public String toString() { return summary(); }
    }

    /** Repräsentiert ein Transkript mit Exons und CDS */
    public static class Transcript {
        private String id;           // Transkript-ID (z.B. "ENST00000261843")
        private String geneId;       // Parent-Gen-ID
        private String name;         // Transkript-Symbol
        private List<GenomicFeature> exons;       // Exon-Features
        private List<GenomicFeature> cdsFeatures;  // CDS-Features

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

        /**总数 von Exons */
        public int exonCount() { return exons.size(); }

        /** CDS-Länge (summiert über alle Exons) */
        public int cdsLength() {
            return cdsFeatures.stream().mapToInt(GenomicFeature::length).sum();
        }

        /** Transkript-Länge (über alle Exons, ohne Introns) */
        public int transcriptLength() {
            return exons.stream().mapToInt(GenomicFeature::length).sum();
        }

        /** Start-Position des längsten Exons */
        public int transcriptionStart() {
            return exons.stream().mapToInt(GenomicFeature::getStart).min().orElse(0);
        }

        /** End-Position des längsten Exons */
        public int transcriptionEnd() {
            return exons.stream().mapToInt(GenomicFeature::getEnd).max().orElse(0);
        }

        /** Generiere Transkript-Zusammenfassung */
        public String summary() {
            return String.format("Transcript: %s (Gen: %s)\n" +
                "  Exons: %d, CDS-Länge: %d bp, Transcript-Länge: %d bp",
                id, geneId, exonCount(), cdsLength(), transcriptLength());
        }

        @Override
        public String toString() { return summary(); }
    }

    // ======== GFF3 PARSER ==========

    /**
     * Parst GFF3 Format (Generic Feature Format Version 3).
     * Standard: 9 Spalten, semikolon-getrennte Attribute.
     */
    public static List<GenomicFeature> parseGFF3(String gff3Content) {
        List<GenomicFeature> features = new ArrayList<>();

        if (gff3Content == null || gff3Content.isEmpty()) {
            return features;
        }

        String[] lines = gff3Content.split("\n");

        for (int lineNum = 0; lineNum < lines.length; lineNum++) {
            String line = lines[lineNum].trim();

            // Überspringe leere Zeilen und Header
            if (line.isEmpty() || line.startsWith("##") ) {
                continue;
            }

            // Überspringe Feature-Zeilen ohne korrekte Struktur
            if (!line.startsWith("#")) {
                try {
                    GenomicFeature feature = parseGFF3Line(line, lineNum + 1);

                    if (feature != null) {
                        features.add(feature);
                    }

                } catch (Exception e) {
                    // Ignoriere fehlerhafte Zeilen, logge aber Warnung
                }
            }
        }

        return features;
    }

    /** Parse einzelne GFF3-Zeile (9 Spalten) */
    private static GenomicFeature parseGFF3Line(String line, int lineNum) {
        String[] columns = line.split("\t");

        if (columns.length < 9) {
            throw new IllegalArgumentException("Ungültige GFF3-Zeile (Zeile " + lineNum
                + ": Mindestens 9 Spalten erforderlich, gefunden: " + columns.length);
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
                score = -1;  // Keine Angabe
            } else {
                score = Double.parseDouble(scoreStr);
            }

        } catch (NumberFormatException e) {
            score = -1;  // Fallback bei ungültigem Score
        }

        char strand = columns[6].trim().charAt(0);

        if (strand != '+' && strand != '-') {
            strand = '.';  // Unbekannter Strang
        }

        String phase = columns[7].trim();  // "0", "1", "2" oder "."

        // Parse Attribute (8. Spalte, semikolon-getrennte key=value Paare)
        Map<String, List<String>> attributes = parseGFF3Attributes(columns[8]);

        return new GenomicFeature(seqId, source, type, start, end, score, strand, phase, attributes);
    }

    /** Parse GFF3 Attribute-Spalte (semikolon-getrennte key=value Paare) */
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
     * Parst GenBank Format (vereinfacht).
     * Extrahiert Features von der FEATURES-Table.
     */
    public static List<GenomicFeature> parseGenBank(String genbankContent) {
        List<GenomicFeature> features = new ArrayList<>();

        if (genbankContent == null || genbankContent.isEmpty()) {
            return features;
        }

        String[] lines = genbankContent.split("\n");
        boolean inFeaturesSection = false;

        for (String line : lines) {
            // Erkennen der FEATURES-Section
            if (line.trim().startsWith("features")) {
                inFeaturesSection = true;
                continue;
            }

            // Ende der Features-Section (nächste Top-Level-Sektion)
            if (inFeaturesSection && line.length() <= 60 && Character.isLetterOrDigit(line.charAt(0))) {
                String topKey = line.substring(0, Math.min(20, line.length())).trim();

                if (topKey.matches("(ORIGIN|REFERENCE|COMMENTS|source)\\b")) {
                    inFeaturesSection = false;
                    continue;
                }
            }

            if (inFeaturesSection) {
                // Feature-Zeilen beginnen mit 6 Leerzeichen, dann Position und Typ
                if (line.trim().startsWith("///") || line.trim().isEmpty()) {
                    continue;
                }

                try {
                    GenomicFeature feature = parseGenBankFeature(line);

                    if (feature != null) {
                        features.add(feature);
                    }

                } catch (Exception e) {
                    // Ignoriere fehlerhafte Features
                }
            }
        }

        return features;
    }

    /** Parse einzelne GenBank Feature-Zeile */
    private static GenomicFeature parseGenBankFeature(String line) {
        String trimmed = line.trim();

        // Überspringe leere Zeilen und "locus_tag" / "product" etc. (Beschreibungszeilen)
        if (trimmed.startsWith("locus_tag") || trimmed.startsWith("product")
            || trimmed.startsWith("note")) {
            return null;  // Keine neue Feature-Zeile, sondern beschreibende Info
        }

        // GenBank Position-Format: z.B. "100..200", "complement(100..200)", "<100"
        int featureStartPos = trimmed.indexOf('<') >= 0 ? trimmed.indexOf('<') :
                             trimmed.indexOf('^') >= 0 ? trimmed.indexOf('^') : 0;

        String posString = "";
        char strand = '+';

        // Prüfe auf complement() für Strang
        if (trimmed.startsWith("complement(")) {
            strand = '-';
            posString = trimmed.substring(13, trimmed.indexOf(')'));

        } else {
            posString = trimmed.substring(featureStartPos, trimmed.indexOf(' ', featureStartPos));
        }

        // Parse Position (Format: "100..200" oder "<100" oder ">300")
        int start, end;

        if (posString.contains("..")) {
            String[] parts = posString.split("\\.\\.");

            start = parseGenBankPosition(parts[0]);
            end = parseGenBankPosition(parts[1]);

        } else {
            // Einzelne Position (z.B. site, binding)
            start = parseGenBankPosition(posString);
            end = start;  // Einzelne Position
        }

        // Feature-Typ (z.B. "CDS", "exon", "gene")
        String typeStr = trimmed.substring(0, featureStartPos).trim();

        if (typeStr.isEmpty()) {
            return null;  // Keine gültige Feature-Zeile
        }

        FeatureType type = FeatureType.fromString(typeStr);

        // Extrahiere Attribute aus Klammern (z.B. "gene=BRCA1", "product=helicase")
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

        // Standard-Werte für GenBank
        return new GenomicFeature(
            "genbank",  // seqId wird aus Header extrahiert (nicht in Features)
            "GenBank",  // source
            type,       // Typ
            start,      // Start-Position (1-basiert)
            end,        // End-Position (1-basiert)
            -1,         // Score (GenBank hat keinen Score in Features)
            strand,     // Strang
            "",         // Phase (GenBank hat keine CDS-Phase in Features)
            attributes  // Attribute
        );
    }

    /** Parse GenBank-Position mit < und > (approximative Positionen) */
    private static int parseGenBankPosition(String posString) {
        String clean = posString.replaceAll("[<>]", "");

        try {
            return Integer.parseInt(clean);

        } catch (NumberFormatException e) {
            return 0;  // Fallback bei ungültiger Position
        }
    }

    // ======== ANNOTATION ZUSAMMENFASSUNG ==========

    /** Generiere Annotations-Zusammenfassung aus Features */
    public static String generateAnnotationSummary(List<GenomicFeature> features) {
        if (features.isEmpty()) return "Keine Annotationen vorhanden.";

        StringBuilder sb = new StringBuilder();

        sb.append("=== Annotations-Zusammenfassung ===\n\n");

        // Feature-Typen zählen
        Map<FeatureType, Integer> typeCounts = new LinkedHashMap<>();

        for (GenomicFeature feature : features) {
            typeCounts.merge(feature.getType(), 1, Integer::sum);
        }

        // Sortiere nach Häufigkeit
        List<Map.Entry<FeatureType, Integer>> sortedTypes = typeCounts.entrySet().stream()
            .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
            .toList();

        sb.append("Feature-Typen:\n");

        for (Map.Entry<FeatureType, Integer> entry : sortedTypes) {
            sb.append(String.format("  %-25s: %d\n", entry.getKey().name(), entry.getValue()));
        }

        // Chromosomen/Sequenzen
        Set<String> seqIds = features.stream().map(GenomicFeature::getSeqId).collect(Collectors.toSet());

        sb.append(String.format("\nSequenzen/Chromosomen: %s", seqIds));

        // Strang-Statistik
        long forwardStrand = features.stream().filter(f -> f.getStrand() == '+').count();
        long reverseStrand = features.stream().filter(f -> f.getStrand() == '-').count();

        sb.append(String.format("\nVorwärts-Strang: %d\n", forwardStrand));
        sb.append(String.format("Rückwärts-Strang: %d\n", reverseStrand));

        // Größe der Features
        sb.append("\nFeature-Größen (bp):\n");

        List<GenomicFeature> sizedFeatures = features.stream()
            .filter(f -> f.length() > 0)
            .collect(Collectors.toList());

        if (!sizedFeatures.isEmpty()) {
            int[] sizes = sizedFeatures.stream().mapToInt(GenomicFeature::length).toArray();

            Arrays.sort(sizes);

            sb.append(String.format("  Mindestgröße: %d bp\n", sizes[0]));
            sb.append(String.format("  Maximalgröße: %d bp\n", sizes[sizes.length - 1]));

            double avgSize = Arrays.stream(sizes).average().orElse(0);
                    sb.append(String.format("  Durchschnitt: %.0f bp\n", avgSize));
        }

        // Gene extrahieren (Features vom Typ GEN)
        List<GenomicFeature> genes = features.stream()
            .filter(f -> f.getType() == FeatureType.GENE)
            .collect(Collectors.toList());

        if (!genes.isEmpty()) {
            sb.append(String.format("\nGene: %d\n", genes.size()));

            for (GenomicFeature gene : genes.subList(0, Math.min(5, genes.size()))) {
                String geneName = gene.getName() != null ? gene.getName() : gene.getId();

                sb.append(String.format("  %s (%s): %d bp auf %c\n",
                    geneName, gene.getId(), gene.length(), gene.getStrand()));
            }

            if (genes.size() > 5) {
                sb.append(String.format("  ... und %d weitere Gene\n", genes.size() - 5));
            }
        }

        return sb.toString();
    }

    /** Extrahiere Gene mit allen ihren Transkripten */
    public static List<Gene> extractGenes(List<GenomicFeature> features) {
        Map<String, List<GenomicFeature>> geneGroups = features.stream()
            .filter(f -> f.getType() == FeatureType.GENE)
            .collect(Collectors.groupingBy(GenomicFeature::getId));

        List<Gene> genes = new ArrayList<>();

        for (Map.Entry<String, List<GenomicFeature>> entry : geneGroups.entrySet()) {
            String geneId = entry.getKey();

            // Finde Gen-Feature
            GenomicFeature geneFeature = entry.getValue().get(0);

            // Finde alle Transkripte (Parent = Gen-ID)
            List<GenomicFeature> transcriptFeatures = features.stream()
                .filter(f -> "transcript".equalsIgnoreCase(f.getType().name())
                    && geneId.equals(f.getParentId()))
                .collect(Collectors.toList());

            // Baue Gene-Objekt mit allen Transkripten
            List<Transcript> transcripts = new ArrayList<>();

            for (GenomicFeature tf : transcriptFeatures) {
                // Finde Exons und CDS für dieses Transkript
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

            // Berechne Gen-Region (über alle Transkripte)
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

    /** Generiere vollständigen Annotations-Bericht */
    public static String generateFullAnnotationReport(List<GenomicFeature> features) {
        StringBuilder sb = new StringBuilder();

        // Zusammenfassung
        sb.append(generateAnnotationSummary(features));

        // Gene-Liste
        List<Gene> genes = extractGenes(features);

        if (!genes.isEmpty()) {
            sb.append("\n=== Gene ===\n");

            for (Gene gene : genes) {
                sb.append(gene.toString()).append("\n");

                // Transkripte auflisten
                if (!gene.transcripts.isEmpty()) {
                    sb.append("  Transkripte:\n");

                    for (Transcript transcript : gene.transcripts) {
                        sb.append("    ").append(transcript.toString()).append("\n");
                    }

                    // LTT Details
                    Transcript ltt = gene.getLongestTranscript();

                    if (ltt != null) {
                        sb.append(String.format("  LTT: %s\n", ltt.getId()));
                    }
                }

                sb.append("\n");  // Leerzeile nach jedem Gen
            }
        }

        return sb.toString();
    }

    /** Exportiere Features als GFF3-String */
    public static String exportToGFF3(List<GenomicFeature> features) {
        StringBuilder sb = new StringBuilder();

        // Header (zwingend erforderlich für GFF3)
        sb.append("##gff-version 3\n");

        for (GenomicFeature feature : features) {
            sb.append(feature.toString()).append("\n");
        }

        return sb.toString();
    }

    /** Exportiere Features als GenBank-Features (vereinfacht) */
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

            // Feature-Typ (linksbündig)
            sb.append(String.format("    %-30s %s\n", feature.getType().name(), position));

            // Qualifier (in Klammern)
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

    /** Finde alle Features eines bestimmten Typs */
    public static List<GenomicFeature> filterByType(List<GenomicFeature> features, FeatureType type) {
        return features.stream()
            .filter(f -> f.getType() == type)
            .collect(Collectors.toList());
    }

    /** Finde alle Features auf einem bestimmten Chromosom */
    public static List<GenomicFeature> filterByChromosome(List<GenomicFeature> features, String seqId) {
        return features.stream()
            .filter(f -> f.getSeqId().equals(seqId))
            .collect(Collectors.toList());
    }

    /** Finde überlappende Features (konfliktreiche Annotationen) */
    public static List<List<GenomicFeature>> findOverlappingFeatures(List<GenomicFeature> features) {
        List<List<GenomicFeature>> overlappingGroups = new ArrayList<>();

        for (int i = 0; i < features.size(); i++) {
            for (int j = i + 1; j < features.size(); j++) {
                if (features.get(i).overlaps(features.get(j))) {
                    List<GenomicFeature> group = new ArrayList<>();

                    group.add(features.get(i));
                    group.add(features.get(j));

                    // Finde alle weiteren überlappenden Features (transitiv)
                    for (int k = 0; k < features.size(); k++) {
                        if (k != i && k != j) {
                            for (GenomicFeature existing : group) {
                                if (existing.overlaps(features.get(k))) {
                                    group.add(features.get(k));
                                    break;  // Kein weiterer Scan für dieses k
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
