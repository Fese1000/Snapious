package com.snapious.cloning;

import com.snapious.core.Sequence;

import java.util.*;

/**
 * Datenbank gängiger Restriktionsenzyme (Daten nach NEB/REBASE) mit Sequenzsuche auf beiden
 * Strängen, IUPAC-Mehrdeutigkeitscodes und korrekter Berechnung der Schnittstellen und Überhänge.
 *
 * <p>Schnittstellen werden als Zahl der Basen links des Schnitts auf dem Oberstrang angegeben,
 * relativ zum Beginn der Erkennungssequenz. Für Enzyme vom Typ IIS kann der Schnitt hinter der
 * Erkennungssequenz liegen (Wert &gt; Länge der Erkennungssequenz).
 */
public class RestrictionEnzymeDB {

    private final List<RestrictionEnzyme> enzymes;

    public RestrictionEnzymeDB() {
        this.enzymes = new ArrayList<>();
        initializeDefaultDatabase();
    }

    public void addEnzyme(RestrictionEnzyme enzyme) {
        enzymes.add(enzyme);
    }

    public List<RestrictionEnzyme> getAllEnzymes() {
        return Collections.unmodifiableList(enzymes);
    }

    public List<RestrictionEnzyme> searchByName(String name) {
        String needle = name.toLowerCase();
        return enzymes.stream()
            .filter(e -> e.getName().toLowerCase().contains(needle))
            .toList();
    }

    public List<RestrictionEnzyme> searchByRecognitionSequence(String sequence) {
        String upper = sequence.toUpperCase();
        return enzymes.stream()
            .filter(e -> e.getRecognitionSequence().equals(upper))
            .toList();
    }

    public List<RestrictionEnzyme> findByCutPosition(int position) {
        return enzymes.stream()
            .filter(e -> e.getCutPositionWithinRecognitionSite() == position)
            .toList();
    }

    public List<RestrictionEnzyme> findByType(EnzymeType type) {
        return enzymes.stream()
            .filter(e -> e.getType() == type)
            .toList();
    }

    public List<RestrictionEnzyme> findByBluntEnd() {
        return enzymes.stream().filter(RestrictionEnzyme::isBluntEnd).toList();
    }

    public List<RestrictionEnzyme> findByStickyEnd() {
        return enzymes.stream().filter(RestrictionEnzyme::isStickyEnd).toList();
    }

    /** Isoschizomere: Enzyme mit derselben Erkennungssequenz wie das Enzym namens {@code name}. */
    public List<RestrictionEnzyme> findByIsochore(String name) {
        return enzymes.stream()
            .filter(e -> e.getName().equalsIgnoreCase(name))
            .findFirst()
            .map(this::findIsochore)
            .orElse(Collections.emptyList());
    }

    // ======== STANDARD-DATENBANK ========

    private void add(String name, String site, int top, int bottom, EnzymeType type,
                     String organism, String temperature) {
        addEnzyme(new RestrictionEnzyme(name, site, top, bottom, type, organism, "NEB", temperature, "siehe Hersteller"));
    }

