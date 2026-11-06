package com.snapious.io;

import com.snapious.core.Sequence;

import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.stream.*;

/**
 * Kompletter File-I/O für alle gängigen Bioinformatik-Formate.
 * Unterstützt: FASTA, FASTQ, GenBank, SAM/VCF, Newick (Baum).
 */
public class FileIO {

    // ======== FASTA PARSER ========

    /** Liest eine FASTA-Datei und gibt Liste von Sequenzen zurück */
    public static List<Sequence> readFASTA(Path filePath) throws IOException {
        List<Sequence> sequences = new ArrayList<>();
        String content = Files.readString(filePath);

        // FASTA-Format: >Header gefolgt von Sequenzzeilen
        String[] blocks = content.split("(?=>)");  // Split auf Header

        for (String block : blocks) {
            if (block.trim().isEmpty()) continue;

            String[] lines = block.split("\n", 2);
            if (lines.length < 2) continue;

            String header = lines[0].trim();
            if (!header.startsWith(">")) continue;

            String id = extractFASTAId(header);
            sequences.add(Sequence.fromFASTA(id, header, lines[1]));
        }

        return sequences;
    }

    /** Liest FASTA aus String */
    public static List<Sequence> readFASTAFromString(String fasta) {
        try {
            Path temp = Files.createTempFile("fasta", ".fa");
            Files.writeString(temp, fasta);
            List<Sequence> seqs = readFASTA(temp);
            Files.delete(temp);
            return seqs;
        } catch (IOException e) {
            throw new RuntimeException("FASTA Parsing fehlgeschlagen", e);
        }
    }

    /** Extrahiert ID aus FASTA-Header (erster whitespace-getrennte Teil nach '>') */
    public static String extractFASTAId(String header) {
        if (header.startsWith(">")) header = header.substring(1);
        String[] parts = header.split("\\s+");
        return parts[0];
    }

    /** Schreibt Sequenzen als FASTA */
    public static String writeFASTA(List<Sequence> sequences) {
        StringBuilder sb = new StringBuilder();
        for (Sequence seq : sequences) {
            sb.append(Sequence.toFASTA(seq));
        }
        return sb.toString();
    }

    // ======== FASTQ PARSER ========

    public static class FastqRecord {
        private String id;
        private String sequence;
        private String qualityString;
        private List<Sequence> sequences;

        public FastqRecord(String id, String sequence, String qualityString) {
            this.id = id;
            this.sequence = sequence.toUpperCase();
            this.qualityString = qualityString;
        }

        public String getId() { return id; }
        public String getSequence() { return sequence; }
        public String getQualityString() { return qualityString; }

        /** Konvertiert zu Sequence mit Qualitätsscore-Statistik */
        public List<Sequence> toSequences() {
            if (sequences == null) {
                sequences = Collections.singletonList(
                    new Sequence(id, id, sequence, Sequence.SequenceType.DNA)
                );
            }
            return sequences;
        }

        /** Durchschnittlicher Phred-Qualitätsscore */
        public double avgQuality() {
            if (qualityString == null || qualityString.isEmpty()) return 0;

            double sum = 0;
            for (char c : qualityString.toCharArray()) {
                // Sanger Phred+33 Encoding
                sum += c - 33;
            }
            return sum / qualityString.length();
        }

        /** Mindest-Qualität */
        public byte minQuality() {
            if (qualityString == null || qualityString.isEmpty()) return 0;

            byte min = Byte.MAX_VALUE;
            for (char c : qualityString.toCharArray()) {
                byte q = (byte) (c - 33);
                if (q < min) min = q;
            }
            return min;
        }
    }

    /** Liest FASTQ-Datei (4-Zeilen pro Record) */
    public static List<FastqRecord> readFASTQ(Path filePath) throws IOException {
        List<FastqRecord> records = new ArrayList<>();
        List<String> lines = Files.readAllLines(filePath);

        for (int i = 0; i + 3 < lines.size(); i += 4) {
            String header = lines.get(i).trim();
            if (!header.startsWith("@")) continue;

            String id = header.substring(1).split("\\s+")[0];
            String sequence = lines.get(i + 1).trim().toUpperCase();
            // Zeile i+2 ist +Info, überspringen
            String quality = lines.get(i + 3).trim();

            records.add(new FastqRecord(id, sequence, quality));
        }

        return records;
    }

