package com.snapious.alignment;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Multiple sequence alignment (MSA) using a progressive algorithm (ClustalW-like).
 * Builds a guide tree and aligns sequences step by step from the most similar to the most divergent.
 */
public class MultipleSequenceAlignment {

    // ======== MSA RESULT ==========

    /** Represents the result of a multiple sequence alignment */
    public static class MSAResult {
        private List<AlignedSequence> alignedSequences;  // aligned sequences with gaps
        private double averageScore;                     // average pairwise score
        private String consensusSequence;                // consensus sequence
        private Map<String, Double> sequenceWeights;     // sequence weights (gaps %)

        public MSAResult(List<AlignedSequence> alignedSequences, double averageScore,
                        String consensusSequence, Map<String, Double> sequenceWeights) {
            this.alignedSequences = alignedSequences;
            this.averageScore = averageScore;
            this.consensusSequence = consensusSequence;
            this.sequenceWeights = sequenceWeights;
        }

        public List<AlignedSequence> getAlignedSequences() { return alignedSequences; }
        public double getAverageScore() { return averageScore; }
        public String getConsensusSequence() { return consensusSequence; }
        public Map<String, Double> getSequenceWeights() { return sequenceWeights; }

        /** Formatted alignment output */
        public String format() {
            StringBuilder sb = new StringBuilder();

            // Header
            sb.append("=== Multiple Sequence Alignment ===\n\n");
            sb.append(String.format("Sequences: %d, Length: %d bp\n",
                alignedSequences.size(), getAlignmentLength()));
            sb.append(String.format("Average pairwise score: %.1f\n", averageScore));
            sb.append(String.format("Consensus: %s\n\n", consensusSequence));

            // Per-sequence output (in blocks of 60bp)
            int blockSize = 60;
            int numBlocks = (getAlignmentLength() + blockSize - 1) / blockSize;

            for (int block = 0; block < numBlocks; block++) {
                int start = block * blockSize;
                int end = Math.min(start + blockSize, getAlignmentLength());

                // Consensus line
                sb.append(String.format("Consensus %3d-%3d  %s\n",
                    start + 1, end, getConsensusBlock(block, blockSize)));

                // Each sequence
                for (AlignedSequence as : alignedSequences) {
                    sb.append(String.format("%-20s %3d-%3d  %s\n",
                        as.getName(), start + 1, end, as.getSequenceBlock(block, blockSize)));
                }

                sb.append("\n");  // blank line after each block
            }

            return sb.toString();
        }

        /** Length of the alignment */
        public int getAlignmentLength() {
            return alignedSequences.isEmpty() ? 0 : alignedSequences.get(0).getAlignedBases().length();
        }

        /** Consensus block for a range */
        private String getConsensusBlock(int block, int blockSize) {
            int start = block * blockSize;
            int end = Math.min(start + blockSize, getAlignmentLength());

            return consensusSequence.substring(start, end);
        }

        @Override
        public String toString() { return format(); }
    }

    /** Represents an aligned sequence with gaps */
    public static class AlignedSequence {
        private String name;       // sequence name/ID
        private String bases;      // original bases (without gaps)
        private String alignedBases;  // aligned bases (with gaps '-')

        public AlignedSequence(String name, String bases, String alignedBases) {
            this.name = name;
            this.bases = bases.toUpperCase();
            this.alignedBases = alignedBases.toUpperCase();
        }

        public String getName() { return name; }
        public String getBases() { return bases; }
        public String getAlignedBases() { return alignedBases; }

        /** Number of gaps */
        public int gapCount() {
            return (int) alignedBases.chars().filter(c -> c == '-').count();
        }

        /** Gap rate in percent */
        public double gapPercent() {
            int len = alignedBases.length();
            return len > 0 ? (double) gapCount() / len * 100 : 0;
        }

        /** Extract a block (for block-wise output) */
        public String getSequenceBlock(int block, int blockSize) {
            int start = block * blockSize;
            int end = Math.min(start + blockSize, alignedBases.length());

            return alignedBases.substring(start, end);
        }

        @Override
        public String toString() {
            return String.format("AlignedSequence[%s, len=%d, gaps=%.0f%%]",
                name, alignedBases.length(), gapPercent());
        }
    }

    // ======== PARAMETERS ========

    private static final int MATCH = 2;
    private static final int MISMATCH = -1;
    private static final int GAP = -1;

