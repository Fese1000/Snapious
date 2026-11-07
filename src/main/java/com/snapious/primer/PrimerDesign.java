package com.snapious.primer;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Complete primer design toolkit for PCR, sequencing and cloning.
 * Supports: Tm calculation (Wallace, GC method), self-dimer check,
 * hairpin formation, 3' end stability, forward/reverse primer pair generation.
 */
public class PrimerDesign {

    // ======== PRIMER QUALITY SCORES ==========

    public static class PrimerQuality {
        private double tm;              // melting temperature (°C)
        private double gcContent;       // GC content (0.0 - 1.0)
        private double hairpinTm;       // hairpin stability (°C)
        private double selfDimerTm;     // self-dimer stability (°C)
        private double threePrimeStability;  // 3' end stability (last 5bp)
        private int length;             // primer length

        public PrimerQuality(double tm, double gcContent, double hairpinTm,
                           double selfDimerTm, double threePrimeStability, int length) {
            this.tm = tm;
            this.gcContent = gcContent;
            this.hairpinTm = hairpinTm;
            this.selfDimerTm = selfDimerTm;
            this.threePrimeStability = threePrimeStability;
            this.length = length;
        }

        public double getTm() { return tm; }
        public double getGcContent() { return gcContent; }
        public double getHairpinTm() { return hairpinTm; }
        public double getSelfDimerTm() { return selfDimerTm; }
        public double getThreePrimeStability() { return threePrimeStability; }
        public int getLength() { return length; }

        /** Overall score (0-100) */
        public int qualityScore() {
            int score = 0;

            // Tm range (50-70°C ideal): 30 points
            if (tm >= 50 && tm <= 70) score += 30;
            else if (tm >= 45 && tm <= 75) score += 15;

            // GC content (40-60% ideal): 25 points
            if (gcContent >= 0.4 && gcContent <= 0.6) score += 25;
            else if (gcContent >= 0.3 && gcContent <= 0.7) score += 15;

            // Hairpin Tm < 45°C: 20 points
            if (hairpinTm < 45) score += 20;
            else if (hairpinTm < 50) score += 10;

            // Self-dimer Tm < 45°C: 15 points
            if (selfDimerTm < 45) score += 15;
            else if (selfDimerTm < 50) score += 7;

            // 3' end stability: 10 points
            if (threePrimeStability < 50) score += 10;

            return Math.min(100, score);
        }

        @Override
        public String toString() {
            return String.format("PrimerQuality[Tm=%.1f°C, GC=%.0f%%, Length=%d, Score=%d/100]",
                tm, gcContent * 100, length, qualityScore());
        }
    }

    /** Represents a forward/reverse primer pair */
    public static class PrimerPair {
        private Sequence forward;
        private Sequence reverse;
        private PrimerQuality forwardQuality;
        private PrimerQuality reverseQuality;
        private int expectedProductLength;  // expected product length

        public PrimerPair(Sequence forward, Sequence reverse,
                         PrimerQuality forwardQuality, PrimerQuality reverseQuality,
                         int expectedProductLength) {
            this.forward = forward;
            this.reverse = reverse;
            this.forwardQuality = forwardQuality;
            this.reverseQuality = reverseQuality;
            this.expectedProductLength = expectedProductLength;
        }

        public Sequence getForward() { return forward; }
        public Sequence getReverse() { return reverse; }

        public PrimerQuality getForwardQuality() { return forwardQuality; }
        public PrimerQuality getReverseQuality() { return reverseQuality; }

        /** Expected PCR product length */
        public int getExpectedProductLength() { return expectedProductLength; }

        /** Tm difference between primers of the pair */
        public double getTmDifference() {
            return Math.abs(forwardQuality.getTm() - reverseQuality.getTm());
        }

        /** Overall score of the primer pair */
        public int getPairQualityScore() {
            return (forwardQuality.qualityScore() + reverseQuality.qualityScore()) / 2;
        }

        /** Check whether the primer pair is compatible */
        public boolean arePrimersCompatible() {
            // Tm difference < 5°C: OK
            if (getTmDifference() > 5.0) return false;

            // Self-dimer between forward and reverse < 45°C
            if (calculateSelfDimerScore(forward.getBases(), reverse.getBases()) > 45)
                return false;

            // No complementary 3' ends (>4bp)
            String fwd3end = forward.getBases().substring(Math.max(0, forward.length() - 5));
            String rev3end = reverse.getBases().substring(Math.max(0, reverse.length() - 5));
            String revComp3end = complement(rev3end).substring(0, Math.min(fwd3end.length(), 5));

            if (fwd3end.contains(revComp3end)) return false;

            return true;
        }

