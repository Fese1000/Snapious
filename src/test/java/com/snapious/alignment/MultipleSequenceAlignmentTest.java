package com.snapious.alignment;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class MultipleSequenceAlignmentTest {

    @Test
    void testProgressiveMSA() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq3", "ID003", "ATGCGTGC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        var result = MultipleSequenceAlignment.progressiveAlignment(seqs);

        assertNotNull(result);
    }

    @Test
    void testGuideTreeBerechnung() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        var result = MultipleSequenceAlignment.progressiveAlignment(seqs);

        assertNotNull(result);
    }

    @Test
    void testNewickTreeGenerierung() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        String newick = MultipleSequenceAlignment.generateNewickFromMSA(seqs);

        assertNotNull(newick);
    }

    @Test
    void testFASTAExport() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        var result = MultipleSequenceAlignment.progressiveAlignment(seqs);

        String fasta = MultipleSequenceAlignment.exportToFASTA(result);

        assertNotNull(fasta);
    }

    @Test
    void testKonsensusSequenz() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        var result = MultipleSequenceAlignment.progressiveAlignment(seqs);

        assertEquals(result.getAlignmentLength(), result.getConsensusSequence().length());
    }

    @Test
    void testGapStatistik() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        var result = MultipleSequenceAlignment.progressiveAlignment(seqs);

        assertTrue(result.getAlignedSequences().stream().allMatch(a -> a.gapPercent() >= 0));
    }

    @Test
    void testPairwiseScores() {
        var seqs = java.util.List.of(
            new com.snapious.core.Sequence("seq1", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA),
            new com.snapious.core.Sequence("seq2", "ID002", "ATGAGTAC", com.snapious.core.Sequence.SequenceType.DNA)
        );

        var result = MultipleSequenceAlignment.progressiveAlignment(seqs);

        assertTrue(result.getAverageScore() > 0);
    }
}
