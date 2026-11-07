package com.snapious.coverage;

import com.snapious.quality.QualityScoreConverter;
import java.util.*;
import java.util.stream.*;

/**
 * Coverage analyzer for NGS (next-generation sequencing) data.
 * Computes depth/coverage over genomic regions, generates coverage histograms,
 * detects gap regions and exports as a BED file.
 */
public class CoverageAnalyzer {

    // ======== READ DATA STRUCTURE ==========

    /** Represents a sequenced read with position and quality */
    public static class Read {
        private String name;          // read ID (e.g. "read_001")
        private String bases;         // sequence bases (A, T, G, C)
        private int position;         // start position on the reference (0-based)
        private String qualityString;  // quality string (for FASTQ)
        private QualityScoreConverter.EncodingType encoding;  // encoding type

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

        /** Read length */
        public int length() { return bases.length(); }

        /** End position on the reference (exclusive) */
        public int endPosition() { return position + bases.length(); }

        /** Check whether the read has valid quality (Q≥20) */
        public boolean hasValidQuality() {
            if (qualityString == null || qualityString.isEmpty()) return true;  // no quality → accept

            List<Integer> phredScores = QualityScoreConverter.toPhredScores(qualityString, encoding);
            return phredScores.stream().allMatch(s -> s >= 20);  // at least Q20
        }

        /** Average quality */
        public double avgQuality() {
            if (qualityString == null || qualityString.isEmpty()) return 30;  // fallback

            List<Integer> phredScores = QualityScoreConverter.toPhredScores(qualityString, encoding);
            return phredScores.stream().mapToInt(Integer::intValue).average().orElse(30);
        }

        @Override
        public String toString() { return name; }
    }

    // ======== REFERENCE SECTION ==========

    /** Represents a reference sequence or region */
    public static class ReferenceRegion {
        private String id;           // chromosome/scaffold ID (e.g. "chr1")
        private int length;          // total length of the region

        public ReferenceRegion(String id, int length) {
            this.id = id;
            this.length = length;
        }

        public String getId() { return id; }
        public int getLength() { return length; }

        /** Generate region string (all Ns) */
        public String generateNSequence() { return "N".repeat(Math.min(length, 1000)); }
    }

    // ======== COVERAGE DATA ==========

    /** Represents the coverage at a given position */
    public static class CoveragePoint {
        private int position;       // 0-based position
        private int depth;          // depth at this position

        public CoveragePoint(int position, int depth) {
            this.position = position;
            this.depth = depth;
        }

        public int getPosition() { return position; }
        public int getDepth() { return depth; }

        /** Is this position above a threshold? */
        public boolean aboveThreshold(int threshold) { return depth >= threshold; }

        @Override
        public String toString() { return String.format("Pos %d: Depth %d", position, depth); }
    }

    /** Represents a coverage region (contiguous depth) */
    public static class CoverageRegion {
        private String seqId;       // chromosome ID
        private int start;          // start position (0-based)
        private int end;            // end position (0-based, exclusive)
        private double meanDepth;   // average depth
        private double percentCovered;  // percentage covered (above threshold)

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

        /** Region length */
        public int length() { return end - start; }

        public double getMeanDepth() { return meanDepth; }
        public double getPercentCovered() { return percentCovered; }

        /** Format as a BED file line */
        public String toBED() {
            int score = (int) Math.min(1000, Math.round(meanDepth));  // BED score 0-1000
            return String.format(Locale.ROOT, "%s\t%d\t%d\tCoverage_%.1fx\t%d\t.",
                seqId, start, end, meanDepth, score);
        }

        /** Format as a text report */
        public String format() {
            return String.format("%s:%d-%d (Depth %.1fx, %.0f%% covered)",
                seqId, start, end, meanDepth, percentCovered);
        }

        @Override
        public String toString() { return format(); }
    }

    /** Represents a gap region (no coverage) */
    public static class GapRegion {
        private String seqId;       // chromosome ID
        private int start;          // start position (0-based)
        private int end;            // end position (exclusive)

        public GapRegion(String seqId, int start, int end) {
            this.seqId = seqId;
            this.start = start;
            this.end = end;
        }

        public String getSeqId() { return seqId; }
        public int getStart() { return start; }
        public int getEnd() { return end; }

        /** Gap length */
        public int length() { return end - start; }

        @Override
        public String toString() {
            return String.format("Gap %s:%d-%d (%d bp)", seqId, start, end, length());
        }
    }

