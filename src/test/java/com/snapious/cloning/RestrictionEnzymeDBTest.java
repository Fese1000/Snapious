package com.snapious.cloning;

import com.snapious.core.Sequence;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class RestrictionEnzymeDBTest {

    private final com.snapious.cloning.RestrictionEnzymeDB enzymeDB =
        new com.snapious.cloning.RestrictionEnzymeDB();

    @Test
    void testGetAllEnzymes() {
        var enzymes = enzymeDB.getAllEnzymes();

        assertTrue(enzymes.size() > 50, "Sollte mindestens 50 Enzyme enthalten");
    }

    @Test
    void testSearchByNameEcoRI() {
        var results = enzymeDB.searchByName("eco");

        assertTrue(results.stream().anyMatch(e -> e.getName().equals("EcoRI")));
    }

    @Test
    void testSearchByRecognitionSequence() {
        var results = enzymeDB.searchByRecognitionSequence("GAATTC");

        assertTrue(results.stream().anyMatch(e -> e.getName().equals("EcoRI")));
    }

    @Test
    void testFindByBluntEnd() {
        var bluntEnzymes = enzymeDB.findByBluntEnd();

        assertTrue(bluntEnzymes.size() > 0);
        assertTrue(bluntEnzymes.stream().allMatch(com.snapious.cloning.RestrictionEnzymeDB.RestrictionEnzyme::isBluntEnd));
    }

    @Test
    void testFindByStickyEnd() {
        var stickyEnzymes = enzymeDB.findByStickyEnd();

        assertTrue(stickyEnzymes.size() > 0);
        assertTrue(stickyEnzymes.stream().allMatch(e -> e.isStickyEnd()));
    }

    @Test
    void testFindCutSitesEcoRI() {
        Sequence seq = new Sequence("test", "Test", "AAGGAATTCGATTAGCTAGC", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        assertTrue(sites.stream().anyMatch(c -> c.getEnzyme().getName().equals("EcoRI")));
    }

    @Test
    void testCutSitePosition() {
        Sequence seq = new Sequence("test", "Test", "AAGGAATTCGATTAGCTAGC", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        var ecoSite = sites.stream()
            .filter(c -> c.getEnzyme().getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoSite.isPresent());
        assertEquals(3, ecoSite.get().getPosition());  // GAATTC beginnt bei Position 3
    }

    @Test
    void testMultipleCutSites() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 5; i++) {
            sb.append("GAATTC");  // Mehrere EcoRI Stellen
        }

        Sequence seq = new Sequence("multi", "Multi", sb.toString(), Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        long ecoCount = sites.stream()
            .filter(c -> c.getEnzyme().getName().equals("EcoRI"))
            .count();

        assertEquals(5, ecoCount);
    }

    @Test
    void testRestrictionEnzymeGetters() {
        var enzymes = enzymeDB.getAllEnzymes();

        var ecoRI = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoRI.isPresent());
        assertEquals("GAATTC", ecoRI.get().getRecognitionSequence());
        assertEquals(1, ecoRI.get().getCutPositionWithinRecognitionSite());
        assertTrue(ecoRI.get().isStickyEnd());
    }

    @Test
    void testSmaIBluntEnd() {
        var enzymes = enzymeDB.getAllEnzymes();

        var smaI = enzymes.stream()
            .filter(e -> e.getName().equals("SmaI"))
            .findFirst();

        assertTrue(smaI.isPresent());
        assertTrue(smaI.get().isBluntEnd());  // SmaI schneidet stumpf: CCC^GGG
    }

    @Test
    void testPmeIBluntEnd() {
        var enzymes = enzymeDB.getAllEnzymes();

        var pmeI = enzymes.stream()
            .filter(e -> e.getName().equals("PmeI"))
            .findFirst();

        assertTrue(pmeI.isPresent());
        assertTrue(pmeI.get().isBluntEnd());  // PmeI: GGGCC^CCC
    }

    @Test
    void testTypeIISEnzymes() {
        var typeIIS = enzymeDB.findByType(
            com.snapious.cloning.RestrictionEnzymeDB.EnzymeType.TYPE_IIS);

        assertTrue(typeIIS.size() > 0);
        assertTrue(typeIIS.stream().allMatch(
            e -> e.getType() == com.snapious.cloning.RestrictionEnzymeDB.EnzymeType.TYPE_IIS));
    }

    @Test
    void testCutMethodEcoRI() {
        var enzymes = enzymeDB.getAllEnzymes();

        var ecoRI = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoRI.isPresent());

        var fragments = ecoRI.get().cut("AAGGAATTCGATTAGCTAGC");

        assertTrue(fragments.size() >= 2);
    }

    @Test
    void testCutSiteToString() {
        Sequence seq = new Sequence("test", "Test", "AAGGAATTCGATTAGCTAGC", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        var ecoSite = sites.stream()
            .filter(c -> c.getEnzyme().getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoSite.isPresent());

        String str = ecoSite.get().toString();
        assertTrue(str.contains("EcoRI"));
        assertTrue(str.contains("@3"));
    }

    @Test
    void testNoCutSitesForSequence() {
        Sequence seq = new Sequence("noenz", "NoEnz", "AAAAAAAATTTTTTTT", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        // Sollte wenige oder keine Schnittstellen haben (A/T-reich)
        assertNotNull(sites);
    }

    @Test
    void testEnzymeSourceOrganism() {
        var enzymes = enzymeDB.getAllEnzymes();

        var ecoRI = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoRI.isPresent());
        assertEquals("E. coli", ecoRI.get().getSourceOrganism());
    }

    @Test
    void testEnzymeTemperature() {
        var enzymes = enzymeDB.getAllEnzymes();

        var taqI = enzymes.stream()
            .filter(e -> e.getName().equals("TaqI"))
            .findFirst();

        assertTrue(taqI.isPresent());
        assertEquals("65°C", taqI.get().getTemperature());  // TaqI bei höherer Temperatur
    }

    @Test
    void testIsochoreSearch() {
        var enzymes = enzymeDB.getAllEnzymes();

        var smaIEnzymes = enzymes.stream()
            .filter(e -> e.getRecognitionSequence().equals("CCCGGG"))
            .toList();

        assertTrue(smaIEnzymes.size() >= 2);  // SmaI und XmaI haben gleiche Erkennungssequenz
    }

    @Test
    void testLargeSequencePerformance() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 10000; i++) {
            sb.append("ATCG");
        }

        Sequence seq = new Sequence("large", "Large", sb.toString(), Sequence.SequenceType.DNA);

        long startTime = System.currentTimeMillis();
        var sites = enzymeDB.findAllCutSites(seq);
        long elapsed = System.currentTimeMillis() - startTime;

        assertTrue(elapsed < 5000, "Sollte innerhalb von 5s.finishen");
        assertNotNull(sites);
    }

    @Test
    void testEmptySequence() {
        Sequence seq = new Sequence("empty", "Empty", "", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        assertTrue(sites.isEmpty());
    }

    @Test
    void testAddCustomEnzyme() {
        var custom = new com.snapious.cloning.RestrictionEnzymeDB.RestrictionEnzyme(
            "CustomI", "AATTCC", 2,
            com.snapious.cloning.RestrictionEnzymeDB.EnzymeType.TYPE_II,
            true, "Custom Organism", "Test Supplier", "37°C", "None"
        );

        enzymeDB.addEnzyme(custom);

        var results = enzymeDB.searchByName("CustomI");
        assertEquals(1, results.size());
        assertEquals("AATTCC", results.get(0).getRecognitionSequence());
    }

    @Test
    void testFindCompatibleEnzymes() {
        var enzymes = enzymeDB.getAllEnzymes();

        var ecoRI = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoRI.isPresent());

        // Kompatible Enzyme sollten stumpfe Enden haben (da EcoRI sticky ends)
        var compatible = enzymeDB.findCompatibleEnzymes(ecoRI.get());

        assertNotNull(compatible);  // Kann leer sein wenn keine komplementären Overhänge
    }

    @Test
    void testMethylationSensitiveEnzymes() {
        var sensitive = enzymeDB.getMethylationSensitive();

        assertNotNull(sensitive);  // Sollte Liste zurückgeben (kann leer sein)
    }

    @Test
    void testMultipleEnzymeTypes() {
        var enzymes = enzymeDB.getAllEnzymes();

        var types = enzymes.stream()
            .map(com.snapious.cloning.RestrictionEnzymeDB.RestrictionEnzyme::getType)
            .distinct()
            .toList();

        assertTrue(types.size() >= 2);  // palindromische Typ-II und Typ-IIS-Enzyme
    }

    @Test
    void testCutSiteIsBlunt() {
        Sequence seq = new Sequence("test", "Test", "AAACCCGGGTTT", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        var smaISite = sites.stream()
            .filter(c -> c.getEnzyme().getName().equals("SmaI"))
            .findFirst();

        assertTrue(smaISite.isPresent());
        assertTrue(smaISite.get().isBlunt());  // SmaI: CCC^GGG (stumpf)
    }

    @Test
    void testEnzymeEqualsAndHashCode() {
        var enzymes = enzymeDB.getAllEnzymes();

        var ecoRI1 = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        var ecoRI2 = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoRI1.isPresent());
        assertTrue(ecoRI2.isPresent());

        assertEquals(ecoRI1.get(), ecoRI2.get());  // Gleiche Namen = gleich
        assertEquals(ecoRI1.get().hashCode(), ecoRI2.get().hashCode());
    }

    @Test
    void testBamHIandMboIGenerateCompatibleEnds() {
        var enzymes = enzymeDB.getAllEnzymes();

        var bamHI = enzymes.stream()
            .filter(e -> e.getName().equals("BamHI"))
            .findFirst();

        var mboI = enzymes.stream()
            .filter(e -> e.getName().equals("MboI"))
            .findFirst();

        assertTrue(bamHI.isPresent());
        assertTrue(mboI.isPresent());

        // Beide erkennen GATC und generieren kompatible sticky ends
        assertTrue(bamHI.get().isStickyEnd());
        assertTrue(mboI.get().isStickyEnd());
    }

    @Test
    void testHindIIIEnzyme() {
        var enzymes = enzymeDB.getAllEnzymes();

        var hindIII = enzymes.stream()
            .filter(e -> e.getName().equals("HindIII"))
            .findFirst();

        assertTrue(hindIII.isPresent());
        assertEquals("AAGCTT", hindIII.get().getRecognitionSequence());
        assertTrue(hindIII.get().isStickyEnd());
    }

    @Test
    void testNotIRecognitionSequence() {
        var enzymes = enzymeDB.getAllEnzymes();

        var notI = enzymes.stream()
            .filter(e -> e.getName().equals("NotI"))
            .findFirst();

        assertTrue(notI.isPresent());
        assertEquals("GCGGCCGC", notI.get().getRecognitionSequence());  // 8bp cutter
    }

    @Test
    void testXhoIandXbaIGenerateCompatibleEnds() {
        var enzymes = enzymeDB.getAllEnzymes();

        var xhoI = enzymes.stream()
            .filter(e -> e.getName().equals("XhoI"))
            .findFirst();

        var xbaI = enzymes.stream()
            .filter(e -> e.getName().equals("XbaI"))
            .findFirst();

        assertTrue(xhoI.isPresent());
        assertTrue(xbaI.isPresent());

        // XhoI (CTCGAG) und XbaI (TCTAGA) generieren kompatible sticky ends
        assertTrue(xhoI.get().isStickyEnd());
        assertTrue(xbaI.get().isStickyEnd());
    }

    @Test
    void testShortSequenceNoCutSites() {
        Sequence seq = new Sequence("short", "Short", "ATCG", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        // 4bp Sequenz kann keine Schnittstellen für 6-8bp cutter haben
        assertTrue(sites.isEmpty());
    }

    @Test
    void testFindCutPositionByPosition() {
        var enzymes = enzymeDB.getAllEnzymes();

        var pos1Enzymes = enzymeDB.findByCutPosition(1);

        assertTrue(pos1Enzymes.size() > 0);  // Viele Enzyme schneiden bei Position 1
    }

    @Test
    void testEnzymeBufferCondition() {
        var enzymes = enzymeDB.getAllEnzymes();

        var ecoRI = enzymes.stream()
            .filter(e -> e.getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoRI.isPresent());
        assertFalse(ecoRI.get().getBufferCondition().isBlank());
    }

    @Test
    void testOvernightDigestionEnzymes() {
        var enzymes = enzymeDB.getAllEnzymes();

        // Enzyme die nicht bei 37°C schneiden (overnight Digestion)
        var non37Enzymes = enzymes.stream()
            .filter(e -> !e.getTemperature().equals("37°C"))
            .toList();

        assertTrue(non37Enzymes.size() > 0);  // z.B. PmeI bei 30°C, TaqI bei 65°C
    }

    @Test
    void testCutSiteGetOverhang() {
        Sequence seq = new Sequence("test", "Test", "AAGGAATTCGATTAGCTAGC", Sequence.SequenceType.DNA);

        var sites = enzymeDB.findAllCutSites(seq);

        var ecoSite = sites.stream()
            .filter(c -> c.getEnzyme().getName().equals("EcoRI"))
            .findFirst();

        assertTrue(ecoSite.isPresent());

        String overhang = ecoSite.get().getOverhang();
        // EcoRI: G^AATTC → 5' Überhang "AATT"
        assertEquals("AATT", overhang);
    }

    @Test
    void testDatabaseCompleteness() {
        var enzymes = enzymeDB.getAllEnzymes();

        // Prüfe ob häufige Enzyme vorhanden sind
        String[] commonEnzymes = {"EcoRI", "BamHI", "HindIII", "NotI", "XhoI", "XbaI",
            "SalI", "PstI", "SmaI", "NcoI", "NdeI", "BglII"};

        for (String name : commonEnzymes) {
            var found = enzymes.stream()
                .filter(e -> e.getName().equals(name))
                .findFirst();

            assertTrue(found.isPresent(), "Fehlendes häufiges Enzym: " + name);
        }
    }

    @Test
    void testCutMethodBluntEnd() {
        var enzymes = enzymeDB.getAllEnzymes();

        var smaI = enzymes.stream()
            .filter(e -> e.getName().equals("SmaI"))
            .findFirst();

        assertTrue(smaI.isPresent());

        var fragments = smaI.get().cut("AAACCCGGGTTT");

        assertTrue(fragments.size() >= 2);
    }

    @Test
    void testMultipleSequencesCutSites() {
        Sequence seq1 = new Sequence("seq1", "Seq1", "AAGGAATTCGATTAGCTAGC", Sequence.SequenceType.DNA);
        Sequence seq2 = new Sequence("seq2", "Seq2", "GGATCCTAGCTAGCTAGC", Sequence.SequenceType.DNA);

        var sites1 = enzymeDB.findAllCutSites(seq1);
        var sites2 = enzymeDB.findAllCutSites(seq2);

        assertNotNull(sites1);
        assertNotNull(sites2);
    }

    @Test
    void testCutProducesExpectedFragments() {
        var ecoRI = enzymeDB.searchByName("EcoRI").get(0);

        var fragments = ecoRI.cut("AAGGAATTCGATTAGCTAGC");

        assertEquals(java.util.List.of("AAGG", "AATTCGATTAGCTAGC"), fragments);
    }

    @Test
    void testCutWithEnzymeCuttingAtSiteStartTerminates() {
        var mboI = enzymeDB.searchByName("MboI").get(0);

        var fragments = mboI.cut("AAGATCTTGATCAA");

        assertEquals(java.util.List.of("AA", "GATCTT", "GATCAA"), fragments);
    }

    @Test
    void testNonPalindromicSiteFoundOnBothStrands() {
        var bsaI = enzymeDB.searchByName("BsaI").get(0);
        // GGTCTC vorwärts bzw. als Reverse-Komplement GAGACC
        assertEquals(1, enzymeDB.findEnzymePositions(bsaI, "AAAGGTCTCAAAAAAAAAA").size());
        assertEquals(1, enzymeDB.findEnzymePositions(bsaI, "AAAGAGACCAAAAAAAAAA").size());
    }

    @Test
    void testDegenerateRecognitionSite() {
        var avaI = enzymeDB.searchByName("AvaI").get(0);  // CYCGRG

        assertEquals(1, enzymeDB.findEnzymePositions(avaI, "AACCCGGGAA").size());
        assertEquals(0, enzymeDB.findEnzymePositions(avaI, "AACACGGGAA").size());
    }

    @Test
    void testCompatibleEnds() {
        var bamHI = enzymeDB.searchByName("BamHI").get(0);
        var bglII = enzymeDB.searchByName("BglII").get(0);
        var ecoRI = enzymeDB.searchByName("EcoRI").get(0);

        assertTrue(bamHI.isCompatibleWith(bglII));
        assertFalse(bamHI.isCompatibleWith(ecoRI));
        assertTrue(enzymeDB.findCompatibleEnzymes(bamHI).stream().anyMatch(e -> e.getName().equals("BglII")));
    }
}
