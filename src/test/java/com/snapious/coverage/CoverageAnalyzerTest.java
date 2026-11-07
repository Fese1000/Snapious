package com.snapious.coverage;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class CoverageAnalyzerTest {

    @Test
    void testCoverageAnalyseGrundlagen() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 1000);

        // Create simple reads covering the first half
        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(200), 0),
            new CoverageAnalyzer.Read("read2", "ATGC".repeat(200), 100)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertNotNull(result);
        assertEquals("chr1", result.getSeqId());
        assertEquals(1000, result.getRegionLength());
    }

    @Test
    void testDepthBerechnung() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        // Two reads that overlap
        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0),
            new CoverageAnalyzer.Read("read2", "ATGC".repeat(50), 25)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertNotNull(result.getCoveragePoints());
    }

    @Test
    void testCoverageHistogramm() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertNotNull(result.getDepthHistogram());
    }

    @Test
    void testGapErkennung() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        // Only the first 48 of 100 positions covered
        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(12), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertEquals(1, result.getGapRegions().size());
        assertEquals(48, result.getGapRegions().get(0).getStart());
        assertEquals(100, result.getGapRegions().get(0).getEnd());
    }

    @Test
    void testCoveredRegionen() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 200);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0),
            new CoverageAnalyzer.Read("read2", "ATGC".repeat(50), 100)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertNotNull(result.getCoveredRegions());
    }

    @Test
    void testProzentualeAbdeckung() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertTrue(result.getPercentCovered() >= 0);
        assertTrue(result.getPercentCovered() <= 100);
    }

    @Test
    void testMeanUndMedianDepth() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        assertTrue(result.getMeanDepth() >= 0);
        assertTrue(result.getMedianDepth() >= 0);
    }

    @Test
    void testCoverageLabel() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        String label = result.coverageLabel();

        assertNotNull(label);
    }

    @Test
    void testCoverageBericht() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        String report = result.generateReport();

        assertNotNull(report);
        assertTrue(report.contains("Coverage Analysis"));
    }

    @Test
    void testBEDExport() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 200);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        String bed = CoverageAnalyzer.exportToBED(result);

        assertNotNull(bed);
    }

    @Test
    void testBinnedCoverageExport() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 200);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var result = CoverageAnalyzer.analyzeCoverage(ref, reads);

        String binned = CoverageAnalyzer.exportToBinnedCoverage(result, 10);

        assertNotNull(binned);
    }

    @Test
    void testSynthetischeReadsGenerierung() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 1000);

        var reads = CoverageAnalyzer.generateSyntheticReads(ref, 30);

        assertFalse(reads.isEmpty());
    }

    @Test
    void testQuickCoverageOverview() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        String overview = CoverageAnalyzer.quickCoverageOverview(ref, reads);

        assertNotNull(overview);
        assertTrue(overview.contains("Coverage"));
    }

    @Test
    void testVergleichZweierProben() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 200);

        var reads1 = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        var reads2 = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 10)
        );

        String comparison = CoverageAnalyzer.compareCoverage(ref, reads1, reads2);

        assertNotNull(comparison);
        assertTrue(comparison.contains("Coverage Comparison"));
    }

    @Test
    void testVollstaendigerCoverageBericht() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 200);

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0)
        );

        String report = CoverageAnalyzer.generateFullCoverageReport(ref, reads);

        assertNotNull(report);
    }

    @Test
    void testQualitaetsGefilterteAnalyse() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        // Reads with quality string (Q30+)
        StringBuilder quality = new StringBuilder("ATGC".repeat(50));

        for (int i = 0; i < quality.length(); i++) {
            quality.setCharAt(i, (char) (quality.charAt(i) + 30));  // Q30+
        }

        var reads = java.util.List.of(
            new CoverageAnalyzer.Read("read1", "ATGC".repeat(50), 0,
                quality.toString(), com.snapious.quality.QualityScoreConverter.EncodingType.SANGER)
        );

        var result = CoverageAnalyzer.analyzeCoverageWithQualityFilter(ref, reads);

        assertNotNull(result);
    }

    @Test
    void testKurzReferenz() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 0);

        var reads = CoverageAnalyzer.generateSyntheticReads(ref, 30);

        assertTrue(reads.isEmpty());
    }

    @Test
    void testReadKlasse() {
        var read = new CoverageAnalyzer.Read("test", "ATGC", 10);

        assertEquals("test", read.getName());
        assertEquals(4, read.length());
        assertEquals(14, read.endPosition());
    }

    @Test
    void testReferenceRegion() {
        var ref = new CoverageAnalyzer.ReferenceRegion("chr1", 100);

        assertEquals("chr1", ref.getId());
        assertEquals(100, ref.getLength());

        // Generate N sequence (max 1000)
        String nSeq = ref.generateNSequence();

        assertEquals(100, nSeq.length());
        assertTrue(nSeq.matches("N+"));
    }

    @Test
    void testCoveragePoint() {
        var cp = new CoverageAnalyzer.CoveragePoint(50, 25);

        assertEquals(50, cp.getPosition());
        assertEquals(25, cp.getDepth());
        assertTrue(cp.aboveThreshold(20));
        assertFalse(cp.aboveThreshold(30));
    }

    @Test
    void testCoverageRegion() {
        var region = new CoverageAnalyzer.CoverageRegion("chr1", 0, 100, 30, 95.0);

        assertEquals("chr1", region.getSeqId());
        assertEquals(100, region.length());
        assertEquals(30, region.getMeanDepth());

        String bed = region.toBED();

        assertEquals("chr1\t0\t100\tCoverage_30.0x\t30\t.", bed);
    }

    @Test
    void testGapRegion() {
        var gap = new CoverageAnalyzer.GapRegion("chr1", 500, 600);

        assertEquals("chr1", gap.getSeqId());
        assertEquals(100, gap.length());
    }

    @Test
    void testEncodingTypes() {
        assertEquals(3, com.snapious.quality.QualityScoreConverter.EncodingType.values().length);

        var sanger = com.snapious.quality.QualityScoreConverter.EncodingType.SANGER;

        assertNotNull(sanger.getDescription());
        assertEquals(33, sanger.getOffset());
    }

    @Test
    void testPhredScoresKonvertierung() {
        String qualityString = "!#$%&";

        var scores = com.snapious.quality.QualityScoreConverter.toPhredScores(qualityString,
            com.snapious.quality.QualityScoreConverter.EncodingType.SANGER);

        assertFalse(scores.isEmpty());
    }
}
