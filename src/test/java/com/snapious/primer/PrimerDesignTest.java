package com.snapious.primer;

import com.snapious.core.Sequence;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class PrimerDesignTest {

    @Test
    void tmNachWallace() {
        // ATGC: 2 AT * 2 + 2 GC * 4 = 12
        assertEquals(12.0, PrimerDesign.calculateTm("ATGC", PrimerDesign.TmMethod.WALLACE), 1e-9);
    }

    @Test
    void gcGehalt() {
        assertEquals(0.5, PrimerDesign.gcContent("ATGC"), 1e-9);
        assertEquals(0.0, PrimerDesign.gcContent(""), 1e-9);
    }

    @Test
    void primerQualitaetWirdBerechnet() {
        var quality = PrimerDesign.evaluatePrimer("ATGCGTACGTAGCTAGCTAG");

        assertEquals(20, quality.getLength());
        assertTrue(quality.getTm() > 0);
    }

    @Test
    void selbstDimerScoreIstNichtNegativ() {
        assertTrue(PrimerDesign.calculateSelfDimerScore("ATGCGTAC", "ATGCGTAC") >= 0);
    }

    @Test
    void zuKurzesTargetWirftException() {
        var target = new Sequence("t", "t", "ATGC", Sequence.SequenceType.DNA);
        assertThrows(IllegalArgumentException.class, () -> PrimerDesign.designDefaultPrimerPair(target));
    }
}