    /** Liest FASTQ aus String */
    public static List<FastqRecord> readFASTQFromString(String fastq) {
        try {
            Path temp = Files.createTempFile("fastq", ".fq");
            Files.writeString(temp, fastq);
            List<FastqRecord> recs = readFASTQ(temp);
            Files.delete(temp);
            return recs;
        } catch (IOException e) {
            throw new RuntimeException("FASTQ Parsing fehlgeschlagen", e);
        }
    }

    /** Schreibt FASTQ zurück */
    public static String writeFASTQ(List<FastqRecord> records) {
        StringBuilder sb = new StringBuilder();

        for (FastqRecord rec : records) {
            sb.append("@").append(rec.id).append("\n");
            sb.append(rec.sequence).append("\n");
            sb.append("+").append(rec.id).append("\n");

            // Qualität neu berechnen (einfache Phred-33)
            String seq = rec.getSequence();  // Nur Basen für Qualität
            for (int i = 0; i < rec.getSequence().length(); i++) {
                sb.append((char) (33 + 30));  // Hohe Qualität als Default
            }
            sb.append("\n");
        }

        return sb.toString();
    }

    // ======== GENBANK PARSER ========

    public static class GenBankRecord {
        private String accession;
        private String organism;
        private Sequence sequence;
        private Map<String, Object> features = new LinkedHashMap<>();

        public GenBankRecord(String accession, String organism, Sequence sequence) {
            this.accession = accession;
            this.organism = organism;
            this.sequence = sequence;
        }

        public String getAccession() { return accession; }
        public String getOrganism() { return organism; }
        public Sequence getSequence() { return sequence; }

        public void addFeature(String type, int start, int end, String details) {
            features.put(type + "_" + start, Map.of("start", start, "end", end, "details", details));
        }

        public Map<String, Object> getFeatures() { return features; }
    }

    /** Liest GenBank-Datei (vereinfacht) */
    public static List<GenBankRecord> readGenBank(Path filePath) throws IOException {
        List<GenBankRecord> records = new ArrayList<>();
        String content = Files.readString(filePath);

        // ACCESSION und ORGANISM extrahieren
        String accession = extractField(content, "ACCESSION");
        String organism = extractField(content, "ORGANISM");

        // Sequenz extrahieren (Zeilen nach "//" vor ORIGIN, bis nächste "//")
        String seqBlock = extractSequenceBlock(content);

        if (accession != null && seqBlock != null) {
            Sequence seq = new Sequence(accession, organism, seqBlock, Sequence.SequenceType.DNA);
            records.add(new GenBankRecord(accession, organism, seq));

            // Features parsen (vereinfacht)
            String featuresSection = extractBetween(content, "features", "ORIGIN");
            if (featuresSection != null) {
                GenBankRecord record = records.get(0);
                parseFeatures(record, featuresSection);
            }
        }

        return records;
    }

    private static String extractField(String content, String fieldName) {
        int idx = content.indexOf(fieldName);
        if (idx < 0) return null;

        String line = content.substring(idx).split("\n")[0].trim();
        return line.replace(fieldName, "").trim();
    }

    private static String extractSequenceBlock(String content) {
        int originIdx = content.indexOf("ORIGIN");
        if (originIdx < 0) return null;

        String seqPart = content.substring(originIdx);
        int endIdx = seqPart.indexOf("//");
        if (endIdx > 0) seqPart = seqPart.substring(0, endIdx);

        // Nummern und Leerzeichen entfernen
        return seqPart.replaceAll("[\\d\\s]", "").toUpperCase();
    }

    private static void parseFeatures(GenBankRecord record, String featuresSection) {
        // Sehr vereinfachtes Feature-Parsing
        // In Realität komplexer - hier nur Platzhalter
    }

    private static String extractBetween(String content, String startKeyword, String endKeyword) {
        int start = content.indexOf(startKeyword);
        if (start < 0) return null;

        int end = content.indexOf(endKeyword, start);
        if (end < 0) end = content.length();

        return content.substring(start, end);
    }