    // ======== GUIDE TREE (UPGMA) ========

    /** Node in the guide tree; leaves carry the index of the input sequence. */
    private static final class GuideTreeNode {
        final int leafIndex;   // >= 0 for leaves, otherwise -1
        final String name;     // leaves only
        final double height;   // distance to the leaves
        final int size;        // number of leaves in the subtree
        final GuideTreeNode left, right;

        GuideTreeNode(int leafIndex, String name) {
            this.leafIndex = leafIndex;
            this.name = name;
            this.height = 0;
            this.size = 1;
            this.left = null;
            this.right = null;
        }

        GuideTreeNode(GuideTreeNode left, GuideTreeNode right, double height) {
            this.leafIndex = -1;
            this.name = null;
            this.height = height;
            this.size = left.size + right.size;
            this.left = left;
            this.right = right;
        }

        boolean isLeaf() { return left == null; }
    }

    /** Rows of a (partial) alignment together with the input-sequence index per row. */
    private record Profile(List<Integer> ids, List<String> rows) {
        int width() { return rows.get(0).length(); }
    }

    // ======== MAIN ALIGNMENT ==========

    /**
     * Progressive multiple sequence alignment (ClustalW-like):
     * pairwise distances from global alignments, UPGMA guide tree, then
     * profile-profile alignment along the tree. The input order is preserved.
     */
    public static MSAResult progressiveAlignment(List<Sequence> sequences) {
        if (sequences == null || sequences.isEmpty()) {
            throw new IllegalArgumentException("At least one sequence required");
        }

        if (sequences.size() == 1) {
            Sequence only = sequences.get(0);
            AlignedSequence single = new AlignedSequence(only.getName(), only.getBases(), only.getBases());
            return new MSAResult(Collections.singletonList(single), 0.0, only.getBases(),
                    Map.of(only.getName(), 1.0));
        }

        GuideTreeNode tree = buildGuideTree(sequences);
        Profile profile = alignSubtree(tree, sequences);

        AlignedSequence[] ordered = new AlignedSequence[sequences.size()];
        for (int r = 0; r < profile.ids().size(); r++) {
            int idx = profile.ids().get(r);
            Sequence s = sequences.get(idx);
            ordered[idx] = new AlignedSequence(s.getName(), s.getBases(), profile.rows().get(r));
        }
        List<AlignedSequence> aligned = Arrays.asList(ordered);

        String consensus = Alignment.createConsensus(
                aligned.stream().map(AlignedSequence::getAlignedBases).toList());

        Map<String, Double> weights = new LinkedHashMap<>();
        for (AlignedSequence as : aligned) {
            weights.put(as.getName(), 1.0 - as.gapPercent() / 100.0);
        }

        return new MSAResult(aligned, calculateAveragePairwiseScore(aligned), consensus, weights);
    }

    // ======== DISTANCES & GUIDE TREE ========

    /** Distance = 1 - identity of the global pairwise alignment (0..1). */
    private static double[][] computeDistanceMatrix(List<Sequence> sequences) {
        int n = sequences.size();
        double[][] dist = new double[n][n];
        Alignment aligner = new Alignment(MATCH, MISMATCH, GAP);

        for (int i = 0; i < n; i++) {
            for (int j = i + 1; j < n; j++) {
                Alignment.AlignResult r = aligner.needlemanWunsch(
                        sequences.get(i).getBases(), sequences.get(j).getBases());
                double identity = Alignment.percentIdentity(r.getSeq1Aln(), r.getSeq2Aln()) / 100.0;
                dist[i][j] = 1.0 - identity;
                dist[j][i] = dist[i][j];
            }
        }
        return dist;
    }