    private void initializeDefaultDatabase() {
        add("EcoRI", "GAATTC", 1, 5, EnzymeType.TYPE_IIP, "E. coli", "37°C");
        add("BamHI", "GGATCC", 1, 5, EnzymeType.TYPE_IIP, "Bacillus amyloliquefaciens", "37°C");
        add("HindIII", "AAGCTT", 1, 5, EnzymeType.TYPE_IIP, "Haemophilus influenzae", "37°C");
        add("NotI", "GCGGCCGC", 2, 6, EnzymeType.TYPE_IIP, "Nocardia otitidis-caviarum", "37°C");
        add("XhoI", "CTCGAG", 1, 5, EnzymeType.TYPE_IIP, "Xanthomonas holcicola", "37°C");
        add("XbaI", "TCTAGA", 1, 5, EnzymeType.TYPE_IIP, "Xanthomonas badrii", "37°C");
        add("SalI", "GTCGAC", 1, 5, EnzymeType.TYPE_IIP, "Streptomyces albus", "37°C");
        add("PstI", "CTGCAG", 5, 1, EnzymeType.TYPE_IIP, "Providencia stuartii", "37°C");
        add("KpnI", "GGTACC", 5, 1, EnzymeType.TYPE_IIP, "Klebsiella pneumoniae", "37°C");
        add("SacI", "GAGCTC", 5, 1, EnzymeType.TYPE_IIP, "Streptomyces achromogenes", "37°C");
        add("SacII", "CCGCGG", 4, 2, EnzymeType.TYPE_IIP, "Streptomyces achromogenes", "37°C");
        add("SmaI", "CCCGGG", 3, 3, EnzymeType.TYPE_IIP, "Serratia marcescens", "25°C");
        add("XmaI", "CCCGGG", 1, 5, EnzymeType.TYPE_IIP, "Xanthomonas malvacearum", "37°C");
        add("NcoI", "CCATGG", 1, 5, EnzymeType.TYPE_IIP, "Nocardia corallina", "37°C");
        add("NdeI", "CATATG", 2, 4, EnzymeType.TYPE_IIP, "Neisseria denitrificans", "37°C");
        add("NheI", "GCTAGC", 1, 5, EnzymeType.TYPE_IIP, "Neisseria mucosa", "37°C");
        add("SpeI", "ACTAGT", 1, 5, EnzymeType.TYPE_IIP, "Sphaerotilus natans", "37°C");
        add("BglII", "AGATCT", 1, 5, EnzymeType.TYPE_IIP, "Bacillus globigii", "37°C");
        add("ClaI", "ATCGAT", 2, 4, EnzymeType.TYPE_IIP, "Caryophanon latum", "37°C");
        add("EcoRV", "GATATC", 3, 3, EnzymeType.TYPE_IIP, "E. coli", "37°C");
        add("SphI", "GCATGC", 5, 1, EnzymeType.TYPE_IIP, "Streptomyces phaeochromogenes", "37°C");
        add("ApaI", "GGGCCC", 5, 1, EnzymeType.TYPE_IIP, "Acetobacter pasteurianus", "25°C");
        add("PvuI", "CGATCG", 4, 2, EnzymeType.TYPE_IIP, "Proteus vulgaris", "37°C");
        add("PvuII", "CAGCTG", 3, 3, EnzymeType.TYPE_IIP, "Proteus vulgaris", "37°C");
        add("HpaI", "GTTAAC", 3, 3, EnzymeType.TYPE_IIP, "Haemophilus parainfluenzae", "37°C");
        add("StuI", "AGGCCT", 3, 3, EnzymeType.TYPE_IIP, "Streptomyces tubercidicus", "37°C");
        add("ScaI", "AGTACT", 3, 3, EnzymeType.TYPE_IIP, "Streptomyces caespitosus", "37°C");
        add("NruI", "TCGCGA", 3, 3, EnzymeType.TYPE_IIP, "Nocardia rubra", "37°C");
        add("SnaBI", "TACGTA", 3, 3, EnzymeType.TYPE_IIP, "Sphaerotilus natans", "37°C");
        add("DraI", "TTTAAA", 3, 3, EnzymeType.TYPE_IIP, "Deinococcus radiodurans", "37°C");
        add("PmeI", "GTTTAAAC", 4, 4, EnzymeType.TYPE_IIP, "Pseudomonas mendocina", "37°C");
        add("SwaI", "ATTTAAAT", 4, 4, EnzymeType.TYPE_IIP, "Staphylococcus warneri", "25°C");
        add("AscI", "GGCGCGCC", 2, 6, EnzymeType.TYPE_IIP, "Arthrobacter species", "37°C");
        add("PacI", "TTAATTAA", 5, 3, EnzymeType.TYPE_IIP, "Pseudomonas alcaligenes", "37°C");
        add("FseI", "GGCCGGCC", 6, 2, EnzymeType.TYPE_IIP, "Frankia species", "37°C");
        add("SbfI", "CCTGCAGG", 6, 2, EnzymeType.TYPE_IIP, "Streptomyces species", "37°C");
        add("AseI", "ATTAAT", 2, 4, EnzymeType.TYPE_IIP, "Aquaspirillum serpens", "37°C");
        add("NsiI", "ATGCAT", 5, 1, EnzymeType.TYPE_IIP, "Neisseria sicca", "37°C");
        add("MluI", "ACGCGT", 1, 5, EnzymeType.TYPE_IIP, "Micrococcus luteus", "37°C");
        add("BssHII", "GCGCGC", 1, 5, EnzymeType.TYPE_IIP, "Bacillus stearothermophilus", "50°C");
        add("AgeI", "ACCGGT", 1, 5, EnzymeType.TYPE_IIP, "Agrobacterium gelatinovorum", "37°C");
        add("BsrGI", "TGTACA", 1, 5, EnzymeType.TYPE_IIP, "Bacillus stearothermophilus", "37°C");
        add("BspEI", "TCCGGA", 1, 5, EnzymeType.TYPE_IIP, "Bacillus species", "37°C");
        add("BclI", "TGATCA", 1, 5, EnzymeType.TYPE_IIP, "Bacillus caldolyticus", "50°C");
        add("AatII", "GACGTC", 5, 1, EnzymeType.TYPE_IIP, "Acetobacter aceti", "37°C");
        add("EagI", "CGGCCG", 1, 5, EnzymeType.TYPE_IIP, "Enterobacter agglomerans", "37°C");
        add("MfeI", "CAATTG", 1, 5, EnzymeType.TYPE_IIP, "Mycoplasma fermentans", "37°C");
        add("BsiWI", "CGTACG", 1, 5, EnzymeType.TYPE_IIP, "Bacillus stearothermophilus", "55°C");
        add("AluI", "AGCT", 2, 2, EnzymeType.TYPE_IIP, "Arthrobacter luteus", "37°C");
        add("HaeIII", "GGCC", 2, 2, EnzymeType.TYPE_IIP, "Haemophilus aegyptius", "37°C");
        add("MspI", "CCGG", 1, 3, EnzymeType.TYPE_IIP, "Moraxella species", "37°C");
        add("HpaII", "CCGG", 1, 3, EnzymeType.TYPE_IIP, "Haemophilus parainfluenzae", "37°C");
        add("TaqI", "TCGA", 1, 3, EnzymeType.TYPE_IIP, "Thermus aquaticus", "65°C");
        add("MboI", "GATC", 0, 4, EnzymeType.TYPE_IIP, "Moraxella bovis", "37°C");
        add("Sau3AI", "GATC", 0, 4, EnzymeType.TYPE_IIP, "Staphylococcus aureus", "37°C");
        add("DpnII", "GATC", 0, 4, EnzymeType.TYPE_IIP, "Diplococcus pneumoniae", "37°C");
        add("RsaI", "GTAC", 2, 2, EnzymeType.TYPE_IIP, "Rhodopseudomonas sphaeroides", "37°C");
        add("MseI", "TTAA", 1, 3, EnzymeType.TYPE_IIP, "Micrococcus species", "37°C");
        add("NlaIII", "CATG", 4, 0, EnzymeType.TYPE_IIP, "Neisseria lactamica", "37°C");
        add("HhaI", "GCGC", 3, 1, EnzymeType.TYPE_IIP, "Haemophilus haemolyticus", "37°C");
        add("CfoI", "GCGC", 3, 1, EnzymeType.TYPE_IIP, "Clostridium formicoaceticum", "37°C");
        add("AvaI", "CYCGRG", 1, 5, EnzymeType.TYPE_IIP, "Anabaena variabilis", "37°C");
        add("HincII", "GTYRAC", 3, 3, EnzymeType.TYPE_IIP, "Haemophilus influenzae", "37°C");
        add("HinfI", "GANTC", 1, 4, EnzymeType.TYPE_IIP, "Haemophilus influenzae", "37°C");
        add("BstNI", "CCWGG", 2, 3, EnzymeType.TYPE_IIP, "Bacillus stearothermophilus", "60°C");
        add("Sau96I", "GGNCC", 1, 4, EnzymeType.TYPE_IIP, "Staphylococcus aureus", "37°C");
        add("AlwNI", "CAGNNNCTG", 6, 3, EnzymeType.TYPE_IIP, "Acinetobacter lwoffii", "37°C");
        add("SfiI", "GGCCNNNNNGGCC", 8, 5, EnzymeType.TYPE_IIP, "Streptomyces fimbriatus", "50°C");
        add("BsaI", "GGTCTC", 7, 11, EnzymeType.TYPE_IIS, "Bacillus stearothermophilus", "37°C");
        add("BsmBI", "CGTCTC", 7, 11, EnzymeType.TYPE_IIS, "Bacillus stearothermophilus", "55°C");
        add("Esp3I", "CGTCTC", 7, 11, EnzymeType.TYPE_IIS, "Erwinia species", "37°C");
        add("BbsI", "GAAGAC", 8, 12, EnzymeType.TYPE_IIS, "Bacillus laterosporus", "37°C");
        add("BpiI", "GAAGAC", 8, 12, EnzymeType.TYPE_IIS, "Bacillus species", "37°C");
        add("SapI", "GCTCTTC", 8, 11, EnzymeType.TYPE_IIS, "Saccharopolyspora species", "37°C");
        add("BspQI", "GCTCTTC", 8, 11, EnzymeType.TYPE_IIS, "Bacillus species", "50°C");
        add("FokI", "GGATG", 14, 18, EnzymeType.TYPE_IIS, "Flavobacterium okeanokoites", "37°C");
        add("BsmAI", "GTCTC", 6, 10, EnzymeType.TYPE_IIS, "Bacillus stearothermophilus", "55°C");
        add("HgaI", "GACGC", 10, 15, EnzymeType.TYPE_IIS, "Haemophilus gallinarum", "37°C");
        add("AarI", "CACCTGC", 11, 15, EnzymeType.TYPE_IIS, "Arthrobacter aurescens", "37°C");
    }

