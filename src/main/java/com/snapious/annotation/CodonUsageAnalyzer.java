package com.snapious.annotation;

import java.util.*;

/**
 * Codon usage analyzer for gene expression optimization.
 * Computes: CAI (Codon Adaptation Index), GC3 content, RSCU (Relative Synonymous Codon Usage).
 */
public class CodonUsageAnalyzer {

    // ======== CODON TABLE (Standard Genetic Code) ==========

    private static final Map<String, String> CODON_TABLE;

    static {
        CODON_TABLE = new HashMap<>();

        // Start codons
        CODON_TABLE.put("ATG", "M");  // methionine (start)
        CODON_TABLE.put("TTG", "L");  // alternative start codons
        CODON_TABLE.put("GTG", "V");

        // Phenylalanine
        CODON_TABLE.put("TTT", "F"); CODON_TABLE.put("TTC", "F");

        // Leucine
        CODON_TABLE.put("TTA", "L"); CODON_TABLE.put("TTG", "L");
        CODON_TABLE.put("CTT", "L"); CODON_TABLE.put("CTC", "L");
        CODON_TABLE.put("CTA", "L"); CODON_TABLE.put("CTG", "L");

        // Isoleucine
        CODON_TABLE.put("ATT", "I"); CODON_TABLE.put("ATC", "I"); CODON_TABLE.put("ATA", "I");

        // Valine
        CODON_TABLE.put("GTT", "V"); CODON_TABLE.put("GTC", "V");
        CODON_TABLE.put("GTA", "V"); CODON_TABLE.put("GTG", "V");

        // Serine
        CODON_TABLE.put("TCT", "S"); CODON_TABLE.put("TCC", "S");
        CODON_TABLE.put("TCA", "S"); CODON_TABLE.put("TCG", "S");
        CODON_TABLE.put("AGT", "S"); CODON_TABLE.put("AGC", "S");

        // Proline
        CODON_TABLE.put("CCT", "P"); CODON_TABLE.put("CCC", "P");
        CODON_TABLE.put("CCA", "P"); CODON_TABLE.put("CCG", "P");

        // Threonine
        CODON_TABLE.put("ACT", "T"); CODON_TABLE.put("ACC", "T");
        CODON_TABLE.put("ACA", "T"); CODON_TABLE.put("ACG", "T");

        // Alanine
        CODON_TABLE.put("GCT", "A"); CODON_TABLE.put("GCC", "A");
        CODON_TABLE.put("GCA", "A"); CODON_TABLE.put("GCG", "A");

        // Tyrosine
        CODON_TABLE.put("TAT", "Y"); CODON_TABLE.put("TAC", "Y");

        // Histidine
        CODON_TABLE.put("CAT", "H"); CODON_TABLE.put("CAC", "H");

        // Glutamine
        CODON_TABLE.put("CAA", "Q"); CODON_TABLE.put("CAG", "Q");

        // Asparagine
        CODON_TABLE.put("AAT", "N"); CODON_TABLE.put("AAC", "N");

        // Lysine
        CODON_TABLE.put("AAA", "K"); CODON_TABLE.put("AAG", "K");

        // Aspartate
        CODON_TABLE.put("GAT", "D"); CODON_TABLE.put("GAC", "D");

        // Glutamate
        CODON_TABLE.put("GAA", "E"); CODON_TABLE.put("GAG", "E");

        // Cysteine
        CODON_TABLE.put("TGT", "C"); CODON_TABLE.put("TGC", "C");

        // Tryptophan
        CODON_TABLE.put("TGG", "W");

        // Arginine
        CODON_TABLE.put("CGT", "R"); CODON_TABLE.put("CGC", "R");
        CODON_TABLE.put("CGA", "R"); CODON_TABLE.put("CGG", "R");
        CODON_TABLE.put("AGA", "R"); CODON_TABLE.put("AGG", "R");

        // Glycine
        CODON_TABLE.put("GGT", "G"); CODON_TABLE.put("GGC", "G");
        CODON_TABLE.put("GGA", "G"); CODON_TABLE.put("GGG", "G");

        // Stop codons
        CODON_TABLE.put("TAA", "*"); CODON_TABLE.put("TAG", "*"); CODON_TABLE.put("TGA", "*");
    }