        @Override
        public String toString() {
            return String.format("PrimerPair[Forward: %s, Reverse: %s, Product=%dbp, Tm-diff=%.1f°C]",
                forward.getName(), reverse.getName(), expectedProductLength, getTmDifference());
        }

        /** Generate a detailed protocol */
        public String generateProtocol() {
            StringBuilder sb = new StringBuilder();

            sb.append("=== Primer pair design ===\n\n");
            sb.append(String.format("Forward Primer: %s\n", forward.getName()));
            sb.append(String.format("  Sequence: %s\n", forward.getBases()));
            sb.append(forwardQuality.toString()).append("\n\n");

            sb.append(String.format("Reverse Primer: %s\n", reverse.getName()));
            sb.append(String.format("  Sequence: %s\n", reverse.getBases()));
            sb.append(reverseQuality.toString()).append("\n\n");

            sb.append(String.format("Expected product: %d bp\n", expectedProductLength));
            sb.append(String.format("Tm difference: %.1f°C\n", getTmDifference()));
            sb.append(String.format("Compatible: %s\n", arePrimersCompatible() ? "YES ✓" : "NO ✗"));
            sb.append(String.format("Overall quality: %d/100\n", getPairQualityScore()));

            return sb.toString();
        }
    }

    // ======== MAIN FUNCTIONS ==========

    /**
     * Design forward and reverse primers for a given target sequence.
     * Searches the target for optimal primer binding sites.
     */
    public static PrimerPair designPrimerPair(Sequence target, int minLength, int maxLength,
                                               double minTm, double maxTm, double gcMin, double gcMax) {
        String sequence = target.getBases().toUpperCase();
        int seqLen = sequence.length();

        // Reverse complement for the backward primer
        Sequence revComp = target.reverseComplement();
        String rcSeq = revComp.getBases().toUpperCase();

        List<PrimerCandidate> forwardCandidates = new ArrayList<>();
        List<PrimerCandidate> reverseCandidates = new ArrayList<>();

        // Search the target for forward primers
        for (int i = 0; i <= seqLen - minLength; i++) {
            for (int len = minLength; len <= Math.min(maxLength, seqLen - i); len++) {
                String primer = sequence.substring(i, i + len);

                // Check GC content
                double gc = gcContent(primer);
                if (gc < gcMin || gc > gcMax) continue;

                // Check Tm
                double tm = calculateTm(primer, TmMethod.WALLACE);
                if (tm < minTm || tm > maxTm) continue;

                // Evaluate primer quality
                PrimerQuality quality = evaluatePrimer(primer);

                // Store if quality is good
                if (quality.qualityScore() >= 60) {
                    forwardCandidates.add(new PrimerCandidate(
                        primer, i, quality, target.getName() + "_F" + i));
                }
            }
        }

        // Search the reverse complement for backward primers
        for (int i = 0; i <= rcSeq.length() - minLength; i++) {
            for (int len = minLength; len <= Math.min(maxLength, rcSeq.length() - i); len++) {
                String primer = rcSeq.substring(i, i + len);

                double gc = gcContent(primer);
                if (gc < gcMin || gc > gcMax) continue;

                double tm = calculateTm(primer, TmMethod.WALLACE);
                if (tm < minTm || tm > maxTm) continue;

                PrimerQuality quality = evaluatePrimer(primer);

                if (quality.qualityScore() >= 60) {
                    reverseCandidates.add(new PrimerCandidate(
                        primer, i, quality, target.getName() + "_R" + (seqLen - i - len)));
                }
            }
        }

        // Find the best primer pair
        if (forwardCandidates.isEmpty() || reverseCandidates.isEmpty()) {
            throw new IllegalArgumentException(
                "No compatible primers found. Check parameters (length: " + minLength + "-" +
                maxLength + ", Tm: " + minTm + "-" + maxTm + "°C)");
        }

        // Sort by quality and find the best pair with minimal Tm difference
        forwardCandidates.sort((a, b) -> Integer.compare(b.quality.qualityScore(), a.quality.qualityScore()));
        reverseCandidates.sort((a, b) -> Integer.compare(b.quality.qualityScore(), a.quality.qualityScore()));

        PrimerPair bestPair = null;
        double minTmDiff = Double.MAX_VALUE;

        for (PrimerCandidate fwd : forwardCandidates.subList(0, Math.min(10, forwardCandidates.size()))) {
            for (PrimerCandidate rev : reverseCandidates.subList(0, Math.min(10, reverseCandidates.size()))) {
                double tmDiff = Math.abs(fwd.quality.getTm() - rev.quality.getTm());

                if (tmDiff < minTmDiff) {
                    // Compute expected product length
                    int productLength = (seqLen - rev.position - rev.primer.length()) - fwd.position;

                    if (productLength > 0 && productLength < 5000) {
                        minTmDiff = tmDiff;

                        Sequence fwdSeq = new Sequence(fwd.id, fwd.name, fwd.primer.toUpperCase(),
                            Sequence.SequenceType.DNA);

                        // Reverse primer back to the target context
                        String actualReverse = complement(rev.primer.toUpperCase());
                        Sequence revSeq = new Sequence(rev.id, rev.name, actualReverse,
                            Sequence.SequenceType.DNA);

                        bestPair = new PrimerPair(fwdSeq, revSeq, fwd.quality, rev.quality, productLength);
                    }
                }
            }
        }

        if (bestPair == null) {
            throw new IllegalArgumentException("No compatible primer pair found");
        }

        return bestPair;
    }