    // ======== ENZYMSUCHFUNKTIONEN FÜR KLONIERUNG ========

    /** Finde alle Schnittstellen aller Enzyme in einer Sequenz (beide Stränge), sortiert nach Position. */
    public List<CutSite> findAllCutSites(Sequence sequence) {
        List<CutSite> allSites = new ArrayList<>();
        String bases = sequence.getBases().toUpperCase().replace('U', 'T');

        for (RestrictionEnzyme enzyme : enzymes) {
            for (RestrictionEnzyme.Match m : enzyme.findMatches(bases)) {
                allSites.add(new CutSite(enzyme, sequence.getId(), m.start(), m.cutTop(),
                        m.forward() ? '+' : '-', overhangAt(bases, m)));
            }
        }

        allSites.sort(Comparator.comparingInt(CutSite::getPosition)
                .thenComparing(c -> c.getEnzyme().getName()));
        return allSites;
    }

    /** Überhang in Oberstrang-Koordinaten (leer bei stumpfen Enden oder Schnitt außerhalb der Sequenz). */
    private static String overhangAt(String bases, RestrictionEnzyme.Match m) {
        int from = Math.min(m.cutTop(), m.cutBottom());
        int to = Math.max(m.cutTop(), m.cutBottom());
        if (from == to || from < 0 || to > bases.length()) return "";
        return bases.substring(from, to);
    }