    // ======== AMINO ACIDS ==========

    private static final Map<String, List<String>> CODONS_BY_AMINO_ACID;

    static {
        CODONS_BY_AMINO_ACID = new HashMap<>();

        CODONS_BY_AMINO_ACID.put("A", Arrays.asList("GCT", "GCC", "GCA", "GCG"));
        CODONS_BY_AMINO_ACID.put("C", Arrays.asList("TGT", "TGC"));
        CODONS_BY_AMINO_ACID.put("D", Arrays.asList("GAT", "AAC"));
        CODONS_BY_AMINO_ACID.put("E", Arrays.asList("GAA", "GAG"));
        CODONS_BY_AMINO_ACID.put("F", Arrays.asList("TTT", "TTC"));
        CODONS_BY_AMINO_ACID.put("G", Arrays.asList("GGT", "GGC", "GGA", "GGG"));
        CODONS_BY_AMINO_ACID.put("H", Arrays.asList("CAT", "CAC"));
        CODONS_BY_AMINO_ACID.put("I", Arrays.asList("ATT", "ATC", "ATA"));
        CODONS_BY_AMINO_ACID.put("K", Arrays.asList("AAA", "AAG"));
        CODONS_BY_AMINO_ACID.put("L", Arrays.asList("TTA", "TTG", "CTT", "CTC", "CTA", "CTG"));
        CODONS_BY_AMINO_ACID.put("M", Arrays.asList("ATG"));
        CODONS_BY_AMINO_ACID.put("N", Arrays.asList("AAT", "AAC"));
        CODONS_BY_AMINO_ACID.put("P", Arrays.asList("CCT", "CCC", "CCA", "CCG"));
        CODONS_BY_AMINO_ACID.put("Q", Arrays.asList("CAA", "CAG"));
        CODONS_BY_AMINO_ACID.put("R", Arrays.asList("CGT", "CGC", "CGA", "CGG", "AGA", "AGG"));
        CODONS_BY_AMINO_ACID.put("S", Arrays.asList("TCT", "TCC", "TCA", "TCG", "AGT", "AGC"));
        CODONS_BY_AMINO_ACID.put("T", Arrays.asList("ACT", "ACC", "ACA", "ACG"));
        CODONS_BY_AMINO_ACID.put("V", Arrays.asList("GTT", "GTC", "GTA", "GTG"));
        CODONS_BY_AMINO_ACID.put("W", Arrays.asList("TGG"));
        CODONS_BY_AMINO_ACID.put("Y", Arrays.asList("TAT", "TAC"));
        CODONS_BY_AMINO_ACID.put("*", Arrays.asList("TAA", "TAG", "TGA"));  // Stop
    }

    // ======== MAIN ANALYSIS ==========

    /**
     * Analyze the codon usage of a DNA sequence.
     */
    public static CodonUsageResult analyzeCodonUsage(String sequence) {
        String seq = sequence.toUpperCase().replaceAll("[^ATGC]", "");

        if (seq.length() < 3) {
            throw new IllegalArgumentException("Sequence too short for codon analysis (< 3bp)");
        }

        // Count all codons
        Map<String, Integer> codonCounts = new LinkedHashMap<>();  // order of appearance

        for (int i = 0; i <= seq.length() - 3; i += 3) {
            String codon = seq.substring(i, Math.min(i + 3, seq.length()));

            if (codon.length() == 3) {
                codonCounts.merge(codon, 1, Integer::sum);
            }
        }

        // Count amino acids (excluding stop)
        Map<String, Integer> aaCounts = new HashMap<>();
        int totalCodons = 0;

        for (Map.Entry<String, Integer> entry : codonCounts.entrySet()) {
            String codon = entry.getKey();
            int count = entry.getValue();

            // Skip incomplete codons and stop codons for CAI
            if (CODON_TABLE.containsKey(codon)) {
                String aa = CODON_TABLE.get(codon);

                if (!"*".equals(aa)) {  // ignore stop codons for CAI
                    aaCounts.merge(aa, count, Integer::sum);
                    totalCodons += count;
                }
            }
        }

        // Compute CAI (Codon Adaptation Index)
        double cai = calculateCAI(codonCounts, aaCounts);

        // Compute GC3 content
        double gc3 = calculateGC3(seq, codonCounts);

        // Compute RSCU (Relative Synonymous Codon Usage)
        Map<String, Double> rscu = calculateRSCU(codonCounts, aaCounts);

        // Generate optimization suggestions
        List<String> recommendations = generateRecommendations(cai, gc3, rscu);

        return new CodonUsageResult(codonCounts, aaCounts, cai, gc3, rscu, recommendations);
    }

