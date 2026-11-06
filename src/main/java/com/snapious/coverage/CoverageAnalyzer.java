package com.snapious.coverage;

import com.snapious.quality.QualityScoreConverter;
import java.util.*;
import java.util.stream.*;

/**
 * Coverage Analyzer für NGS (Next-Generation Sequencing) Daten.
 * Berechnet Depth/Coverage über genomische Regionen, generiert Coverage-Histogramme,
 * erkennt Gap-Regionen und exportiert als BED-Datei.
 */
public class CoverageAnalyzer {

    // ======== READ DATENSTRUKTUR ==========

    /** Repräsentiert einen sequenzierten Read mit Position und Quality */
    public static class Read {
        private String name;          // Read-ID (z.B. "read_001")
        private String bases;         // Sequenz-Basen (A, T, G, C)
        private int position;         // Start-Position auf Referenz (0-basiert)
        private String qualityString;  // Quality-Zeichenkette (für FASTQ)
        private QualityScoreConverter.EncodingType encoding;  // Encoding-Typ

        public Read(String name, String bases, int position) {
            this(name, bases, position, null, QualityScoreConverter.EncodingType.SANGER);
        }

        public Read(String name, String bases, int position,
                   String qualityString, QualityScoreConverter.EncodingType encoding) {
            this.name = name;
            this.bases = bases.toUpperCase();
            this.position = position;
            this.qualityString = qualityString;
            this.encoding = encoding != null ? encoding : QualityScoreConverter.EncodingType.SANGER;
        }

        public String getName() { return name; }
        public String getBases() { return bases; }
        public int getPosition() { return position; }

        /** Read-Länge */
        public int length() { return bases.length(); }

        /** End-Position auf Referenz (exklusiv) */
        public int endPosition() { return position + bases.length(); }

        /** Prüfe ob Read gültige Quality hat (Q≥20) */
        public boolean hasValidQuality() {
            if (qualityString == null || qualityString.isEmpty()) return true;  // Keine Quality → akzeptieren

            List<Integer> phredScores = QualityScoreConverter.toPhredScores(qualityString, encoding);
            return phredScores.stream().allMatch(s -> s >= 20);  // Mindestens Q20
        }

        /** Durchschnittliche Quality */
        public double avgQuality() {
            if (qualityString == null || qualityString.isEmpty()) return 30;  // Fallback

            List<Integer> phredScores = QualityScoreConverter.toPhredScores(qualityString, encoding);
            return phredScores.stream().mapToInt(Integer::intValue).average().orElse(30);
        }

        @Override
        public String toString() { return name; }
    }

    // ======== REFERENZ-SEKTION ==========

    /** Repräsentiert eine Referenz-Sequenz oder Region */
    public static class ReferenceRegion {
        private String id;           // Chromosom/Scaffold-ID (z.B. "chr1")
        private int length;          // Gesamtlänge der Region

        public ReferenceRegion(String id, int length) {
            this.id = id;
            this.length = length;
        }

        public String getId() { return id; }
        public int getLength() { return length; }

        /** Generiere Region-String (alle Ns) */
        public String generateNSequence() { return "N".repeat(Math.min(length, 1000)); }
    }

    // ======== COVERAGE DATEN ==========

    /** Repräsentiert die Coverage an einer bestimmten Position */
    public static class CoveragePoint {
        private int position;       // 0-basierte Position
        private int depth;          // Depth an dieser Position

        public CoveragePoint(int position, int depth) {
            this.position = position;
            this.depth = depth;
        }

        public int getPosition() { return position; }
        public int getDepth() { return depth; }

        /** Ist diese Position über einem Threshold? */
        public boolean aboveThreshold(int threshold) { return depth >= threshold; }

        @Override
        public String toString() { return String.format("Pos %d: Depth %d", position, depth); }
    }

