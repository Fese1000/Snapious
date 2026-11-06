package com.snapious.quality;

import java.util.*;

/**
 * Quality-Score-Konvertierung und -Analyse für NGS-Daten.
 * Unterstützt: Sanger (Phred+33), Solexa (Phred-11), Illumina 1.3+/1.9+ Encoding.
 * Erkennt automatisch Encoding und konvertiert zwischen Formaten.
 */
public class QualityScoreConverter {

    // ======== QUALITY ENCODING TYPES ==========

    public enum EncodingType {
        SANGER("Sanger / Illumina 1.8+", "Phred+33", 33),
        SOLEXA("Solexa / Illumina 1.0", "Solexa+64", 64),
        ILLUMINA_13("Illumina 1.3-1.7", "Phred+64", 64);

        private final String description;
        private final String notation;
        private final int offset;

        EncodingType(String description, String notation, int offset) {
            this.description = description;
            this.notation = notation;
            this.offset = offset;
        }

        public String getDescription() { return description; }
        public String getNotation() { return notation; }
        public int getOffset() { return offset; }
    }

    // ======== QUALITY SCORE REPRESANTATION ==========

    /** Repräsentiert einen Quality-Score in Phred-Notation */
    public static class QualityScore {
        private final int phredScore;  // Phred-Qualität: Q = -10 * log10(P_error)

        public QualityScore(int phredScore) {
            this.phredScore = Math.max(0, Math.min(93, phredScore));  // Range 0-93
        }

        public int getPhredScore() { return phredScore; }

        /** Fehlerwahrscheinlichkeit berechnen */
        public double getErrorProbability() {
            return Math.pow(10.0, -phredScore / 10.0);
        }

        /** Encoding in ASCII-Zeichen (je nach Format) */
        public String toAscii(EncodingType encoding) {
            return String.valueOf((char) (phredScore + encoding.getOffset()));
        }

        /** Von ASCII-Zeichen erstellen */
        public static QualityScore fromAscii(char asciiChar, EncodingType encoding) {
            return new QualityScore(asciiChar - encoding.getOffset());
        }

        @Override
        public String toString() {
            return "Q" + phredScore;
        }

        public int hashCode() { return phredScore; }

        @Override
        public boolean equals(Object o) {
            return this == o || (o instanceof QualityScore qs && phredScore == qs.phredScore);
        }
    }

    // ======== ENCODING ERKENNUNG ==========

    /**
     * Erkenne Quality-Encoding automatisch aus einer Sequenz von ASCII-Zeichen.
     * Analysiert den Zeichenbereich und bestimmt das wahrscheinlichste Encoding.
     */
    public static EncodingType detectEncoding(String qualityString) {
        if (qualityString == null || qualityString.isEmpty()) {
            throw new IllegalArgumentException("Qualitätsstring darf nicht leer sein");
        }
        return classify(qualityString.chars().min().getAsInt());
    }

    /** Erkenne Encoding für mehrere Quality-Strings (für FASTQ) */
    public static EncodingType detectEncoding(List<String> qualityStrings) {
        int globalMin = Integer.MAX_VALUE;
        for (String qs : qualityStrings) {
            if (qs == null || qs.isEmpty()) continue;
            globalMin = Math.min(globalMin, qs.chars().min().getAsInt());
        }
        return globalMin == Integer.MAX_VALUE ? EncodingType.SANGER : classify(globalMin);
    }

    /**
     * Klassifikation über das kleinste vorkommende Zeichen (übliche Heuristik):
     * &lt; ';' (59) nur bei Phred+33, &lt; '@' (64) nur bei Solexa (Scores bis -5),
     * ab '@' Phred+64. Ohne niedrige Qualitäten ist Phred+64 nicht von Phred+33
     * zu unterscheiden; dann gilt das heute übliche Phred+64 erst ab '@'.
     */
    private static EncodingType classify(int minChar) {
        if (minChar < 59) return EncodingType.SANGER;
        if (minChar < 64) return EncodingType.SOLEXA;
        return EncodingType.ILLUMINA_13;
    }

    // ======== CONVERSION ==========

    /**
     * Konvertiere Quality-String von einem Encoding in ein anderes.
     */
    public static String convertEncoding(String qualityString, EncodingType from, EncodingType to) {
        if (qualityString == null || qualityString.isEmpty()) return "";
        return fromPhredScores(toPhredScores(qualityString, from), to);
    }

