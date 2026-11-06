package com.snapious.alignment;

import com.snapious.alignment.Alignment;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class AlignmentTest {

    @Test
    void testNeedlemanWunschPerfectMatch() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("ATCG", "ATCG");

        assertEquals(8, result.getScore());  // 4 matches * 2
        assertEquals(Alignment.AlignType.GLOBAL, result.getType());
    }

    @Test
    void testNeedlemanWunschWithMismatches() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("ATCG", "ATCG");
        assertEquals(8, result.getScore());

        var result2 = align.needlemanWunsch("ATCG", "AACG");
        assertTrue(result2.getScore() < 8);  // Ein Mismatch reduziert Score
    }

    @Test
    void testNeedlemanWunschWithGaps() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("ATCG", "A-TCG");

        assertNotNull(result.getSeq1Aln());
        assertNotNull(result.getSeq2Aln());
    }

    @Test
    void testNeedlemanWunschEmptySequence() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("", "ATCG");

        assertEquals(-4, result.getScore());  // 4 Gaps à -1
    }

    @Test
    void testNeedlemanWunschDifferentLengths() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("ATCG", "AT");

        assertNotNull(result.getSeq1Aln());
        assertNotNull(result.getSeq2Aln());
    }

    @Test
    void testSmithWatermanPerfectMatch() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.smithWaterman("ATCG", "ATCG");

        assertEquals(8, result.getScore());
        assertEquals(Alignment.AlignType.LOCAL, result.getType());
    }

    @Test
    void testSmithWatermanLocalMatch() {
        Alignment align = new Alignment(2, -1, -1);

        // Nur der Mitte-Teil "TCG" matcht
        var result = align.smithWaterman("XXATCGYY", "AATCG");

        assertEquals(Alignment.AlignType.LOCAL, result.getType());
        assertTrue(result.getScore() > 0);
    }

    @Test
    void testSmithWatermanNoMatch() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.smithWaterman("ZZZZ", "XXXX");

        // Bei komplett unterschiedlichen Sequenzen sollte Score niedrig sein
        assertTrue(result.getScore() <= 0);
    }

    @Test
    void testSmithWatermanEmptySequence() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.smithWaterman("", "ATCG");

        assertEquals(0, result.getScore());
    }

    @Test
    void testAlignmentFormat() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("ATCG", "ATCG");
        String formatted = result.format();

        assertNotNull(formatted);
        assertTrue(formatted.contains("Score:"));
    }

    @Test
    void testCustomScoring() {
        // Proteine: höherer Match-Score, anderer Mismatch
        Alignment align = new Alignment(5, -2, -3);

        var result = align.needlemanWunsch("AAAA", "AAAA");
        assertEquals(20, result.getScore());  // 4 * 5
    }

    @Test
    void testCreateConsensus() {
        var aligned1 = "ATCG";
        var aligned2 = "A-TG";
        var aligned3 = "ATCG";

        var result = Alignment.createConsensus(
            java.util.List.of(aligned1, aligned2, aligned3));

        assertNotNull(result);
    }

    @Test
    void testCreateConsensusEmpty() {
        assertEquals("", Alignment.createConsensus(null));
        assertEquals("", Alignment.createConsensus(java.util.List.of()));
    }

    @Test
    void testEditDistanceIdentical() {
        assertEquals(0, Alignment.editDistance("ATCG", "ATCG"));
    }

    @Test
    void testEditDistanceEmpty() {
        assertEquals(4, Alignment.editDistance("", "ATCG"));
    }

    @Test
    void testEditDistanceSingleChange() {
        assertEquals(1, Alignment.editDistance("ATCG", "ATGG"));
    }

    @Test
    void testEditDistanceDifferentLengths() {
        double dist = Alignment.editDistance("ATCG", "AT");

        assertTrue(dist > 0);
    }

    @Test
    void testPercentIdentityPerfect() {
        assertEquals(100.0, Alignment.percentIdentity("ATCG", "ATCG"));
    }

    @Test
    void testPercentIdentityPartial() {
        double identity = Alignment.percentIdentity("ATCG", "AATG");

        assertTrue(identity < 100);
        assertTrue(identity > 0);
    }

    @Test
    void testPercentIdentityEmpty() {
        assertEquals(100, Alignment.percentIdentity("", ""));
    }

    @Test
    void testDifferentScoringMatrices() {
        // DNA Default: match=2, mismatch=-1, gap=-1
        Alignment dnaAlign = new Alignment();

        var dnaResult = dnaAlign.needlemanWunsch("ATCG", "ATCG");
        assertEquals(8, dnaResult.getScore());

        // Proteine: match=5, mismatch=-2, gap=-3
        Alignment proteinAlign = new Alignment(5, -2, -3);

        var protResult = proteinAlign.needlemanWunsch("AAAA", "AAAA");
        assertEquals(20, protResult.getScore());

        // Unterschiedliche Matrizen sollten unterschiedliche Scores liefern
        var mixedResult = proteinAlign.needlemanWunsch("ATCG", "AATG");
        assertTrue(mixedResult.getScore() > 0);
    }

    @Test
    void testAlignmentResultGetters() {
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch("ATCG", "ATCG");

        assertNotNull(result.getSeq1Aln());
        assertNotNull(result.getSeq2Aln());
        assertEquals(8, result.getScore());
        assertEquals(Alignment.AlignType.GLOBAL, result.getType());

        String formatted = result.format();
        assertNotNull(formatted);
    }

    @Test
    void testLargeSequenceAlignment() {
        StringBuilder sb1 = new StringBuilder();
        StringBuilder sb2 = new StringBuilder();

        for (int i = 0; i < 500; i++) {
            sb1.append("ATCG");
            sb2.append("ATCG");
        }

        long startTime = System.currentTimeMillis();
        Alignment align = new Alignment(2, -1, -1);

        var result = align.needlemanWunsch(sb1.toString(), sb2.toString());

        long elapsed = System.currentTimeMillis() - startTime;
        assertTrue(elapsed < 5000, "Alignment sollte innerhalb von 5s fertig sein");

        assertEquals(4000, result.getScore());
    }

    @Test
    void testSmithWatermanLongerMatch() {
        Alignment align = new Alignment(2, -1, -1);

        // Die längere Übereinstimmung sollte gefunden werden
        var result = align.smithWaterman(
            "AAATTTTTTGGG",
            "TTTGTTTTTCCC");

        assertEquals(Alignment.AlignType.LOCAL, result.getType());
        assertTrue(result.getScore() > 0);
    }

    @Test
    void testMultipleAlignments() {
        Alignment align = new Alignment(2, -1, -1);

        // Verschiedene Paare testen
        var pair1 = align.needlemanWunsch("A", "A");
        assertEquals(2, pair1.getScore());

        var pair2 = align.needlemanWunsch("A", "T");
        assertTrue(pair2.getScore() <= 0);

        var pair3 = align.smithWaterman("AT", "AT");
        assertEquals(4, pair3.getScore());
    }

    @Test
    void testConsensusMultipleSequences() {
        var aligned = java.util.List.of(
            "ATCG",
            "A-TG",
            "ATCG",
            "A-CA");

        String consensus = Alignment.createConsensus(aligned);

        assertNotNull(consensus);
        assertTrue(!consensus.isEmpty());
    }

    @Test
    void testEditDistanceMatrixProperty() {
        // Dreiecksungleichung für Edit-Distanz
        String a = "AAA";
        String b = "AAB";
        String c = "BBB";

        double dAB = Alignment.editDistance(a, b);
        double dBC = Alignment.editDistance(b, c);
        double dAC = Alignment.editDistance(a, c);

        assertTrue(dAC <= dAB + dBC, "Dreiecksungleichung muss gelten");
    }

    @Test
    void testPercentIdentitySymmetric() {
        double id1 = Alignment.percentIdentity("ATCG", "AAGG");
        double id2 = Alignment.percentIdentity("AAGG", "ATCG");

        assertEquals(id1, id2);  // Symmetrisch
    }
}
