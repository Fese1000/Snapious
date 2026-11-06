package com.snapious.alignment;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Multiple Sequence Alignment (MSA) nach progressivem Algorithmus (ClustalW-ähnlich).
 * Erstellt Guide-Tree, aligniert Sequenzen schrittweise von den ähnlichsten zur unterschiedlichsten.
 */
public class MultipleSequenceAlignment {

    // ======== MSA RESULT ==========

    /** Repräsentiert das Ergebnis eines Multi-Sequence-Alginments */
    public static class MSAResult {
        private List<AlignedSequence> alignedSequences;  // Alignierte Sequenzen mit Gaps
        private double averageScore;                     // Durchschnittlicher Pairwise-Score
        private String consensusSequence;                // Konsensus-Sequenz
        private Map<String, Double> sequenceWeights;     // Sequenz-Gewichte (Gaps %)

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

        /** Formatierter Alignment-Output */
        public String format() {
            StringBuilder sb = new StringBuilder();

            // Header
            sb.append("=== Multiple Sequence Alignment ===\n\n");
            sb.append(String.format("Sequenzen: %d, Länge: %d bp\n",
                alignedSequences.size(), getAlignmentLength()));
            sb.append(String.format("Durchschnittlicher Pairwise-Score: %.1f\n", averageScore));
            sb.append(String.format("Konsensus: %s\n\n", consensusSequence));

            // Sequenz-weise Ausgabe (blöckweise à 60bp)
            int blockSize = 60;
            int numBlocks = (getAlignmentLength() + blockSize - 1) / blockSize;

            for (int block = 0; block < numBlocks; block++) {
                int start = block * blockSize;
                int end = Math.min(start + blockSize, getAlignmentLength());

                // Konsensus-Zeile
                sb.append(String.format("Consensus %3d-%3d  %s\n",
                    start + 1, end, getConsensusBlock(block, blockSize)));

                // Jede Sequenz
                for (AlignedSequence as : alignedSequences) {
                    sb.append(String.format("%-20s %3d-%3d  %s\n",
                        as.getName(), start + 1, end, as.getSequenceBlock(block, blockSize)));
                }

                sb.append("\n");  // Leerzeile nach jedem Block
            }

            return sb.toString();
        }

        /** Länge des Alignments */
        public int getAlignmentLength() {
            return alignedSequences.isEmpty() ? 0 : alignedSequences.get(0).getAlignedBases().length();
        }

        /** Konsensus-Block für eine Range */
        private String getConsensusBlock(int block, int blockSize) {
            int start = block * blockSize;
            int end = Math.min(start + blockSize, getAlignmentLength());

            return consensusSequence.substring(start, end);
        }

        @Override
        public String toString() { return format(); }
    }

    /** Repräsentiert eine alignierte Sequenz mit Gaps */
    public static class AlignedSequence {
        private String name;       // Sequenz-Name/ID
        private String bases;      // Original-Basen (ohne Gaps)
        private String alignedBases;  // Alignierte Basen (mit Gaps '-')

        public AlignedSequence(String name, String bases, String alignedBases) {
            this.name = name;
            this.bases = bases.toUpperCase();
            this.alignedBases = alignedBases.toUpperCase();
        }

        public String getName() { return name; }
        public String getBases() { return bases; }
        public String getAlignedBases() { return alignedBases; }

        /** Anzahl der Gaps */
        public int gapCount() {
            return (int) alignedBases.chars().filter(c -> c == '-').count();
        }

        /** Prozentuale Gap-Rate */
        public double gapPercent() {
            int len = alignedBases.length();
            return len > 0 ? (double) gapCount() / len * 100 : 0;
        }

        /** Block extrahieren (für blockweise Ausgabe) */
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

    // ======== PARAMETER ========

    private static final int MATCH = 2;
    private static final int MISMATCH = -1;
    private static final int GAP = -1;

