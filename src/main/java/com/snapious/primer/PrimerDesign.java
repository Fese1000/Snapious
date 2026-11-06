package com.snapious.primer;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Komplettes Primer-Design-Toolkit für PCR, Sequenzierung und Klonierung.
 * Unterstützt: Tm-Berechnung (Wallace, GC-Method), Self-Dimer Prüfung,
 * Hairpin-Bildung, 3'-Ende Stabilität, Forward/Reverse Primer-Paar Generierung.
 */
public class PrimerDesign {

    // ======== PRIMER-QUALITÄTSWERTE ==========

    public static class PrimerQuality {
        private double tm;              // Schmelztemperatur (°C)
        private double gcContent;       // GC-Gehalt (0.0 - 1.0)
        private double hairpinTm;       // Hairpin-Stabilität (°C)
        private double selfDimerTm;     // Self-Dimer-Stabilität (°C)
        private double threePrimeStability;  // 3'-Ende Stabilität (last 5bp)
        private int length;             // Primer-Länge

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

        /** Gesamtbewertung (0-100) */
        public int qualityScore() {
            int score = 0;

            // Tm-Bereich (50-70°C ideal): 30 Punkte
            if (tm >= 50 && tm <= 70) score += 30;
            else if (tm >= 45 && tm <= 75) score += 15;

            // GC-Gehalt (40-60% ideal): 25 Punkte
            if (gcContent >= 0.4 && gcContent <= 0.6) score += 25;
            else if (gcContent >= 0.3 && gcContent <= 0.7) score += 15;

            // Hairpin-Tm < 45°C: 20 Punkte
            if (hairpinTm < 45) score += 20;
            else if (hairpinTm < 50) score += 10;

            // Self-Dimer-Tm < 45°C: 15 Punkte
            if (selfDimerTm < 45) score += 15;
            else if (selfDimerTm < 50) score += 7;

            // 3'-Ende Stabilität: 10 Punkte
            if (threePrimeStability < 50) score += 10;

            return Math.min(100, score);
        }

        @Override
        public String toString() {
            return String.format("PrimerQuality[Tm=%.1f°C, GC=%.0f%%, Length=%d, Score=%d/100]",
                tm, gcContent * 100, length, qualityScore());
        }
    }

    /** Repräsentiert ein Forward/Reverse Primer-Paar */
    public static class PrimerPair {
        private Sequence forward;
        private Sequence reverse;
        private PrimerQuality forwardQuality;
        private PrimerQuality reverseQuality;
        private int expectedProductLength;  // Erwartete Produktlänge

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

        /** Erwartete PCR-Produktlänge */
        public int getExpectedProductLength() { return expectedProductLength; }

        /** Tm-Differenz zwischen Primer-Paar */
        public double getTmDifference() {
            return Math.abs(forwardQuality.getTm() - reverseQuality.getTm());
        }

        /** Gesamtbewertung des Primer-Paars */
        public int getPairQualityScore() {
            return (forwardQuality.qualityScore() + reverseQuality.qualityScore()) / 2;
        }

        /** Prüfe ob Primer-Paar kompatibel ist */
        public boolean arePrimersCompatible() {
            // Tm-Differenz < 5°C: OK
            if (getTmDifference() > 5.0) return false;

            // Self-Dimer zwischen Forward und Reverse < 45°C
            if (calculateSelfDimerScore(forward.getBases(), reverse.getBases()) > 45)
                return false;

            // Keine komplementären 3'-Enden (>4bp)
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

        /** Generiere ausführliches Protokoll */
        public String generateProtocol() {
            StringBuilder sb = new StringBuilder();

            sb.append("=== Primer-Paar-Design ===\n\n");
            sb.append(String.format("Forward Primer: %s\n", forward.getName()));
            sb.append(String.format("  Sequence: %s\n", forward.getBases()));
            sb.append(forwardQuality.toString()).append("\n\n");

            sb.append(String.format("Reverse Primer: %s\n", reverse.getName()));
            sb.append(String.format("  Sequence: %s\n", reverse.getBases()));
            sb.append(reverseQuality.toString()).append("\n\n");

            sb.append(String.format("Erwartetes Produkt: %d bp\n", expectedProductLength));
            sb.append(String.format("Tm-Differenz: %.1f°C\n", getTmDifference()));
            sb.append(String.format("Kompatibel: %s\n", arePrimersCompatible() ? "JA ✓" : "NEIN ✗"));
            sb.append(String.format("Gesamtqualität: %d/100\n", getPairQualityScore()));

            return sb.toString();
        }
    }

    // ======== HAUPTFUNKTIONEN ==========