    private static GuideTreeNode buildGuideTree(List<Sequence> sequences) {
        int n = sequences.size();
        double[][] d = computeDistanceMatrix(sequences);
        GuideTreeNode[] node = new GuideTreeNode[n];
        boolean[] active = new boolean[n];
        for (int i = 0; i < n; i++) {
            node[i] = new GuideTreeNode(i, sequences.get(i).getName());
            active[i] = true;
        }

        for (int step = 0; step < n - 1; step++) {
            int minI = -1, minJ = -1;
            double min = Double.MAX_VALUE;
            for (int i = 0; i < n; i++) {
                if (!active[i]) continue;
                for (int j = i + 1; j < n; j++) {
                    if (active[j] && d[i][j] < min) {
                        min = d[i][j];
                        minI = i;
                        minJ = j;
                    }
                }
            }

            double sizeI = node[minI].size;
            double sizeJ = node[minJ].size;
            for (int k = 0; k < n; k++) {
                if (!active[k] || k == minI || k == minJ) continue;
                d[minI][k] = (sizeI * d[minI][k] + sizeJ * d[minJ][k]) / (sizeI + sizeJ);
                d[k][minI] = d[minI][k];
            }
            node[minI] = new GuideTreeNode(node[minI], node[minJ], min / 2.0);
            active[minJ] = false;
        }

        for (int i = 0; i < n; i++) {
            if (active[i]) return node[i];
        }
        throw new IllegalStateException("Guide tree empty");
    }

    // ======== PROGRESSIVE ALIGNMENT ==========

    private static Profile alignSubtree(GuideTreeNode node, List<Sequence> sequences) {
        if (node.isLeaf()) {
            return new Profile(List.of(node.leafIndex), List.of(sequences.get(node.leafIndex).getBases()));
        }
        return mergeProfiles(alignSubtree(node.left, sequences), alignSubtree(node.right, sequences));
    }

    private static int pairScore(char x, char y) {
        if (x == '-' && y == '-') return 0;
        if (x == '-' || y == '-') return GAP;
        return x == y ? MATCH : MISMATCH;
    }

    /** Average pair score of column {@code col} against a pure gap column. */
    private static double gapColumnScore(List<String> rows, int col) {
        double sum = 0;
        for (String row : rows) {
            if (row.charAt(col) != '-') sum += GAP;
        }
        return sum / rows.size();
    }

    private static double columnScore(List<String> a, int i, List<String> b, int j) {
        double sum = 0;
        for (String ra : a) {
            char x = ra.charAt(i);
            for (String rb : b) {
                sum += pairScore(x, rb.charAt(j));
            }
        }
        return sum / ((double) a.size() * b.size());
    }

    /** Profile-profile alignment (Needleman-Wunsch on columns, sum of pairs). */
    private static Profile mergeProfiles(Profile a, Profile b) {
        int la = a.width(), lb = b.width();
        double[][] f = new double[la + 1][lb + 1];
        double[] gapA = new double[la];
        double[] gapB = new double[lb];
        for (int i = 0; i < la; i++) gapA[i] = gapColumnScore(a.rows(), i);
        for (int j = 0; j < lb; j++) gapB[j] = gapColumnScore(b.rows(), j);

        for (int i = 1; i <= la; i++) f[i][0] = f[i - 1][0] + gapA[i - 1];
        for (int j = 1; j <= lb; j++) f[0][j] = f[0][j - 1] + gapB[j - 1];
        for (int i = 1; i <= la; i++) {
            for (int j = 1; j <= lb; j++) {
                double diag = f[i - 1][j - 1] + columnScore(a.rows(), i - 1, b.rows(), j - 1);
                double up = f[i - 1][j] + gapA[i - 1];
                double left = f[i][j - 1] + gapB[j - 1];
                f[i][j] = Math.max(diag, Math.max(up, left));
            }
        }

        List<StringBuilder> outA = new ArrayList<>();
        List<StringBuilder> outB = new ArrayList<>();
        for (int r = 0; r < a.rows().size(); r++) outA.add(new StringBuilder());
        for (int r = 0; r < b.rows().size(); r++) outB.add(new StringBuilder());

        final double eps = 1e-9;
        int i = la, j = lb;
        while (i > 0 || j > 0) {
            if (i > 0 && j > 0
                    && Math.abs(f[i][j] - (f[i - 1][j - 1] + columnScore(a.rows(), i - 1, b.rows(), j - 1))) < eps) {
                appendColumn(outA, a.rows(), i - 1);
                appendColumn(outB, b.rows(), j - 1);
                i--; j--;
            } else if (i > 0 && Math.abs(f[i][j] - (f[i - 1][j] + gapA[i - 1])) < eps) {
                appendColumn(outA, a.rows(), i - 1);
                appendGap(outB);
                i--;
            } else {
                appendGap(outA);
                appendColumn(outB, b.rows(), j - 1);
                j--;
            }
        }

        List<Integer> ids = new ArrayList<>(a.ids());
        ids.addAll(b.ids());
        List<String> rows = new ArrayList<>();
        for (StringBuilder sb : outA) rows.add(sb.reverse().toString());
        for (StringBuilder sb : outB) rows.add(sb.reverse().toString());
        return new Profile(ids, rows);
    }

