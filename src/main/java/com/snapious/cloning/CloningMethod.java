package com.snapious.cloning;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Complete cloning toolkit covering all common cloning methods.
 * Supports: ligation-based, Gateway, Gibson Assembly, Golden Gate,
 * SLIC, In-Fusion, TOPO, PCR-based, BAC/YAC cloning.
 */
public class CloningMethod {

    // ======== CLONING TYPES ==========

    public enum CloningType {
        RESTRICTION_LIGASE("Restriction ligase cloning"),
        GATEWAY("Gateway recombination (AttL/AttR)"),
        GIBSON_ASSEMBLY("Gibson Assembly (Exonuclease, Polymerase, Ligase)"),
        GOLDEN_GATE("Golden Gate (Type IIS restriction enzymes)"),
        SLIC("SLIC (Sequence and Ligation Independent Cloning)"),
        IN_FUSION("In-Fusion Cloning (homology-based)"),
        TOPO_TOPOLOGICAL("TOPO Cloning (Topoisomerase I)"),
        PCR_BASED("PCR-based cloning (Overlap-Extension)"),
        BAC_CLONING("BAC/cosmid cloning (large inserts)"),
        YAC_CLONING("YAC cloning (yeast chromosomes)"),
        CRISPR_KLONIERUNG("CRISPR/Cas9-based cloning"),
        CASSETTE_KLONIERUNG("Cassette cloning (multi-site)");

        private final String description;
        CloningType(String desc) { this.description = desc; }
        public String getDescription() { return description; }
    }

    // ======== MAIN CLASS FOR CLONING PLANNING ==========

    /** Creates a complete cloning plan based on insert and vector */
    public static CloningPlan createCloningPlan(Sequence insert, Sequence vector,
                                               CloningType method) {
        return switch (method) {
            case RESTRICTION_LIGASE -> createRestrictionLigationPlan(insert, vector);
            case GATEWAY -> createGatewayPlan(insert, vector);
            case GIBSON_ASSEMBLY -> createGibsonAssemblyPlan(insert, vector);
            case GOLDEN_GATE -> createGoldenGatePlan(insert, vector);
            case SLIC -> createSLICPlan(insert, vector);
            case IN_FUSION -> createInFusionPlan(insert, vector);
            case TOPO_TOPOLOGICAL -> createTOPOPlan(insert, vector);
            case PCR_BASED -> createPCRBasedPlan(insert, vector);
            case BAC_CLONING -> createBACCloningPlan(insert, vector);
            case YAC_CLONING -> createYACCloningPlan(insert, vector);
            case CRISPR_KLONIERUNG -> createCRISPRPlan(insert, vector);
            case CASSETTE_KLONIERUNG -> createCassettePlan(insert, vector);
        };
    }

    // ======== 1. RESTRICTION LIGASE CLONING ==========

    /**
     * Candidates are enzymes that cut the vector exactly once (linearization).
     * Digestion uses the first candidate that also cuts in the insert.
     */
    private static CloningPlan createRestrictionLigationPlan(Sequence insert, Sequence vector) {
        RestrictionEnzymeDB enzymeDB = new RestrictionEnzymeDB();

        List<RestrictionEnzymeDB.CutSite> insertAll = enzymeDB.findAllCutSites(insert);
        List<RestrictionEnzymeDB.CutSite> vectorAll = enzymeDB.findAllCutSites(vector);

        Map<String, Long> vectorCounts = vectorAll.stream()
            .collect(Collectors.groupingBy(c -> c.getEnzyme().getName(), Collectors.counting()));

        // Enzymes with exactly one cut site in the vector
        Set<String> uniqueVectorCutters = vectorCounts.entrySet().stream()
            .filter(e -> e.getValue() == 1)
            .map(Map.Entry::getKey)
            .collect(Collectors.toSet());

        List<RestrictionEnzymeDB.CutSite> vectorSites = vectorAll.stream()
            .filter(c -> uniqueVectorCutters.contains(c.getEnzyme().getName()))
            .toList();
        List<RestrictionEnzymeDB.CutSite> insertSites = insertAll.stream()
            .filter(c -> uniqueVectorCutters.contains(c.getEnzyme().getName()))
            .toList();

        List<String> fragments = new ArrayList<>();
        insertSites.stream().findFirst().ifPresent(site -> {
            fragments.addAll(site.getEnzyme().cut(insert.getBases()));
            fragments.addAll(site.getEnzyme().cut(vector.getBases()));
        });

        return new CloningPlan(
            CloningType.RESTRICTION_LIGASE,
            insert, vector,
            insertSites,
            vectorSites,
            fragments,
            "Restriction ligase cloning:\n" +
            "1. Digest insert and vector with enzymes that cut the vector exactly once\n" +
            "2. For directional cloning choose two enzymes with different overhangs\n" +
            "3. Dephosphorylate the vector, purify the fragments\n" +
            "4. Ligate with T4 DNA ligase (16°C overnight or 25°C, 10 min)"
        );
    }