    // ======== COVERAGE ANALYSIS RESULT ==========

    /** Main result of the coverage analysis */
    public static class CoverageResult {
        private String seqId;           // chromosome/reference ID
        private int regionLength;       // total length of the region
        private List<CoveragePoint> coveragePoints;  // depth per position
        private Map<Integer, Integer> depthHistogram;  // coverage histogram (depth → number of positions)
        private List<CoverageRegion> coveredRegions;  // covered regions (above threshold)
        private List<GapRegion> gapRegions;  // gap regions (no coverage)
        private double percentCovered;  // overall percentage covered
        private double meanDepth;       // average depth over the whole region
        private int medianDepth;        // median depth

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

        /** Coverage rating */
        public String coverageLabel() {
            if (meanDepth >= 30) return "High coverage ✓✓";
            if (meanDepth >= 10) return "Medium coverage ✓";
            if (meanDepth >= 5) return "Low coverage";
            return "Insufficient coverage ✗";
        }

        /** Generate the full coverage report */
        public String generateReport() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("=== Coverage Analysis: %s ===\n\n", seqId));
            sb.append(String.format("Reference length: %d bp\n", regionLength));
            sb.append(String.format("Average depth: %.1fx\n", (double) meanDepth));
            sb.append(String.format("Median depth: %dx\n", medianDepth));
            sb.append(String.format("Coverage rating: %s\n", coverageLabel()));
            sb.append(String.format("Covered region: %.0f%%\n", percentCovered));

            // Coverage histogram
            sb.append("\nCoverage histogram:\n");

            depthHistogram.entrySet().stream()
                .sorted(Map.Entry.comparingByKey())
                .forEach(entry -> {
                    // Visualize as a bar chart (max 50 characters wide)
                    int barWidth = Math.min(50, entry.getValue() / Math.max(1, regionLength / 20));

                    sb.append(String.format("  %3dx: ", entry.getKey()));

                    for (int i = 0; i < barWidth; i++) {
                        sb.append('█');
                    }

                    sb.append(String.format(" (%d bp, %.0f%%)\n",
                        entry.getValue(), (double) entry.getValue() / regionLength * 100));
                });

            // Gap regions
            if (!gapRegions.isEmpty()) {
                sb.append(String.format("\nGap regions (0x Coverage): %d\n", gapRegions.size()));

                for (GapRegion gap : gapRegions.subList(0, Math.min(10, gapRegions.size()))) {
                    sb.append(String.format("  %s\n", gap));
                }

                if (gapRegions.size() > 10) {
                    sb.append(String.format("  ... and %d more gap regions\n", gapRegions.size() - 10));
                }

                long totalGapBp = gapRegions.stream().mapToInt(GapRegion::length).sum();
                sb.append(String.format("\nTotal gap length: %d bp (%.0f%%)\n",
                    totalGapBp, (double) totalGapBp / regionLength * 100));
            }

            // High coverage regions (>30x)
            List<CoverageRegion> highCoverage = coveredRegions.stream()
                .filter(r -> r.getMeanDepth() >= 30)
                .sorted((a, b) -> Double.compare(b.getMeanDepth(), a.getMeanDepth()))
                .toList();

            if (!highCoverage.isEmpty()) {
                sb.append(String.format("\nHigh coverage regions (≥30x): %d\n", highCoverage.size()));

                for (int i = 0; i < Math.min(5, highCoverage.size()); i++) {
                    sb.append(String.format("  %s\n", highCoverage.get(i)));
                }
            }