    /** Positionen (Beginn der Erkennungssequenz, 0-basiert) eines Enzyms auf beiden Strängen. */
    public List<Integer> findEnzymePositions(RestrictionEnzyme enzyme, String sequence) {
        return enzyme.findMatches(sequence.toUpperCase().replace('U', 'T')).stream()
            .map(RestrictionEnzyme.Match::start)
            .toList();
    }

    /** Finde alle Enzyme mit stumpfen Enden */
    public List<RestrictionEnzyme> getBluntEndEnzymes() {
        return findByBluntEnd();
    }

    /** Finde alle Enzyme mit klebrigen Enden (sticky ends) */
    public List<RestrictionEnzyme> getStickyEndEnzymes() {
        return findByStickyEnd();
    }

    /** Finde Enzyme anderer Erkennungssequenz, deren Enden mit denen des Enzyms ligierbar sind. */
    public List<RestrictionEnzyme> findCompatibleEnzymes(RestrictionEnzyme enzyme) {
        return enzymes.stream()
            .filter(e -> !e.getName().equals(enzyme.getName()))
            .filter(e -> e.isCompatibleWith(enzyme))
            .toList();
    }

    /** Finde Isoschizomere (gleiche Erkennungssequenz, anderes Enzym) */
    public List<RestrictionEnzyme> findIsochore(RestrictionEnzyme enzyme) {
        return enzymes.stream()
            .filter(e -> e.getRecognitionSequence().equals(enzyme.getRecognitionSequence()))
            .filter(e -> !e.getName().equals(enzyme.getName()))
            .toList();
    }

