package com.snapious.editor;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class SequenceEditorTest {

    @Test
    void testSNPTransition() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var result = SequenceEditor.performSNP(seq, 0, 'G');

        assertEquals("GTGCGTAC", result.getEditedSequence());
        assertTrue(result.getDescription().contains("Transition"));
    }

    @Test
    void testSNPTransversion() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var result = SequenceEditor.performSNP(seq, 0, 'C');

        assertEquals("CTGCGTAC", result.getEditedSequence());
    }

    @Test
    void testSNPPositionAußerhalb() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGC", com.snapious.core.Sequence.SequenceType.DNA);

        assertThrows(IndexOutOfBoundsException.class, () -> {
            SequenceEditor.performSNP(seq, 10, 'A');
        });
    }

    @Test
    void testInsertion() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var result = SequenceEditor.performInsertion(seq, 3, "GC");

        assertEquals("ATGGCCGTAC", result.getEditedSequence());
    }

    @Test
    void testDeletion() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var result = SequenceEditor.performDeletion(seq, 2, 6);

        assertEquals("ATAC", result.getEditedSequence());  // Basen 2..5 (GCGT) entfernt
    }

    @Test
    void testSubstitution() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var result = SequenceEditor.performSubstitution(seq, 2, 6, "TT");

        assertEquals("ATTTAC", result.getEditedSequence());
        assertThrows(IllegalArgumentException.class,
            () -> SequenceEditor.performSubstitution(seq, 2, 6, "XXXX"));
    }

    @Test
    void testReverseComplement() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGC", com.snapious.core.Sequence.SequenceType.DNA);

        var result = SequenceEditor.performReverseComplement(seq);

        assertEquals("GCAT", result.getEditedSequence());
    }

    @Test
    void testBatchEdits() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var ops = java.util.List.of(
            SequenceEditor.EditOperation.snp(0, 'G'),
            SequenceEditor.EditOperation.insertion(3, "GC")
        );

        var results = SequenceEditor.batchEdits(seq, ops);

        assertNotNull(results);
        assertFalse(results.isEmpty());
    }

    @Test
    void testEditingSuggestions() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGGAATTCCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var suggestions = SequenceEditor.getEditingSuggestions(seq);

        assertNotNull(suggestions);
    }

    @Test
    void testEditReportGenerierung() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var ops = java.util.List.of(
            SequenceEditor.EditOperation.snp(0, 'G')
        );

        var results = SequenceEditor.batchEdits(seq, ops);

        String report = SequenceEditor.generateEditReport(seq, results);

        assertNotNull(report);
        assertTrue(report.contains("Editier-Bericht"));
    }

    @Test
    void testUndoRedo() {
        var seq = new com.snapious.core.Sequence("test", "ID001", "ATGCGTAC", com.snapious.core.Sequence.SequenceType.DNA);

        var edited = SequenceEditor.performSNP(seq, 0, 'G');
        var undone = SequenceEditor.undoEdit(
            new com.snapious.core.Sequence("test", "ID001", edited.getEditedSequence(), com.snapious.core.Sequence.SequenceType.DNA),
            seq.getBases());

        assertEquals("ATGCGTAC", undone.getBases());
    }
}
