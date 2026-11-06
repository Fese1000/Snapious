package com.snapious.cloning;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Komplettes Klonierungs-Toolkit mit allen gängigen Klonierungsmethoden.
 * Unterstützt: Ligation-basiert, Gateway, Gibson Assembly, Golden Gate,
 * SLIC, In-Fusion, TOPO, PCR-basiert, BAC/YAC Klonierung.
 */
public class CloningMethod {

    // ======== KLONIERUNGSARTEN ==========

    public enum CloningType {
        RESTRICTION_LIGASE("Restriktionsligase-Klonierung"),
        GATEWAY("Gateway Rekombination (AttL/AttR)"),
        GIBSON_ASSEMBLY("Gibson Assembly (Exonuklease, Polymerase, Ligase)"),
        GOLDEN_GATE("Golden Gate (Type IIS Restriktionsenzyme)"),
        SLIC("SLIC (Sequence and Ligation Independent Cloning)"),
        IN_FUSION("In-Fusion Cloning (Homologie-basiert)"),
        TOPO_TOPOLOGICAL("TOPO Cloning (Topoisomerase I)"),
        PCR_BASED("PCR-basierte Klonierung (Overlap-Extension)"),
        BAC_CLONING("BAC/Kosmid-Klonierung (große Inserts)"),
        YAC_CLONING("YAC-Klonierung (Hefe-Chromosomen)"),
        CRISPR_KLONIERUNG("CRISPR/Cas9-basierte Klonierung"),
        CASSETTE_KLONIERUNG("Cassette-Klonierung (multi-site)");

        private final String description;
        CloningType(String desc) { this.description = desc; }
        public String getDescription() { return description; }
    }

    // ======== HAUPTKLASSE FÜR KLONIERUNGSPLANUNG ==========

    /** Erstellt einen vollständigen Klonierungsplan basierend auf Insert und Vektor */
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

    // ======== 1. RESTRIKTIONS-LIGASE KLONIERUNG ==========