    /** Enzyme, deren Aktivität durch Dam-/CpG-Methylierung blockiert werden kann. */
    public List<RestrictionEnzyme> getMethylationSensitive() {
        return enzymes.stream()
            .filter(RestrictionEnzyme::isMethylationSensitive)
            .toList();
    }

    // ======== IUPAC-HILFSFUNKTIONEN ========

    private static final String IUPAC = "ACGTRYKMSWBDHVN";
    private static final String IUPAC_COMPLEMENT = "TGCAYRMKWSVHDBN";

    /** Reverse-Komplement einer Sequenz mit IUPAC-Codes. */
    static String reverseComplement(String seq) {
        StringBuilder sb = new StringBuilder(seq.length());
        for (int i = seq.length() - 1; i >= 0; i--) {
            int idx = IUPAC.indexOf(seq.charAt(i));
            sb.append(idx >= 0 ? IUPAC_COMPLEMENT.charAt(idx) : 'N');
        }
        return sb.toString();
    }

    /** Passt die konkrete Base {@code base} (A/C/G/T/N) auf den IUPAC-Code {@code code}? */
    static boolean matches(char base, char code) {
        if (base == code) return true;
        return switch (code) {
            case 'N' -> true;
            case 'R' -> base == 'A' || base == 'G';
            case 'Y' -> base == 'C' || base == 'T';
            case 'K' -> base == 'G' || base == 'T';
            case 'M' -> base == 'A' || base == 'C';
            case 'S' -> base == 'C' || base == 'G';
            case 'W' -> base == 'A' || base == 'T';
            case 'B' -> base == 'C' || base == 'G' || base == 'T';
            case 'D' -> base == 'A' || base == 'G' || base == 'T';
            case 'H' -> base == 'A' || base == 'C' || base == 'T';
            case 'V' -> base == 'A' || base == 'C' || base == 'G';
            default -> false;
        };
    }

    // ======== DATENKLASSEN ========

    public enum EnzymeType {
        TYPE_I,
        TYPE_II,
        TYPE_IIS,  // schneidet außerhalb der Erkennungssequenz (Golden Gate)
        TYPE_IIP   // palindromische Erkennungssequenz (klassische Restriktionsenzyme)
    }

    /** Repräsentiert ein einzelnes Restriktionsenzym */
    public static class RestrictionEnzyme {

        /** Treffer einer Erkennungssequenz: Beginn, Orientierung und Schnittstellen (Oberstrang-Koordinaten). */
        record Match(int start, boolean forward, int cutTop, int cutBottom) {}

        /** Methylierungsempfindlich (Dam oder CpG) laut NEB. */
        private static final Set<String> METHYLATION_SENSITIVE = Set.of(
            "AscI", "BclI", "BssHII", "ClaI", "HpaII", "MboI", "NotI", "SmaI", "XbaI");

        private final String name;
        private final String recognitionSequence;  // IUPAC, ohne Schnittmarkierung
        private final int cutTop;      // Schnitt auf dem Oberstrang, ab Beginn der Erkennungssequenz
        private final int cutBottom;   // Schnitt auf dem Unterstrang (in Oberstrang-Koordinaten)
        private final EnzymeType type;
        private final String sourceOrganism;
        private final String supplier;
        private final String temperature;
        private final String bufferCondition;
        private final String reverseSite;
        private final boolean palindromic;