    // ======== 2. GATEWAY CLONING (att recombination) ==========

    private static final String ATT_B1 = "ACAAGTTTGTACAAAAAAGCAGGCT";
    private static final String ATT_B2 = "ACCACTTTGTACAAGAAAGCTGGGT";

    private static CloningPlan createGatewayPlan(Sequence insert, Sequence vector) {
        String bases = insert.getBases();
        List<String> found = new ArrayList<>();
        if (bases.contains(ATT_B1)) found.add("attB1: " + ATT_B1);
        if (bases.contains(ATT_B2)) found.add("attB2: " + ATT_B2);

        String status = found.size() == 2
            ? "Insert already carries attB1 and attB2."
            : "Insert does not carry both attB sites; add them by PCR: 5' attB1 (" + ATT_B1
                + "), 3' attB2 (" + ATT_B2 + ").";

        return new CloningPlan(
            CloningType.GATEWAY,
            insert, vector,
            Collections.emptyList(),  // no restriction sites needed
            Collections.emptyList(),
            found,
            "Gateway cloning (att recombination):\n" + status + "\n" +
            "1. PCR with attB primers\n" +
            "2. BP reaction (attB × attP → attL, Entry-Klon)\n" +
            "3. LR reaction (attL × attR → attB, expression clone)"
        );
    }

    // ======== 3. GIBSON ASSEMBLY ==========

    private static CloningPlan createGibsonAssemblyPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 30;  // 20-40 bp is common

        String insert5end = head(insert, overlapLength);
        String insert3end = tail(insert, overlapLength);
        String vector5end = head(vector, overlapLength);
        String vector3end = tail(vector, overlapLength);

        int junction1 = overlap(vector3end, insert5end);
        int junction2 = overlap(insert3end, vector5end);