    private static void appendColumn(List<StringBuilder> out, List<String> rows, int col) {
        for (int r = 0; r < rows.size(); r++) out.get(r).append(rows.get(r).charAt(col));
    }

    private static void appendGap(List<StringBuilder> out) {
        for (StringBuilder sb : out) sb.append('-');
    }

    // ======== SCORE & STATISTICS ==========

    /** Average sum-of-pairs score over all sequence pairs of the alignment. */
    private static double calculateAveragePairwiseScore(List<AlignedSequence> aligned) {
        int n = aligned.size();
        if (n < 2) return 0.0;

        double total = 0;
        int pairs = 0;
        for (int i = 0; i < n; i++) {
            for (int j = i + 1; j < n; j++) {
                String s1 = aligned.get(i).getAlignedBases();
                String s2 = aligned.get(j).getAlignedBases();
                for (int c = 0; c < s1.length(); c++) {
                    total += pairScore(s1.charAt(c), s2.charAt(c));
                }
                pairs++;
            }
        }
        return total / pairs;
    }

    // ======== MAIN FUNCTIONS FOR UI ==========

    /** Generate an MSA report for a list of sequences */
    public static String generateMSAReport(List<Sequence> sequences) {
        try {
            MSAResult result = progressiveAlignment(sequences);

            StringBuilder sb = new StringBuilder();
            sb.append("=== MSA Summary ===\n\n");
            sb.append(String.format("Sequences: %d\n", sequences.size()));
            sb.append(String.format("Alignment length: %d bp\n", result.getAlignmentLength()));
            sb.append(String.format("Average score: %.1f\n", result.getAverageScore()));

            sb.append("\nGap statistics:\n");
            for (AlignedSequence as : result.getAlignedSequences()) {
                sb.append(String.format("  %s: %.0f%% Gaps (%d bp)\n",
                        as.getName(), as.gapPercent(), as.gapCount()));
            }

            String consensusPreview = result.getConsensusSequence();
            if (consensusPreview.length() > 100) {
                consensusPreview = consensusPreview.substring(0, 100) + "...";
            }
            sb.append(String.format("\nConsensus (preview): %s\n", consensusPreview));
            sb.append("\n").append(result.format());
            return sb.toString();

        } catch (IllegalArgumentException e) {
            return "MSA error: " + e.getMessage();
        }
    }

    /** Align sequences and return the result */
    public static MSAResult alignSequences(List<Sequence> sequences) {
        return progressiveAlignment(sequences);
    }

    /** Guide tree of the sequences in Newick format (branch lengths from UPGMA heights). */
    public static String generateNewickFromMSA(List<Sequence> sequences) {
        if (sequences == null || sequences.isEmpty()) {
            throw new IllegalArgumentException("At least one sequence required");
        }
        if (sequences.size() == 1) return newickLabel(sequences.get(0).getName()) + ";";
        GuideTreeNode root = buildGuideTree(sequences);
        return toNewick(root, root.height) + ";";
    }

    private static String toNewick(GuideTreeNode node, double parentHeight) {
        String length = String.format(Locale.ROOT, ":%.4f", Math.max(0, parentHeight - node.height));
        if (node.isLeaf()) return newickLabel(node.name) + length;
        String inner = "(" + toNewick(node.left, node.height) + "," + toNewick(node.right, node.height) + ")";
        return inner + length;
    }

    private static String newickLabel(String name) {
        return name.replaceAll("[\\s(),:;\\[\\]']", "_");
    }

    /** Export the alignment as a FASTA file (with gaps) */
    public static String exportToFASTA(MSAResult result) {
        StringBuilder sb = new StringBuilder();
        for (AlignedSequence as : result.getAlignedSequences()) {
            sb.append('>').append(as.getName()).append('\n');
            String aligned = as.getAlignedBases();
            for (int i = 0; i < aligned.length(); i += 60) {
                sb.append(aligned, i, Math.min(i + 60, aligned.length())).append('\n');
            }
        }
        return sb.toString();
    }
}
