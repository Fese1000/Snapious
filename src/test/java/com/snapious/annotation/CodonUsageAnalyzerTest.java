package com.snapious.annotation;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class CodonUsageAnalyzerTest {

    @Test
    void testCodonUsageAnalyse() {
        String sequence = "ATGGCATAA";  // ATG=GCA=TAA (Met-Ala-Stop)

        var result = CodonUsageAnalyzer.analyzeCodonUsage(sequence);

        assertNotNull(result);
        assertTrue(result.getCai() >= 0);
        assertFalse(result.getRecommendations().isEmpty());
    }

    @Test
    void testCAIBerechnung() {
        String sequence = "ATGGCATAA";

        var result = CodonUsageAnalyzer.analyzeCodonUsage(sequence);

        assertTrue(result.getCai() >= 0);
        assertTrue(result.getCai() <= 1);
    }

    @Test
    void testGC3Gehalt() {
        String sequence = "ATGGCATAA";

        var result = CodonUsageAnalyzer.analyzeCodonUsage(sequence);

        assertTrue(result.getGc3Content() >= 0);
        assertTrue(result.getGc3Content() <= 1);
    }

    @Test
    void testRSCUBerechnung() {
        String sequence = "ATGGCATAA";

        var result = CodonUsageAnalyzer.analyzeCodonUsage(sequence);

        assertNotNull(result.getRscu());
    }

    @Test
    void testBerichtGenerierung() {
        String sequence = "ATGGCATAA";

        var result = CodonUsageAnalyzer.analyzeCodonUsage(sequence);

        String report = result.generateReport();

        assertNotNull(report);
        assertTrue(report.contains("Codon Usage Analysis"));
    }

    @Test
    void testVergleichZweierSequenzen() {
        String seq1 = "ATGGCATAA";
        String seq2 = "ATGAACTAA";

        String comparison = CodonUsageAnalyzer.compareCodonUsage(seq1, seq2);

        assertNotNull(comparison);
        assertTrue(comparison.contains("Codon Usage Comparison"));
    }

    @Test
    void testCodonOptimierung() {
        String sequence = "ATGGCATAA";

        String optimized = CodonUsageAnalyzer.optimizeCodons(sequence, "E. coli");

        assertNotNull(optimized);
    }

    @Test
    void testZuKurzeSequenz() {
        assertThrows(IllegalArgumentException.class, () -> {
            CodonUsageAnalyzer.analyzeCodonUsage("AT");  // < 3bp
        });
    }

    @Test
    void testAnalyseUndBericht() {
        String sequence = "ATGGCATAA";

        String report = CodonUsageAnalyzer.analyzeAndReport(sequence);

        assertNotNull(report);
        assertTrue(report.contains("Codon Usage Analysis"));
    }

    @Test
    void testFehlerhafteSequenz() {
        String report = CodonUsageAnalyzer.analyzeAndReport("AT");

        assertTrue(report.contains("Error"));
    }
}