    /**
     * Designe Forward und Reverse Primer für gegebene Target-Sequenz.
     * Sucht im Target nach optimalen Primer-Bindungsstellen.
     */
    public static PrimerPair designPrimerPair(Sequence target, int minLength, int maxLength,
                                               double minTm, double maxTm, double gcMin, double gcMax) {
        String sequence = target.getBases().toUpperCase();
        int seqLen = sequence.length();

        // Reverse-Komplement für Backward-Primer
        Sequence revComp = target.reverseComplement();
        String rcSeq = revComp.getBases().toUpperCase();

        List<PrimerCandidate> forwardCandidates = new ArrayList<>();
        List<PrimerCandidate> reverseCandidates = new ArrayList<>();

        // Durchsuche Target für Forward-Primer
        for (int i = 0; i <= seqLen - minLength; i++) {
            for (int len = minLength; len <= Math.min(maxLength, seqLen - i); len++) {
                String primer = sequence.substring(i, i + len);

                // Prüfe GC-Gehalt
                double gc = gcContent(primer);
                if (gc < gcMin || gc > gcMax) continue;

                // Prüfe Tm
                double tm = calculateTm(primer, TmMethod.WALLACE);
                if (tm < minTm || tm > maxTm) continue;

                // Bewerte Primer-Qualität
                PrimerQuality quality = evaluatePrimer(primer);

                // Speichere wenn gute Qualität
                if (quality.qualityScore() >= 60) {
                    forwardCandidates.add(new PrimerCandidate(
                        primer, i, quality, target.getName() + "_F" + i));
                }
            }
        }

        // Durchsuche Reverse-Komplement für Backward-Primer
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

        // Finde bestes Primer-Paar
        if (forwardCandidates.isEmpty() || reverseCandidates.isEmpty()) {
            throw new IllegalArgumentException(
                "Keine kompatiblen Primer gefunden. Prüfe Parameter (Länge: " + minLength + "-" +
                maxLength + ", Tm: " + minTm + "-" + maxTm + "°C)");
        }

        // Sortiere nach Qualität und finde bestes Paar mit minimaler Tm-Differenz
        forwardCandidates.sort((a, b) -> Integer.compare(b.quality.qualityScore(), a.quality.qualityScore()));
        reverseCandidates.sort((a, b) -> Integer.compare(b.quality.qualityScore(), a.quality.qualityScore()));

        PrimerPair bestPair = null;
        double minTmDiff = Double.MAX_VALUE;

        for (PrimerCandidate fwd : forwardCandidates.subList(0, Math.min(10, forwardCandidates.size()))) {
            for (PrimerCandidate rev : reverseCandidates.subList(0, Math.min(10, reverseCandidates.size()))) {
                double tmDiff = Math.abs(fwd.quality.getTm() - rev.quality.getTm());

                if (tmDiff < minTmDiff) {
                    // Berechne erwartete Produktlänge
                    int productLength = (seqLen - rev.position - rev.primer.length()) - fwd.position;

                    if (productLength > 0 && productLength < 5000) {
                        minTmDiff = tmDiff;

                        Sequence fwdSeq = new Sequence(fwd.id, fwd.name, fwd.primer.toUpperCase(),
                            Sequence.SequenceType.DNA);

                        // Reverse-Primer zurück zum Target-Kontext
                        String actualReverse = complement(rev.primer.toUpperCase());
                        Sequence revSeq = new Sequence(rev.id, rev.name, actualReverse,
                            Sequence.SequenceType.DNA);

                        bestPair = new PrimerPair(fwdSeq, revSeq, fwd.quality, rev.quality, productLength);
                    }
                }
            }
        }

        if (bestPair == null) {
            throw new IllegalArgumentException("Kein kompatibles Primer-Paar gefunden");
        }

        return bestPair;
    }

    /** Default-Parameter: 18-30bp, Tm 50-70°C, GC 40-60% */
    public static PrimerPair designDefaultPrimerPair(Sequence target) {
        return designPrimerPair(target, 18, 30, 50.0, 70.0, 0.4, 0.6);
    }