        /**
         * @param cutTop    Basen links vom Schnitt im Oberstrang (EcoRI G^AATTC: 1)
         * @param cutBottom Basen links vom Schnitt im Unterstrang, in Oberstrang-Koordinaten (EcoRI: 5)
         */
        public RestrictionEnzyme(String name, String recognitionSequence, int cutTop, int cutBottom,
                                 EnzymeType type, String sourceOrganism, String supplier,
                                 String temperature, String bufferCondition) {
            if (recognitionSequence == null || recognitionSequence.isBlank()) {
                throw new IllegalArgumentException("Erkennungssequenz fehlt für " + name);
            }
            this.name = name;
            this.recognitionSequence = recognitionSequence.toUpperCase().replace("^", "");
            this.cutTop = cutTop;
            this.cutBottom = cutBottom;
            this.type = type;
            this.sourceOrganism = sourceOrganism;
            this.supplier = supplier;
            this.temperature = temperature;
            this.bufferCondition = bufferCondition;
            this.reverseSite = reverseComplement(this.recognitionSequence);
            this.palindromic = this.recognitionSequence.equals(reverseSite);
        }

        /**
         * Kurzform für symmetrische Schnitte: der Unterstrang wird an der gespiegelten Position
         * geschnitten (Länge - cutPosition). {@code stickyEnds=false} erzwingt stumpfe Enden.
         */
        public RestrictionEnzyme(String name, String recognitionSequence, int cutPosition,
                                 EnzymeType type, boolean stickyEnds, String sourceOrganism,
                                 String supplier, String temperature, String bufferCondition) {
            this(name, recognitionSequence, cutPosition,
                 stickyEnds ? recognitionSequence.replace("^", "").length() - cutPosition : cutPosition,
                 type, sourceOrganism, supplier, temperature, bufferCondition);
        }

        public String getName() { return name; }
        public String getRecognitionSequence() { return recognitionSequence; }

        /** Schnittstelle (Oberstrang) relativ zum Beginn der Erkennungssequenz */
        public int getCutPositionWithinRecognitionSite() { return cutTop; }

        public int getBottomStrandCutPosition() { return cutBottom; }

        public EnzymeType getType() { return type; }
        public boolean isBluntEnd() { return cutTop == cutBottom; }
        public boolean isStickyEnd() { return cutTop != cutBottom; }

        /** Überhang überstehende Basen: &gt;0 = 5'-Überhang, &lt;0 = 3'-Überhang, 0 = stumpf */
        public int getOverhangLength() { return cutBottom - cutTop; }

        public String getSourceOrganism() { return sourceOrganism; }
        public String getSupplier() { return supplier; }
        public String getTemperature() { return temperature; }
        public String getBufferCondition() { return bufferCondition; }

        /** Dam-/CpG-Methylierung kann das Enzym blockieren. */
        public boolean isMethylationSensitive() { return METHYLATION_SENSITIVE.contains(name); }

        /**
         * Überhangsequenz (Oberstrang-Richtung) für Enzyme, deren Schnitt innerhalb der
         * Erkennungssequenz liegt und deren Überhang damit festgelegt ist; sonst {@code null}
         * (stumpf, Typ IIS oder Überhang mit N/mehrdeutigen Basen).
         */
        public String getOverhangSequence() {
            if (isBluntEnd()) return null;
            int from = Math.min(cutTop, cutBottom);
            int to = Math.max(cutTop, cutBottom);
            if (to > recognitionSequence.length()) return null;
            String overhang = recognitionSequence.substring(from, to);
            return overhang.chars().allMatch(c -> "ACGT".indexOf(c) >= 0) ? overhang : null;
        }

        /** Alle Treffer in {@code sequence} (Großbuchstaben, T statt U), beide Stränge. */
        List<Match> findMatches(String sequence) {
            List<Match> found = new ArrayList<>();
            int len = recognitionSequence.length();

            for (int i = 0; i + len <= sequence.length(); i++) {
                if (matchesAt(sequence, i, recognitionSequence)) {
                    found.add(new Match(i, true, i + cutTop, i + cutBottom));
                }
                if (!palindromic && matchesAt(sequence, i, reverseSite)) {
                    // Spiegelung: Schnitte relativ zum Ende der Erkennungssequenz
                    found.add(new Match(i, false, i + len - cutBottom, i + len - cutTop));
                }
            }
            return found;
        }

