package com.snapious.search;

import com.snapious.core.Sequence;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class BLASTLikeSearchTest {

    private static Sequence dna(String id, String bases) {
        return new Sequence(id, id, bases, Sequence.SequenceType.DNA);
    }

    @Test
    void findetExakteTeilsequenz() {
        var db = List.of(dna("subject", "TTTTATGCGTACGTAGCTAGCCCC"));
        var hits = BLASTLikeSearch.search("ATGCGTACGTAGCTAG", db, 8);

        assertFalse(hits.isEmpty());
        assertEquals("subject", hits.get(0).getSubjectId());
        assertEquals(100.0, hits.get(0).percentIdentity(), 0.01);
    }

    @Test
    void keinTrefferBeiUnverwandterSequenz() {
        var db = List.of(dna("subject", "CCCCCCCCCCCCCCCC"));
        var hits = BLASTLikeSearch.search("AAAAAAAAAAAAAAAA", db, 8);

        assertTrue(hits.isEmpty());
    }

    @Test
    void findetReverseKomplementAufMinusStrang() {
        var db = List.of(dna("subject", "CCCCATGCGTACGTAGCTAGCCCC"));
        // Reverse complement of the subsequence
        var hits = BLASTLikeSearch.search("CTAGCTACGTACGCAT", db, 8);

        assertFalse(hits.isEmpty());
        assertEquals('-', hits.get(0).getStrand());
        assertEquals(16, hits.get(0).length());
    }

    @Test
    void kmerGenerierung() {
        var kmers = BLASTLikeSearch.getUniqueKmers("ATGCGT", 3);

        assertEquals(4, kmers.size());
        assertTrue(kmers.contains("ATG"));
    }

    @Test
    void kmerHaeufigkeiten() {
        var freq = BLASTLikeSearch.getKmerFrequencies("AAAA", 2);

        assertEquals(3, freq.get("AA"));
    }
}