    /** Repräsentiert eine Coverage-Region (zusammenhängende Depth) */
    public static class CoverageRegion {
        private String seqId;       // Chromosom-ID
        private int start;          // Start-Position (0-basiert)
        private int end;            // End-Position (0-basiert, exklusiv)
        private double meanDepth;   // Durchschnittliche Depth
        private double percentCovered;  // Prozentsatz abdeckend (ab Threshold)

        public CoverageRegion(String seqId, int start, int end,
                             double meanDepth, double percentCovered) {
            this.seqId = seqId;
            this.start = start;
            this.end = end;
            this.meanDepth = meanDepth;
            this.percentCovered = percentCovered;
        }

        public String getSeqId() { return seqId; }
        public int getStart() { return start; }
        public int getEnd() { return end; }

        /** Region-Länge */
        public int length() { return end - start; }

        public double getMeanDepth() { return meanDepth; }
        public double getPercentCovered() { return percentCovered; }

        /** Format als BED-Datei-Zeile */
        public String toBED() {
            int score = (int) Math.min(1000, Math.round(meanDepth));  // BED-Score 0-1000
            return String.format(Locale.ROOT, "%s\t%d\t%d\tCoverage_%.1fx\t%d\t.",
                seqId, start, end, meanDepth, score);
        }

        /** Format als Text-Report */
        public String format() {
            return String.format("%s:%d-%d (Depth %.1fx, %.0f%% abgedeckt)",
                seqId, start, end, meanDepth, percentCovered);
        }

        @Override
        public String toString() { return format(); }
    }

    /** Repräsentiert eine Gap-Region (keine Coverage) */
    public static class GapRegion {
        private String seqId;       // Chromosom-ID
        private int start;          // Start-Position (0-basiert)
        private int end;            // End-Position (exklusiv)

        public GapRegion(String seqId, int start, int end) {
            this.seqId = seqId;
            this.start = start;
            this.end = end;
        }

        public String getSeqId() { return seqId; }
        public int getStart() { return start; }
        public int getEnd() { return end; }

        /** Gap-Länge */
        public int length() { return end - start; }

        @Override
        public String toString() {
            return String.format("Gap %s:%d-%d (%d bp)", seqId, start, end, length());
        }
    }

    // ======== COVERAGE ANALYSE ERGEBNIS ==========

    /** Haupt-Ergebnis der Coverage-Analyse */
    public static class CoverageResult {
        private String seqId;           // Chromosom/Referenz-ID
        private int regionLength;       // Gesamtlänge der Region
        private List<CoveragePoint> coveragePoints;  // Depth pro Position
        private Map<Integer, Integer> depthHistogram;  // Coverage-Histogramm (depth → Anzahl Positionen)
        private List<CoverageRegion> coveredRegions;  // Abgedeckte Regionen (ab Threshold)
        private List<GapRegion> gapRegions;  // Gap-Regionen (keine Abdeckung)
        private double percentCovered;  // Gesamte Prozentsatz abgedeckt
        private double meanDepth;       // Durchschnittliche Depth über gesamte Region
        private int medianDepth;        // Median-Depth

        public CoverageResult(String seqId, int regionLength,
                             List<CoveragePoint> coveragePoints,
                             Map<Integer, Integer> depthHistogram,
                             List<CoverageRegion> coveredRegions,
                             List<GapRegion> gapRegions,
                             double percentCovered, double meanDepth, int medianDepth) {
            this.seqId = seqId;
            this.regionLength = regionLength;
            this.coveragePoints = coveragePoints;
            this.depthHistogram = depthHistogram;
            this.coveredRegions = coveredRegions;
            this.gapRegions = gapRegions;
            this.percentCovered = percentCovered;
            this.meanDepth = meanDepth;
            this.medianDepth = medianDepth;
        }

        public String getSeqId() { return seqId; }
        public int getRegionLength() { return regionLength; }
        public List<CoveragePoint> getCoveragePoints() { return coveragePoints; }
        public Map<Integer, Integer> getDepthHistogram() { return depthHistogram; }
        public List<CoverageRegion> getCoveredRegions() { return coveredRegions; }
        public List<GapRegion> getGapRegions() { return gapRegions; }
        public double getPercentCovered() { return percentCovered; }
        public double getMeanDepth() { return meanDepth; }
        public int getMedianDepth() { return medianDepth; }