    /** Berechne Schmelztemperatur (Tm) mit Wallace-Regel */
    public static double calculateTm(String primer, TmMethod method) {
        String seq = primer.toUpperCase();
        int len = seq.length();

        return switch (method) {
            case WALLACE -> {
                // Wallace-Regel für kurze Primer (<50bp): Tm = 2(A+T) + 4(G+C)
                long gcCount = seq.chars().filter(c -> c == 'G' || c == 'C').count();
                long atCount = len - gcCount;
                yield 2.0 * atCount + 4.0 * gcCount;
            }
            case NEARLY_MMCHELL -> {
                // Näherungsformel: Tm = 81.5 + 0.41*(%GC) - 675/len
                double gcPercent = (double) seq.chars().filter(c -> c == 'G' || c == 'C').count() / len;
                yield 81.5 + 0.41 * (gcPercent * 100) - 675.0 / len;
            }
            case SALT_CORRECTED -> {
                // Salzkorrigierte Formel (präziser)
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

    /** Berechne GC-Gehalt einer Sequenz */
    public static double gcContent(String sequence) {
        if (sequence == null || sequence.isEmpty()) return 0.0;

        long gc = sequence.chars().filter(c -> c == 'G' || c == 'C').count();
        return (double) gc / sequence.length();
    }

    /** Bewerte Primer-Qualität umfassend */
    public static PrimerQuality evaluatePrimer(String primer) {
        String seq = primer.toUpperCase();
        int len = seq.length();

        // Tm berechnen (Wallace-Regel)
        double tm = calculateTm(seq, TmMethod.WALLACE);

        // GC-Gehalt
        double gc = gcContent(seq);

        // Hairpin-Bildung prüfen (selbst-komplementär)
        double hairpinTm = calculateHairpinTm(seq);

        // Self-Dimer (mit sich selbst)
        double selfDimerTm = calculateSelfDimerScore(seq, seq);

        // 3'-Ende Stabilität (letzte 5bp)
        int lastBases = Math.min(5, len);
        String threePrime = seq.substring(len - lastBases);
        double threePrimeStability = calculateTm(threePrime, TmMethod.WALLACE);

        return new PrimerQuality(tm, gc, hairpinTm, selfDimerTm, threePrimeStability, len);
    }

    /** Berechne Hairpin-Tm (selbst-komplementäre Faltungen) */
    public static double calculateHairpinTm(String sequence) {
        String seq = sequence.toUpperCase();
        int len = seq.length();

        // Prüfe auf selbst-komplementäre Regionen (letzte 10bp des 3'-Endes)
        int hairpinRegion = Math.min(len, 10);

        for (int i = 0; i < len - hairpinRegion; i++) {
            // Prüfe 4-8bp Komplementarität am 3'-Ende
            for (int hlen = 4; hlen <= Math.min(8, len - i); hlen++) {
                String region = seq.substring(i, i + hlen);
                String complement3end = seq.substring(Math.max(len - hlen, len - 5), len);

                // Prüfe ob Region komplementär zu 3'-Ende
                String compRegion = complement(region);
                if (compRegion.equals(complement3end)) {
                    // Hairpin gefunden - berechne Tm
                    return calculateTm(region, TmMethod.WALLACE);
                }
            }
        }

        // Kein signifikanter Hairpin gefunden
        return 0.0;
    }

    /** Berechne Self-Dimer Score (Primer + Primer Interaktion) */
    public static double calculateSelfDimerScore(String primer1, String primer2) {
        String seq1 = primer1.toUpperCase();
        String seq2 = primer2.toUpperCase();

        // Prüfe 3'-Enden auf Komplementarität (kritisch für Primer-Dimer)
        int checkLength = Math.min(5, Math.min(seq1.length(), seq2.length()));

        String fwd3end = seq1.substring(Math.max(0, seq1.length() - checkLength));
        String rev3end = seq2.substring(Math.max(0, seq2.length() - checkLength));

        String compFwd3end = complement(fwd3end);
        if (compFwd3end.equals(rev3end)) {
            // 3'-Enden sind komplementär → Primer-Dimer wahrscheinlich
            return calculateTm(fwd3end, TmMethod.WALLACE);
        }

        // Prüfe gesamte Sequenz auf Dimer-Bildung (kurze Regionen)
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

        return 0.0; // Keine signifikante Dimer-Bildung
    }

    /** Komplementäre Sequenz berechnen */
    private static String complement(String sequence) {
        StringBuilder sb = new StringBuilder(sequence.length());

        for (char c : sequence.toCharArray()) {
            switch (c) {
                case 'A': sb.append('T'); break;
                case 'T': sb.append('A'); break;
                case 'G': sb.append('C'); break;
                case 'C': sb.append('G'); break;
                default: sb.append(c);  // N, R, Y, etc. → unverändert
            }
        }

        return sb.toString();
    }

    /** Helper-Klasse für Primer-Kandidaten */
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

    // ======== HAUPTFUNKTIONEN FÜR UI ==========

    /** Generiere Primer-Empfehlungen für gegebene Target-Sequenz */
    public static List<String> getPrimerRecommendations(Sequence target) {
        List<String> recommendations = new ArrayList<>();

        try {
            PrimerPair pair = designDefaultPrimerPair(target);

            recommendations.add(pair.generateProtocol());
            recommendations.add("Empfohlene PCR-Bedingungen:");
            recommendations.add(String.format("  Start-Tm: %.1f°C (Durchschnitt Forward/Reverse)",
                (pair.getForwardQuality().getTm() + pair.getReverseQuality().getTm()) / 2));
            recommendations.add(String.format("  Annealing-Temperatur: %.1f°C (Tm - 5°C)",
                pair.getForwardQuality().getTm() - 5));
            recommendations.add(String.format("  Extension: %d bp bei 72°C", pair.getExpectedProductLength()));

        } catch (IllegalArgumentException e) {
            recommendations.add("Fehler: " + e.getMessage());
            recommendations.add("Tipp: Vergrößere den Target-Bereich oder passe Primer-Parameter an.");
        }

        return recommendations;
    }
}
