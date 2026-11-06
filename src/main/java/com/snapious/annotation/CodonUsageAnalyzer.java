package com.snapious.annotation;

import java.util.*;

/**
 * Codon Usage Analyzer für Genexpressions-Optimierung.
 * Berechnet: CAI (Codon Adaptation Index), GC3-Gehalt, RSCU (Relative Synonymous Codon Usage).
 */
public class CodonUsageAnalyzer {

    // ======== KODON-TABELLE (Standard Genetic Code) ==========

    private static final Map<String, String> CODON_TABLE;

    static {
        CODON_TABLE = new HashMap<>();

        // Start-Codons
        CODON_TABLE.put("ATG", "M");  // Methionin (Start)
        CODON_TABLE.put("TTG", "L");  // Alternative Start-Codons
        CODON_TABLE.put("GTG", "V");

        // Phenylalanin
        CODON_TABLE.put("TTT", "F"); CODON_TABLE.put("TTC", "F");

        // Leucin
        CODON_TABLE.put("TTA", "L"); CODON_TABLE.put("TTG", "L");
        CODON_TABLE.put("CTT", "L"); CODON_TABLE.put("CTC", "L");
        CODON_TABLE.put("CTA", "L"); CODON_TABLE.put("CTG", "L");

        // Isoleucin
        CODON_TABLE.put("ATT", "I"); CODON_TABLE.put("ATC", "I"); CODON_TABLE.put("ATA", "I");

        // Valin
        CODON_TABLE.put("GTT", "V"); CODON_TABLE.put("GTC", "V");
        CODON_TABLE.put("GTA", "V"); CODON_TABLE.put("GTG", "V");

        // Serin
        CODON_TABLE.put("TCT", "S"); CODON_TABLE.put("TCC", "S");
        CODON_TABLE.put("TCA", "S"); CODON_TABLE.put("TCG", "S");
        CODON_TABLE.put("AGT", "S"); CODON_TABLE.put("AGC", "S");

        // Prolin
        CODON_TABLE.put("CCT", "P"); CODON_TABLE.put("CCC", "P");
        CODON_TABLE.put("CCA", "P"); CODON_TABLE.put("CCG", "P");

        // Threonin
        CODON_TABLE.put("ACT", "T"); CODON_TABLE.put("ACC", "T");
        CODON_TABLE.put("ACA", "T"); CODON_TABLE.put("ACG", "T");

        // Alanin
        CODON_TABLE.put("GCT", "A"); CODON_TABLE.put("GCC", "A");
        CODON_TABLE.put("GCA", "A"); CODON_TABLE.put("GCG", "A");

        // Tyrosin
        CODON_TABLE.put("TAT", "Y"); CODON_TABLE.put("TAC", "Y");

        // Histidin
        CODON_TABLE.put("CAT", "H"); CODON_TABLE.put("CAC", "H");

        // Glutamin
        CODON_TABLE.put("CAA", "Q"); CODON_TABLE.put("CAG", "Q");

        // Asparagin
        CODON_TABLE.put("AAT", "N"); CODON_TABLE.put("AAC", "N");

        // Lysin
        CODON_TABLE.put("AAA", "K"); CODON_TABLE.put("AAG", "K");

        // Aspartat
        CODON_TABLE.put("GAT", "D"); CODON_TABLE.put("GAC", "D");

        // Glutamat
        CODON_TABLE.put("GAA", "E"); CODON_TABLE.put("GAG", "E");

        // Cystein
        CODON_TABLE.put("TGT", "C"); CODON_TABLE.put("TGC", "C");

        // Tryptophan
        CODON_TABLE.put("TGG", "W");

        // Arginin
        CODON_TABLE.put("CGT", "R"); CODON_TABLE.put("CGC", "R");
        CODON_TABLE.put("CGA", "R"); CODON_TABLE.put("CGG", "R");
        CODON_TABLE.put("AGA", "R"); CODON_TABLE.put("AGG", "R");

        // Glycin
        CODON_TABLE.put("GGT", "G"); CODON_TABLE.put("GGC", "G");
        CODON_TABLE.put("GGA", "G"); CODON_TABLE.put("GGG", "G");

        // Stop-Codons
        CODON_TABLE.put("TAA", "*"); CODON_TABLE.put("TAG", "*"); CODON_TABLE.put("TGA", "*");
    }