        /** Coverage-Bewertung */
        public String coverageLabel() {
            if (meanDepth >= 30) return "Hoch Coverage ✓✓";
            if (meanDepth >= 10) return "Mittelwertige Coverage ✓";
            if (meanDepth >= 5) return "Niedrige Coverage";
            return "Unzureichende Coverage ✗";
        }

        /** Generiere vollständigen Coverage-Bericht */
        public String generateReport() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("=== Coverage-Analyse: %s ===\n\n", seqId));
            sb.append(String.format("Referenz-Länge: %d bp\n", regionLength));
            sb.append(String.format("Durchschnittliche Depth: %.1fx\n", (double) meanDepth));
            sb.append(String.format("Median-Depth: %dx\n", medianDepth));
            sb.append(String.format("Coverage-Bewertung: %s\n", coverageLabel()));
            sb.append(String.format("Abgedeckte Region: %.0f%%\n", percentCovered));

            // Coverage-Histogramm
            sb.append("\nCoverage-Histogramm:\n");

            depthHistogram.entrySet().stream()
                .sorted(Map.Entry.comparingByKey())
                .forEach(entry -> {
                    // Visualisiere als Balkendiagramm (max 50 Zeichen Breite)
                    int barWidth = Math.min(50, entry.getValue() / Math.max(1, regionLength / 20));

                    sb.append(String.format("  %3dx: ", entry.getKey()));

                    for (int i = 0; i < barWidth; i++) {
                        sb.append('█');
                    }

                    sb.append(String.format(" (%d bp, %.0f%%)\n",
                        entry.getValue(), (double) entry.getValue() / regionLength * 100));
                });

            // Gap-Regionen
            if (!gapRegions.isEmpty()) {
                sb.append(String.format("\nGap-Regionen (0x Coverage): %d\n", gapRegions.size()));

                for (GapRegion gap : gapRegions.subList(0, Math.min(10, gapRegions.size()))) {
                    sb.append(String.format("  %s\n", gap));
                }

                if (gapRegions.size() > 10) {
                    sb.append(String.format("  ... und %d weitere Gap-Regionen\n", gapRegions.size() - 10));
                }

                long totalGapBp = gapRegions.stream().mapToInt(GapRegion::length).sum();
                sb.append(String.format("\nGesamte Gap-Länge: %d bp (%.0f%%)\n",
                    totalGapBp, (double) totalGapBp / regionLength * 100));
            }

            // Hohe Coverage-Regionen (>30x)
            List<CoverageRegion> highCoverage = coveredRegions.stream()
                .filter(r -> r.getMeanDepth() >= 30)
                .sorted((a, b) -> Double.compare(b.getMeanDepth(), a.getMeanDepth()))
                .toList();

            if (!highCoverage.isEmpty()) {
                sb.append(String.format("\nHohe Coverage-Regionen (≥30x): %d\n", highCoverage.size()));

                for (int i = 0; i < Math.min(5, highCoverage.size()); i++) {
                    sb.append(String.format("  %s\n", highCoverage.get(i)));
                }
            }