            return sb.toString();
        }

        @Override
        public String toString() { return generateReport(); }
    }

    // ======== MAIN ANALYSIS ==========

    /**
     * Analyze coverage/depth of a reference region over reads.
     * @param reference reference region (chromosome, length)
     * @param reads sequencing reads
     * @return CoverageResult with depth, histogram, gap detection
     */
    public static CoverageResult analyzeCoverage(ReferenceRegion reference, List<Read> reads) {
        return analyzeCoverage(reference, reads, 10);  // default threshold: 10x
    }

    /**
     * Analyze coverage/depth with a user-defined threshold.
     */
    public static CoverageResult analyzeCoverage(ReferenceRegion reference,
                                                List<Read> reads, int coverageThreshold) {
        String seqId = reference.getId();
        int regionLength = reference.getLength();

        // Create depth array (0-based)
        int[] depthArray = new int[regionLength];

        // Count coverage over all reads
        for (Read read : reads) {
            if (read.getPosition() >= regionLength) continue;  // read outside the region

            int readEnd = Math.min(read.endPosition(), regionLength);

            // Increase depth for all positions of the read
            for (int pos = read.getPosition(); pos < readEnd; pos++) {
                depthArray[pos]++;
            }
        }

        // Generate CoveragePoints
        List<CoveragePoint> coveragePoints = new ArrayList<>(regionLength);

        for (int i = 0; i < regionLength; i++) {
            coveragePoints.add(new CoveragePoint(i, depthArray[i]));
        }

        // Compute depth histogram (depth → number of positions)
        Map<Integer, Integer> depthHistogram = new TreeMap<>();

        for (int depth : depthArray) {
            depthHistogram.merge(depth, 1, Integer::sum);
        }

        // Compute statistics (mean, median)
        int[] sortedDepths = depthArray.clone();
        Arrays.sort(sortedDepths);

        double meanDepth = Arrays.stream(depthArray).average().orElse(0);
        int medianDepth = sortedDepths.length == 0 ? 0 : sortedDepths[sortedDepths.length / 2];

        // Find covered regions (above threshold)
        List<CoverageRegion> coveredRegions = findCoveredRegions(seqId, depthArray, regionLength, coverageThreshold);

        // Find gap regions (0x coverage)
        List<GapRegion> gapRegions = findGaps(seqId, depthArray, regionLength);

        // Compute percentage covered (positions ≥ threshold)
        long coveredPositions = Arrays.stream(depthArray).filter(d -> d >= coverageThreshold).count();
        double percentCovered = (double) coveredPositions / regionLength * 100;

        return new CoverageResult(
            seqId, regionLength, coveragePoints, depthHistogram,
            coveredRegions, gapRegions, percentCovered, meanDepth, medianDepth);
    }

    /** Find covered regions (contiguous areas ≥ threshold) */
    private static List<CoverageRegion> findCoveredRegions(String seqId, int[] depthArray,
                                                          int regionLength, int threshold) {
        List<CoverageRegion> regions = new ArrayList<>();

        int regionStart = -1;
        List<Integer> depthsInRegion = new ArrayList<>();

        for (int i = 0; i < regionLength; i++) {
            if (depthArray[i] >= threshold) {
                // Part of a covered region
                if (regionStart == -1) {
                    regionStart = i;  // region start
                    depthsInRegion.clear();
                }

                depthsInRegion.add(depthArray[i]);

            } else {
                // End region (if active)
                if (regionStart != -1) {
                    int regionEnd = i;  // exclusive
                    double meanDepth = depthsInRegion.stream().mapToInt(Integer::intValue).average().orElse(0);

                    // Percentage covered within this region
                    long coveredInRegion = depthsInRegion.stream().filter(d -> d >= threshold).count();
                    double percentCovered = (double) coveredInRegion / depthsInRegion.size() * 100;

                    regions.add(new CoverageRegion(seqId, regionStart, regionEnd,
                        meanDepth, percentCovered));

                    regionStart = -1;  // reset
                }
            }
        }

        // Close the open region at the end
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

    /** Find gap regions (0x coverage) */
    private static List<GapRegion> findGaps(String seqId, int[] depthArray, int regionLength) {
        List<GapRegion> gaps = new ArrayList<>();

        int gapStart = -1;

        for (int i = 0; i < regionLength; i++) {
            if (depthArray[i] == 0) {
                // Part of a gap
                if (gapStart == -1) {
                    gapStart = i;  // gap start

                }

            } else {
                // End gap (if active)
                if (gapStart != -1) {
                    gaps.add(new GapRegion(seqId, gapStart, i));  // exclusive
                    gapStart = -1;  // reset
                }
            }
        }

        // Close the open gap at the end
        if (gapStart != -1) {
            gaps.add(new GapRegion(seqId, gapStart, regionLength));
        }

        return gaps;
    }

    // ======== QUALITY-FILTERED ANALYSIS ==========

    /**
     * Analyze coverage with quality filtering (only reads ≥ Q20 count).
     */
    public static CoverageResult analyzeCoverageWithQualityFilter(ReferenceRegion reference,
                                                                   List<Read> reads) {
        // Filter reads by quality
        List<Read> validReads = reads.stream()
            .filter(Read::hasValidQuality)
            .collect(Collectors.toList());

        return analyzeCoverage(reference, validReads);
    }

    /** Count reads above a quality threshold */
    public static long countReadsAboveQuality(List<Read> reads, int minQ) {
        return reads.stream()
            .filter(read -> read.qualityString != null && !read.qualityString.isEmpty())
            .filter(read -> {
                List<Integer> phredScores = QualityScoreConverter.toPhredScores(read.qualityString, read.encoding);
                return phredScores.stream().allMatch(s -> s >= minQ);
            })
            .count();
    }

    // ======== EXPORT FUNCTIONS ==========

    /** Export coverage as a BED file */
    public static String exportToBED(CoverageResult result) {
        StringBuilder sb = new StringBuilder();

        // BED header (optional, but useful)
        sb.append(String.format("track type=coverage name=\"%s\" visibility=all\n", result.getSeqId()));

        for (CoverageRegion region : result.getCoveredRegions()) {
            sb.append(region.toBED()).append("\n");
        }

        return sb.toString();
    }

    /** Export coverage as a BigWig-like format (10bp bins) */
    public static String exportToBinnedCoverage(CoverageResult result, int binSize) {
        StringBuilder sb = new StringBuilder();

        int regionLength = result.getRegionLength();

        for (int i = 0; i < regionLength; i += binSize) {
            int binEnd = Math.min(i + binSize, regionLength);

            // Compute average depth over this bin
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

    // ======== UI FUNCTIONS ==========

    /** Generate a quick coverage overview */
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

    /** Compare coverage of two samples */
    public static String compareCoverage(ReferenceRegion reference,
                                        List<Read> reads1, List<Read> reads2) {
        CoverageResult result1 = analyzeCoverage(reference, reads1);
        CoverageResult result2 = analyzeCoverage(reference, reads2);

        StringBuilder sb = new StringBuilder();

        sb.append("=== Coverage Comparison ===\n\n");

        sb.append(String.format("%s:\n", result1.getSeqId()));
        sb.append(String.format("  Sample 1: Mean %.1fx, Coverage %.0f%%, Gaps %d\n",
            result1.getMeanDepth(), result1.getPercentCovered(), result1.getGapRegions().size()));
        sb.append(String.format("  Sample 2: Mean %.1fx, Coverage %.0f%%, Gaps %d\n",
            result2.getMeanDepth(), result2.getPercentCovered(), result2.getGapRegions().size()));

        double depthDiff = Math.abs(result1.getMeanDepth() - result2.getMeanDepth());
        sb.append(String.format("\nDepth difference: %.1fx\n", depthDiff));

        if (depthDiff < 5) {
            sb.append("Both samples have similar coverage ✓");
        } else {
            sb.append("Coverage difference significant — check sample preparation");
        }

        return sb.toString();
    }

    /** Generate the full coverage report for output */
    public static String generateFullCoverageReport(ReferenceRegion reference, List<Read> reads) {
        CoverageResult result = analyzeCoverage(reference, reads);

        StringBuilder sb = new StringBuilder();

        // Summary
        sb.append(result.generateReport());

        // Export options
        sb.append("\n=== Export ===\n");
        sb.append("BED-Format:\n");
        sb.append(exportToBED(result));

        return sb.toString();
    }

    // ======== GENERATE EXAMPLE READS (for tests) ==========

    /** Generate synthetic test reads for a reference */
    public static List<Read> generateSyntheticReads(ReferenceRegion reference, int avgDepth) {
        List<Read> reads = new ArrayList<>();

        if (reference.getLength() == 0) return reads;

        // Compute the number of reads based on the desired depth
        int readLength = 150;  // standard Illumina read length
        long totalReads = (long) avgDepth * reference.getLength() / readLength;

        // Deterministic pseudo-random generation (for reproducibility)
        Random rand = new Random(42);  // fixed seed

        for (long i = 0; i < totalReads; i++) {
            String readName = String.format("read_%08d", i);

            // Random position
            int position;

            if (reference.getLength() <= readLength) {
                position = 0;  // very short reference

            } else {
                position = rand.nextInt(reference.getLength() - readLength);
            }

            // Synthetic sequence (random bases)
            StringBuilder bases = new StringBuilder(readLength);

            String possibleBases = "ATGC";
            for (int j = 0; j < readLength && (position + j) < reference.getLength(); j++) {
                bases.append(possibleBases.charAt(rand.nextInt(4)));
            }

            // Quality string (medium quality ~Q30)
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
