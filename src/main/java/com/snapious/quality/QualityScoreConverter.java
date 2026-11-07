package com.snapious.quality;

import java.util.*;

/**
 * Quality score conversion and analysis for NGS data.
 * Supports: Sanger (Phred+33), Solexa (Phred-11), Illumina 1.3+/1.9+ encoding.
 * Automatically detects the encoding and converts between formats.
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

    // ======== QUALITY SCORE REPRESENTATION ==========

    /** Represents a quality score in Phred notation */
    public static class QualityScore {
        private final int phredScore;  // Phred quality: Q = -10 * log10(P_error)

        public QualityScore(int phredScore) {
            this.phredScore = Math.max(0, Math.min(93, phredScore));  // Range 0-93
        }

        public int getPhredScore() { return phredScore; }

        /** Compute the error probability */
        public double getErrorProbability() {
            return Math.pow(10.0, -phredScore / 10.0);
        }

        /** Encoding as an ASCII character (depending on format) */
        public String toAscii(EncodingType encoding) {
            return String.valueOf((char) (phredScore + encoding.getOffset()));
        }

        /** Create from an ASCII character */
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

    // ======== ENCODING DETECTION ==========

    /**
     * Automatically detect the quality encoding from a sequence of ASCII characters.
     * Analyzes the character range and determines the most likely encoding.
     */
    public static EncodingType detectEncoding(String qualityString) {
        if (qualityString == null || qualityString.isEmpty()) {
            throw new IllegalArgumentException("Quality string must not be empty");
        }
        return classify(qualityString.chars().min().getAsInt());
    }

    /** Detect the encoding for several quality strings (for FASTQ) */
    public static EncodingType detectEncoding(List<String> qualityStrings) {
        int globalMin = Integer.MAX_VALUE;
        for (String qs : qualityStrings) {
            if (qs == null || qs.isEmpty()) continue;
            globalMin = Math.min(globalMin, qs.chars().min().getAsInt());
        }
        return globalMin == Integer.MAX_VALUE ? EncodingType.SANGER : classify(globalMin);
    }

    /**
     * Classification by the smallest character present (common heuristic):
     * &lt; ';' (59) only for Phred+33, &lt; '@' (64) only for Solexa (scores down to -5),
     * from '@' Phred+64. Without low qualities Phred+64 cannot be distinguished from Phred+33;
     * in that case the now common Phred+64 applies only from '@'.
     */
    private static EncodingType classify(int minChar) {
        if (minChar < 59) return EncodingType.SANGER;
        if (minChar < 64) return EncodingType.SOLEXA;
        return EncodingType.ILLUMINA_13;
    }

    // ======== CONVERSION ==========

    /**
     * Convert a quality string from one encoding to another.
     */
    public static String convertEncoding(String qualityString, EncodingType from, EncodingType to) {
        if (qualityString == null || qualityString.isEmpty()) return "";
        return fromPhredScores(toPhredScores(qualityString, from), to);
    }

    /** Convert a whole quality string to Phred values */
    public static List<Integer> toPhredScores(String qualityString, EncodingType encoding) {
        List<Integer> scores = new ArrayList<>(qualityString.length());

        for (int i = 0; i < qualityString.length(); i++) {
            int raw = qualityString.charAt(i) - encoding.getOffset();
            int phred = encoding == EncodingType.SOLEXA ? solexaToPhred(raw) : raw;
            scores.add(Math.max(0, phred));
        }

        return scores;
    }

    /** Convert Phred scores back to an ASCII string */
    public static String fromPhredScores(List<Integer> phredScores, EncodingType encoding) {
        StringBuilder sb = new StringBuilder(phredScores.size());

        for (int phred : phredScores) {
            int raw = encoding == EncodingType.SOLEXA ? phredToSolexa(phred) : phred;
            int ascii = raw + encoding.getOffset();
            sb.append((char) Math.max('!', Math.min(ascii, '~')));  // limit to printable characters
        }

        return sb.toString();
    }

    /** Solexa score (odds-based) → Phred: Q = 10*log10(10^(Qs/10) + 1) */
    private static int solexaToPhred(int solexa) {
        return (int) Math.round(10.0 * Math.log10(Math.pow(10.0, solexa / 10.0) + 1.0));
    }

    /** Phred → Solexa score: Qs = 10*log10(10^(Q/10) - 1); Q=0 is clamped to -5 */
    private static int phredToSolexa(int phred) {
        if (phred <= 0) return -5;
        return (int) Math.round(10.0 * Math.log10(Math.pow(10.0, phred / 10.0) - 1.0));
    }

    // ======== QUALITY ANALYSIS ==========

    /**
     * Analyze the quality statistics of a FASTQ quality string.
     */
    public static class QualityStatistics {
        private final double mean;       // mean
        private final double median;     // median
        private final double min;        // minimum
        private final double max;        // maximum
        private final int length;        // sequence length

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

        /** Basic quality rating */
        public String qualityLabel() {
            if (mean >= 30) return "High quality ✓✓";       // Q30+
            if (mean >= 20) return "Medium quality ✓";        // Q20+
            if (mean >= 10) return "Low";               // Q10+
            return "Insufficient ✗";                         // <Q10
        }

        @Override
        public String toString() {
            return String.format("QualityStatistics[Mean=%.1f, Median=%.1f, Range=[%.0f-%.0f], Len=%d, %s]",
                mean, median, min, max, length, qualityLabel());
        }
    }

    /** Compute quality statistics for a string */
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

        // Compute statistics (now with int[])
        double mean = Arrays.stream(scores).average().orElse(0);
        double median = scores[scores.length / 2];
        double min = scores[0];
        double max = scores[scores.length - 1];

        return new QualityStatistics(mean, median, min, max, qualityString.length());
    }

    /** Compute statistics over several quality strings (for a FASTQ file) */
    public static List<QualityStatistics> calculateBatchStats(List<String> qualityStrings, EncodingType encoding) {
        List<QualityStatistics> stats = new ArrayList<>();

        for (String qs : qualityStrings) {
            if (qs != null && !qs.isEmpty()) {
                stats.add(calculateQualityStats(qs, encoding));
            }
        }

        return stats;
    }

    /** Generate a quality histogram for visualization */
    public static Map<Integer, Integer> qualityHistogram(String qualityString, EncodingType encoding) {
        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        Map<Integer, Integer> histogram = new TreeMap<>();  // sorted by score

        for (int score : phredScores) {
            histogram.merge(score, 1, Integer::sum);
        }

        return histogram;
    }

    /** Filter reads with low quality (quality threshold) */
    public static String filterByQuality(String qualityString, EncodingType encoding, int minQuality) {
        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        // Check whether all scores are >= minQuality
        boolean allPass = phredScores.stream().allMatch(s -> s >= minQuality);

        if (allPass) return qualityString;  // all OK → return the original

        // Otherwise: replace low quality with '!' (Q0)
        StringBuilder sb = new StringBuilder(qualityString.length());

        for (int i = 0; i < qualityString.length(); i++) {
            int phredScore = qualityString.charAt(i) - encoding.getOffset();

            if (phredScore < minQuality) {
                sb.append('!');  // replace with the lowest quality
            } else {
                sb.append(qualityString.charAt(i));  // keep original
            }
        }

        return sb.toString();
    }

    /** Count bases with quality >= threshold */
    public static int countBasesAboveQuality(String qualityString, EncodingType encoding, int minQuality) {
        List<Integer> phredScores = toPhredScores(qualityString, encoding);

        return (int) phredScores.stream().filter(s -> s >= minQuality).count();
    }

    /** Generate quality-based masking (N for low quality) */
    public static String maskLowQualityBases(String sequence, EncodingType encoding, String qualityStr, int minQuality) {
        if (sequence == null || sequence.isEmpty() || qualityStr == null || qualityStr.isEmpty()) {
            return "";
        }

        List<Integer> phredScores = toPhredScores(qualityStr, encoding);
        StringBuilder masked = new StringBuilder(sequence.length());

        for (int i = 0; i < sequence.length(); i++) {
            int phredScore = (i < phredScores.size()) ? phredScores.get(i) : 0;

            if (phredScore >= minQuality) {
                masked.append(sequence.charAt(i));  // high quality → keep
            } else {
                masked.append('N');  // low quality → mask
            }
        }

        return masked.toString();
    }

    // ======== MAIN FUNCTIONS FOR UI ==========

    /** Generate a quality report for FASTQ data */
    public static String generateQualityReport(List<String> qualityStrings) {
        if (qualityStrings.isEmpty()) return "No data available.";

        EncodingType encoding = detectEncoding(qualityStrings);
        List<QualityStatistics> statsList = calculateBatchStats(qualityStrings, encoding);

        StringBuilder sb = new StringBuilder();
        sb.append("=== Quality Report ===\n\n");
        sb.append(String.format("Detected encoding: %s (%s)\n", encoding.getDescription(), encoding.getNotation()));
        sb.append(String.format("Number of reads: %d\n", statsList.size()));

        // Overall statistics
        double[] allMeans = statsList.stream().mapToDouble(QualityStatistics::getMean).toArray();
        double overallMean = Arrays.stream(allMeans).average().orElse(0);

        sb.append(String.format("Overall mean: %.1f\n", overallMean));
        sb.append(String.format("Quality rating: %s\n", qualityLabel(overallMean)));

        // Per-read summary
        sb.append("\nTop 10 Reads (by mean quality):\n");

        List<QualityStatistics> sorted = statsList.stream()
            .sorted((a, b) -> Double.compare(b.getMean(), a.getMean()))
            .limit(10)
            .toList();

        for (int i = 0; i < sorted.size(); i++) {
            QualityStatistics qs = sorted.get(i);
            sb.append(String.format("%d. %s\n", i + 1, qs));
        }

        // Low-quality statistics
        long lowQualityCount = statsList.stream().filter(s -> s.getMean() < 20).count();
        sb.append(String.format("\nLow-Quality Reads (Mean < 20): %d (%.0f%%)\n",
            lowQualityCount, (double) lowQualityCount / statsList.size() * 100));

        return sb.toString();
    }

    /** Quality rating as text */
    private static String qualityLabel(double meanQ) {
        if (meanQ >= 30) return "High quality ✓✓";
        if (meanQ >= 20) return "Medium quality ✓";
        if (meanQ >= 10) return "Low";
        return "Insufficient ✗";
    }

    /** Convert FASTQ quality strings to Phred values for export */
    public static List<List<Integer>> convertToPhred(List<String> qualityStrings, EncodingType encoding) {
        return qualityStrings.stream()
            .map(qs -> toPhredScores(qs, encoding))
            .toList();
    }

    /** Convert Phred values back to FASTQ quality strings */
    public static List<String> convertFromPhred(List<List<Integer>> phredLists, EncodingType encoding) {
        return phredLists.stream()
            .map(scores -> fromPhredScores(scores, encoding))
            .toList();
    }
}