    /** Compute the CAI (Codon Adaptation Index). 0-1, where 1 is optimal codon usage. */
    private static double calculateCAI(Map<String, Integer> codonCounts, Map<String, Integer> aaCounts) {
        // Use reference codon usage weights (simplified)
        // A full implementation would use a reference sequence database

        double sumWeights = 0;
        int nonStopCodons = 0;

        for (Map.Entry<String, Integer> entry : codonCounts.entrySet()) {
            String codon = entry.getKey();

            if (CODON_TABLE.containsKey(codon) && !"*".equals(CODON_TABLE.get(codon))) {
                String aa = CODON_TABLE.get(codon);
                List<String> codonsForAA = CODONS_BY_AMINO_ACID.get(aa);

                if (codonsForAA != null) {
                    // Simplified weighting: 1/number of synonyms (optimistic)
                    double weight = codonsForAA.size() == 1 ? 1.0 : (double) 1 / codonsForAA.size();

                    sumWeights += weight * entry.getValue();
                    nonStopCodons += entry.getValue();
                }
            }
        }

        if (nonStopCodons == 0) return 0.0;

        // Geometric mean
        double geometricMean = Math.pow(sumWeights / nonStopCodons, 1.0 / nonStopCodons);

        return Math.min(1.0, Math.max(0.0, geometricMean));
    }

    /** Compute GC3 content (GC content of the 3rd codon position) */
    private static double calculateGC3(String sequence, Map<String, Integer> codonCounts) {
        int thirdPositionG = 0;
        int thirdPositionC = 0;
        int totalThirdPositions = 0;

        for (Map.Entry<String, Integer> entry : codonCounts.entrySet()) {
            String codon = entry.getKey();

            if (codon.length() == 3 && CODON_TABLE.containsKey(codon)) {
                char thirdBase = codon.charAt(2);

                if (thirdBase == 'G' || thirdBase == 'C') {
                    thirdPositionG += (thirdBase == 'G' ? 1 : 0);
                    thirdPositionC += (thirdBase == 'C' ? 1 : 0);
                    totalThirdPositions += entry.getValue();
                }
            }
        }

        if (totalThirdPositions == 0) return 0.0;

        return (double) (thirdPositionG + thirdPositionC) / totalThirdPositions;
    }

    /** RSCU (Relative Synonymous Codon Usage) berechnen. 1 = expected usage, >1 overused, <1 underused */
    private static Map<String, Double> calculateRSCU(Map<String, Integer> codonCounts,
                                                      Map<String, Integer> aaCounts) {
        Map<String, Double> rscu = new LinkedHashMap<>();

        for (Map.Entry<String, List<String>> aaEntry : CODONS_BY_AMINO_ACID.entrySet()) {
            String aa = aaEntry.getKey();

            if ("*".equals(aa)) continue;  // ignore stop codons

            List<String> codons = aaEntry.getValue();
            int totalAA = aaCounts.getOrDefault(aa, 0);

            for (String codon : codons) {
                int count = codonCounts.getOrDefault(codon, 0);

                if (totalAA == 0) {
                    rscu.put(codon, 0.0);
                } else {
                    // RSCU = observed / expected (expected = totalAA / number of synonyms)
                    double rscuValue = (double) count / (totalAA / (double) codons.size());
                    rscu.put(codon, Math.round(rscuValue * 100.0) / 100.0);
                }
            }
        }

        return rscu;
    }