    // ======== GUIDE TREE (UPGMA) ========

    /** Knoten im Guide-Tree; Blätter tragen den Index der Eingabesequenz. */
    private static final class GuideTreeNode {
        final int leafIndex;   // >= 0 bei Blättern, sonst -1
        final String name;     // nur bei Blättern
        final double height;   // Abstand zu den Blättern
        final int size;        // Anzahl Blätter im Teilbaum
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

    /** Zeilen einer (Teil-)Alignierung samt Index der Eingabesequenz pro Zeile. */
    private record Profile(List<Integer> ids, List<String> rows) {
        int width() { return rows.get(0).length(); }
    }

    // ======== HAUPTALIGNMENT ==========

    /**
     * Progressives Multiple Sequence Alignment (ClustalW-ähnlich):
     * paarweise Distanzen aus globalen Alignments, UPGMA-Guide-Tree, anschließend
     * Profil-Profil-Alignment entlang des Baums. Die Reihenfolge der Eingabe bleibt erhalten.
     */
    public static MSAResult progressiveAlignment(List<Sequence> sequences) {
        if (sequences == null || sequences.isEmpty()) {
            throw new IllegalArgumentException("Mindestens eine Sequenz erforderlich");
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

    // ======== DISTANZEN & GUIDE TREE ==========

    /** Distanz = 1 - Identität des globalen paarweisen Alignments (0..1). */
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
        throw new IllegalStateException("Guide-Tree leer");
    }

    // ======== PROGRESSIVES ALIGNMENT ==========

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

    /** Durchschnittlicher Paar-Score der Spalte {@code col} gegen eine reine Gap-Spalte. */
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

    /** Profil-Profil-Alignment (Needleman-Wunsch auf Spalten, Sum-of-Pairs). */
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

    // ======== SCORE & STATISTIK ==========

    /** Durchschnittlicher Sum-of-Pairs-Score über alle Sequenzpaare des Alignments. */
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

    // ======== HAUPTFUNKTIONEN FÜR UI ==========

    /** Generiere MSA-Bericht für Sequenz-Liste */
    public static String generateMSAReport(List<Sequence> sequences) {
        try {
            MSAResult result = progressiveAlignment(sequences);

            StringBuilder sb = new StringBuilder();
            sb.append("=== MSA Zusammenfassung ===\n\n");
            sb.append(String.format("Sequenzen: %d\n", sequences.size()));
            sb.append(String.format("Alignment-Länge: %d bp\n", result.getAlignmentLength()));
            sb.append(String.format("Durchschnittlicher Score: %.1f\n", result.getAverageScore()));

            sb.append("\nGap-Statistik:\n");
            for (AlignedSequence as : result.getAlignedSequences()) {
                sb.append(String.format("  %s: %.0f%% Gaps (%d bp)\n",
                        as.getName(), as.gapPercent(), as.gapCount()));
            }

            String consensusPreview = result.getConsensusSequence();
            if (consensusPreview.length() > 100) {
                consensusPreview = consensusPreview.substring(0, 100) + "...";
            }
            sb.append(String.format("\nKonsensus (Preview): %s\n", consensusPreview));
            sb.append("\n").append(result.format());
            return sb.toString();

        } catch (IllegalArgumentException e) {
            return "Fehler beim MSA: " + e.getMessage();
        }
    }

    /** Aligniere Sequenzen und gib Ergebnis zurück */
    public static MSAResult alignSequences(List<Sequence> sequences) {
        return progressiveAlignment(sequences);
    }

    /** Guide-Tree der Sequenzen im Newick-Format (Astlängen aus UPGMA-Höhen). */
    public static String generateNewickFromMSA(List<Sequence> sequences) {
        if (sequences == null || sequences.isEmpty()) {
            throw new IllegalArgumentException("Mindestens eine Sequenz erforderlich");
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

    /** Exportiere Alignment als FASTA-Datei (mit Gaps) */
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