    // ======== SAM/BAM PARSER (vereinfacht) ========

    public static class SamRecord {
        private String readName;
        int flags;
        private String chromosome;
        private int position;  // 1-basiert
        private int mappingQuality;
        private String sequence;
        private String quality;

        public SamRecord(String readName, int flags, String chromosome,
                        int position, int mappingQuality, String sequence, String quality) {
            this.readName = readName;
            this.flags = flags;
            this.chromosome = chromosome;
            this.position = position;
            this.mappingQuality = mappingQuality;
            this.sequence = sequence.toUpperCase();
            this.quality = quality;
        }

        public String getReadName() { return readName; }
        public int getFlags() { return flags; }
        public String getChromosome() { return chromosome; }
        public int getPosition() { return position; }
        public String getSequence() { return sequence; }

        /** Ist die Read auf der forward-Strang? */
        public boolean isForward() { return (flags & 0x10) == 0; }

        /** Ist die Read alignet? */
        public boolean isAligned() { return (flags & 0x4) == 0; }

        /** Ist die Read paart? */
        public boolean isPaired() { return (flags & 0x1) != 0; }
    }

    /** Liest SAM-Datei (vereinfacht, ohne BAM-Support) */
    public static List<SamRecord> readSAM(Path filePath) throws IOException {
        List<SamRecord> records = new ArrayList<>();
        List<String> lines = Files.readAllLines(filePath);

        for (String line : lines) {
            if (line.startsWith("@")) continue;  // Header überspringen

            String[] fields = line.split("\t");
            if (fields.length < 11) continue;

            String readName = fields[0];
            int flags = Integer.parseInt(fields[1]);
            String chromosome = fields[2];
            int position = Integer.parseInt(fields[3]);
            int mappingQuality = Integer.parseInt(fields[4]);
            String sequence = fields[9];
            String quality = fields[10];

            records.add(new SamRecord(readName, flags, chromosome,
                    position, mappingQuality, sequence, quality));
        }

        return records;
    }

    // ======== VCF PARSER (Variant Call Format) ========

    public static class VariantCall {
        private String chromosome;
        private int position;
        private String id;
        private String refAllele;
        private String altAllele;
        private double quality;
        private String filter;

        public VariantCall(String chromosome, int position, String id,
                          String refAllele, String altAllele, double quality, String filter) {
            this.chromosome = chromosome;
            this.position = position;
            this.id = id;
            this.refAllele = refAllele;
            this.altAllele = altAllele;
            this.quality = quality;
            this.filter = filter;
        }

        public String getChromosome() { return chromosome; }
        public int getPosition() { return position; }
        public String getId() { return id; }
        public String getRefAllele() { return refAllele; }
        public String getAltAllele() { return altAllele; }
        public double getQuality() { return quality; }
        public String getFilter() { return filter; }

        /** Ist die Variante filtered? */
        public boolean isFiltered() { return "PASS".equals(filter); }

        /** Typ der Variante (SNP, INDEL, etc.) */
        public String getType() {
            if (refAllele.length() == 1 && altAllele.length() == 1) return "SNP";
            if (refAllele.length() > altAllele.length()) return "DEL";
            return "INS";
        }
    }

    /** Liest VCF-Datei (vereinfacht) */
    public static List<VariantCall> readVCF(Path filePath) throws IOException {
        List<VariantCall> variants = new ArrayList<>();
        List<String> lines = Files.readAllLines(filePath);

        for (String line : lines) {
            if (line.startsWith("#") || line.trim().isEmpty()) continue;

            String[] fields = line.split("\t");
            if (fields.length < 8) continue;

            String chromosome = fields[0];
            int position = Integer.parseInt(fields[1]);
            String id = fields[2].equals(".") ? "" : fields[2];
            String refAllele = fields[3].toUpperCase();
            String altAllele = fields[4].toUpperCase();
            double quality = fields[5].equals(".") ? 0 : Double.parseDouble(fields[5]);
            String filter = fields[6];

            variants.add(new VariantCall(chromosome, position, id,
                    refAllele, altAllele, quality, filter));
        }

        return variants;
    }

