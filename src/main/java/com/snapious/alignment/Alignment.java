package com.snapious.alignment;

import java.util.*;

/**
 * Implements pairwise alignment algorithms:
 * - Needleman-Wunsch (global)
 * - Smith-Waterman (local)
 * 
 * With configurable scoring matrices for DNA, RNA and proteins.
 */
public class Alignment {

    public static final class AlignResult {
        private final String seq1Aln;
        private final String seq2Aln;
        private final double score;
        private final AlignType type;

        public AlignResult(String seq1Aln, String seq2Aln, double score, AlignType type) {
            this.seq1Aln = seq1Aln;
            this.seq2Aln = seq2Aln;
            this.score = score;
            this.type = type;
        }

        public String getSeq1Aln() { return seq1Aln; }
        public String getSeq2Aln() { return seq2Aln; }
        public double getScore() { return score; }
        public AlignType getType() { return type; }

        /** Formatted alignment output in blocks of 60 columns */
        public String format() {
            StringBuilder sb = new StringBuilder();
            sb.append("Score: ").append(String.format(java.util.Locale.ROOT, "%.1f", score)).append("\n\n");

            int len = Math.min(seq1Aln.length(), seq2Aln.length());
            for (int start = 0; start < len; start += 60) {
                int end = Math.min(start + 60, len);
                StringBuilder marks = new StringBuilder();
                for (int i = start; i < end; i++) {
                    char a = seq1Aln.charAt(i);
                    char b = seq2Aln.charAt(i);
                    if (a == '-' || b == '-') marks.append(' ');
                    else if (a == b) marks.append('|');
                    else marks.append(':');
                }
                sb.append(seq1Aln, start, end).append('\n')
                  .append(marks).append('\n')
                  .append(seq2Aln, start, end).append('\n');
                if (end < len) sb.append('\n');
            }
            return sb.toString();
        }
    }

    public enum AlignType { GLOBAL, LOCAL }

    private final int matchScore;
    private final int mismatchPenalty;
    private final int gapPenalty;

    public Alignment() {
        this(2, -1, -1);  // default: DNA scores
    }

    public Alignment(int matchScore, int mismatchPenalty, int gapPenalty) {
        this.matchScore = matchScore;
        this.mismatchPenalty = mismatchPenalty;
        this.gapPenalty = gapPenalty;
    }

    private int substitution(char a, char b) {
        return a == b ? matchScore : mismatchPenalty;
    }

    // ======== NEEDLEMAN-WUNSCH (Global Alignment) ========

    /** Global pairwise alignment (Needleman-Wunsch), O(n*m) time and memory. */
    public AlignResult needlemanWunsch(String seq1, String seq2) {
        int n = seq1.length();
        int m = seq2.length();

        int[][] scoreMatrix = new int[n + 1][m + 1];
        for (int i = 0; i <= n; i++) scoreMatrix[i][0] = i * gapPenalty;
        for (int j = 0; j <= m; j++) scoreMatrix[0][j] = j * gapPenalty;

        for (int i = 1; i <= n; i++) {
            for (int j = 1; j <= m; j++) {
                int diagonal = scoreMatrix[i - 1][j - 1] + substitution(seq1.charAt(i - 1), seq2.charAt(j - 1));
                int up = scoreMatrix[i - 1][j] + gapPenalty;
                int left = scoreMatrix[i][j - 1] + gapPenalty;
                scoreMatrix[i][j] = Math.max(diagonal, Math.max(up, left));
            }
        }

        String[] aln = traceback(seq1, seq2, scoreMatrix, n, m, false);
        return new AlignResult(aln[0], aln[1], scoreMatrix[n][m], AlignType.GLOBAL);
    }

    // ======== SMITH-WATERMAN (Local Alignment) ========