    /** Default parameters: 18-30bp, Tm 50-70°C, GC 40-60% */
    public static PrimerPair designDefaultPrimerPair(Sequence target) {
        return designPrimerPair(target, 18, 30, 50.0, 70.0, 0.4, 0.6);
    }

    /** Compute melting temperature (Tm) using the Wallace rule */
    public static double calculateTm(String primer, TmMethod method) {
        String seq = primer.toUpperCase();
        int len = seq.length();

        return switch (method) {
            case WALLACE -> {
                // Wallace rule for short primers (<50bp): Tm = 2(A+T) + 4(G+C)
                long gcCount = seq.chars().filter(c -> c == 'G' || c == 'C').count();
                long atCount = len - gcCount;
                yield 2.0 * atCount + 4.0 * gcCount;
            }
            case NEARLY_MMCHELL -> {
                // Approximation formula: Tm = 81.5 + 0.41*(%GC) - 675/len
                double gcPercent = (double) seq.chars().filter(c -> c == 'G' || c == 'C').count() / len;
                yield 81.5 + 0.41 * (gcPercent * 100) - 675.0 / len;
            }
            case SALT_CORRECTED -> {
                // Salt-corrected formula (more precise)
                double saltConc = 50.0; // mM Na+
                long gcCount = seq.chars().filter(c -> c == 'G' || c == 'C').count();
                double gcPercent = (double) gcCount / len;

                if (len <= 18) {
                    yield 2.0 * (len - (int) gcCount) + 4.0 * gcCount;
                } else {
                    yield 100.5 + 41.0 * gcPercent
                        + (41.0 * ((gcCount - 8.0) / len))
                        - (500.0 / len)
                        + 16.6 * Math.log10(saltConc / 1000.0);
                }
            }
        };
    }

    public enum TmMethod { WALLACE, NEARLY_MMCHELL, SALT_CORRECTED }

    /** Compute GC content of a sequence */
    public static double gcContent(String sequence) {
        if (sequence == null || sequence.isEmpty()) return 0.0;

        long gc = sequence.chars().filter(c -> c == 'G' || c == 'C').count();
        return (double) gc / sequence.length();
    }

    /** Comprehensive primer quality assessment */
    public static PrimerQuality evaluatePrimer(String primer) {
        String seq = primer.toUpperCase();
        int len = seq.length();

        // Compute Tm (Wallace rule)
        double tm = calculateTm(seq, TmMethod.WALLACE);

        // GC content
        double gc = gcContent(seq);

        // Check hairpin formation (self-complementary)
        double hairpinTm = calculateHairpinTm(seq);

        // Self-dimer (with itself)
        double selfDimerTm = calculateSelfDimerScore(seq, seq);

        // 3' end stability (last 5bp)
        int lastBases = Math.min(5, len);
        String threePrime = seq.substring(len - lastBases);
        double threePrimeStability = calculateTm(threePrime, TmMethod.WALLACE);

        return new PrimerQuality(tm, gc, hairpinTm, selfDimerTm, threePrimeStability, len);
    }

