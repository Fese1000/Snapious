package com.snapious.cloning;

import com.snapious.core.Sequence;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class CloningMethodTest {

    private Sequence makeSeq(String id, String bases) {
        return new Sequence(id, id, bases, Sequence.SequenceType.DNA);
    }

    @Test
    void testCreateCloningPlanRestrictionLigase() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.RESTRICTION_LIGASE, plan.getMethod());
        assertNotNull(plan.getProtocol());
    }

    @Test
    void testCreateCloningPlanGateway() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GATEWAY);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.GATEWAY, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanGibsonAssembly() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GIBSON_ASSEMBLY);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.GIBSON_ASSEMBLY, plan.getMethod());
        assertTrue(plan.getFragmentsOrOverlaps().size() >= 2);  // Mindestens 2 Overlaps
    }

    @Test
    void testCreateCloningPlanGoldenGate() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GOLDEN_GATE);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.GOLDEN_GATE, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanSLIC() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.SLIC);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.SLIC, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanInFusion() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.IN_FUSION);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.IN_FUSION, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanTOPO() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.TOPO_TOPOLOGICAL);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.TOPO_TOPOLOGICAL, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanPCRBased() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.PCR_BASED);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.PCR_BASED, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanBAC() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.BAC_CLONING);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.BAC_CLONING, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanYAC() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.YAC_CLONING);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.YAC_CLONING, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanCRISPR() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.CRISPR_KLONIERUNG);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.CRISPR_KLONIERUNG, plan.getMethod());
    }

    @Test
    void testCreateCloningPlanCassette() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.CASSETTE_KLONIERUNG);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.CASSETTE_KLONIERUNG, plan.getMethod());
    }

    @Test
    void testCloningPlanGetInsert() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        assertEquals("insert", plan.getInsert().getId());
        assertEquals("vector", plan.getVector().getId());
    }

    @Test
    void testCloningPlanGetProtocol() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GIBSON_ASSEMBLY);

        String protocol = plan.getProtocol();
        assertNotNull(protocol);
        assertTrue(protocol.contains("Gibson"));
    }

    @Test
    void testCloningPlanGenerateProtocol() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        String protocol = plan.generateProtocol();
        assertNotNull(protocol);
        assertTrue(protocol.contains("Restriktionsligase"));
    }

    @Test
    void testCloningPlanToString() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GOLDEN_GATE);

        String str = plan.toString();
        assertNotNull(str);
        assertTrue(str.contains("Golden Gate"));
    }

    @Test
    void testGetCloningSummary() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var summary = CloningMethod.getCloningSummary(insert, vector);

        assertTrue(summary.size() > 0);  // Sollte alle Klonierungstypen enthalten
    }

    @Test
    void testAllCloningTypesExist() {
        CloningMethod.CloningType[] types = CloningMethod.CloningType.values();

        assertEquals(12, types.length);  // Alle 12 Typen vorhanden
    }

    @Test
    void testCloningTypeDescriptions() {
        CloningMethod.CloningType[] types = CloningMethod.CloningType.values();

        for (CloningMethod.CloningType type : types) {
            String desc = type.getDescription();
            assertNotNull(desc);
            assertFalse(desc.isEmpty());
        }
    }

    @Test
    void testShortInsertCloningPlans() {
        Sequence insert = makeSeq("short", "ATCG");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        // Alle Klonierungstypen sollten mit kurzen Inserts funktionieren
        for (CloningMethod.CloningType type : CloningMethod.CloningType.values()) {
            var plan = CloningMethod.createCloningPlan(insert, vector, type);
            assertNotNull(plan, "Failed for type: " + type);
        }
    }

    @Test
    void testEmptyInsertCloningPlans() {
        Sequence insert = makeSeq("empty", "");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        // Leeres Insert sollte keine Exceptions werfen
        for (CloningMethod.CloningType type : CloningMethod.CloningType.values()) {
            var plan = CloningMethod.createCloningPlan(insert, vector, type);
            assertNotNull(plan, "Failed for type: " + type);
        }
    }

    @Test
    void testLargeInsertPerformance() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 5000; i++) {
            sb.append("ATCG");
        }

        Sequence insert = makeSeq("large", sb.toString());
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        long startTime = System.currentTimeMillis();
        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GIBSON_ASSEMBLY);
        long elapsed = System.currentTimeMillis() - startTime;

        assertTrue(elapsed < 5000, "Sollte innerhalb von 5s.finishen");
        assertNotNull(plan);
    }

    @Test
    void testGibsonAssemblyOverlaps() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GIBSON_ASSEMBLY);

        var overlaps = plan.getFragmentsOrOverlaps();
        assertEquals(4, overlaps.size());  // Insert5end, Insert3end, Vector5end, Vector3end
    }

    @Test
    void testSLICOverlaps() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.SLIC);

        var overlaps = plan.getFragmentsOrOverlaps();
        assertEquals(2, overlaps.size());  // Insert5end, Insert3end
    }

    @Test
    void testInFusionOverlaps() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.IN_FUSION);

        var overlaps = plan.getFragmentsOrOverlaps();
        assertEquals(2, overlaps.size());  // Insert5end, Insert3end
    }

    @Test
    void testRestrictionLigationFragments() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        // Fragments sollten populated sein wenn Enzyme schneiden
        assertNotNull(plan.getFragmentsOrOverlaps());
    }

    @Test
    void testVectorOnlyCutSites() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        // Vector sollte mehr Schnittstellen haben als Insert
        assertTrue(plan.getVectorCutSites().size() >= plan.getInsertCutSites().size());
    }

    @Test
    void testGoldenGateTypeIISites() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GOLDEN_GATE);

        // Golden Gate sollte Type IIS Enzyme finden
        assertNotNull(plan);
    }

    @Test
    void testGatewayAttSites() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GATEWAY);

        // Gateway benötigt keine Restriktionsstellen
        assertTrue(plan.getInsertCutSites().isEmpty());
        assertTrue(plan.getVectorCutSites().isEmpty());
    }

    @Test
    void testTOPOAttSite() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.TOPO_TOPOLOGICAL);

        // TOPO benötigt CCCTT Sequenz
        assertNotNull(plan);
    }

    @Test
    void testPCRBasedOverlaps() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.PCR_BASED);

        var overlaps = plan.getFragmentsOrOverlaps();
        assertEquals(2, overlaps.size());  // Insert5end, Insert3end
    }

    @Test
    void testBACCloningPlan() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.BAC_CLONING);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.BAC_CLONING, plan.getMethod());
    }

    @Test
    void testYACCloningPlan() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.YAC_CLONING);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.YAC_CLONING, plan.getMethod());
    }

    @Test
    void testCRISPRCloningPlan() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.CRISPR_KLONIERUNG);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.CRISPR_KLONIERUNG, plan.getMethod());
    }

    @Test
    void testCassetteCloningPlan() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.CASSETTE_KLONIERUNG);

        assertNotNull(plan);
        assertEquals(CloningMethod.CloningType.CASSETTE_KLONIERUNG, plan.getMethod());
    }

    @Test
    void testCloningPlanGetters() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        assertNotNull(plan.getMethod());
        assertNotNull(plan.getInsert());
        assertNotNull(plan.getVector());
        assertNotNull(plan.getInsertCutSites());
        assertNotNull(plan.getVectorCutSites());
        assertNotNull(plan.getFragmentsOrOverlaps());
        assertNotNull(plan.getProtocol());
    }

    @Test
    void testGetCloningSummaryReturnsAllTypes() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var summary = CloningMethod.getCloningSummary(insert, vector);

        assertEquals(12, summary.size());  // Alle 12 Klonierungstypen
    }

    @Test
    void testEmptyVectorCloningPlans() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("empty", "");

        // Leerer Vektor sollte keine Exceptions werfen
        for (CloningMethod.CloningType type : CloningMethod.CloningType.values()) {
            var plan = CloningMethod.createCloningPlan(insert, vector, type);
            assertNotNull(plan, "Failed for type: " + type);
        }
    }

    @Test
    void testBothEmptySequences() {
        Sequence insert = makeSeq("empty", "");
        Sequence vector = makeSeq("empty", "");

        // Beide leer sollten keine Exceptions werfen
        for (CloningMethod.CloningType type : CloningMethod.CloningType.values()) {
            var plan = CloningMethod.createCloningPlan(insert, vector, type);
            assertNotNull(plan, "Failed for type: " + type);
        }
    }

    @Test
    void testLargeVectorPerformance() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 10000; i++) {
            sb.append("ATCG");
        }

        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("large", sb.toString());

        long startTime = System.currentTimeMillis();
        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);
        long elapsed = System.currentTimeMillis() - startTime;

        assertTrue(elapsed < 10000, "Sollte innerhalb von 10s.finishen");
        assertNotNull(plan);
    }

    @Test
    void testProtocolContainsMethodDescription() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.GIBSON_ASSEMBLY);

        String protocol = plan.getProtocol();
        assertTrue(protocol.contains("Gibson"));
        assertTrue(protocol.contains("Overlap"));
    }

    @Test
    void testRestrictionLigationProtocolContainsEnzymes() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        var plan = CloningMethod.createCloningPlan(insert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        String protocol = plan.generateProtocol();
        assertTrue(protocol.contains("Schnittstellen"));
    }

    @Test
    void testAllMethodsHaveProtocol() {
        Sequence insert = makeSeq("insert", "AAGGAATTCGATTAGCTAGC");
        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        for (CloningMethod.CloningType type : CloningMethod.CloningType.values()) {
            var plan = CloningMethod.createCloningPlan(insert, vector, type);
            assertNotNull(plan.getProtocol());
            assertFalse(plan.getProtocol().isEmpty());
        }
    }

    @Test
    void testCLoningMethodEnumValues() {
        CloningMethod.CloningType[] types = CloningMethod.CloningType.values();

        String[] expectedNames = {"RESTRICTION_LIGASE", "GATEWAY", "GIBSON_ASSEMBLY",
            "GOLDEN_GATE", "SLIC", "IN_FUSION", "TOPO_TOPOLOGICAL", "PCR_BASED",
            "BAC_CLONING", "YAC_CLONING", "CRISPR_KLONIERUNG", "CASSETTE_KLONIERUNG"};

        for (int i = 0; i < expectedNames.length; i++) {
            assertEquals(expectedNames[i], types[i].name());
        }
    }

    @Test
    void testCloningPlanWithDifferentInsertTypes() {
        Sequence dnaInsert = makeSeq("dna", "AAGGAATTCGATTAGCTAGC");
        Sequence rnaInsert = new Sequence("rna", "RNA", "AAGAAUUCGAUUAGCUAGC",
            Sequence.SequenceType.RNA);

        Sequence vector = makeSeq("vector", "GGATCCGCGGCCGCCTCGAG");

        // DNA und RNA sollten beide funktionieren
        var dnaPlan = CloningMethod.createCloningPlan(dnaInsert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        var rnaPlan = CloningMethod.createCloningPlan(rnaInsert, vector,
            CloningMethod.CloningType.RESTRICTION_LIGASE);

        assertNotNull(dnaPlan);
        assertNotNull(rnaPlan);
    }
}
