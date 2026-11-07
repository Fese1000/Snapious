package com.snapious.core;

/**
 * Represents a biological sequence with all basic operations.
 * Supports DNA, RNA and proteins with validation and transformation.
 */
public class Sequence {
    private String id;
    private String name;
    private String description;
    private SequenceType type;
    private String bases;  // uppercase, valid bases
    private long timestamp;

    public enum SequenceType {
        DNA("ATCG"),
        RNA("AUCG"),
        PROTEIN("ACDEFGHIKLMNPQRSTVWY");

        private final String allowedChars;
        SequenceType(String allowed) { this.allowedChars = allowed; }
        public String getAllowedChars() { return allowedChars; }
    }

    public Sequence(String id, String name, String bases, SequenceType type) {
        if (type == null) throw new IllegalArgumentException("Sequence type must not be null");
        this.id = id != null ? id : "";
        this.name = name != null ? name : "Unnamed";
        this.type = type;
        this.bases = validate(bases, type);
        this.description = "";
        this.timestamp = System.currentTimeMillis();
    }

    /** Short form: the name is also used as the ID. */
    public Sequence(String bases, String name, SequenceType type) {
        this(name, name, bases, type);
    }

    private static String validate(String seq, SequenceType type) {
        if (seq == null || seq.isEmpty()) return "";
        String upper = seq.toUpperCase().replaceAll("\\s", "");
        if (type != SequenceType.PROTEIN) upper = upper.replaceAll("\\d", "");
        String allowed = type.allowedChars;

        for (char c : upper.toCharArray()) {
            boolean ok = allowed.indexOf(c) >= 0
                    || (type != SequenceType.PROTEIN && c == 'N')
                    || (type == SequenceType.PROTEIN && (c == 'X' || c == '*'));
            if (!ok) {
                throw new IllegalArgumentException("Invalid base '" + c + "' for " + type);
            }
        }
        return upper;
    }

    // ======== BASIC SEQUENCE OPERATIONS ========

    /** Length of the sequence */
    public int length() { return bases.length(); }

    /** Alias for {@link #length()} */
    public int getLength() { return bases.length(); }

    /** Base at position (0-based) */
    public char baseAt(int index) {
        if (index < 0 || index >= bases.length())
            throw new IndexOutOfBoundsException("Index " + index + " at length " + bases.length());
        return bases.charAt(index);
    }

    /** Subsequence from start (inclusive) to end (exclusive) */
    public Sequence subsequence(int start, int end) {
        if (start < 0 || end > bases.length())
            throw new IndexOutOfBoundsException("Bereich " + start + "-" + end + " outside 0-" + bases.length());
        if (start > end)
            throw new IllegalArgumentException("Invalid range: " + start + "-" + end);
        return new Sequence(id, name + "[sub:" + start + "-" + end + "]",
                bases.substring(start, end), type);
    }

    /** Complementary sequence */
    public Sequence complement() {
        requireNucleotide();
        StringBuilder sb = new StringBuilder(bases.length());
        for (char c : bases.toCharArray()) {
            switch (c) {
                case 'A': sb.append(type == SequenceType.DNA ? 'T' : 'U'); break;
                case 'T': case 'U': sb.append('A'); break;
                case 'G': sb.append('C'); break;
                case 'C': sb.append('G'); break;
                default: sb.append('N');
            }
        }
        return new Sequence(id, name + " (complement)", sb.toString(), type);
    }

    /** Reverse complement */
    public Sequence reverseComplement() {
        Sequence comp = complement();
        String rc = new StringBuilder(comp.bases).reverse().toString();
        return new Sequence(id, name + " (rev-comp)", rc, type);
    }

    /** Transcription DNA → RNA */
    public Sequence transcribe() {
        if (type != SequenceType.DNA) throw new IllegalStateException("Only DNA can be transcribed");
        String rna = bases.replace('T', 'U');
        return new Sequence(id, name + " (transcribed)", rna, SequenceType.RNA);
    }

    /** Reverse transcription RNA → DNA */
    public Sequence toDNA() {
        if (type != SequenceType.RNA) throw new IllegalStateException("Only RNA can be reverse-transcribed");
        return new Sequence(id, name + " (cDNA)", toDnaAlphabet(bases), SequenceType.DNA);
    }

    /**
     * Translation (DNA or RNA) in the first reading frame → protein.
     * Stop codons are output as '*', incomplete trailing codons are ignored.
     */
    public Sequence translate() {
        requireNucleotide();
        String dna = toDnaAlphabet(bases);
        StringBuilder protein = new StringBuilder(dna.length() / 3);
        for (int i = 0; i + 2 < dna.length(); i += 3) {
            protein.append(codonToAminoAcid(dna.substring(i, i + 3)));
        }
        return new Sequence(id, name + " (translated)", protein.toString(), SequenceType.PROTEIN);
    }

    private void requireNucleotide() {
        if (type == SequenceType.PROTEIN) {
            throw new IllegalStateException("Operation only available for DNA/RNA");
        }
    }

    private static String toDnaAlphabet(String s) {
        return s.replace('U', 'T');
    }

