package com.snapious.search;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Lokale BLAST-ähnliche Sequenzähnlichkeitssuche nach dem Seed-and-Extend-Prinzip:
 * k-tuple-Treffer zwischen Query und Subject werden ungapped mit X-drop-Abbruch
 * zu High-scoring Segment Pairs (HSPs) verlängert. Nukleotidsequenzen werden auf
 * beiden Strängen durchsucht. Bewertet wird einfach nach Identität (kein BLOSUM).
 */
public class BLASTLikeSearch {

    private static final int MATCH = 2;
    private static final int MISMATCH = -1;
    private static final int X_DROP = 10;

    // ======== HSP (High-scoring Segment Pair) ==========

    public static class HSP {
        private final String queryId;
        private final String subjectId;
        private final int queryStart;     // 0-basiert, auf dem Vorwärtsstrang der Query
        private final int queryEnd;       // exklusiv
        private final int subjectStart;   // 0-basiert
        private final int subjectEnd;     // exklusiv
        private final double score;
        private final int matches;
        private final int mismatches;
        private final char strand;        // '+' oder '-' (Query als Reverse-Komplement)

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
        /** Die Extension ist ungapped, daher immer 0. */
        public int getGaps() { return 0; }

        /** Länge des HSPs */
        public int length() { return subjectEnd - subjectStart; }

        /** Prozent-Identität */
        public double percentIdentity() {
            int len = length();
            return len > 0 ? (double) matches / len * 100 : 0;
        }

        /** Formatierter Output */
        public String format() {
            return String.format(Locale.ROOT, "HSP[%s → %s (%c): %d-%d vs %d-%d, Score=%.1f, ID=%.0f%%]",
                queryId, subjectId, strand, queryStart, queryEnd, subjectStart, subjectEnd,
                score, percentIdentity());
        }

        @Override
        public String toString() { return format(); }
    }

    // ======== HAUPTSUCHE ==========

    /**
     * Durchsuche Datenbank mit k-tuple/seed-and-extend Algorithmus.
     * @param query Query-Sequenz als String
     * @param database Datenbank-Sequenzen
     * @param wordSize k-tuple Größe (DNA: 11, Protein: 3)
     * @return Liste gefundener HSPs sortiert nach Score
     */
    public static List<HSP> search(String query, List<Sequence> database, int wordSize) {
        return search("query", query, database, wordSize);
    }

    /** Suche mit Sequence-Objekt als Query (deren ID erscheint in den HSPs). */
    public static List<HSP> search(Sequence query, List<Sequence> database, int wordSize) {
        return search(query.getId(), query.getBases(), database, wordSize);
    }

    /** DNA-Suche mit default k=11 */
    public static List<HSP> searchDNA(Sequence query, List<Sequence> database) {
        return search(query, database, 11);
    }

    /** Protein-Suche mit default k=3 */
    public static List<HSP> searchProtein(Sequence query, List<Sequence> database) {
        return search(query, database, 3);
    }

    private static List<HSP> search(String queryId, String query, List<Sequence> database, int wordSize) {
        if (wordSize < 1) throw new IllegalArgumentException("wordSize muss >= 1 sein");
        if (query == null || database == null) throw new IllegalArgumentException("Query und Datenbank erforderlich");

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

        // Index über die Query-Wörter
        Map<String, List<Integer>> index = new HashMap<>();
        for (int i = 0; i + k <= query.length(); i++) {
            index.computeIfAbsent(query.substring(i, i + k), w -> new ArrayList<>()).add(i);
        }

        List<int[]> extents = new ArrayList<>();  // {queryStart, subjectStart, length} im Strang-Koordinatensystem
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

    /** True, wenn der Seed auf derselben Diagonale bereits in einem Extent liegt. */
    private static boolean covered(List<int[]> extents, int qPos, int sPos) {
        for (int[] e : extents) {
            if (sPos - qPos == e[1] - e[0] && qPos >= e[0] && qPos < e[0] + e[2]) return true;
        }
        return false;
    }

    // ======== SEED-AND-EXTEND ==========

    /**
     * Verlängere einen Seed ungapped in beide Richtungen mit X-drop-Abbruch.
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

    /** Filtere Redundanzen (gleiches Subject und Strang) und sortiere nach Score */
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

    // ======== HILFSFUNKTIONEN ==========

    private static String toDna(String s) {
        return s.replace('U', 'T');
    }

    /** Reverse-komplement für DNA-Suche */
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

    /** Generiere Zusammenfassung der Suchergebnisse */
    public static String getSearchSummary(List<HSP> hspList, String queryName) {
        StringBuilder sb = new StringBuilder();

        sb.append(String.format("=== BLAST-like Suchergebnisse für: %s ===\n\n", queryName));
        sb.append(String.format("Fundstellen (HSPs): %d\n", hspList.size()));

        if (!hspList.isEmpty()) {
            sb.append("\nTop 10 Ergebnisse:\n");

            for (int i = 0; i < Math.min(10, hspList.size()); i++) {
                HSP hsp = hspList.get(i);
                sb.append(String.format("%d. %s\n", i + 1, hsp.format()));
                sb.append(String.format("   Länge: %d bp, Identität: %.0f%%\n\n",
                    hsp.length(), hsp.percentIdentity()));
            }

            // Statistik
            double avgScore = hspList.stream().mapToDouble(HSP::getScore).average().orElse(0);
            double avgIdentity = hspList.stream().mapToDouble(HSP::percentIdentity).average().orElse(0);

            sb.append(String.format("Durchschnittlicher Score: %.1f\n", avgScore));
            sb.append(String.format("Durchschnittliche Identität: %.0f%%\n", avgIdentity));
        } else {
            sb.append("Keine signifikanten Übereinstimmungen gefunden.\n");
        }

        return sb.toString();
    }

    /** Finde alle unique k-tuples in einer Sequenz */
    public static Set<String> getUniqueKmers(String sequence, int k) {
        String seq = sequence.toUpperCase();
        Set<String> kmers = new HashSet<>();

        for (int i = 0; i <= seq.length() - k; i++) {
            kmers.add(seq.substring(i, i + k));
        }

        return kmers;
    }

    /** Berechne k-tuple Frequenz-Verteilung */
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
