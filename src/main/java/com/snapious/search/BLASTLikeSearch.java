package com.snapious.search;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Local BLAST-like sequence similarity search using the seed-and-extend principle:
 * k-tuple hits between query and subject are extended ungapped with X-drop termination
 * into high-scoring segment pairs (HSPs). Nucleotide sequences are searched on
 * both strands. Scoring is simply by identity (no BLOSUM).
 */
public class BLASTLikeSearch {

    private static final int MATCH = 2;
    private static final int MISMATCH = -1;
    private static final int X_DROP = 10;

    // ======== HSP (High-scoring Segment Pair) ==========

    public static class HSP {
        private final String queryId;
        private final String subjectId;
        private final int queryStart;     // 0-based, on the forward strand of the query
        private final int queryEnd;       // exclusive
        private final int subjectStart;   // 0-based
        private final int subjectEnd;     // exclusive
        private final double score;
        private final int matches;
        private final int mismatches;
        private final char strand;        // '+' or '-' (query as reverse complement)

        public HSP(String queryId, String subjectId, int queryStart, int queryEnd,
                   int subjectStart, int subjectEnd, double score,
                   int matches, int mismatches, char strand) {
            this.queryId = queryId;
            this.subjectId = subjectId;
            this.queryStart = queryStart;
            this.queryEnd = queryEnd;
            this.subjectStart = subjectStart;
            this.subjectEnd = subjectEnd;
            this.score = score;
            this.matches = matches;
            this.mismatches = mismatches;
            this.strand = strand;
        }

        public String getQueryId() { return queryId; }
        public String getSubjectId() { return subjectId; }

        public int getQueryStart() { return queryStart; }
        public int getQueryEnd() { return queryEnd; }
        public int getSubjectStart() { return subjectStart; }
        public int getSubjectEnd() { return subjectEnd; }
        public char getStrand() { return strand; }

        public double getScore() { return score; }
        public int getMatches() { return matches; }
        public int getMismatches() { return mismatches; }
        /** The extension is ungapped, hence always 0. */
        public int getGaps() { return 0; }

        /** Length of the HSP */
        public int length() { return subjectEnd - subjectStart; }

        /** Percent identity */
        public double percentIdentity() {
            int len = length();
            return len > 0 ? (double) matches / len * 100 : 0;
        }

        /** Formatted output */
        public String format() {
            return String.format(Locale.ROOT, "HSP[%s → %s (%c): %d-%d vs %d-%d, Score=%.1f, ID=%.0f%%]",
                queryId, subjectId, strand, queryStart, queryEnd, subjectStart, subjectEnd,
                score, percentIdentity());
        }

        @Override
        public String toString() { return format(); }
    }

    // ======== MAIN SEARCH ==========

    /**
     * Search a database with the k-tuple/seed-and-extend algorithm.
     * @param query query sequence as a string
     * @param database database sequences
     * @param wordSize k-tuple size (DNA: 11, protein: 3)
     * @return list of HSPs found, sorted by score
     */
    public static List<HSP> search(String query, List<Sequence> database, int wordSize) {
        return search("query", query, database, wordSize);
    }

    /** Search with a Sequence object as the query (its ID appears in the HSPs). */
    public static List<HSP> search(Sequence query, List<Sequence> database, int wordSize) {
        return search(query.getId(), query.getBases(), database, wordSize);
    }

    /** DNA search with default k=11 */
    public static List<HSP> searchDNA(Sequence query, List<Sequence> database) {
        return search(query, database, 11);
    }

    /** Protein search with default k=3 */
    public static List<HSP> searchProtein(Sequence query, List<Sequence> database) {
        return search(query, database, 3);
    }

