package com.snapious.core;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;

class SequenceTest {

    @Test
    void testDnaComplement() {
        Sequence seq = new Sequence("ATCG", "Test-DNA", Sequence.SequenceType.DNA);
        assertEquals("TAGC", seq.complement().getBases());
    }

    @Test
    void testRnaComplement() {
        Sequence seq = new Sequence("AUCG", "Test-RNA", Sequence.SequenceType.RNA);
        assertEquals("UAGC", seq.complement().getBases());
    }

    @Test
    void testReverseComplement() {
        Sequence seq = new Sequence("ATCG", "Test-DNA", Sequence.SequenceType.DNA);
        assertEquals("CGAT", seq.reverseComplement().getBases());
    }

    @Test
    void testTranscribe() {
        Sequence dna = new Sequence("ATGCGT", "Test-DNA", Sequence.SequenceType.DNA);
        Sequence rna = dna.transcribe();

        assertEquals("AUGCGU", rna.getBases());
        assertEquals(Sequence.SequenceType.RNA, rna.getType());
    }

    @Test
    void testTranslate() {
        // Startcodon ATG -> M, dann TTT -> F
        Sequence dna = new Sequence("ATGTTTTAA", "Test-DNA", Sequence.SequenceType.DNA);
        String protein = dna.translate().getBases();

        assertTrue(protein.contains("M"));  // Start-Codon
    }

    @Test
    void testGcContent() {
        Sequence seq = new Sequence("ATCG", "Test-DNA", Sequence.SequenceType.DNA);
        assertEquals(0.5, seq.gcContent(), 0.01);

        Sequence gcHigh = new Sequence("GCGC", "Test-DNA", Sequence.SequenceType.DNA);
        assertEquals(1.0, gcHigh.gcContent(), 0.01);
    }

    @Test
    void testSubsequence() {
        Sequence seq = new Sequence("ATCGTAGC", "Test-DNA", Sequence.SequenceType.DNA);
        Sequence sub = seq.subsequence(2, 5);

        assertEquals("CGT", sub.getBases());
    }

    @Test
    void testFindOrfs() {
        // ATG CCG TAG: Startcodon, ein Codon, Stopp (Rahmen 3)
        String bases = "AAATGCCGTAGCTAA";
        Sequence seq = new Sequence(bases, "Test-ORF", Sequence.SequenceType.DNA);

        List<Sequence> orfs = seq.findORFs(2);  // Mindestlänge in Codons

        assertEquals(1, orfs.size());
        assertEquals("ATGCCG", orfs.get(0).getBases());
        assertTrue(seq.findORFs(3).isEmpty());
    }

    @Test
    void testFindRestrictionSites() {
        Sequence seq = new Sequence("GAATTCGCGAATTC", "Test-Restriction", Sequence.SequenceType.DNA);

        List<Integer> sites = seq.findRestrictionSites("GAATTC");  // EcoRI

        assertEquals(2, sites.size());
        assertTrue(sites.contains(0));
        assertTrue(sites.contains(8));
    }

    @Test
    void testFastaFormat() {
        Sequence seq = new Sequence("ATCG", "Test-ID", Sequence.SequenceType.DNA);
        String fasta = Sequence.toFASTA(seq);

        assertTrue(fasta.startsWith(">Test-ID"));
        assertTrue(fasta.contains("ATCG"));
    }

    @Test
    void testBaseFrequencies() {
        Sequence seq = new Sequence("AATTCCGG", "Test-DNA", Sequence.SequenceType.DNA);

        var freq = seq.baseFrequencies();
        assertEquals(0.25, freq.get('A'), 1e-9);
        assertEquals(0.25, freq.get('T'), 1e-9);
        assertEquals(0.25, freq.get('C'), 1e-9);
        assertEquals(0.25, freq.get('G'), 1e-9);
    }

    @Test
    void testInvalidDnaSequence() {
        assertThrows(IllegalArgumentException.class, () -> {
            new Sequence("ATXG", "Invalid", Sequence.SequenceType.DNA);
        });
    }

