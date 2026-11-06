package com.snapious.core;

/**
 * Repräsentiert eine biologische Sequenz mit allen grundlegenden Operationen.
 * Unterstützt DNA, RNA und Proteine mit Validierung und Transformation.
 */
public class Sequence {
    private String id;
    private String name;
    private String description;
    private SequenceType type;
    private String bases;  // Großbuchstaben, valide basen
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
        if (type == null) throw new IllegalArgumentException("Sequenztyp darf nicht null sein");
        this.id = id != null ? id : "";
        this.name = name != null ? name : "Unnamed";
        this.type = type;
        this.bases = validate(bases, type);
        this.description = "";
        this.timestamp = System.currentTimeMillis();
    }

    /** Kurzform: Name wird auch als ID verwendet. */
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
                throw new IllegalArgumentException("Ungültige Base '" + c + "' für " + type);
            }
        }
        return upper;
    }

    // ======== BASIC SEQUENCE OPERATIONS ========

    /** Länge der Sequenz */
    public int length() { return bases.length(); }

    /** Alias für {@link #length()} */
    public int getLength() { return bases.length(); }

    /** Basis an Position (0-basiert) */
    public char baseAt(int index) {
        if (index < 0 || index >= bases.length())
            throw new IndexOutOfBoundsException("Index " + index + " bei Länge " + bases.length());
        return bases.charAt(index);
    }

    /** Subsequenz von start (inkl.) bis end (exkl.) */
    public Sequence subsequence(int start, int end) {
        if (start < 0 || end > bases.length())
            throw new IndexOutOfBoundsException("Bereich " + start + "-" + end + " außerhalb 0-" + bases.length());
        if (start > end)
            throw new IllegalArgumentException("Ungültiger Bereich: " + start + "-" + end);
        return new Sequence(id, name + "[sub:" + start + "-" + end + "]",
                bases.substring(start, end), type);
    }

    /** Komplementäre Sequenz */
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
        return new Sequence(id, name + " (komplementär)", sb.toString(), type);
    }

    /** Reverse-Komplement */
    public Sequence reverseComplement() {
        Sequence comp = complement();
        String rc = new StringBuilder(comp.bases).reverse().toString();
        return new Sequence(id, name + " (rev-kom)", rc, type);
    }

    /** Transkription DNA → RNA */
    public Sequence transcribe() {
        if (type != SequenceType.DNA) throw new IllegalStateException("Nur DNA transkribierbar");
        String rna = bases.replace('T', 'U');
        return new Sequence(id, name + " (transkribiert)", rna, SequenceType.RNA);
    }

    /** Rücktranskription RNA → DNA */
    public Sequence toDNA() {
        if (type != SequenceType.RNA) throw new IllegalStateException("Nur RNA rücktranskribierbar");
        return new Sequence(id, name + " (cDNA)", toDnaAlphabet(bases), SequenceType.DNA);
    }

    /**
     * Translation (DNA oder RNA) im ersten Leserahmen → Protein.
     * Stopcodons werden als '*' ausgegeben, unvollständige Schlusscodons ignoriert.
     */
    public Sequence translate() {
        requireNucleotide();
        String dna = toDnaAlphabet(bases);
        StringBuilder protein = new StringBuilder(dna.length() / 3);
        for (int i = 0; i + 2 < dna.length(); i += 3) {
            protein.append(codonToAminoAcid(dna.substring(i, i + 3)));
        }
        return new Sequence(id, name + " (translatiert)", protein.toString(), SequenceType.PROTEIN);
    }

    private void requireNucleotide() {
        if (type == SequenceType.PROTEIN) {
            throw new IllegalStateException("Operation nur für DNA/RNA verfügbar");
        }
    }

    private static String toDnaAlphabet(String s) {
        return s.replace('U', 'T');
    }

    /** Standard-Codontabelle (DNA-Alphabet); unbekannte Codons (z. B. mit N) ergeben 'X'. */
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

    /** GC-Gehalt (0.0 - 1.0) */
    public double gcContent() {
        if (bases.isEmpty()) return 0.0;
        long gc = bases.chars().filter(c -> c == 'G' || c == 'C').count();
        return (double) gc / bases.length();
    }

    /** AT-Gehalt für DNA/RNA */
    public double atContent() {
        if (bases.isEmpty()) return 0.0;
        long at = bases.chars().filter(c -> c == 'A' || c == 'T' || c == 'U').count();
        return (double) at / bases.length();
    }

    /** Häufigkeit einzelner Basen */
    public java.util.Map<Character, Double> baseFrequencies() {
        java.util.Map<Character, Double> freq = new java.util.HashMap<>();
        for (char c : "ATCGUN".toCharArray()) {
            long count = bases.chars().filter(ch -> ch == c).count();
            freq.put(c, bases.isEmpty() ? 0.0 : (double) count / bases.length());
        }
        return freq;
    }

    /**
     * Offene Leserahmen (ATG bis Stopcodon oder Sequenzende) in den drei Vorwärts-Rahmen.
     * Verschachtelte Startcodons innerhalb eines ORFs werden nicht separat gemeldet.
     *
     * @param minLength Mindestlänge in Codons (ohne Stopcodon)
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

    /** Restriktionsstellen finden (vereinfacht) */
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

        // Zeilenumbruch alle 60 Zeichen
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