    /**
     * Kandidaten sind Enzyme, die den Vektor genau einmal schneiden (Linearisierung).
     * Verdaut wird mit dem ersten Kandidaten, der auch im Insert schneidet.
     */
    private static CloningPlan createRestrictionLigationPlan(Sequence insert, Sequence vector) {
        RestrictionEnzymeDB enzymeDB = new RestrictionEnzymeDB();

        List<RestrictionEnzymeDB.CutSite> insertAll = enzymeDB.findAllCutSites(insert);
        List<RestrictionEnzymeDB.CutSite> vectorAll = enzymeDB.findAllCutSites(vector);

        Map<String, Long> vectorCounts = vectorAll.stream()
            .collect(Collectors.groupingBy(c -> c.getEnzyme().getName(), Collectors.counting()));

        // Enzyme mit genau einer Schnittstelle im Vektor
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
            "Restriktionsligase-Klonierung:\n" +
            "1. Insert und Vektor mit Enzymen verdauen, die den Vektor genau einmal schneiden\n" +
            "2. Für gerichtete Klonierung zwei Enzyme mit unterschiedlichen Überhängen wählen\n" +
            "3. Vektor dephosphorylieren, Fragmente reinigen\n" +
            "4. Mit T4-DNA-Ligase ligieren (16°C über Nacht oder 25°C, 10 Min.)"
        );
    }

    // ======== 2. GATEWAY KLONIERUNG (Att-Recombination) ==========

    private static final String ATT_B1 = "ACAAGTTTGTACAAAAAAGCAGGCT";
    private static final String ATT_B2 = "ACCACTTTGTACAAGAAAGCTGGGT";

    private static CloningPlan createGatewayPlan(Sequence insert, Sequence vector) {
        String bases = insert.getBases();
        List<String> found = new ArrayList<>();
        if (bases.contains(ATT_B1)) found.add("attB1: " + ATT_B1);
        if (bases.contains(ATT_B2)) found.add("attB2: " + ATT_B2);

        String status = found.size() == 2
            ? "Insert trägt bereits attB1 und attB2."
            : "Insert trägt nicht beide attB-Stellen; per PCR ergänzen: 5' attB1 (" + ATT_B1
                + "), 3' attB2 (" + ATT_B2 + ").";

        return new CloningPlan(
            CloningType.GATEWAY,
            insert, vector,
            Collections.emptyList(),  // Keine Restriktionsstellen benötigt
            Collections.emptyList(),
            found,
            "Gateway-Klonierung (Att-Rekombination):\n" + status + "\n" +
            "1. PCR mit attB-Primern\n" +
            "2. BP-Reaktion (attB × attP → attL, Entry-Klon)\n" +
            "3. LR-Reaktion (attL × attR → attB, Expressionsklon)"
        );
    }

    // ======== 3. GIBSON ASSEMBLY ==========

    private static CloningPlan createGibsonAssemblyPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 30;  // 20-40 bp üblich

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
            "Gibson Assembly (Exonuklease + Polymerase + Ligase):\n" +
            "Vorhandener Overlap Vektor→Insert: " + junction1 + " bp, Insert→Vektor: " + junction2 + " bp\n" +
            "1. 20-40 bp Overlap zwischen den Fragmenten über Primer-Überhänge herstellen\n" +
            "2. Vektor linearisieren, Insert per PCR amplifizieren\n" +
            "3. Gibson-Mix, 50°C für 15-60 Min.\n" +
            "4. Transformieren"
        );
    }

    // ======== 4. GOLDEN GATE (Type IIS Enzyme) ==========

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
            "Golden Gate Assembly (Type IIS Enzyme):\n" +
            "1. Enzym wählen (BsaI, BsmBI/Esp3I, BbsI/BpiI, SapI), das nicht intern im Insert schneidet\n" +
            "2. 4-bp-Überhänge über Primer festlegen\n" +
            "3. Verdau und Ligation im selben Ansatz (Thermocycler: 37°C ↔ 16°C)"
        );
    }

    // ======== 5. SLIC ==========

    private static CloningPlan createSLICPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 40;  // 30-60 bp üblich

        return new CloningPlan(
            CloningType.SLIC,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            List.of(head(insert, overlapLength), tail(insert, overlapLength)),
            "SLIC (Sequence and Ligation Independent Cloning):\n" +
            "1. PCR mit 30-60 bp Overlap zum Vektor\n" +
            "2. 3'-Exonuklease-Behandlung (T4-DNA-Polymerase) legt einzelsträngige Overlaps frei\n" +
            "3. Annealing, anschließend transformieren"
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
            "1. Linearisierten Vektor und PCR-Insert mit 15 bp Overlap herstellen\n" +
            "2. In-Fusion-Enzymmix, 50°C, 15 Min.\n" +
            "3. Transformieren"
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
            "1. PCR mit Taq-Polymerase (3'-A-Überhang) bzw. mit CACC-Primer für gerichtetes TOPO\n" +
            "2. Mit linearisiertem TOPO-Vektor mischen (Raumtemperatur, 5 Min.)\n" +
            "3. Transformieren"
        );
    }

    // ======== 8. PCR-BASIERTE KLONIERUNG ==========

    private static CloningPlan createPCRBasedPlan(Sequence insert, Sequence vector) {
        final int overlapLength = 20;

        return new CloningPlan(
            CloningType.PCR_BASED,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            List.of(head(insert, overlapLength), tail(insert, overlapLength)),
            "PCR-basierte Klonierung (Overlap-Extension-PCR):\n" +
            "1. Teilfragmente mit überlappenden Primern amplifizieren\n" +
            "2. Fragmente mischen und per Overlap-Extension zusammenfügen\n" +
            "3. Produkt reinigen und klonieren"
        );
    }

    // ======== 9. BAC/KOSMID KLONIERUNG ==========

    private static CloningPlan createBACCloningPlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.BAC_CLONING,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "BAC-Klonierung (bis ca. 300 kb):\n" +
            "1. Hochmolekulare DNA schonend isolieren, Partialverdau mit z. B. HindIII\n" +
            "2. Größenselektion per Pulsfeld-Gelelektrophorese\n" +
            "3. Ligation in den BAC-Vektor, Elektroporation in E. coli"
        );
    }

    // ======== 10. YAC KLONIERUNG ==========

    private static CloningPlan createYACCloningPlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.YAC_CLONING,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "YAC-Klonierung (bis ca. 2 Mb):\n" +
            "1. YAC-Vektor mit Zentromer (CEN), Replikationsursprung (ARS) und Telomeren\n" +
            "2. Ligation mit hochmolekularer DNA\n" +
            "3. Transformation von Hefe-Sphäroplasten (Saccharomyces cerevisiae)"
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
            "CRISPR/Cas9-basierte Klonierung (Knock-in über HDR):\n" +
            "1. sgRNA (20 nt) neben einer NGG-PAM entwerfen\n" +
            "2. Reparaturvorlage mit 50-100 bp Homologiearmen\n" +
            "3. Cas9 (RNP oder Plasmid), sgRNA und Vorlage einbringen"
        );
    }

    // ======== 12. CASSETTE-KLONIERUNG ==========

    private static CloningPlan createCassettePlan(Sequence insert, Sequence vector) {
        return new CloningPlan(
            CloningType.CASSETTE_KLONIERUNG,
            insert, vector,
            Collections.emptyList(),
            Collections.emptyList(),
            Collections.emptyList(),
            "Cassette-Klonierung (MoClo / Golden-Gate-basiert):\n" +
            "1. Module mit standardisierten 4-bp-Überhängen\n" +
            "2. Mehrere Fragmente in einer Reaktion (BsaI/BpiI)\n" +
            "3. Mehrstufige Assemblierung über Level-0/1/2-Vektoren"
        );
    }

    // ======== HILFSFUNKTIONEN ==========

    private static String head(Sequence seq, int n) {
        String b = seq.getBases();
        return b.substring(0, Math.min(n, b.length()));
    }

    private static String tail(Sequence seq, int n) {
        String b = seq.getBases();
        return b.substring(Math.max(0, b.length() - n));
    }

    /** Länge des längsten Suffixes von {@code left}, das Präfix von {@code right} ist. */
    private static int overlap(String left, String right) {
        for (int len = Math.min(left.length(), right.length()); len > 0; len--) {
            if (left.endsWith(right.substring(0, len))) return len;
        }
        return 0;
    }

    // ======== DATENKLASSEN ==========

    /** Vollständiger Klonierungsplan für Insert und Vektor */
    public static class CloningPlan {
        private final CloningType method;
        private Sequence insert;
        private Sequence vector;
        private List<RestrictionEnzymeDB.CutSite> insertCutSites;
        private List<RestrictionEnzymeDB.CutSite> vectorCutSites;
        private List<String> fragmentsOrOverlaps;  // Fragmente (Ligase) oder Overlaps (Gibson/SLIC)
        private String protocol;  // Schritt-für-Schritt-Anleitung

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

        /** Generiere Schritt-für-Schritt-Anleitung */
        public String generateProtocol() {
            StringBuilder sb = new StringBuilder();

            sb.append(String.format("=== %s Protokoll ===\n\n", method.getDescription()));
            sb.append(protocol).append("\n\n");

            if (!insertCutSites.isEmpty()) {
                sb.append("Verfügbare Schnittstellen im Insert:\n");
                for (RestrictionEnzymeDB.CutSite site : insertCutSites) {
                    sb.append(String.format("  - %s@%d (Überhang: %s)\n",
                        site.getEnzyme().getName(), site.getPosition(),
                        site.getOverhang() == null ? "stumpf" : site.getOverhang()));
                }
            }

            if (!vectorCutSites.isEmpty()) {
                sb.append("Verfügbare Schnittstellen im Vektor:\n");
                for (RestrictionEnzymeDB.CutSite site : vectorCutSites) {
                    sb.append(String.format("  - %s@%d (Überhang: %s)\n",
                        site.getEnzyme().getName(), site.getPosition(),
                        site.getOverhang() == null ? "stumpf" : site.getOverhang()));
                }
            }

            if (!fragmentsOrOverlaps.isEmpty()) {
                sb.append("Fragmente/Overlaps:\n");
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

    // ======== HAUPTFUNKTIONEN FÜR DIE UI ==========

    /** Erstellt einen vollständigen Klonierungsplan für Insert und Vektor */
    public static Map<String, Object> getCloningSummary(Sequence insert, Sequence vector) {
        Map<String, Object> summary = new LinkedHashMap<>();

        // Alle Klonierungsmethoden durchgehen
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