        private static boolean matchesAt(String sequence, int start, String pattern) {
            for (int k = 0; k < pattern.length(); k++) {
                if (!matches(sequence.charAt(start + k), pattern.charAt(k))) return false;
            }
            return true;
        }

        /** Schneidet eine Sequenz an allen Erkennungsstellen (Oberstrang-Fragmente). */
        public List<String> cut(String sequence) {
            String upper = sequence.toUpperCase().replace('U', 'T');

            TreeSet<Integer> cuts = new TreeSet<>();
            for (Match m : findMatches(upper)) {
                if (m.cutTop() > 0 && m.cutTop() < upper.length()) cuts.add(m.cutTop());
            }

            List<String> fragments = new ArrayList<>();
            int last = 0;
            for (int cut : cuts) {
                fragments.add(upper.substring(last, cut));
                last = cut;
            }
            fragments.add(upper.substring(last));
            return fragments;
        }

        /**
         * Ligierbar mit {@code other}: beide stumpf, oder gleiche Überhangsart und komplementäre
         * Überhänge (Reverse-Komplement des einen entspricht dem anderen).
         */
        public boolean isCompatibleWith(RestrictionEnzyme other) {
            if (this.isBluntEnd() || other.isBluntEnd()) {
                return this.isBluntEnd() && other.isBluntEnd();
            }
            if (Integer.signum(getOverhangLength()) != Integer.signum(other.getOverhangLength())) return false;

            String a = getOverhangSequence();
            String b = other.getOverhangSequence();
            return a != null && b != null && reverseComplement(a).equals(b);
        }

        @Override
        public String toString() {
            return String.format("RestrictionEnzyme[%s, %s, cut=%d/%d, blunt=%b]",
                name, recognitionSequence, cutTop, cutBottom, isBluntEnd());
        }

        @Override
        public boolean equals(Object o) {
            if (this == o) return true;
            if (!(o instanceof RestrictionEnzyme that)) return false;
            return name.equals(that.name);
        }

        @Override
        public int hashCode() { return name.hashCode(); }
    }

    /** Eine Schnittstelle eines Enzyms in einer DNA-Sequenz. */
    public static class CutSite {
        private final RestrictionEnzyme enzyme;
        private final String sequenceId;
        private final int position;     // Beginn der Erkennungssequenz, 0-basiert
        private final int cutPosition;  // Basen links vom Schnitt im Oberstrang
        private final char strand;      // '+' oder '-'
        private final String overhang;

        public CutSite(RestrictionEnzyme enzyme, String sequenceId, int position) {
            this(enzyme, sequenceId, position, position + enzyme.getCutPositionWithinRecognitionSite(), '+', "");
        }

        CutSite(RestrictionEnzyme enzyme, String sequenceId, int position, int cutPosition,
                char strand, String overhang) {
            this.enzyme = enzyme;
            this.sequenceId = sequenceId;
            this.position = position;
            this.cutPosition = cutPosition;
            this.strand = strand;
            this.overhang = overhang;
        }

        public RestrictionEnzyme getEnzyme() { return enzyme; }
        public String getSequenceId() { return sequenceId; }

        /** Beginn der Erkennungssequenz (0-basiert) */
        public int getPosition() { return position; }

        /** Anzahl Basen links vom Schnitt im Oberstrang */
        public int getCutPosition() { return cutPosition; }

        /** '+' wenn die Erkennungssequenz so im Oberstrang steht, '-' wenn auf dem Gegenstrang */
        public char getStrand() { return strand; }

        /** Überhangsequenz in Oberstrang-Richtung; {@code null} bei stumpfen Enden. */
        public String getOverhang() { return overhang == null || overhang.isEmpty() ? null : overhang; }

        public boolean isBlunt() { return enzyme.isBluntEnd(); }

        @Override
        public String toString() {
            return String.format("CutSite[%s@%d in %s]", enzyme.getName(), position, sequenceId);
        }
    }
}
