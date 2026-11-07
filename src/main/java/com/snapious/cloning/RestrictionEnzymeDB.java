package com.snapious.cloning;

import com.snapious.core.Sequence;

import java.util.*;

/**
 * Database of common restriction enzymes (data from NEB/REBASE) with sequence search on both
 * strands, IUPAC ambiguity codes and correct computation of cut sites and overhangs.
 *
 * <p>Cut sites are given as the number of bases to the left of the cut on the top strand,
 * relative to the start of the recognition sequence. For type IIS enzymes the cut can lie after the
 * recognition sequence (value &gt; length of the recognition sequence).
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

    /** Isoschizomers: enzymes with the same recognition sequence as the enzyme named {@code name}. */
    public List<RestrictionEnzyme> findByIsochore(String name) {
        return enzymes.stream()
            .filter(e -> e.getName().equalsIgnoreCase(name))
            .findFirst()
            .map(this::findIsochore)
            .orElse(Collections.emptyList());
    }

    // ======== STANDARD DATABASE ========

    private void add(String name, String site, int top, int bottom, EnzymeType type,
                     String organism, String temperature) {
        addEnzyme(new RestrictionEnzyme(name, site, top, bottom, type, organism, "NEB", temperature, "see manufacturer"));
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

    // ======== ENZYME SEARCH FUNCTIONS FOR CLONING ========

    /** Find all cut sites of all enzymes in a sequence (both strands), sorted by position. */
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

    /** Overhang in top-strand coordinates (empty for blunt ends or cuts outside the sequence). */
    private static String overhangAt(String bases, RestrictionEnzyme.Match m) {
        int from = Math.min(m.cutTop(), m.cutBottom());
        int to = Math.max(m.cutTop(), m.cutBottom());
        if (from == to || from < 0 || to > bases.length()) return "";
        return bases.substring(from, to);
    }

    /** Positions (start of the recognition sequence, 0-based) of an enzyme on both strands. */
    public List<Integer> findEnzymePositions(RestrictionEnzyme enzyme, String sequence) {
        return enzyme.findMatches(sequence.toUpperCase().replace('U', 'T')).stream()
            .map(RestrictionEnzyme.Match::start)
            .toList();
    }

    /** Find all enzymes with blunt ends */
    public List<RestrictionEnzyme> getBluntEndEnzymes() {
        return findByBluntEnd();
    }

    /** Find all enzymes with sticky ends */
    public List<RestrictionEnzyme> getStickyEndEnzymes() {
        return findByStickyEnd();
    }

    /** Find enzymes with a different recognition sequence whose ends can be ligated with those of the enzyme. */
    public List<RestrictionEnzyme> findCompatibleEnzymes(RestrictionEnzyme enzyme) {
        return enzymes.stream()
            .filter(e -> !e.getName().equals(enzyme.getName()))
            .filter(e -> e.isCompatibleWith(enzyme))
            .toList();
    }

    /** Find isoschizomers (same recognition sequence, different enzyme) */
    public List<RestrictionEnzyme> findIsochore(RestrictionEnzyme enzyme) {
        return enzymes.stream()
            .filter(e -> e.getRecognitionSequence().equals(enzyme.getRecognitionSequence()))
            .filter(e -> !e.getName().equals(enzyme.getName()))
            .toList();
    }

    /** Enzymes whose activity can be blocked by Dam/CpG methylation. */
    public List<RestrictionEnzyme> getMethylationSensitive() {
        return enzymes.stream()
            .filter(RestrictionEnzyme::isMethylationSensitive)
            .toList();
    }

    // ======== IUPAC HELPER FUNCTIONS ========

    private static final String IUPAC = "ACGTRYKMSWBDHVN";
    private static final String IUPAC_COMPLEMENT = "TGCAYRMKWSVHDBN";

    /** Reverse complement of a sequence with IUPAC codes. */
    static String reverseComplement(String seq) {
        StringBuilder sb = new StringBuilder(seq.length());
        for (int i = seq.length() - 1; i >= 0; i--) {
            int idx = IUPAC.indexOf(seq.charAt(i));
            sb.append(idx >= 0 ? IUPAC_COMPLEMENT.charAt(idx) : 'N');
        }
        return sb.toString();
    }

    /** Does the concrete base {@code base} (A/C/G/T/N) match the IUPAC code {@code code}? */
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

    // ======== DATA CLASSES ========

    public enum EnzymeType {
        TYPE_I,
        TYPE_II,
        TYPE_IIS,  // cuts outside the recognition sequence (Golden Gate)
        TYPE_IIP   // palindromic recognition sequence (classic restriction enzymes)
    }

    /** Represents a single restriction enzyme */
    public static class RestrictionEnzyme {

        /** Hit of a recognition sequence: start, orientation and cut sites (top-strand coordinates). */
        record Match(int start, boolean forward, int cutTop, int cutBottom) {}

        /** Methylation sensitive (Dam or CpG) according to NEB. */
        private static final Set<String> METHYLATION_SENSITIVE = Set.of(
            "AscI", "BclI", "BssHII", "ClaI", "HpaII", "MboI", "NotI", "SmaI", "XbaI");

        private final String name;
        private final String recognitionSequence;  // IUPAC, without cut marker
        private final int cutTop;      // cut on the top strand, from the start of the recognition sequence
        private final int cutBottom;   // cut on the bottom strand (in top-strand coordinates)
        private final EnzymeType type;
        private final String sourceOrganism;
        private final String supplier;
        private final String temperature;
        private final String bufferCondition;
        private final String reverseSite;
        private final boolean palindromic;

        /**
         * @param cutTop    bases to the left of the cut on the top strand (EcoRI G^AATTC: 1)
         * @param cutBottom bases to the left of the cut on the bottom strand, in top-strand coordinates (EcoRI: 5)
         */
        public RestrictionEnzyme(String name, String recognitionSequence, int cutTop, int cutBottom,
                                 EnzymeType type, String sourceOrganism, String supplier,
                                 String temperature, String bufferCondition) {
            if (recognitionSequence == null || recognitionSequence.isBlank()) {
                throw new IllegalArgumentException("Recognition sequence missing for " + name);
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
         * Short form for symmetric cuts: the bottom strand is cut at the mirrored position
         * (length - cutPosition). {@code stickyEnds=false} forces blunt ends.
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

        /** Cut site (top strand) relative to the start of the recognition sequence */
        public int getCutPositionWithinRecognitionSite() { return cutTop; }

        public int getBottomStrandCutPosition() { return cutBottom; }

        public EnzymeType getType() { return type; }
        public boolean isBluntEnd() { return cutTop == cutBottom; }
        public boolean isStickyEnd() { return cutTop != cutBottom; }

        /** Overhang protruding bases: &gt;0 = 5' overhang, &lt;0 = 3' overhang, 0 = blunt */
        public int getOverhangLength() { return cutBottom - cutTop; }

        public String getSourceOrganism() { return sourceOrganism; }
        public String getSupplier() { return supplier; }
        public String getTemperature() { return temperature; }
        public String getBufferCondition() { return bufferCondition; }

        /** Dam/CpG methylation can block the enzyme. */
        public boolean isMethylationSensitive() { return METHYLATION_SENSITIVE.contains(name); }

        /**
         * Overhang sequence (top-strand direction) for enzymes whose cut lies within the
         * recognition sequence and whose overhang is therefore fixed; otherwise {@code null}
         * (blunt, type IIS, or overhang with N/ambiguous bases).
         */
        public String getOverhangSequence() {
            if (isBluntEnd()) return null;
            int from = Math.min(cutTop, cutBottom);
            int to = Math.max(cutTop, cutBottom);
            if (to > recognitionSequence.length()) return null;
            String overhang = recognitionSequence.substring(from, to);
            return overhang.chars().allMatch(c -> "ACGT".indexOf(c) >= 0) ? overhang : null;
        }

        /** All hits in {@code sequence} (uppercase, T instead of U), both strands. */
        List<Match> findMatches(String sequence) {
            List<Match> found = new ArrayList<>();
            int len = recognitionSequence.length();

            for (int i = 0; i + len <= sequence.length(); i++) {
                if (matchesAt(sequence, i, recognitionSequence)) {
                    found.add(new Match(i, true, i + cutTop, i + cutBottom));
                }
                if (!palindromic && matchesAt(sequence, i, reverseSite)) {
                    // Mirroring: cuts relative to the end of the recognition sequence
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

        /** Cuts a sequence at all recognition sites (top-strand fragments). */
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
         * Ligatable with {@code other}: both blunt, or the same overhang type and complementary
         * overhangs (reverse complement of one equals the other).
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

    /** A cut site of an enzyme in a DNA sequence. */
    public static class CutSite {
        private final RestrictionEnzyme enzyme;
        private final String sequenceId;
        private final int position;     // start of the recognition sequence, 0-based
        private final int cutPosition;  // bases to the left of the cut on the top strand
        private final char strand;      // '+' or '-'
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

        /** Start of the recognition sequence (0-based) */
        public int getPosition() { return position; }

        /** Number of bases to the left of the cut on the top strand */
        public int getCutPosition() { return cutPosition; }

        /** '+' if the recognition sequence is on the top strand this way, '-' if on the opposite strand */
        public char getStrand() { return strand; }

        /** Overhang sequence in top-strand direction; {@code null} for blunt ends. */
        public String getOverhang() { return overhang == null || overhang.isEmpty() ? null : overhang; }

        public boolean isBlunt() { return enzyme.isBluntEnd(); }

        @Override
        public String toString() {
            return String.format("CutSite[%s@%d in %s]", enzyme.getName(), position, sequenceId);
        }
    }
}