    private static List<HSP> search(String queryId, String query, List<Sequence> database, int wordSize) {
        if (wordSize < 1) throw new IllegalArgumentException("wordSize must be >= 1");
        if (query == null || database == null) throw new IllegalArgumentException("Query and database required");

        String forward = query.toUpperCase();
        List<HSP> all = new ArrayList<>();

        for (Sequence subject : database) {
            boolean nucleotide = subject.getType() != Sequence.SequenceType.PROTEIN;
            String subjectSeq = nucleotide ? toDna(subject.getBases()) : subject.getBases();
            String queryFwd = nucleotide ? toDna(forward) : forward;

            searchStrand(queryId, queryFwd, subject.getId(), subjectSeq, wordSize, '+', all);
            if (nucleotide) {
                searchStrand(queryId, reverseComplement(queryFwd), subject.getId(), subjectSeq,
                        wordSize, '-', all);
            }
        }
        return filterAndSortHSPs(all);
    }

    private static void searchStrand(String queryId, String query, String subjectId, String subject,
                                     int k, char strand, List<HSP> out) {
        if (query.length() < k || subject.length() < k) return;

        // Index over the query words
        Map<String, List<Integer>> index = new HashMap<>();
        for (int i = 0; i + k <= query.length(); i++) {
            index.computeIfAbsent(query.substring(i, i + k), w -> new ArrayList<>()).add(i);
        }

        List<int[]> extents = new ArrayList<>();  // {queryStart, subjectStart, length} in the strand coordinate system
        for (int sPos = 0; sPos + k <= subject.length(); sPos++) {
            List<Integer> qPositions = index.get(subject.substring(sPos, sPos + k));
            if (qPositions == null) continue;

            for (int qPos : qPositions) {
                if (covered(extents, qPos, sPos)) continue;
                int[] ext = extendSeed(query, subject, qPos, sPos, k);
                extents.add(ext);

                int qs = ext[0], ss = ext[1], len = ext[2];
                int qStart = strand == '+' ? qs : query.length() - (qs + len);
                int qEnd = strand == '+' ? qs + len : query.length() - qs;
                out.add(new HSP(queryId, subjectId, qStart, qEnd, ss, ss + len, ext[4],
                        ext[3], len - ext[3], strand));
            }
        }
    }

    /** True if the seed already lies in an extent on the same diagonal. */
    private static boolean covered(List<int[]> extents, int qPos, int sPos) {
        for (int[] e : extents) {
            if (sPos - qPos == e[1] - e[0] && qPos >= e[0] && qPos < e[0] + e[2]) return true;
        }
        return false;
    }

    // ======== SEED-AND-EXTEND ==========

    /**
     * Extend a seed ungapped in both directions with X-drop termination.
     * @return {queryStart, subjectStart, length, matches, score}
     */
    private static int[] extendSeed(String query, String subject, int qPos, int sPos, int k) {
        int bestRight = 0, bestRightScore = 0, run = 0;
        for (int i = 0; qPos + k + i < query.length() && sPos + k + i < subject.length(); i++) {
            run += query.charAt(qPos + k + i) == subject.charAt(sPos + k + i) ? MATCH : MISMATCH;
            if (run > bestRightScore) {
                bestRightScore = run;
                bestRight = i + 1;
            } else if (bestRightScore - run > X_DROP) {
                break;
            }
        }

        int bestLeft = 0, bestLeftScore = 0;
        run = 0;
        for (int i = 1; qPos - i >= 0 && sPos - i >= 0; i++) {
            run += query.charAt(qPos - i) == subject.charAt(sPos - i) ? MATCH : MISMATCH;
            if (run > bestLeftScore) {
                bestLeftScore = run;
                bestLeft = i;
            } else if (bestLeftScore - run > X_DROP) {
                break;
            }
        }

        int qs = qPos - bestLeft;
        int ss = sPos - bestLeft;
        int len = bestLeft + k + bestRight;
        int matches = 0;
        for (int i = 0; i < len; i++) {
            if (query.charAt(qs + i) == subject.charAt(ss + i)) matches++;
        }
        return new int[] { qs, ss, len, matches, k * MATCH + bestLeftScore + bestRightScore };
    }

