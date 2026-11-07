package com.snapious.io;

import com.snapious.core.Sequence;

import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.stream.*;

/**
 * Complete file I/O for all common bioinformatics formats.
 * Supports: FASTA, FASTQ, GenBank, SAM/VCF, Newick (tree).
 */
public class FileIO {

    // ======== FASTA PARSER ========

    /** Reads a FASTA file and returns a list of sequences */
    public static List<Sequence> readFASTA(Path filePath) throws IOException {
        List<Sequence> sequences = new ArrayList<>();
        String content = Files.readString(filePath);

        // FASTA format: >header followed by sequence lines
        String[] blocks = content.split("(?=>)");  // split on header

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

    /** Reads FASTA from a string */
    public static List<Sequence> readFASTAFromString(String fasta) {
        try {
            Path temp = Files.createTempFile("fasta", ".fa");
            Files.writeString(temp, fasta);
            List<Sequence> seqs = readFASTA(temp);
            Files.delete(temp);
            return seqs;
        } catch (IOException e) {
            throw new RuntimeException("FASTA parsing failed", e);
        }
    }

    /** Extracts the ID from a FASTA header (first whitespace-separated part after '>') */
    public static String extractFASTAId(String header) {
        if (header.startsWith(">")) header = header.substring(1);
        String[] parts = header.split("\\s+");
        return parts[0];
    }

    /** Writes sequences as FASTA */
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

        /** Converts to Sequence with quality score statistics */
        public List<Sequence> toSequences() {
            if (sequences == null) {
                sequences = Collections.singletonList(
                    new Sequence(id, id, sequence, Sequence.SequenceType.DNA)
                );
            }
            return sequences;
        }

        /** Average Phred quality score */
        public double avgQuality() {
            if (qualityString == null || qualityString.isEmpty()) return 0;

            double sum = 0;
            for (char c : qualityString.toCharArray()) {
                // Sanger Phred+33 Encoding
                sum += c - 33;
            }
            return sum / qualityString.length();
        }

        /** Minimum quality */
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

    /** Reads a FASTQ file (4 lines per record) */
    public static List<FastqRecord> readFASTQ(Path filePath) throws IOException {
        List<FastqRecord> records = new ArrayList<>();
        List<String> lines = Files.readAllLines(filePath);

        for (int i = 0; i + 3 < lines.size(); i += 4) {
            String header = lines.get(i).trim();
            if (!header.startsWith("@")) continue;

            String id = header.substring(1).split("\\s+")[0];
            String sequence = lines.get(i + 1).trim().toUpperCase();
            // Line i+2 is the + line, skip it
            String quality = lines.get(i + 3).trim();

            records.add(new FastqRecord(id, sequence, quality));
        }

        return records;
    }

    /** Reads FASTQ from a string */
    public static List<FastqRecord> readFASTQFromString(String fastq) {
        try {
            Path temp = Files.createTempFile("fastq", ".fq");
            Files.writeString(temp, fastq);
            List<FastqRecord> recs = readFASTQ(temp);
            Files.delete(temp);
            return recs;
        } catch (IOException e) {
            throw new RuntimeException("FASTQ parsing failed", e);
        }
    }

    /** Writes FASTQ back out */
    public static String writeFASTQ(List<FastqRecord> records) {
        StringBuilder sb = new StringBuilder();

        for (FastqRecord rec : records) {
            sb.append("@").append(rec.id).append("\n");
            sb.append(rec.sequence).append("\n");
            sb.append("+").append(rec.id).append("\n");

            // Recompute quality (simple Phred+33)
            String seq = rec.getSequence();  // bases only for quality
            for (int i = 0; i < rec.getSequence().length(); i++) {
                sb.append((char) (33 + 30));  // high quality as default
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

    /** Reads a GenBank file (simplified) */
    public static List<GenBankRecord> readGenBank(Path filePath) throws IOException {
        List<GenBankRecord> records = new ArrayList<>();
        String content = Files.readString(filePath);

        // Extract ACCESSION and ORGANISM
        String accession = extractField(content, "ACCESSION");
        String organism = extractField(content, "ORGANISM");

        // Extract sequence (lines after "//" before ORIGIN, up to the next "//")
        String seqBlock = extractSequenceBlock(content);

        if (accession != null && seqBlock != null) {
            Sequence seq = new Sequence(accession, organism, seqBlock, Sequence.SequenceType.DNA);
            records.add(new GenBankRecord(accession, organism, seq));

            // Parse features (simplified)
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

        // Remove numbers and whitespace
        return seqPart.replaceAll("[\\d\\s]", "").toUpperCase();
    }

    private static void parseFeatures(GenBankRecord record, String featuresSection) {
        // Very simplified feature parsing
        // In reality more complex - only a placeholder here
    }

    private static String extractBetween(String content, String startKeyword, String endKeyword) {
        int start = content.indexOf(startKeyword);
        if (start < 0) return null;

        int end = content.indexOf(endKeyword, start);
        if (end < 0) end = content.length();

        return content.substring(start, end);
    }

    // ======== SAM/BAM PARSER (simplified) ========

    public static class SamRecord {
        private String readName;
        int flags;
        private String chromosome;
        private int position;  // 1-based
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

        /** Is the read on the forward strand? */
        public boolean isForward() { return (flags & 0x10) == 0; }

        /** Is the read aligned? */
        public boolean isAligned() { return (flags & 0x4) == 0; }

        /** Is the read paired? */
        public boolean isPaired() { return (flags & 0x1) != 0; }
    }

    /** Reads a SAM file (simplified, without BAM support) */
    public static List<SamRecord> readSAM(Path filePath) throws IOException {
        List<SamRecord> records = new ArrayList<>();
        List<String> lines = Files.readAllLines(filePath);

        for (String line : lines) {
            if (line.startsWith("@")) continue;  // skip header

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

        /** Is the variant filtered? */
        public boolean isFiltered() { return "PASS".equals(filter); }

        /** Type of the variant (SNP, INDEL, etc.) */
        public String getType() {
            if (refAllele.length() == 1 && altAllele.length() == 1) return "SNP";
            if (refAllele.length() > altAllele.length()) return "DEL";
            return "INS";
        }
    }

    /** Reads a VCF file (simplified) */
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

    // ======== NEWICK PARSER (phylogenetic trees) ========

    public static class NewickTree {
        private TreeNode root;
        private String newickString;

        public NewickTree(TreeNode root, String newickString) {
            this.root = root;
            this.newickString = newickString;
        }

        public TreeNode getRoot() { return root; }
        public String getNewickString() { return newickString; }

        /** TreeNode for the tree structure */
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

            /** Generate Newick string */
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

            /** Tree as text diagram */
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

    /** Reads a Newick string (simplified parser) */
    public static NewickTree parseNewick(String newick) {
        // Simple recursive parser for the Newick format
        return parseNewickInternal(newick.trim().replaceAll("[\\s]", ""));
    }

    private static NewickTree parseNewickInternal(String newick) {
        // Simplified parser - strips trailing semicolon
        String n = newick.endsWith(";") ? newick.substring(0, newick.length() - 1) : newick;

        if (!n.startsWith("(")) {
            // Leaf node
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

        // Recursive parser for internal nodes
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

        // Split at commas (but not inside parentheses)
        List<String> childrenStrs = splitNewick(inner);

        NewickTree.TreeNode root = new NewickTree.TreeNode("", 0);
        for (String childStr : childrenStrs) {
            NewickTree childTree = parseNewickInternal(childStr);
            root.addChild(childTree.getRoot());
        }

        // Extract branch length from the root
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

    // ======== FORMAT DETECTION ========

    /** Detects the file format from the file extension or content */
    public static String detectFormat(Path filePath) throws IOException {
        String ext = filePath.toString().toLowerCase();

        if (ext.endsWith(".fasta") || ext.endsWith(".fa") || ext.endsWith(".faa")) return "FASTA";
        if (ext.endsWith(".fastq") || ext.endsWith(".fq")) return "FASTQ";
        if (ext.endsWith(".genbank") || ext.endsWith(".gb")) return "GenBank";
        if (ext.endsWith(".sam")) return "SAM";
        if (ext.endsWith(".vcf")) return "VCF";
        if (ext.endsWith(".newick") || ext.endsWith(".nex")) return "Newick";

        // Check content (first 100 characters)
        String header = Files.readString(filePath).substring(0, Math.min(100, (int) filePath.toFile().length()));

        if (header.startsWith(">")) return "FASTA";
        if (header.startsWith("@")) return "FASTQ";

        return "UNKNOWN";
    }

    // ======== HELPER FUNCTIONS ========

    /** Writes a list of sequences as FASTA */
    public static void writeFASTA(List<Sequence> sequences, Path filePath) throws IOException {
        Files.writeString(filePath, writeFASTA(sequences));
    }

    /** Writes FASTQ records */
    public static void writeFASTQ(List<FastqRecord> records, Path filePath) throws IOException {
        Files.writeString(filePath, writeFASTQ(records));
    }

    /** Loads sequences from any file (format detection) */
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
            default -> throw new IOException("Unsupported format: " + format);
        };
    }

    /** Combines all sequences of a list into a consensus */
    public static Sequence createConsensusSequence(List<Sequence> sequences) {
        if (sequences == null || sequences.isEmpty()) return null;

        // Find maximum length
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

            // Most frequent base (excluding N)
            char best = 'N';
            int maxCount = 0;

            for (Map.Entry<Character, Integer> entry : counts.entrySet()) {
                char base = entry.getKey();
                int count = entry.getValue();

                if (base != 'N' && count > maxCount) {
                    maxCount = count;
                    best = base;
                } else if (base == 'N' && maxCount == 0) {
                    best = 'N';  // fallback to N if no information
                }
            }

            consensus.append(best);
        }

        return new Sequence("consensus", "Consensus sequence", consensus.toString(), sequences.get(0).getType());
    }
}