    /** Konvertiere gesamten Quality-String zu Phred-Werten */
    public static List<Integer> toPhredScores(String qualityString, EncodingType encoding) {
        List<Integer> scores = new ArrayList<>(qualityString.length());

        for (int i = 0; i < qualityString.length(); i++) {
            int raw = qualityString.charAt(i) - encoding.getOffset();
            int phred = encoding == EncodingType.SOLEXA ? solexaToPhred(raw) : raw;
            scores.add(Math.max(0, phred));
        }

        return scores;
    }

    /** Konvertiere Phred-Scores zurück zu ASCII-String */
    public static String fromPhredScores(List<Integer> phredScores, EncodingType encoding) {
        StringBuilder sb = new StringBuilder(phredScores.size());

        for (int phred : phredScores) {
            int raw = encoding == EncodingType.SOLEXA ? phredToSolexa(phred) : phred;
            int ascii = raw + encoding.getOffset();
            sb.append((char) Math.max('!', Math.min(ascii, '~')));  // Begrenze auf druckbare Zeichen
        }

        return sb.toString();
    }

    /** Solexa-Score (Odds-basiert) → Phred: Q = 10*log10(10^(Qs/10) + 1) */
    private static int solexaToPhred(int solexa) {
        return (int) Math.round(10.0 * Math.log10(Math.pow(10.0, solexa / 10.0) + 1.0));
    }

    /** Phred → Solexa-Score: Qs = 10*log10(10^(Q/10) - 1); Q=0 wird auf -5 begrenzt */
    private static int phredToSolexa(int phred) {
        if (phred <= 0) return -5;
        return (int) Math.round(10.0 * Math.log10(Math.pow(10.0, phred / 10.0) - 1.0));
    }

    // ======== QUALITY ANALYSE ==========

    /**
     * Analysiere Quality-Statistik eines FASTQ-Quality-Sstrings.
     */
    public static class QualityStatistics {
        private final double mean;       // Mittelwert
        private final double median;     // Median
        private final double min;        // Minimum
        private final double max;        // Maximum
        private final int length;        // Sequenzlänge

        public QualityStatistics(double mean, double median, double min, double max, int length) {
            this.mean = mean;
            this.median = median;
            this.min = min;
            this.max = max;
            this.length = length;
        }

        public double getMean() { return mean; }
        public double getMedian() { return median; }
        public double getMin() { return min; }
        public double getMax() { return max; }
        public int getLength() { return length; }

        /** Basis-Qualitätsbewertung */
        public String qualityLabel() {
            if (mean >= 30) return "Hochwertig ✓✓";       // Q30+
            if (mean >= 20) return "Mittelwertig ✓";        // Q20+
            if (mean >= 10) return "Niedrig";               // Q10+
            return "Unzureichend ✗";                         // <Q10
        }

        @Override
        public String toString() {
            return String.format("QualityStatistics[Mean=%.1f, Median=%.1f, Range=[%.0f-%.0f], Len=%d, %s]",
                mean, median, min, max, length, qualityLabel());
        }
    }

    /** Berechne Quality-Statistik für einen String */
    public static QualityStatistics calculateQualityStats(String qualityString, EncodingType encoding) {
        if (qualityString == null || qualityString.isEmpty()) {
            return new QualityStatistics(0, 0, 0, 0, 0);
        }

        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        if (phredScores.isEmpty()) {
            return new QualityStatistics(0, 0, 0, 0, 0);
        }

        int[] scores = phredScores.stream().mapToInt(Integer::intValue).toArray();
        Arrays.sort(scores);

        // Statistiken berechnen (jetzt mit int[])
        double mean = Arrays.stream(scores).average().orElse(0);
        double median = scores[scores.length / 2];
        double min = scores[0];
        double max = scores[scores.length - 1];

        return new QualityStatistics(mean, median, min, max, qualityString.length());
    }

    /** Berechne Statistik über mehrere Quality-Strings (für FASTQ-Datei) */
    public static List<QualityStatistics> calculateBatchStats(List<String> qualityStrings, EncodingType encoding) {
        List<QualityStatistics> stats = new ArrayList<>();

        for (String qs : qualityStrings) {
            if (qs != null && !qs.isEmpty()) {
                stats.add(calculateQualityStats(qs, encoding));
            }
        }

        return stats;
    }

    /** Generiere Quality-Histogramm für Visualisierung */
    public static Map<Integer, Integer> qualityHistogram(String qualityString, EncodingType encoding) {
        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        Map<Integer, Integer> histogram = new TreeMap<>();  // Sortiert nach Score

        for (int score : phredScores) {
            histogram.merge(score, 1, Integer::sum);
        }

        return histogram;
    }