    /** Generate optimization suggestions based on the analysis */
    private static List<String> generateRecommendations(double cai, double gc3, Map<String, Double> rscu) {
        List<String> recommendations = new ArrayList<>();

        // CAI assessment
        if (cai >= 0.8) {
            recommendations.add("CAI: Excellent (" + String.format("%.2f", cai) + ") - high expression likely ✓");
        } else if (cai >= 0.6) {
            recommendations.add("CAI: Good (" + String.format("%.2f", cai) + ") - adequate expression expected ✓");
        } else if (cai >= 0.4) {
            recommendations.add("CAI: Medium (" + String.format("%.2f", cai) + ") - optimization recommended");
            recommendations.add("  → Replace underused codons with more frequent synonyms");
        } else {
            recommendations.add("CAI: Low (" + String.format("%.2f", cai) + ") - strong optimization required");
            recommendations.add("  → Redesign the sequence with host-specific codon optimization");
        }

        // GC3 assessment
        if (gc3 >= 0.5) {
            recommendations.add(String.format("GC3: %.0f%% - balanced 3rd position ✓", gc3 * 100));
        } else {
            recommendations.add(String.format("GC3: %.0f%% - low, GC-rich codons could improve expression", gc3 * 100));
        }

        // Identify overused codons (RSCU > 1.5)
        List<Map.Entry<String, Double>> overused = rscu.entrySet().stream()
            .filter(e -> e.getValue() > 1.5)
            .sorted((a, b) -> Double.compare(b.getValue(), a.getValue()))
            .toList();

        if (!overused.isEmpty()) {
            recommendations.add("\nOverused codons (RSCU > 1.5):");

            for (int i = 0; i < Math.min(5, overused.size()); i++) {
                Map.Entry<String, Double> entry = overused.get(i);
                String aa = CODON_TABLE.getOrDefault(entry.getKey(), "?");
                recommendations.add(String.format("  %s → %s (RSCU=%.2f)", entry.getKey(), aa, entry.getValue()));
            }

            recommendations.add("  → Consider replacing with rarer synonyms");
        }

        return recommendations;
    }

    // ======== RESULT ==========

    /** Result of the codon usage analysis */
    public static class CodonUsageResult {
        private Map<String, Integer> codonCounts;
        private Map<String, Integer> aminoAcidCounts;
        private double cai;  // Codon Adaptation Index (0-1)
        private double gc3Content;  // GC content of the 3rd codon position
        private Map<String, Double> rscu;  // Relative Synonymous Codon Usage
        private List<String> recommendations;

        public CodonUsageResult(Map<String, Integer> codonCounts, Map<String, Integer> aminoAcidCounts,
                                double cai, double gc3Content, Map<String, Double> rscu, List<String> recommendations) {
            this.codonCounts = codonCounts;
            this.aminoAcidCounts = aminoAcidCounts;
            this.cai = cai;
            this.gc3Content = gc3Content;
            this.rscu = rscu;
            this.recommendations = recommendations;
        }

        public Map<String, Integer> getCodonCounts() { return codonCounts; }
        public Map<String, Integer> getAminoAcidCounts() { return aminoAcidCounts; }
        public double getCai() { return cai; }
        public double getGc3Content() { return gc3Content; }
        public Map<String, Double> getRscu() { return rscu; }
        public List<String> getRecommendations() { return recommendations; }

        /** CAI as text */
        public String caiLabel() {
            if (cai >= 0.8) return "Hoch (" + String.format("%.2f", cai) + ") ✓✓";
            if (cai >= 0.6) return "Medium-High (" + String.format("%.2f", cai) + ") ✓";
            if (cai >= 0.4) return "Medium (" + String.format("%.2f", cai) + ")";
            return "Low (" + String.format("%.2f", cai) + ") ✗";
        }