    /** Standard codon table (DNA alphabet); unknown codons (e.g. containing N) yield 'X'. */
    private static char codonToAminoAcid(String codon) {
        return switch (codon) {
            case "TTT", "TTC" -> 'F';
            case "TTA", "TTG", "CTT", "CTC", "CTA", "CTG" -> 'L';
            case "ATT", "ATC", "ATA" -> 'I';
            case "ATG" -> 'M';
            case "GTT", "GTC", "GTA", "GTG" -> 'V';
            case "TCT", "TCC", "TCA", "TCG", "AGT", "AGC" -> 'S';
            case "CCT", "CCC", "CCA", "CCG" -> 'P';
            case "ACT", "ACC", "ACA", "ACG" -> 'T';
            case "GCT", "GCC", "GCA", "GCG" -> 'A';
            case "TAT", "TAC" -> 'Y';
            case "CAT", "CAC" -> 'H';
            case "CAA", "CAG" -> 'Q';
            case "AAT", "AAC" -> 'N';
            case "AAA", "AAG" -> 'K';
            case "GAT", "GAC" -> 'D';
            case "GAA", "GAG" -> 'E';
            case "TGT", "TGC" -> 'C';
            case "TGG" -> 'W';
            case "CGT", "CGC", "CGA", "CGG", "AGA", "AGG" -> 'R';
            case "GGT", "GGC", "GGA", "GGG" -> 'G';
            case "TAA", "TAG", "TGA" -> '*';
            default -> 'X';
        };
    }

    public static Sequence fromFASTA(String id, String header, String sequenceLines) {
        StringBuilder seq = new StringBuilder();
        if (sequenceLines != null) {
            for (String line : sequenceLines.split("\\R")) {
                seq.append(line.trim());
            }
        }
        String bases = seq.toString().toUpperCase();

        SequenceType type;
        if (bases.matches("[ACGTN]*")) {
            type = SequenceType.DNA;
        } else if (bases.matches("[ACGUN]*")) {
            type = SequenceType.RNA;
        } else {
            type = SequenceType.PROTEIN;
        }

        String name = header == null ? null : (header.startsWith(">") ? header.substring(1) : header);
        return new Sequence(id, name, bases, type);
    }

    // ======== STATISTICS ========

    /** GC content (0.0 - 1.0) */
    public double gcContent() {
        if (bases.isEmpty()) return 0.0;
        long gc = bases.chars().filter(c -> c == 'G' || c == 'C').count();
        return (double) gc / bases.length();
    }

    /** AT content for DNA/RNA */
    public double atContent() {
        if (bases.isEmpty()) return 0.0;
        long at = bases.chars().filter(c -> c == 'A' || c == 'T' || c == 'U').count();
        return (double) at / bases.length();
    }

    /** Frequency of individual bases */
    public java.util.Map<Character, Double> baseFrequencies() {
        java.util.Map<Character, Double> freq = new java.util.HashMap<>();
        for (char c : "ATCGUN".toCharArray()) {
            long count = bases.chars().filter(ch -> ch == c).count();
            freq.put(c, bases.isEmpty() ? 0.0 : (double) count / bases.length());
        }
        return freq;
    }

    /**
     * Open reading frames (ATG to stop codon or end of sequence) in the three forward frames.
     * Nested start codons inside an ORF are not reported separately.
     *
     * @param minLength minimum length in codons (excluding the stop codon)
     */
    public java.util.List<Sequence> findORFs(int minLength) {
        requireNucleotide();
        java.util.List<Sequence> orfs = new java.util.ArrayList<>();
        String dna = toDnaAlphabet(bases);

        for (int frame = 0; frame < 3; frame++) {
            int i = frame;
            while (i + 2 < dna.length()) {
                if (!dna.startsWith("ATG", i)) {
                    i += 3;
                    continue;
                }
                int j = i;
                while (j + 2 < dna.length() && codonToAminoAcid(dna.substring(j, j + 3)) != '*') {
                    j += 3;
                }
                int codons = (j - i) / 3;
                if (codons >= minLength) {
                    String orf = dna.substring(i, j);
                    if (type == SequenceType.RNA) orf = orf.replace('T', 'U');
                    orfs.add(new Sequence(id + "_ORF_frame" + (frame + 1) + "_" + i,
                            name + " ORF frame" + (frame + 1) + ":" + i, orf, type));
                }
                i = j + 3;
            }
        }
        return orfs;
    }

    /** Find restriction sites (simplified) */
    public java.util.List<Integer> findRestrictionSites(String motif) {
        java.util.List<Integer> sites = new java.util.ArrayList<>();
        String upper = motif.toUpperCase();
        int idx = 0;
        while ((idx = bases.indexOf(upper, idx)) != -1) {
            sites.add(idx);
            idx++;
        }
        return sites;
    }

    public static String toFASTA(Sequence seq) {
        StringBuilder sb = new StringBuilder();
        sb.append(">").append(seq.id).append(" ").append(seq.name);
        if (!seq.description.isEmpty()) sb.append(" ").append(seq.description);
        sb.append("\n");

        // Line break every 60 characters
        for (int i = 0; i < seq.bases.length(); i += 60) {
            int end = Math.min(i + 60, seq.bases.length());
            sb.append(seq.bases.substring(i, end));
            if (end < seq.bases.length()) sb.append("\n");
        }
        return sb.toString();
    }

    // Getters & Setters
    public String getId() { return id; }
    public void setId(String id) { this.id = id != null ? id : ""; }

    public String getName() { return name; }
    public void setName(String name) { this.name = name; }

    public String getDescription() { return description; }
    public void setDescription(String desc) { this.description = desc != null ? desc : ""; }

    public SequenceType getType() { return type; }
    public String getBases() { return bases; }
    public void setBases(String bases) { this.bases = validate(bases, type); }
    public long getTimestamp() { return timestamp; }

    @Override
    public String toString() {
        String preview = bases.length() > 30 ? bases.substring(0, 30) + "..." : bases;
        return String.format("Sequence[%s, %s, len=%d, type=%s, %s]", id, name, bases.length(), type, preview);
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof Sequence)) return false;
        Sequence s = (Sequence) o;
        return id.equals(s.id) && type == s.type && bases.equals(s.bases);
    }

    @Override
    public int hashCode() { return java.util.Objects.hash(id, type, bases); }
}