    /** Filtere Reads mit niedriger Quality (Quality-Threshold) */
    public static String filterByQuality(String qualityString, EncodingType encoding, int minQuality) {
        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        // Prüfe ob alle Scores >= minQuality
        boolean allPass = phredScores.stream().allMatch(s -> s >= minQuality);

        if (allPass) return qualityString;  // Alles OK → original zurück

        // Ansonsten: Ersetze niedrige Quality durch '!' (Q0)
        StringBuilder sb = new StringBuilder(qualityString.length());

        for (int i = 0; i < qualityString.length(); i++) {
            int phredScore = qualityString.charAt(i) - encoding.getOffset();

            if (phredScore < minQuality) {
                sb.append('!');  // Ersetze durch niedrigste Qualität
            } else {
                sb.append(qualityString.charAt(i));  // Behalte original
            }
        }

        return sb.toString();
    }

    /** Zähle Basis mit Quality >= threshold */
    public static int countBasesAboveQuality(String qualityString, EncodingType encoding, int minQuality) {
        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        return (int) phredScores.stream().filter(s -> s >= minQuality).count();
    }

    /** Generiere qualitätsbasierte Maskierung (N für niedrige Quality) */
    public static String maskLowQualityBases(String sequence, EncodingType encoding, String qualityStr, int minQuality) {
        if (sequence == null || sequence.isEmpty() || qualityStr == null || qualityStr.isEmpty()) {
            return "";
        }

        List<Integer> phredScores = toPhredScores(qualityStr, encoding);
        StringBuilder masked = new StringBuilder(sequence.length());

        for (int i = 0; i < sequence.length(); i++) {
            int phredScore = (i < phredScores.size()) ? phredScores.get(i) : 0;

            if (phredScore >= minQuality) {
                masked.append(sequence.charAt(i));  // Hohe Quality → behalten
            } else {
                masked.append('N');  // Niedrige Quality → maskieren
            }
        }

        return masked.toString();
    }

    // ======== HAUPTFUNKTIONEN FÜR UI ==========

    /** Generiere Quality-Bericht für FASTQ-Daten */
    public static String generateQualityReport(List<String> qualityStrings) {
        if (qualityStrings.isEmpty()) return "Keine Daten vorhanden.";

        EncodingType encoding = detectEncoding(qualityStrings);
        List<QualityStatistics> statsList = calculateBatchStats(qualityStrings, encoding);

        StringBuilder sb = new StringBuilder();
        sb.append("=== Quality-Bericht ===\n\n");
        sb.append(String.format("Erkanntes Encoding: %s (%s)\n", encoding.getDescription(), encoding.getNotation()));
        sb.append(String.format("Anzahl Reads: %d\n", statsList.size()));

        // Gesamte Statistik
        double[] allMeans = statsList.stream().mapToDouble(QualityStatistics::getMean).toArray();
        double overallMean = Arrays.stream(allMeans).average().orElse(0);

        sb.append(String.format("Gesamter Mittelwert: %.1f\n", overallMean));
        sb.append(String.format("Qualitätsbewertung: %s\n", qualityLabel(overallMean)));

        // Pro-Read-Zusammenfassung
        sb.append("\nTop 10 Reads (nach Mean-Quality):\n");

        List<QualityStatistics> sorted = statsList.stream()
            .sorted((a, b) -> Double.compare(b.getMean(), a.getMean()))
            .limit(10)
            .toList();

        for (int i = 0; i < sorted.size(); i++) {
            QualityStatistics qs = sorted.get(i);
            sb.append(String.format("%d. %s\n", i + 1, qs));
        }

        // Low-Quality-Statistics
        long lowQualityCount = statsList.stream().filter(s -> s.getMean() < 20).count();
        sb.append(String.format("\nLow-Quality Reads (Mean < 20): %d (%.0f%%)\n",
            lowQualityCount, (double) lowQualityCount / statsList.size() * 100));

        return sb.toString();
    }

    /** Quality-Bewertung als Text */
    private static String qualityLabel(double meanQ) {
        if (meanQ >= 30) return "Hochwertig ✓✓";
        if (meanQ >= 20) return "Mittelwertig ✓";
        if (meanQ >= 10) return "Niedrig";
        return "Unzureichend ✗";
    }

    /** Konvertiere FASTQ-Quality-Strings zu Phred-Werten für Export */
    public static List<List<Integer>> convertToPhred(List<String> qualityStrings, EncodingType encoding) {
        return qualityStrings.stream()
            .map(qs -> toPhredScores(qs, encoding))
            .toList();
    }

    /** Konvertiere Phred-Werte zurück zu FASTQ-Quality-Strings */
    public static List<String> convertFromPhred(List<List<Integer>> phredLists, EncodingType encoding) {
        return phredLists.stream()
            .map(scores -> fromPhredScores(scores, encoding))
            .toList();
    }
}