    // ======== NEWICK PARSER (Phylogenetische Bäume) ========

    public static class NewickTree {
        private TreeNode root;
        private String newickString;

        public NewickTree(TreeNode root, String newickString) {
            this.root = root;
            this.newickString = newickString;
        }

        public TreeNode getRoot() { return root; }
        public String getNewickString() { return newickString; }

        /** TreeNode für Baum-Struktur */
        public static class TreeNode {
            private String name;
            private double branchLength;
            private List<TreeNode> children = new ArrayList<>();

            public TreeNode(String name, double branchLength) {
                this.name = name;
                this.branchLength = branchLength;
            }

            public void addChild(TreeNode child) { children.add(child); }

            public String getName() { return name; }
            public double getBranchLength() { return branchLength; }
            public List<TreeNode> getChildren() { return children; }

            /** Newick-String generieren */
            public String toNewick() {
                if (children.isEmpty()) return name + (branchLength > 0 ? ":" + branchLength : "");

                StringBuilder sb = new StringBuilder();
                sb.append("(");
                for (int i = 0; i < children.size(); i++) {
                    if (i > 0) sb.append(",");
                    sb.append(children.get(i).toNewick());
                }
                sb.append(")");

                if (branchLength > 0) {
                    sb.append(name).append(":").append(String.format("%.4f", branchLength));
                }

                return sb.toString();
            }

            /** Baum als Text-Diagramm */
            public String toTextDiagram(int depth) {
                StringBuilder sb = new StringBuilder();

                for (int i = 0; i < depth; i++) sb.append("    ");
                sb.append(name);

                if (branchLength > 0) {
                    sb.append(String.format(" (%.2f)", branchLength));
                }

                sb.append("\n");

                for (TreeNode child : children) {
                    sb.append(child.toTextDiagram(depth + 1));
                }

                return sb.toString();
            }
        }
    }

    /** Liest Newick-String (vereinfachter Parser) */
    public static NewickTree parseNewick(String newick) {
        // Einfacher rekursiver Parser für Newick-Format
        return parseNewickInternal(newick.trim().replaceAll("[\\s]", ""));
    }

    private static NewickTree parseNewickInternal(String newick) {
        // Vereinfachter Parser - entfernt trailing Semikolon
        String n = newick.endsWith(";") ? newick.substring(0, newick.length() - 1) : newick;

        if (!n.startsWith("(")) {
            // Blatt-Knoten
            String name = n;
            double branchLength = 0;

            int colonIdx = n.indexOf(':');
            if (colonIdx > 0) {
                name = n.substring(0, colonIdx);
                try {
                    branchLength = Double.parseDouble(n.substring(colonIdx + 1));
                } catch (NumberFormatException e) {
                    branchLength = 0;
                }
            }

            return new NewickTree(new NewickTree.TreeNode(name, branchLength), n);
        }

        // Rekursiver Parser für interne Knoten
        int depth = 0;
        int closeIdx = -1;

        for (int i = 0; i < n.length(); i++) {
            char c = n.charAt(i);
            if (c == '(') depth++;
            else if (c == ')') {
                depth--;
                if (depth == 0) {
                    closeIdx = i;
                    break;
                }
            }
        }

        String inner = n.substring(1, closeIdx);
        String afterClose = closeIdx < n.length() - 1 ? n.substring(closeIdx + 1) : "";

        // Aufteilen an Kommas (aber nicht in Klammern)
        List<String> childrenStrs = splitNewick(inner);

        NewickTree.TreeNode root = new NewickTree.TreeNode("", 0);
        for (String childStr : childrenStrs) {
            NewickTree childTree = parseNewickInternal(childStr);
            root.addChild(childTree.getRoot());
        }

        // Branch-Length vom Root extrahieren
        double branchLength = 0;
        if (afterClose.startsWith(":")) {
            try {
                branchLength = Double.parseDouble(afterClose.substring(1));
            } catch (NumberFormatException e) {
                branchLength = 0;
            }
        }

        String treeStr = root.toNewick();
        return new NewickTree(root, treeStr);
    }