    // ======== AMINOSÄUREN ==========

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

    // ======== HAUPTANALYSE ==========

    /**
     * Analysiere Codon Usage einer DNA-Sequenz.
     */
    public static CodonUsageResult analyzeCodonUsage(String sequence) {
        String seq = sequence.toUpperCase().replaceAll("[^ATGC]", "");

        if (seq.length() < 3) {
            throw new IllegalArgumentException("Sequenz zu kurz für Codon-Analyse (< 3bp)");
        }

        // Zähle alle Codons
        Map<String, Integer> codonCounts = new LinkedHashMap<>();  // Reihenfolge der Erscheinung

        for (int i = 0; i <= seq.length() - 3; i += 3) {
            String codon = seq.substring(i, Math.min(i + 3, seq.length()));

            if (codon.length() == 3) {
                codonCounts.merge(codon, 1, Integer::sum);
            }
        }

        // Zähle Aminosäuren (ohne Stop)
        Map<String, Integer> aaCounts = new HashMap<>();
        int totalCodons = 0;

        for (Map.Entry<String, Integer> entry : codonCounts.entrySet()) {
            String codon = entry.getKey();
            int count = entry.getValue();

            // Überspringe unvollständige Codons und Stop-Codons für CAI
            if (CODON_TABLE.containsKey(codon)) {
                String aa = CODON_TABLE.get(codon);

                if (!"*".equals(aa)) {  // Ignoriere Stop-Codons für CAI
                    aaCounts.merge(aa, count, Integer::sum);
                    totalCodons += count;
                }
            }
        }

        // Berechne CAI (Codon Adaptation Index)
        double cai = calculateCAI(codonCounts, aaCounts);

        // Berechne GC3-Gehalt
        double gc3 = calculateGC3(seq, codonCounts);

        // Berechne RSCU (Relative Synonymous Codon Usage)
        Map<String, Double> rscu = calculateRSCU(codonCounts, aaCounts);

        // Generiere Optimierungsvorschläge
        List<String> recommendations = generateRecommendations(cai, gc3, rscu);

        return new CodonUsageResult(codonCounts, aaCounts, cai, gc3, rscu, recommendations);
    }

    /** CAI (Codon Adaptation Index) berechnen. 0-1, wobei 1 optimale Codon-Nutzung ist. */
    private static double calculateCAI(Map<String, Integer> codonCounts, Map<String, Integer> aaCounts) {
        // Verwende Referenz-Codon-Usage-Gewichte (vereinfacht)
        // In einer vollständigen Implementierung würde man eine Referenz-Sequenzbank verwenden

        double sumWeights = 0;
        int nonStopCodons = 0;

        for (Map.Entry<String, Integer> entry : codonCounts.entrySet()) {
            String codon = entry.getKey();

            if (CODON_TABLE.containsKey(codon) && !"*".equals(CODON_TABLE.get(codon))) {
                String aa = CODON_TABLE.get(codon);
                List<String> codonsForAA = CODONS_BY_AMINO_ACID.get(aa);

                if (codonsForAA != null) {
                    // Vereinfachte Gewichtung: 1/Nr der Synonyme (optimistisch)
                    double weight = codonsForAA.size() == 1 ? 1.0 : (double) 1 / codonsForAA.size();

                    sumWeights += weight * entry.getValue();
                    nonStopCodons += entry.getValue();
                }
            }
        }

        if (nonStopCodons == 0) return 0.0;

        // Geometrischer Mittelwert
        double geometricMean = Math.pow(sumWeights / nonStopCodons, 1.0 / nonStopCodons);

        return Math.min(1.0, Math.max(0.0, geometricMean));
    }

    /** GC3-Gehalt berechnen (GC-Gehalt der 3. Codon-Position) */
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

            if ("*".equals(aa)) continue;  // Ignoriere Stop-Codons

            List<String> codons = aaEntry.getValue();
            int totalAA = aaCounts.getOrDefault(aa, 0);

