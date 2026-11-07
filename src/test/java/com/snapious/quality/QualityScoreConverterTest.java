package com.snapious.quality;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class QualityScoreConverterTest {

    @Test
    void testEncodingErkennungSanger() {
        // Sanger: ASCII 33-93 ('!' to '~')
        String qualityString = "!#$$$%&'()*+";

        assertEquals(QualityScoreConverter.EncodingType.SANGER,
            QualityScoreConverter.detectEncoding(qualityString));
    }

    @Test
    void testEncodingErkennungIllumina13() {
        // Illumina 1.0-1.3: ASCII 64-126 ('@' to '~')
        String qualityString = "@ABCDEFGHI";

        assertEquals(QualityScoreConverter.EncodingType.ILLUMINA_13,
            QualityScoreConverter.detectEncoding(qualityString));
    }

    @Test
    void testKonvertierungZwischenEncodings() {
        String sangerQuality = "!#$$$";

        String converted = QualityScoreConverter.convertEncoding(
            sangerQuality,
            QualityScoreConverter.EncodingType.SANGER,
            QualityScoreConverter.EncodingType.ILLUMINA_13);

        assertNotNull(converted);
    }

    @Test
    void testPhredScoresBerechnung() {
        String qualityString = "!#$%&";

        var scores = QualityScoreConverter.toPhredScores(qualityString,
            QualityScoreConverter.EncodingType.SANGER);

        assertNotNull(scores);
        assertFalse(scores.isEmpty());
    }

    @Test
    void testQualityStatistik() {
        String qualityString = "!#$%&'()*+";

        var stats = QualityScoreConverter.calculateQualityStats(qualityString,
            QualityScoreConverter.EncodingType.SANGER);

        assertNotNull(stats);
        assertTrue(stats.getMean() >= 0);
    }

    @Test
    void testQualityHistogramm() {
        String qualityString = "!!!###";

        var histogram = QualityScoreConverter.qualityHistogram(qualityString,
            QualityScoreConverter.EncodingType.SANGER);

        assertNotNull(histogram);
    }

    @Test
    void testQualityBericht() {
        var qualityStrings = java.util.List.of("!!!###", "%%%&&&");

        String report = QualityScoreConverter.generateQualityReport(qualityStrings);

        assertNotNull(report);
        assertTrue(report.contains("Quality Report"));
    }

    @Test
    void testFilterNachQuality() {
        String qualityString = "!#$%&'()*+";

        String filtered = QualityScoreConverter.filterByQuality(
            qualityString, QualityScoreConverter.EncodingType.SANGER, 20);

        assertNotNull(filtered);
    }
}