    /** Compute hairpin Tm (self-complementary folds) */
    public static double calculateHairpinTm(String sequence) {
        String seq = sequence.toUpperCase();
        int len = seq.length();

        // Check for self-complementary regions (last 10bp of the 3' end)
        int hairpinRegion = Math.min(len, 10);

        for (int i = 0; i < len - hairpinRegion; i++) {
            // Check 4-8bp complementarity at the 3' end
            for (int hlen = 4; hlen <= Math.min(8, len - i); hlen++) {
                String region = seq.substring(i, i + hlen);
                String complement3end = seq.substring(Math.max(len - hlen, len - 5), len);

                // Check whether the region is complementary to the 3' end
                String compRegion = complement(region);
                if (compRegion.equals(complement3end)) {
                    // Hairpin found - compute Tm
                    return calculateTm(region, TmMethod.WALLACE);
                }
            }
        }

        // No significant hairpin found
        return 0.0;
    }

    /** Compute self-dimer score (primer + primer interaction) */
    public static double calculateSelfDimerScore(String primer1, String primer2) {
        String seq1 = primer1.toUpperCase();
        String seq2 = primer2.toUpperCase();

        // Check 3' ends for complementarity (critical for primer dimers)
        int checkLength = Math.min(5, Math.min(seq1.length(), seq2.length()));

        String fwd3end = seq1.substring(Math.max(0, seq1.length() - checkLength));
        String rev3end = seq2.substring(Math.max(0, seq2.length() - checkLength));

        String compFwd3end = complement(fwd3end);
        if (compFwd3end.equals(rev3end)) {
            // 3' ends are complementary → primer dimer likely
            return calculateTm(fwd3end, TmMethod.WALLACE);
        }

        // Check the entire sequence for dimer formation (short regions)
        for (int i = 0; i < seq1.length() - 3; i++) {
            for (int j = 0; j < seq2.length() - 3; j++) {
                for (int dlen = 4; dlen <= Math.min(8, Math.min(seq1.length() - i, seq2.length() - j)); dlen++) {
                    String sub1 = seq1.substring(i, i + dlen);
                    String sub2 = seq2.substring(j, j + dlen);

                    if (complement(sub1).equals(sub2)) {
                        return calculateTm(sub1, TmMethod.WALLACE);
                    }
                }
            }
        }

        return 0.0; // no significant dimer formation
    }

    /** Compute complementary sequence */
    private static String complement(String sequence) {
        StringBuilder sb = new StringBuilder(sequence.length());

        for (char c : sequence.toCharArray()) {
            switch (c) {
                case 'A': sb.append('T'); break;
                case 'T': sb.append('A'); break;
                case 'G': sb.append('C'); break;
                case 'C': sb.append('G'); break;
                default: sb.append(c);  // N, R, Y, etc. → unchanged
            }
        }

        return sb.toString();
    }

    /** Helper class for primer candidates */
    private static class PrimerCandidate {
        String primer;
        int position;
        PrimerQuality quality;
        String id;
        String name;

        PrimerCandidate(String primer, int position, PrimerQuality quality, String id) {
            this.primer = primer;
            this.position = position;
            this.quality = quality;
            this.id = id;
            this.name = "Primer@" + position;
        }
    }

    // ======== MAIN FUNCTIONS FOR UI ==========

    /** Generate primer recommendations for a given target sequence */
    public static List<String> getPrimerRecommendations(Sequence target) {
        List<String> recommendations = new ArrayList<>();

        try {
            PrimerPair pair = designDefaultPrimerPair(target);

            recommendations.add(pair.generateProtocol());
            recommendations.add("Recommended PCR conditions:");
            recommendations.add(String.format("  Start Tm: %.1f°C (average forward/reverse)",
                (pair.getForwardQuality().getTm() + pair.getReverseQuality().getTm()) / 2));
            recommendations.add(String.format("  Annealing temperature: %.1f°C (Tm - 5°C)",
                pair.getForwardQuality().getTm() - 5));
            recommendations.add(String.format("  Extension: %d bp at 72°C", pair.getExpectedProductLength()));

        } catch (IllegalArgumentException e) {
            recommendations.add("Error: " + e.getMessage());
            recommendations.add("Tip: enlarge the target region or adjust the primer parameters.");
        }

        return recommendations;
    }
}