            return sb.toString();
        }

        @Override
        public String toString() { return generateReport(); }
    }

    // ======== HAUPTANALYSE ==========

    /**
     * Analysiere Coverage/Depth einer Referenzregion über Reads.
     * @param reference Referenz-Region (Chromosom, Länge)
     * @param reads Sequenzierungs-Reads
     * @return CoverageResult mit Depth, Histogramm, Gap-Erkennung
     */
    public static CoverageResult analyzeCoverage(ReferenceRegion reference, List<Read> reads) {
        return analyzeCoverage(reference, reads, 10);  // Standard-Threshold: 10x
    }

    /**
     * Analysiere Coverage/Depth mit benutzerdefiniertem Threshold.
     */
    public static CoverageResult analyzeCoverage(ReferenceRegion reference,
                                                List<Read> reads, int coverageThreshold) {
        String seqId = reference.getId();
        int regionLength = reference.getLength();

        // Erstelle Depth-Array (0-basiert)
        int[] depthArray = new int[regionLength];

        // Zähle Coverage über alle Reads
        for (Read read : reads) {
            if (read.getPosition() >= regionLength) continue;  // Read außerhalb der Region

            int readEnd = Math.min(read.endPosition(), regionLength);

            // Erhöhe Depth für alle Positionen des Reads
            for (int pos = read.getPosition(); pos < readEnd; pos++) {
                depthArray[pos]++;
            }
        }

        // Generiere CoveragePoints
        List<CoveragePoint> coveragePoints = new ArrayList<>(regionLength);

        for (int i = 0; i < regionLength; i++) {
            coveragePoints.add(new CoveragePoint(i, depthArray[i]));
        }

        // Berechne Depth-Histogramm (depth → Anzahl Positionen)
        Map<Integer, Integer> depthHistogram = new TreeMap<>();

        for (int depth : depthArray) {
            depthHistogram.merge(depth, 1, Integer::sum);
        }

        // Berechne Statistiken (Mean, Median)
        int[] sortedDepths = depthArray.clone();
        Arrays.sort(sortedDepths);

        double meanDepth = Arrays.stream(depthArray).average().orElse(0);
        int medianDepth = sortedDepths.length == 0 ? 0 : sortedDepths[sortedDepths.length / 2];

        // Finde abgedeckte Regionen (ab Threshold)
        List<CoverageRegion> coveredRegions = findCoveredRegions(seqId, depthArray, regionLength, coverageThreshold);

        // Finde Gap-Regionen (0x Coverage)
        List<GapRegion> gapRegions = findGaps(seqId, depthArray, regionLength);

        // Berechne prozentuale Abdeckung (Positionen ≥ Threshold)
        long coveredPositions = Arrays.stream(depthArray).filter(d -> d >= coverageThreshold).count();
        double percentCovered = (double) coveredPositions / regionLength * 100;

        return new CoverageResult(
            seqId, regionLength, coveragePoints, depthHistogram,
            coveredRegions, gapRegions, percentCovered, meanDepth, medianDepth);
    }

    /** Finde abgedeckte Regionen (zusammenhängende Bereiche ≥ Threshold) */
    private static List<CoverageRegion> findCoveredRegions(String seqId, int[] depthArray,
                                                          int regionLength, int threshold) {
        List<CoverageRegion> regions = new ArrayList<>();

        int regionStart = -1;
        List<Integer> depthsInRegion = new ArrayList<>();

        for (int i = 0; i < regionLength; i++) {
            if (depthArray[i] >= threshold) {
                // Teil einer abgedeckten Region
                if (regionStart == -1) {
                    regionStart = i;  // Region-Start
                    depthsInRegion.clear();
                }

                depthsInRegion.add(depthArray[i]);

            } else {
                // Region enden (wenn aktiv)
                if (regionStart != -1) {
                    int regionEnd = i;  // Exklusiv
                    double meanDepth = depthsInRegion.stream().mapToInt(Integer::intValue).average().orElse(0);

                    // Prozentuale Abdeckung innerhalb dieser Region
                    long coveredInRegion = depthsInRegion.stream().filter(d -> d >= threshold).count();
                    double percentCovered = (double) coveredInRegion / depthsInRegion.size() * 100;

                    regions.add(new CoverageRegion(seqId, regionStart, regionEnd,
                        meanDepth, percentCovered));

                    regionStart = -1;  // Reset
                }
            }
        }

        // Schließe offene Region am Ende ab
        if (regionStart != -1) {
            int regionEnd = regionLength;
            double meanDepth = depthsInRegion.stream().mapToInt(Integer::intValue).average().orElse(0);

            long coveredInRegion = depthsInRegion.stream().filter(d -> d >= threshold).count();
            double percentCovered = (double) coveredInRegion / depthsInRegion.size() * 100;

            regions.add(new CoverageRegion(seqId, regionStart, regionEnd,
                meanDepth, percentCovered));
        }

        return regions;
    }

    /** Finde Gap-Regionen (0x Coverage) */
    private static List<GapRegion> findGaps(String seqId, int[] depthArray, int regionLength) {
        List<GapRegion> gaps = new ArrayList<>();

        int gapStart = -1;

        for (int i = 0; i < regionLength; i++) {
            if (depthArray[i] == 0) {
                // Teil eines Gaps
                if (gapStart == -1) {
                    gapStart = i;  // Gap-Start

                }

            } else {
                // Gap enden (wenn aktiv)
                if (gapStart != -1) {
                    gaps.add(new GapRegion(seqId, gapStart, i));  // Exklusiv
                    gapStart = -1;  // Reset
                }
            }
        }

        // Schließe offenes Gap am Ende ab
        if (gapStart != -1) {
            gaps.add(new GapRegion(seqId, gapStart, regionLength));
        }

        return gaps;
    }

    // ======== QUALITY-GEFILTERTE ANALYSE ==========

    /**
     * Analysiere Coverage mit Quality-Filterung (nur Reads ≥ Q20 zählen).
     */
    public static CoverageResult analyzeCoverageWithQualityFilter(ReferenceRegion reference,
                                                                   List<Read> reads) {
        // Filtere Reads nach Quality
        List<Read> validReads = reads.stream()
            .filter(Read::hasValidQuality)
            .collect(Collectors.toList());

        return analyzeCoverage(reference, validReads);
    }

    /** Zähle Reads über einem Quality-Threshold */
    public static long countReadsAboveQuality(List<Read> reads, int minQ) {
        return reads.stream()
            .filter(read -> read.qualityString != null && !read.qualityString.isEmpty())
            .filter(read -> {
                List<Integer> phredScores = QualityScoreConverter.toPhredScores(read.qualityString, read.encoding);
                return phredScores.stream().allMatch(s -> s >= minQ);
            })
            .count();
    }

    // ======== EXPORT FUNKTIONEN ==========

    /** Exportiere Coverage als BED-Datei */
    public static String exportToBED(CoverageResult result) {
        StringBuilder sb = new StringBuilder();

        // BED-Header (optional, aber nützlich)
        sb.append(String.format("track type=coverage name=\"%s\" visibility=all\n", result.getSeqId()));

        for (CoverageRegion region : result.getCoveredRegions()) {
            sb.append(region.toBED()).append("\n");
        }

        return sb.toString();
    }

    /** Exportiere Coverage als BigWind-ähnliches Format (bin 10bp-Bins) */
    public static String exportToBinnedCoverage(CoverageResult result, int binSize) {
        StringBuilder sb = new StringBuilder();

        int regionLength = result.getRegionLength();

        for (int i = 0; i < regionLength; i += binSize) {
            int binEnd = Math.min(i + binSize, regionLength);

            // Berechne Durchschnittliche Depth über diesen Bin
            final int binStart = i;
            double avgDepth = result.getCoveragePoints().stream()
                .filter(cp -> cp.getPosition() >= binStart && cp.getPosition() < binEnd)
                .mapToInt(CoveragePoint::getDepth)
                .average()
                .orElse(0);

            sb.append(String.format("%s\t%d\t%d\t%.1f\n",
                result.getSeqId(), i, binEnd, avgDepth));
        }

        return sb.toString();
    }

    // ======== UI-FUNKTIONEN ==========

    /** Generiere schnellen Coverage-Überblick */
    public static String quickCoverageOverview(ReferenceRegion reference, List<Read> reads) {
        CoverageResult result = analyzeCoverage(reference, reads);

        StringBuilder sb = new StringBuilder();

        sb.append(String.format("=== Coverage: %s ===\n", result.getSeqId()));
        sb.append(String.format("Reads: %d\n", reads.size()));
        sb.append(String.format("Mean Depth: %.1fx\n", (double) result.getMeanDepth()));
        sb.append(String.format("Coverage: %.0f%%\n", result.getPercentCovered()));
        sb.append(String.format("Gaps: %d\n", result.getGapRegions().size()));

        return sb.toString();
    }

    /** Vergleiche Coverage zweier Proben */
    public static String compareCoverage(ReferenceRegion reference,
                                        List<Read> reads1, List<Read> reads2) {
        CoverageResult result1 = analyzeCoverage(reference, reads1);
        CoverageResult result2 = analyzeCoverage(reference, reads2);

        StringBuilder sb = new StringBuilder();

        sb.append("=== Coverage-Vergleich ===\n\n");

        sb.append(String.format("%s:\n", result1.getSeqId()));
        sb.append(String.format("  Probe 1: Mean %.1fx, Coverage %.0f%%, Gaps %d\n",
            result1.getMeanDepth(), result1.getPercentCovered(), result1.getGapRegions().size()));
        sb.append(String.format("  Probe 2: Mean %.1fx, Coverage %.0f%%, Gaps %d\n",
            result2.getMeanDepth(), result2.getPercentCovered(), result2.getGapRegions().size()));

        double depthDiff = Math.abs(result1.getMeanDepth() - result2.getMeanDepth());
        sb.append(String.format("\nDepth-Differenz: %.1fx\n", depthDiff));

        if (depthDiff < 5) {
            sb.append("Beide Proben haben ähnliche Coverage ✓");
        } else {
            sb.append("Coverage-Unterschied signifikant — Probenvorbereitung prüfen");
        }

        return sb.toString();
    }

    /** Generiere vollständigen Coverage-Bericht für Ausgabe */
    public static String generateFullCoverageReport(ReferenceRegion reference, List<Read> reads) {
        CoverageResult result = analyzeCoverage(reference, reads);

        StringBuilder sb = new StringBuilder();

        // Zusammenfassung
        sb.append(result.generateReport());

        // Export-Optionen
        sb.append("\n=== Export ===\n");
        sb.append("BED-Format:\n");
        sb.append(exportToBED(result));

        return sb.toString();
    }

    // ======== BEISPIEL-READS GENERIEREN (für Tests) ==========

    /** Generiere synthetische Test-Reads für eine Referenz */
    public static List<Read> generateSyntheticReads(ReferenceRegion reference, int avgDepth) {
        List<Read> reads = new ArrayList<>();

        if (reference.getLength() == 0) return reads;

        // Berechne Anzahl der Reads basierend auf gewünschter Depth
        int readLength = 150;  // Standard Illumina Read-Länge
        long totalReads = (long) avgDepth * reference.getLength() / readLength;

        // Deterministische Pseudo-Zufalls-Generierung (für Reproduzierbarkeit)
        Random rand = new Random(42);  // Fester Seed

        for (long i = 0; i < totalReads; i++) {
            String readName = String.format("read_%08d", i);

            // Zufällige Position
            int position;

            if (reference.getLength() <= readLength) {
                position = 0;  // Sehr kurze Referenz

            } else {
                position = rand.nextInt(reference.getLength() - readLength);
            }

            // Synetische Sequenz (zufällige Basen)
            StringBuilder bases = new StringBuilder(readLength);

            String possibleBases = "ATGC";
            for (int j = 0; j < readLength && (position + j) < reference.getLength(); j++) {
                bases.append(possibleBases.charAt(rand.nextInt(4)));
            }

            // Quality-String (mittlere Qualität ~Q30)
            StringBuilder quality = new StringBuilder(bases.length());

            for (int j = 0; j < bases.length(); j++) {
                int phredScore = rand.nextInt(41) + 10;  // Q10-50
                quality.append((char) (phredScore + QualityScoreConverter.EncodingType.SANGER.getOffset()));
            }

            reads.add(new Read(readName, bases.toString(), position, quality.toString(), QualityScoreConverter.EncodingType.SANGER));
        }

        return reads;
    }
}