        return new CloningPlan(
            CloningType.GIBSON_ASSEMBLY,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            List.of(insert5end, insert3end, vector5end, vector3end),
            "Gibson Assembly (Exonuclease + Polymerase + Ligase):\n" +
            "Existing overlap vector→insert: " + junction1 + " bp, insert→vector: " + junction2 + " bp\n" +
            "1. Create a 20-40 bp overlap between the fragments via primer overhangs\n" +
            "2. Linearize the vector, amplify the insert by PCR\n" +
            "3. Gibson mix, 50°C for 15-60 min\n" +
            "4. Transform"
        );
    }

    // ======== 4. GOLDEN GATE (Type IIS enzymes) ==========

    private static CloningPlan createGoldenGatePlan(Sequence insert, Sequence vector) {
        RestrictionEnzymeDB enzymeDB = new RestrictionEnzymeDB();

        List<RestrictionEnzymeDB.CutSite> typeIISInsert = enzymeDB.findAllCutSites(insert).stream()
            .filter(c -> c.getEnzyme().getType() == RestrictionEnzymeDB.EnzymeType.TYPE_IIS)
            .toList();
        List<RestrictionEnzymeDB.CutSite> typeIISVector = enzymeDB.findAllCutSites(vector).stream()
            .filter(c -> c.getEnzyme().getType() == RestrictionEnzymeDB.EnzymeType.TYPE_IIS)
            .toList();

        return new CloningPlan(
            CloningType.GOLDEN_GATE,
            insert, vector,
            typeIISInsert,
            typeIISVector,
            Collections.emptyList(),
            "Golden Gate Assembly (Type IIS enzymes):\n" +
            "1. Choose an enzyme (BsaI, BsmBI/Esp3I, BbsI/BpiI, SapI) that does not cut internally in the insert\n" +
            "2. Define 4-bp overhangs via primers\n" +
            "3. Digestion and ligation in the same reaction (Thermocycler: 37°C ↔ 16°C)"
        );
    }

    // ======== 5. SLIC ==========

    private static CloningPlan createSLICPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 40;  // 30-60 bp is common

        return new CloningPlan(
            CloningType.SLIC,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            List.of(head(insert, overlapLength), tail(insert, overlapLength)),
            "SLIC (Sequence and Ligation Independent Cloning):\n" +
            "1. PCR with 30-60 bp overlap to the vector\n" +
            "2. 3' exonuclease treatment (T4 DNA polymerase) exposes single-stranded overlaps\n" +
            "3. Annealing, then transform"
        );
    }

    // ======== 6. IN-FUSION ==========

    private static CloningPlan createInFusionPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 15;

        return new CloningPlan(
            CloningType.IN_FUSION,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            List.of(head(insert, overlapLength), tail(insert, overlapLength)),
            "In-Fusion Cloning:\n" +
            "1. Prepare linearized vector and PCR insert with a 15 bp overlap\n" +
            "2. In-Fusion enzyme mix, 50°C, 15 min\n" +
            "3. Transform"
        );
    }

    // ======== 7. TOPO ==========

    private static CloningPlan createTOPOPlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.TOPO_TOPOLOGICAL,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "TOPO Cloning (Topoisomerase I):\n" +
            "1. PCR with Taq polymerase (3' A overhang) or with a CACC primer for directional TOPO\n" +
            "2. Mix with linearized TOPO vector (room temperature, 5 min)\n" +
            "3. Transform"
        );
    }

    // ======== 8. PCR-BASED CLONING ==========

    private static CloningPlan createPCRBasedPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 20;

        return new CloningPlan(
            CloningType.PCR_BASED,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            List.of(head(insert, overlapLength), tail(insert, overlapLength)),
            "PCR-based cloning (Overlap-Extension-PCR):\n" +
            "1. Amplify partial fragments with overlapping primers\n" +
            "2. Mix the fragments and join them by overlap extension\n" +
            "3. Purify the product and clone it"
        );
    }

    // ======== 9. BAC/COSMID CLONING ==========

    private static CloningPlan createBACCloningPlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.BAC_CLONING,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "BAC cloning (up to approx. 300 kb):\n" +
            "1. Gently isolate high-molecular-weight DNA, partial digest with e.g. HindIII\n" +
            "2. Size selection by pulsed-field gel electrophoresis\n" +
            "3. Ligation into the BAC vector, electroporation into E. coli"
        );
    }

    // ======== 10. YAC CLONING ==========

    private static CloningPlan createYACCloningPlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.YAC_CLONING,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "YAC cloning (up to approx. 2 Mb):\n" +
            "1. YAC vector with centromere (CEN), replication origin (ARS) and telomeres\n" +
            "2. Ligation with high-molecular-weight DNA\n" +
            "3. Transformation of yeast spheroplasts (Saccharomyces cerevisiae)"
        );
    }

    // ======== 11. CRISPR/CAS9 ==========

    private static CloningPlan createCRISPRPlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.CRISPR_KLONIERUNG,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "CRISPR/Cas9-based cloning (knock-in via HDR):\n" +
            "1. Design an sgRNA (20 nt) next to an NGG PAM\n" +
            "2. Repair template with 50-100 bp homology arms\n" +
            "3. Deliver Cas9 (RNP or plasmid), sgRNA and template"
        );
    }

    // ======== 12. CASSETTE CLONING ==========

    private static CloningPlan createCassettePlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.CASSETTE_KLONIERUNG,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "Cassette cloning (MoClo / Golden-Gate-basiert):\n" +
            "1. Modules with standardized 4-bp overhangs\n" +
            "2. Several fragments in one reaction (BsaI/BpiI)\n" +
            "3. Multi-level assembly via level 0/1/2 vectors"
        );
    }

    // ======== HELPER FUNCTIONS ==========

    private static String head(Sequence seq, int n) {
        String b = seq.getBases();
        return b.substring(0, Math.min(n, b.length()));
    }

    private static String tail(Sequence seq, int n) {
        String b = seq.getBases();
        return b.substring(Math.max(0, b.length() - n));
    }

    /** Length of the longest suffix of {@code left} that is a prefix of {@code right}. */
    private static int overlap(String left, String right) {
        for (int len = Math.min(left.length(), right.length()); len > 0; len--) {
            if (left.endsWith(right.substring(0, len))) return len;
        }
        return 0;
    }

    // ======== DATA CLASSES ==========

    /** Complete cloning plan for insert and vector */
    public static class CloningPlan {
        private final CloningType method;
        private Sequence insert;
        private Sequence vector;
        private List<RestrictionEnzymeDB.CutSite> insertCutSites;
        private List<RestrictionEnzymeDB.CutSite> vectorCutSites;
        private List<String> fragmentsOrOverlaps;  // fragments (ligase) or overlaps (Gibson/SLIC)
        private String protocol;  // step-by-step instructions

        public CloningPlan(CloningType method, Sequence insert, Sequence vector,
                          List<RestrictionEnzymeDB.CutSite> insertCutSites,
                          List<RestrictionEnzymeDB.CutSite> vectorCutSites,
                          List<String> fragmentsOrOverlaps, String protocol) {
            this.method = method;
            this.insert = insert;
            this.vector = vector;
            this.insertCutSites = insertCutSites;
            this.vectorCutSites = vectorCutSites;
            this.fragmentsOrOverlaps = fragmentsOrOverlaps;
            this.protocol = protocol;
        }

        public CloningType getMethod() { return method; }
        public Sequence getInsert() { return insert; }
        public Sequence getVector() { return vector; }

        public List<RestrictionEnzymeDB.CutSite> getInsertCutSites() { return insertCutSites; }
        public List<RestrictionEnzymeDB.CutSite> getVectorCutSites() { return vectorCutSites; }

        public List<String> getFragmentsOrOverlaps() { return fragmentsOrOverlaps; }
        public String getProtocol() { return protocol; }

        /** Generate step-by-step instructions */
        public String generateProtocol() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("=== %s Protocol ===\n\n", method.getDescription()));
            sb.append(protocol).append("\n\n");

            if (!insertCutSites.isEmpty()) {
                sb.append("Available cut sites in the insert:\n");
                for (RestrictionEnzymeDB.CutSite site : insertCutSites) {
                    sb.append(String.format("  - %s@%d (Overhang: %s)\n",
                        site.getEnzyme().getName(), site.getPosition(),
                        site.getOverhang() == null ? "blunt" : site.getOverhang()));
                }
            }

            if (!vectorCutSites.isEmpty()) {
                sb.append("Available cut sites in the vector:\n");
                for (RestrictionEnzymeDB.CutSite site : vectorCutSites) {
                    sb.append(String.format("  - %s@%d (Overhang: %s)\n",
                        site.getEnzyme().getName(), site.getPosition(),
                        site.getOverhang() == null ? "blunt" : site.getOverhang()));
                }
            }

            if (!fragmentsOrOverlaps.isEmpty()) {
                sb.append("Fragments/overlaps:\n");
                for (int i = 0; i < fragmentsOrOverlaps.size(); i++) {
                    sb.append(String.format("  [%d] %s\n", i + 1, fragmentsOrOverlaps.get(i)));
                }
            }

            return sb.toString();
        }

        @Override
        public String toString() {
            return generateProtocol();
        }
    }

    // ======== MAIN FUNCTIONS FOR THE UI ==========

    /** Creates a complete cloning plan for insert and vector */
    public static Map<String, Object> getCloningSummary(Sequence insert, Sequence vector) {
        Map<String, Object> summary = new LinkedHashMap<>();

        // Go through all cloning methods
        for (CloningType method : CloningType.values()) {
            CloningPlan plan = createCloningPlan(insert, vector, method);

            summary.put(method.getDescription(), Map.of(
                "insertCutSites", plan.getInsertCutSites().size(),
                "vectorCutSites", plan.getVectorCutSites().size(),
                "protocol", plan.generateProtocol()
            ));
        }

        return summary;
    }
}