    private static List<String> splitNewick(String inner) {
        List<String> parts = new ArrayList<>();
        int depth = 0;
        StringBuilder current = new StringBuilder();

        for (char c : inner.toCharArray()) {
            if (c == '(') depth++;
            else if (c == ')') depth--;

            if (c == ',' && depth == 0) {
                parts.add(current.toString());
                current = new StringBuilder();
            } else {
                current.append(c);
            }
        }

        if (current.length() > 0) {
            parts.add(current.toString());
        }

        return parts;
    }

    // ======== FORMAT-ERKENNUNG ========

    /** Erkennt Dateiformat anhand Dateiendung oder Inhalt */
    public static String detectFormat(Path filePath) throws IOException {
        String ext = filePath.toString().toLowerCase();

        if (ext.endsWith(".fasta") || ext.endsWith(".fa") || ext.endsWith(".faa")) return "FASTA";
        if (ext.endsWith(".fastq") || ext.endsWith(".fq")) return "FASTQ";
        if (ext.endsWith(".genbank") || ext.endsWith(".gb")) return "GenBank";
        if (ext.endsWith(".sam")) return "SAM";
        if (ext.endsWith(".vcf")) return "VCF";
        if (ext.endsWith(".newick") || ext.endsWith(".nex")) return "Newick";

        // Inhalt prüfen (erste 100 Zeichen)
        String header = Files.readString(filePath).substring(0, Math.min(100, (int) filePath.toFile().length()));

        if (header.startsWith(">")) return "FASTA";
        if (header.startsWith("@")) return "FASTQ";

        return "UNKNOWN";
    }

    // ======== HILFSFUNKTIONEN ========

    /** Schreibt Liste von Sequenzen als FASTA */
    public static void writeFASTA(List<Sequence> sequences, Path filePath) throws IOException {
        Files.writeString(filePath, writeFASTA(sequences));
    }

    /** Schreibt FASTQ-Records */
    public static void writeFASTQ(List<FastqRecord> records, Path filePath) throws IOException {
        Files.writeString(filePath, writeFASTQ(records));
    }

    /** Lädt Sequenzen aus beliebiger Datei (Format-Erkennung) */
    public static List<Sequence> loadSequences(Path filePath) throws IOException {
        String format = detectFormat(filePath);

        return switch (format) {
            case "FASTA" -> readFASTA(filePath);
            case "FASTQ" -> readFASTQ(filePath).stream()
                    .flatMap(r -> r.toSequences().stream())
                    .collect(Collectors.toList());
            case "GenBank" -> readGenBank(filePath).stream()
                    .map(GenBankRecord::getSequence)
                    .collect(Collectors.toList());
            default -> throw new IOException("Nicht unterstütztes Format: " + format);
        };
    }

    /** Kombiniert alle Sequenzen einer Liste zu einem Konsensus */
    public static Sequence createConsensusSequence(List<Sequence> sequences) {
        if (sequences == null || sequences.isEmpty()) return null;

        // Finde maximale Länge
        int maxLen = sequences.stream().mapToInt(Sequence::length).max().orElse(0);

        StringBuilder consensus = new StringBuilder(maxLen);
        for (int i = 0; i < maxLen; i++) {
            Map<Character, Integer> counts = new HashMap<>();

            for (Sequence seq : sequences) {
                if (i < seq.length()) {
                    char c = seq.baseAt(i);
                    counts.put(c, counts.getOrDefault(c, 0) + 1);
                }
            }

            // Häufigste Base (ohne N)
            char best = 'N';
            int maxCount = 0;

            for (Map.Entry<Character, Integer> entry : counts.entrySet()) {
                char base = entry.getKey();
                int count = entry.getValue();

                if (base != 'N' && count > maxCount) {
                    maxCount = count;
                    best = base;
                } else if (base == 'N' && maxCount == 0) {
                    best = 'N';  // Fallback auf N wenn keine Info
                }
            }

            consensus.append(best);
        }

        return new Sequence("consensus", "Konsensus-Sequenz", consensus.toString(), sequences.get(0).getType());
    }
}