    /** Filter redundancies (same subject and strand) and sort by score */
    private static List<HSP> filterAndSortHSPs(List<HSP> hspList) {
        hspList.sort((a, b) -> Double.compare(b.getScore(), a.getScore()));

        List<HSP> uniqueHSPs = new ArrayList<>();
        for (HSP hsp : hspList) {
            boolean redundant = false;
            for (HSP existing : uniqueHSPs) {
                if (!existing.subjectId.equals(hsp.subjectId) || existing.strand != hsp.strand) continue;
                boolean queryOverlap = Math.min(hsp.queryEnd, existing.queryEnd)
                        > Math.max(hsp.queryStart, existing.queryStart);
                boolean subjectOverlap = Math.min(hsp.subjectEnd, existing.subjectEnd)
                        > Math.max(hsp.subjectStart, existing.subjectStart);
                if (queryOverlap && subjectOverlap) {
                    redundant = true;
                    break;
                }
            }
            if (!redundant) uniqueHSPs.add(hsp);
        }
        return uniqueHSPs;
    }

    // ======== HELPER FUNCTIONS ==========

    private static String toDna(String s) {
        return s.replace('U', 'T');
    }

    /** Reverse complement for DNA search */
    private static String reverseComplement(String sequence) {
        StringBuilder sb = new StringBuilder(sequence.length());
        for (int i = sequence.length() - 1; i >= 0; i--) {
            char c = sequence.charAt(i);
            switch (c) {
                case 'A': sb.append('T'); break;
                case 'T': sb.append('A'); break;
                case 'G': sb.append('C'); break;
                case 'C': sb.append('G'); break;
                default: sb.append(c);  // N, R, Y, etc.
            }
        }
        return sb.toString();
    }

    /** Generate a summary of the search results */
    public static String getSearchSummary(List<HSP> hspList, String queryName) {
        StringBuilder sb = new StringBuilder();

        sb.append(String.format("=== BLAST-like search results for: %s ===\n\n", queryName));
        sb.append(String.format("Hits (HSPs): %d\n", hspList.size()));

        if (!hspList.isEmpty()) {
            sb.append("\nTop 10 results:\n");

            for (int i = 0; i < Math.min(10, hspList.size()); i++) {
                HSP hsp = hspList.get(i);
                sb.append(String.format("%d. %s\n", i + 1, hsp.format()));
                sb.append(String.format("   Length: %d bp, Identity: %.0f%%\n\n",
                    hsp.length(), hsp.percentIdentity()));
            }

            // Statistics
            double avgScore = hspList.stream().mapToDouble(HSP::getScore).average().orElse(0);
            double avgIdentity = hspList.stream().mapToDouble(HSP::percentIdentity).average().orElse(0);

            sb.append(String.format("Average score: %.1f\n", avgScore));
            sb.append(String.format("Average identity: %.0f%%\n", avgIdentity));
        } else {
            sb.append("No significant matches found.\n");
        }

        return sb.toString();
    }

    /** Find all unique k-tuples in a sequence */
    public static Set<String> getUniqueKmers(String sequence, int k) {
        String seq = sequence.toUpperCase();
        Set<String> kmers = new HashSet<>();

        for (int i = 0; i <= seq.length() - k; i++) {
            kmers.add(seq.substring(i, i + k));
        }

        return kmers;
    }

    /** Compute the k-tuple frequency distribution */
    public static Map<String, Integer> getKmerFrequencies(String sequence, int k) {
        String seq = sequence.toUpperCase();
        Map<String, Integer> freqMap = new HashMap<>();

        for (int i = 0; i <= seq.length() - k; i++) {
            String kmer = seq.substring(i, i + k);
            freqMap.merge(kmer, 1, Integer::sum);
        }

        return freqMap;
    }
}