    /** Local pairwise alignment (Smith-Waterman). */
    public AlignResult smithWaterman(String seq1, String seq2) {
        int n = seq1.length();
        int m = seq2.length();

        int[][] scoreMatrix = new int[n + 1][m + 1];
        int maxScore = 0;
        int maxI = 0, maxJ = 0;

        for (int i = 1; i <= n; i++) {
            for (int j = 1; j <= m; j++) {
                int diagonal = scoreMatrix[i - 1][j - 1] + substitution(seq1.charAt(i - 1), seq2.charAt(j - 1));
                int up = scoreMatrix[i - 1][j] + gapPenalty;
                int left = scoreMatrix[i][j - 1] + gapPenalty;
                scoreMatrix[i][j] = Math.max(0, Math.max(diagonal, Math.max(up, left)));

                if (scoreMatrix[i][j] > maxScore) {
                    maxScore = scoreMatrix[i][j];
                    maxI = i;
                    maxJ = j;
                }
            }
        }

        String[] aln = traceback(seq1, seq2, scoreMatrix, maxI, maxJ, true);
        return new AlignResult(aln[0], aln[1], maxScore, AlignType.LOCAL);
    }

    /**
     * Shared backtracking. Global runs to (0,0), local ends at score 0.
     * Returns {aligned sequence 1, aligned sequence 2}.
     */
    private String[] traceback(String seq1, String seq2, int[][] matrix, int endI, int endJ, boolean local) {
        StringBuilder aln1 = new StringBuilder();
        StringBuilder aln2 = new StringBuilder();
        int i = endI, j = endJ;

        while (local ? (i > 0 && j > 0 && matrix[i][j] > 0) : (i > 0 || j > 0)) {
            if (i > 0 && j > 0
                    && matrix[i][j] == matrix[i - 1][j - 1] + substitution(seq1.charAt(i - 1), seq2.charAt(j - 1))) {
                aln1.append(seq1.charAt(i - 1));
                aln2.append(seq2.charAt(j - 1));
                i--; j--;
            } else if (i > 0 && matrix[i][j] == matrix[i - 1][j] + gapPenalty) {
                aln1.append(seq1.charAt(i - 1));
                aln2.append('-');
                i--;
            } else {
                aln1.append('-');
                aln2.append(seq2.charAt(j - 1));
                j--;
            }
        }
        return new String[] { aln1.reverse().toString(), aln2.reverse().toString() };
    }

    // ======== CONSENSUS SEQUENCE ========

    /** Creates a consensus sequence from several aligned sequences */
    public static String createConsensus(List<String> alignedSeqs) {
        if (alignedSeqs == null || alignedSeqs.isEmpty()) return "";

        int len = alignedSeqs.get(0).length();
        StringBuilder consensus = new StringBuilder(len);

        for (int i = 0; i < len; i++) {
            Map<Character, Integer> counts = new HashMap<>();

            for (String seq : alignedSeqs) {
                if (i < seq.length()) {
                    char c = seq.charAt(i);
                    counts.put(c, counts.getOrDefault(c, 0) + 1);
                }
            }

            // Base with the highest weight (gaps lowest)
            char bestBase = 'N';  // default when unclear
            int maxCount = 0;

            for (Map.Entry<Character, Integer> entry : counts.entrySet()) {
                char base = entry.getKey();
                int count = entry.getValue();

                if (base != '-' && count > maxCount) {
                    maxCount = count;
                    bestBase = base;
                }
            }

            consensus.append(bestBase);
        }

        return consensus.toString();
    }

    // ======== PAIRWISE DISTANCE ========

    /** Computes the edit distance (Levenshtein) between two sequences */
    public static double editDistance(String seq1, String seq2) {
        int n = seq1.length();
        int m = seq2.length();

        int[][] dp = new int[n + 1][m + 1];

        for (int i = 0; i <= n; i++) dp[i][0] = i;
        for (int j = 0; j <= m; j++) dp[0][j] = j;

        for (int i = 1; i <= n; i++) {
            for (int j = 1; j <= m; j++) {
                int cost = seq1.charAt(i - 1) == seq2.charAt(j - 1) ? 0 : 1;
                dp[i][j] = Math.min(
                        Math.min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                        dp[i - 1][j - 1] + cost
                );
            }
        }

        return dp[n][m];
    }

    /** Computes the percent identity between two sequences */
    public static double percentIdentity(String seq1, String seq2) {
        int matches = 0;
        int len = Math.min(seq1.length(), seq2.length());

        for (int i = 0; i < len; i++) {
            if (seq1.charAt(i) == seq2.charAt(i)) matches++;
        }

        if (len == 0) return seq1.length() == seq2.length() ? 100.0 : 0.0;
        return (double) matches / len * 100;
    }
}