    @Test
    void testEmptySequence() {
        Sequence seq = new Sequence("", "Empty", Sequence.SequenceType.DNA);

        assertEquals(0, seq.getLength());
        assertTrue(seq.getBases().isEmpty());
    }

    @Test
    void testProteinSequence() {
        Sequence seq = new Sequence("MDEK", "Test-Protein", Sequence.SequenceType.PROTEIN);

        assertEquals(4, seq.getLength());
        assertEquals("MDEK", seq.getBases());
    }

    @Test
    void testEqualsAndHashCode() {
        Sequence seq1 = new Sequence("ATCG", "Test", Sequence.SequenceType.DNA);
        Sequence seq2 = new Sequence("ATCG", "Test", Sequence.SequenceType.DNA);

        assertEquals(seq1, seq2);
        assertEquals(seq1.hashCode(), seq2.hashCode());
    }

    @Test
    void testToString() {
        Sequence seq = new Sequence("ATCG", "Test-ID", Sequence.SequenceType.DNA);
        String str = seq.toString();

        assertTrue(str.contains("ATCG"));
        assertTrue(str.contains("Test-ID"));
    }

    @Test
    void testLargeSequence() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 10000; i++) {
            sb.append("ATCG");
        }

        long startTime = System.currentTimeMillis();
        Sequence seq = new Sequence(sb.toString(), "Large", Sequence.SequenceType.DNA);

        assertTrue(seq.getLength() == 40000);
        System.out.println("Large sequence created in " + (System.currentTimeMillis() - startTime) + "ms");
    }

    @Test
    void testCodonTranslation() {
        // ATG = M (Methionin), TTT = F (Phenylalanin)
        Sequence dna = new Sequence("ATGTTTTAA", "Codon-Test", Sequence.SequenceType.DNA);
        String protein = dna.translate().getBases();

        assertEquals("MF*", protein);
    }

    @Test
    void testSubsequenceBounds() {
        Sequence seq = new Sequence("ATCG", "Test-DNA", Sequence.SequenceType.DNA);

        assertThrows(IndexOutOfBoundsException.class, () -> {
            seq.subsequence(2, 10);  // End > Länge
        });

        assertThrows(IndexOutOfBoundsException.class, () -> {
            seq.subsequence(-1, 3);  // Negativer Start
        });
    }

    @Test
    void testGcContentEdgeCases() {
        Sequence empty = new Sequence("", "Empty", Sequence.SequenceType.DNA);

        // GC-Content von leerer Sequenz sollte 0 sein oder Exception
        try {
            double gc = empty.gcContent();
            assertEquals(0, gc, 0.01);
        } catch (IllegalStateException e) {
            // Auch akzeptabel
        }

        Sequence allA = new Sequence("AAAA", "All-A", Sequence.SequenceType.DNA);
        assertEquals(0.0, allA.gcContent(), 0.01);
    }

    @Test
    void testTranscribeRnaToDna() {
        Sequence rna = new Sequence("AUCG", "Test-RNA", Sequence.SequenceType.RNA);

        // RNA zu DNA Transkription (ersetzt U durch T)
        String dna = rna.toDNA().getBases();

        assertEquals("ATCG", dna);
    }

    @Test
    void testTranslateRnaUndStopcodon() {
        Sequence rna = new Sequence("AUGUUUUAA", "rna", Sequence.SequenceType.RNA);

        assertEquals("MF*", rna.translate().getBases());
    }

    @Test
    void testFromFastaTypErkennung() {
        assertEquals(Sequence.SequenceType.DNA, Sequence.fromFASTA("a", ">x", "ACGGCT\nGGA").getType());
        assertEquals(Sequence.SequenceType.RNA, Sequence.fromFASTA("a", ">x", "ACGUCU").getType());
        assertEquals(Sequence.SequenceType.PROTEIN, Sequence.fromFASTA("a", ">x", "MKVLAAGIV").getType());
    }

    @Test
    void testComplementVonProteinWirftException() {
        Sequence p = new Sequence("MDEK", "p", Sequence.SequenceType.PROTEIN);

        assertThrows(IllegalStateException.class, p::complement);
    }
}