            for (String codon : codons) {
                int count = codonCounts.getOrDefault(codon, 0);

                if (totalAA == 0) {
                    rscu.put(codon, 0.0);
                } else {
                    // RSCU = observed / expected (expected = totalAA / Nr der Synonyme)
                    double rscuValue = (double) count / (totalAA / (double) codons.size());
                    rscu.put(codon, Math.round(rscuValue * 100.0) / 100.0);
                }
            }
        }

        return rscu;
    }

    /** Generiere Optimierungsvorschläge basierend auf der Analyse */
    private static List<String> generateRecommendations(double cai, double gc3, Map<String, Double> rscu) {
        List<String> recommendations = new ArrayList<>();

        // CAI-Bewertung
        if (cai >= 0.8) {
            recommendations.add("CAI: Ausgezeichnet (" + String.format("%.2f", cai) + ") - hohe Expression wahrscheinlich ✓");
        } else if (cai >= 0.6) {
            recommendations.add("CAI: Gut (" + String.format("%.2f", cai) + ") - angemessene Expression erwartet ✓");
        } else if (cai >= 0.4) {
            recommendations.add("CAI: Mittel (" + String.format("%.2f", cai) + ") - Optimierung empfohlen");
            recommendations.add("  → Ersetze unternutzte Codons durch häufigere Synonyme");
        } else {
            recommendations.add("CAI: Niedrig (" + String.format("%.2f", cai) + ") - starke Optimierung erforderlich");
            recommendations.add("  → Redesign der Sequenz mit Wirtsspezifischer Codon-Optimierung");
        }

        // GC3-Bewertung
        if (gc3 >= 0.5) {
            recommendations.add(String.format("GC3: %.0f%% - ausgewogene 3. Position ✓", gc3 * 100));
        } else {
            recommendations.add(String.format("GC3: %.0f%% - niedrig, GC-reiche Codons könnten Expression verbessern", gc3 * 100));
        }

        // Übernutzte Codons identifizieren (RSCU > 1.5)
        List<Map.Entry<String, Double>> overused = rscu.entrySet().stream()
            .filter(e -> e.getValue() > 1.5)
            .sorted((a, b) -> Double.compare(b.getValue(), a.getValue()))
            .toList();

        if (!overused.isEmpty()) {
            recommendations.add("\nÜbernutzte Codons (RSCU > 1.5):");

            for (int i = 0; i < Math.min(5, overused.size()); i++) {
                Map.Entry<String, Double> entry = overused.get(i);
                String aa = CODON_TABLE.getOrDefault(entry.getKey(), "?");
                recommendations.add(String.format("  %s → %s (RSCU=%.2f)", entry.getKey(), aa, entry.getValue()));
            }

            recommendations.add("  → Erwäge Ersatz durch seltenere Synonyme");
        }

        return recommendations;
    }

    // ======== RESULT ==========

    /** Ergebnis der Codon Usage Analyse */
    public static class CodonUsageResult {
        private Map<String, Integer> codonCounts;
        private Map<String, Integer> aminoAcidCounts;
        private double cai;  // Codon Adaptation Index (0-1)
        private double gc3Content;  // GC-Gehalt der 3. Codon-Position
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

        /** CAI als Text */
        public String caiLabel() {
            if (cai >= 0.8) return "Hoch (" + String.format("%.2f", cai) + ") ✓✓";
            if (cai >= 0.6) return "Mittel-Hoch (" + String.format("%.2f", cai) + ") ✓";
            if (cai >= 0.4) return "Mittel (" + String.format("%.2f", cai) + ")";
            return "Niedrig (" + String.format("%.2f", cai) + ") ✗";
        }

        /** Generiere vollständigen Analysebericht */
        public String generateReport() {
            StringBuilder sb = new StringBuilder();

            sb.append("=== Codon Usage Analyse ===\n\n");
            sb.append(String.format("Codon Adaptation Index (CAI): %s\n", caiLabel()));
            sb.append(String.format("GC3-Gehalt: %.0f%%\n\n", gc3Content * 100));

            // Top-Codons
            sb.append("Top 15 Codons (Häufigkeit):\n");

            codonCounts.entrySet().stream()
                .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
                .limit(15)
                .forEach(entry -> {
                    String aa = CODON_TABLE.getOrDefault(entry.getKey(), "?");
                    sb.append(String.format("  %s → %s: %d\n", entry.getKey(), aa, entry.getValue()));
                });

            // Aminosäure-Zusammensetzung
            sb.append("\nAminosäuren (Anzahl):\n");

            aminoAcidCounts.entrySet().stream()
                .sorted((a, b) -> Integer.compare(b.getValue(), a.getValue()))
                .forEach(entry -> {
                    sb.append(String.format("  %s: %d\n", entry.getKey(), entry.getValue()));
                });

            // Optimierungsvorschläge
            sb.append("\nOptimierungsempfehlungen:\n");

            for (String rec : recommendations) {
                sb.append(rec).append("\n");
            }

            // RSCU-Tabelle (übernutzte Codons)
            List<Map.Entry<String, Double>> overused = rscu.entrySet().stream()
                .filter(e -> e.getValue() > 1.5)
                .sorted((a, b) -> Double.compare(b.getValue(), a.getValue()))
                .toList();

            if (!overused.isEmpty()) {
                sb.append("\nÜbernutzte Codons (RSCU > 1.5):\n");

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

    // ======== HAUPTFUNKTIONEN FÜR UI ==========

    /** Analysiere Sequenz und gib Bericht zurück */
    public static String analyzeAndReport(String sequence) {
        try {
            CodonUsageResult result = analyzeCodonUsage(sequence);
            return result.generateReport();

        } catch (IllegalArgumentException e) {
            return "Fehler: " + e.getMessage();
        }
    }

    /** Vergleiche Codon Usage zweier Sequenzen */
    public static String compareCodonUsage(String sequence1, String sequence2) {
        try {
            CodonUsageResult result1 = analyzeCodonUsage(sequence1);
            CodonUsageResult result2 = analyzeCodonUsage(sequence2);

            StringBuilder sb = new StringBuilder();
            sb.append("=== Codon Usage Vergleich ===\n\n");

            sb.append(String.format("%s:\n", "Sequenz 1"));
            sb.append(String.format("  CAI: %s\n", result1.caiLabel()));
            sb.append(String.format("  GC3: %.0f%%\n", result1.gc3Content * 100));

            sb.append(String.format("\n%s:\n", "Sequenz 2"));
            sb.append(String.format("  CAI: %s\n", result2.caiLabel()));
            sb.append(String.format("  GC3: %.0f%%\n", result2.gc3Content * 100));

            double caiDiff = Math.abs(result1.cai - result2.cai);
            sb.append(String.format("\nCAI-Differenz: %.2f\n", caiDiff));

            if (caiDiff < 0.1) {
                sb.append("Sequenzen haben ähnliche Codon-Nutzung ✓");
            } else {
                sb.append("Codon-Nutzung unterscheidet sich signifikant");
            }

            return sb.toString();

        } catch (IllegalArgumentException e) {
            return "Fehler: " + e.getMessage();
        }
    }

    /** Generiere optimierte Sequenz (vereinfachte Codon-Optimierung) */
    public static String optimizeCodons(String sequence, String targetOrganism) {
        // Vereinfachte Optimierung: Ersetze unternutzte Codons durch häufigere Synonyme
        // In einer vollständigen Implementierung würde man eine Referenz-Codon-Usage-Tabelle für den Organismus verwenden

        String seq = sequence.toUpperCase().replaceAll("[^ATGC]", "");
        StringBuilder optimized = new StringBuilder(seq.length());

        for (int i = 0; i <= seq.length() - 3; i += 3) {
            String codon = seq.substring(i, Math.min(i + 3, seq.length()));

            if (codon.length() == 3 && CODON_TABLE.containsKey(codon)) {
                String aa = CODON_TABLE.get(codon);

                if (!"*".equals(aa)) {
                    // Ersetze durch häufigstes Codon für diesen Organismus (vereinfacht)
                    List<String> codonsForAA = CODONS_BY_AMINO_ACID.get(aa);

                    if (codonsForAA != null && codonsForAA.size() > 1) {
                        // Wähle mittleres Codon (als Platzhalter für Organismus-spezifische Optimierung)
                        String preferred = codonsForAA.get(codonsForAA.size() / 2);
                        optimized.append(preferred);
                    } else {
                        optimized.append(codon);  // Einziges Codon → unverändert
                    }
                } else {
                    optimized.append(codon);  // Stop-Codons unverändert
                }
            } else {
                optimized.append(codon);  // Unvollständige Codons unverändert
            }
        }

        return optimized.toString();
    }
}