        /** Generate the full analysis report */
        public String generateReport() {
            StringBuilder sb = new StringBuilder();

            sb.append("=== Codon Usage Analysis ===\n\n");
            sb.append(String.format("Codon Adaptation Index (CAI): %s\n", caiLabel()));
            sb.append(String.format("GC3 content: %.0f%%\n\n", gc3Content * 100));

            // Top codons
            sb.append("Top 15 Codons (frequency):\n");

            codonCounts.entrySet().stream()
                .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
                .limit(15)
                .forEach(entry -> {
                    String aa = CODON_TABLE.getOrDefault(entry.getKey(), "?");
                    sb.append(String.format("  %s → %s: %d\n", entry.getKey(), aa, entry.getValue()));
                });

            // Amino acid composition
            sb.append("\nAmino acids (count):\n");

            aminoAcidCounts.entrySet().stream()
                .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
                .forEach(entry -> {
                    sb.append(String.format("  %s: %d\n", entry.getKey(), entry.getValue()));
                });

            // Optimization suggestions
            sb.append("\nOptimization recommendations:\n");

            for (String rec : recommendations) {
                sb.append(rec).append("\n");
            }

            // RSCU table (overused codons)
            List<Map.Entry<String, Double>> overused = rscu.entrySet().stream()
                .filter(e -> e.getValue() > 1.5)
                .sorted((a, b) -> Double.compare(b.getValue(), a.getValue()))
                .toList();

            if (!overused.isEmpty()) {
                sb.append("\nOverused codons (RSCU > 1.5):\n");

                for (Map.Entry<String, Double> entry : overused) {
                    sb.append(String.format("  %s: RSCU=%.2f\n", entry.getKey(), entry.getValue()));
                }
            }

            return sb.toString();
        }

        @Override
        public String toString() {
            return generateReport();
        }
    }

    // ======== MAIN FUNCTIONS FOR UI ==========

    /** Analyze a sequence and return the report */
    public static String analyzeAndReport(String sequence) {
        try {
            CodonUsageResult result = analyzeCodonUsage(sequence);
            return result.generateReport();

        } catch (IllegalArgumentException e) {
            return "Error: " + e.getMessage();
        }
    }

    /** Compare codon usage of two sequences */
    public static String compareCodonUsage(String sequence1, String sequence2) {
        try {
            CodonUsageResult result1 = analyzeCodonUsage(sequence1);
            CodonUsageResult result2 = analyzeCodonUsage(sequence2);

            StringBuilder sb = new StringBuilder();
            sb.append("=== Codon Usage Comparison ===\n\n");

            sb.append(String.format("%s:\n", "Sequence 1"));
            sb.append(String.format("  CAI: %s\n", result1.caiLabel()));
            sb.append(String.format("  GC3: %.0f%%\n", result1.gc3Content * 100));

            sb.append(String.format("\n%s:\n", "Sequence 2"));
            sb.append(String.format("  CAI: %s\n", result2.caiLabel()));
            sb.append(String.format("  GC3: %.0f%%\n", result2.gc3Content * 100));

            double caiDiff = Math.abs(result1.cai - result2.cai);
            sb.append(String.format("\nCAI difference: %.2f\n", caiDiff));

            if (caiDiff < 0.1) {
                sb.append("Sequences have similar codon usage ✓");
            } else {
                sb.append("Codon usage differs significantly");
            }

            return sb.toString();

        } catch (IllegalArgumentException e) {
            return "Error: " + e.getMessage();
        }
    }

    /** Generate an optimized sequence (simplified codon optimization) */
    public static String optimizeCodons(String sequence, String targetOrganism) {
        // Simplified optimization: replace underused codons with more frequent synonyms
        // A full implementation would use a reference codon usage table for the organism

        String seq = sequence.toUpperCase().replaceAll("[^ATGC]", "");
        StringBuilder optimized = new StringBuilder(seq.length());

        for (int i = 0; i <= seq.length() - 3; i += 3) {
            String codon = seq.substring(i, Math.min(i + 3, seq.length()));

            if (codon.length() == 3 && CODON_TABLE.containsKey(codon)) {
                String aa = CODON_TABLE.get(codon);

                if (!"*".equals(aa)) {
                    // Replace with the most frequent codon for this organism (simplified)
                    List<String> codonsForAA = CODONS_BY_AMINO_ACID.get(aa);

                    if (codonsForAA != null && codonsForAA.size() > 1) {
                        // Pick a middle codon (as a placeholder for organism-specific optimization)
                        String preferred = codonsForAA.get(codonsForAA.size() / 2);
                        optimized.append(preferred);
                    } else {
                        optimized.append(codon);  // only codon → unchanged
                    }
                } else {
                    optimized.append(codon);  // stop codons unchanged
                }
            } else {
                optimized.append(codon);  // incomplete codons unchanged
            }
        }

        return optimized.toString();
    }
}
