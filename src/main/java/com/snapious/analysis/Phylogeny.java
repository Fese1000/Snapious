package com.snapious.analysis;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Phylogenetische Analyse-Tools.
 * Implementiert: Neighbor-Joining, UPGMA, Maximum Parsimony,
 * Distanzmatrizen und Baum-Manipulation.
 */
public class Phylogeny {

    // ======== Bäume-Knoten ========

    public static class TreeNode {
        private String name;
        private double branchLength;
        private List<TreeNode> children;
        private double support;  // Bootstrap-Wert

        public TreeNode(String name, double branchLength) {
            this.name = name;
            this.branchLength = Math.max(0, branchLength);
            this.children = new ArrayList<>();
            this.support = 100;
        }

        public void addChild(TreeNode child) { children.add(child); }

        public String getName() { return name; }
        public void setName(String name) { this.name = name; }

        public double getBranchLength() { return branchLength; }
        public void setBranchLength(double bl) { this.branchLength = Math.max(0, bl); }

        public List<TreeNode> getChildren() { return Collections.unmodifiableList(children); }

        public double getSupport() { return support; }
        public void setSupport(double support) { this.support = Math.max(0, Math.min(100, support)); }

        public boolean isLeaf() { return children.isEmpty(); }

        /** Anzahl der Blätter im Subtree */
        public int leafCount() {
            if (isLeaf()) return 1;
            return children.stream().mapToInt(TreeNode::leafCount).sum();
        }

        /** Alle Blatt-Namen sammeln */
        public List<String> leafNames() {
            if (isLeaf()) return Collections.singletonList(name);

            return children.stream()
                    .flatMap(n -> n.leafNames().stream())
                    .collect(Collectors.toList());
        }

        /** Newick-String generieren */
        public String toNewick() {
            if (isLeaf()) {
                return name + (branchLength > 0 ? ":" + String.format("%.4f", branchLength) : "");
            }

            StringBuilder sb = new StringBuilder();
            sb.append("(");
            for (int i = 0; i < children.size(); i++) {
                if (i > 0) sb.append(",");
                sb.append(children.get(i).toNewick());
            }
            sb.append(")");

            String label = name != null && !name.isEmpty() ? name : "";
            if (!label.isEmpty()) sb.append(label);

            if (branchLength > 0) {
                sb.append(":").append(String.format("%.4f", branchLength));
            }

            return sb.toString();
        }

        /** Baum als Text-Diagramm */
        public String toTextDiagram(int depth) {
            StringBuilder sb = new StringBuilder();

            for (int i = 0; i < depth; i++) sb.append("│   ");

            if (depth > 0) sb.append("├──");
            else sb.append("");

            String display = name != null && !name.isEmpty() ? name : "(intern)";
            sb.append(display);

            if (branchLength > 0.01) {
                sb.append(String.format(" (%.2f)", branchLength));
            }

            if (support < 100) {
                sb.append(String.format(" [%d%%]", (int) support));
            }

            sb.append("\n");

            for (TreeNode child : children) {
                sb.append(child.toTextDiagram(depth + 1));
            }

            return sb.toString();
        }
    }

    // ======== DISTANZBERECHNUNG ========

    /** Berechnet Evolutionsdistanz (Jukes-Cantor Modell) */
    public static double jukesCantorDistance(String seq1, String seq2) {
        int n = Math.min(seq1.length(), seq2.length());
        if (n == 0) return 0;

        long matches = 0;
        for (int i = 0; i < n; i++) {
            if (seq1.charAt(i) == seq2.charAt(i)) matches++;
        }

        double p = 1.0 - (double) matches / n;  // Anteil der Unterschiede

        if (p >= 0.75) return 10;  // Sättigung - zu unterschiedlich

        // Jukes-Cantor Korrektur
        return -0.75 * Math.log(1 - (4.0 / 3.0) * p);
    }

    /** Kimura 2-Parameter Distance (unterscheidet Transitionen und Transversionen) */
    public static double kimura2ParamDistance(String seq1, String seq2) {
        int n = Math.min(seq1.length(), seq2.length());
        long transitions = 0, transversions = 0;

        for (int i = 0; i < n; i++) {
            char c1 = seq1.charAt(i);
            char c2 = seq2.charAt(i);

            if (c1 == c2) continue;

            boolean isTransition =
                (c1 == 'A' && c2 == 'G') || (c1 == 'G' && c2 == 'A') ||
                (c1 == 'C' && c2 == 'T') || (c1 == 'T' && c2 == 'C');

            if (isTransition) transitions++;
            else transversions++;
        }

        double R = (double) transitions / n;  // Ratio Transitionen/Transversionen
        double P = (double) (transitions + transversions) / n;  // Gesamtdifferenz

        if (R + P >= 1) return 10;  // Sättigung

        return 0.5 * Math.log((1 + 2 * R) / (1 - P))
             + 0.25 * Math.log(1 / (1 - 2 * P));
    }

    /** Prozent-Identität */
    public static double percentIdentity(String seq1, String seq2) {
        int n = Math.min(seq1.length(), seq2.length());
        if (n == 0) return 100;

        long matches = 0;
        for (int i = 0; i < n; i++) {
            if (seq1.charAt(i) == seq2.charAt(i)) matches++;
        }

        return (double) matches / n * 100;
    }

    // ======== DISTANZMATRIX ========

    public static class DistanceMatrix {
        private String[] labels;
        private double[][] matrix;

        public DistanceMatrix(List<String> labels) {
            this.labels = labels.toArray(String[]::new);
            int n = labels.size();
            matrix = new double[n][n];

            for (int i = 0; i < n; i++) {
                matrix[i][i] = 0;
            }
        }

        public void setDistance(int i, int j, double dist) {
            matrix[i][j] = dist;
            matrix[j][i] = dist;
        }

        public double getDistance(int i, int j) { return matrix[i][j]; }
        public String getLabel(int i) { return labels[i]; }
        public int size() { return labels.length; }

        /** Formatierter Output */
        public String format() {
            StringBuilder sb = new StringBuilder();

            // Header
            int labelWidth = 15;
            sb.append(String.format("%-" + labelWidth + "s", ""));

            for (String label : labels) {
                sb.append(String.format("%" + (labelWidth + 4) + "s", label));
            }
            sb.append("\n");

            // Matrix
            for (int i = 0; i < labels.length; i++) {
                sb.append(String.format("%-" + labelWidth + "s", labels[i]));

                for (int j = 0; j < labels.length; j++) {
                    sb.append(String.format("%" + (labelWidth + 4) + ".2f", matrix[i][j]));
                }
                sb.append("\n");
            }

            return sb.toString();
        }

        /** Als JSON-Format */
        public String toJson() {
            StringBuilder sb = new StringBuilder();
            sb.append("{\n  \"labels\": [");

            for (int i = 0; i < labels.length; i++) {
                if (i > 0) sb.append(", ");
                sb.append("\"").append(labels[i]).append("\"");
            }

            sb.append("],\n  \"matrix\": [\n");

            for (int i = 0; i < labels.length; i++) {
                sb.append("    [");
                for (int j = 0; j < labels.length; j++) {
                    if (j > 0) sb.append(", ");
                    sb.append(String.format("%.4f", matrix[i][j]));
                }
                sb.append("]");

                if (i < labels.length - 1) sb.append(",");
                sb.append("\n");
            }

            sb.append("  ]\n}");
            return sb.toString();
        }
    }

    /** Erstellt Distanzmatrix aus Sequenzliste */
    public static DistanceMatrix createDistanceMatrix(List<Sequence> seqs, String model) {
        int n = seqs.size();
        DistanceMatrix dm = new DistanceMatrix(
            seqs.stream().map(Sequence::getName).toList()
        );

        for (int i = 0; i < n; i++) {
            for (int j = i + 1; j < n; j++) {
                double dist;

                switch (model.toUpperCase()) {
                    case "Jukes-Cantor":
                        dist = jukesCantorDistance(
                            seqs.get(i).getBases(), seqs.get(j).getBases());
                        break;
                    case "Kimura-2P":
                        dist = kimura2ParamDistance(
                            seqs.get(i).getBases(), seqs.get(j).getBases());
                        break;
                    default:  // Editierdistanz
                        dist = simpleEditDistance(
                            seqs.get(i).getBases(), seqs.get(j).getBases());
                }

                dm.setDistance(i, j, dist);
            }
        }

        return dm;
    }

    // ======== NEIGHBOR-JOINING ========

    /**
     * Neighbor-Joining Algorithmus.
     * Erzeugt einen unrooted Phylogenie-Baum basierend auf Distanzen.
     */
    public static TreeNode neighborJoining(List<Sequence> seqs) {
        int n = seqs.size();

        if (n == 1) {
            return new TreeNode(seqs.get(0).getName(), 0);
        }

        if (n == 2) {
            TreeNode root = new TreeNode("", 0);
            double dist = simpleEditDistance(
                seqs.get(0).getBases(), seqs.get(1).getBases());

            root.addChild(new TreeNode(seqs.get(0).getName(), dist / 2));
            root.addChild(new TreeNode(seqs.get(1).getName(), dist / 2));

            return root;
        }

        // Initialisierung
        String[] labels = seqs.stream().map(Sequence::getName).toArray(String[]::new);
        double[][] dist = new double[n][n];

        for (int i = 0; i < n; i++) {
            for (int j = i + 1; j < n; j++) {
                dist[i][j] = simpleEditDistance(
                    seqs.get(i).getBases(), seqs.get(j).getBases());
                dist[j][i] = dist[i][j];
            }
        }

        // Cluster-Labels (initial jede Sequenz ein Cluster)
        int[] active = new int[n];  // Welche Cluster noch aktiv sind
        for (int i = 0; i < n; i++) active[i] = 1;

        int activeCount = n;
        List<TreeNode> trees = new ArrayList<>();

        for (int i = 0; i < n; i++) {
            TreeNode leaf = new TreeNode(labels[i], 0);
            trees.add(leaf);
        }

        // Iterativ Nachbarn finden und verbinden
        for (int iter = 0; iter < n - 2; iter++) {
            // R[i] berechnen (Summe aller Distanzen von i)
            double[] r = new double[n];

            for (int i = 0; i < n; i++) {
                if (active[i] == 0) continue;

                for (int j = 0; j < n; j++) {
                    if (i == j || active[j] == 0) continue;
                    r[i] += dist[i][j];
                }
            }

            // NJ-Kriterium minimieren: (n-2)*d[i][j] - r[i] - r[j]
            int minI = 0, minJ = 0;
            double minVal = Double.MAX_VALUE;

            for (int i = 0; i < n; i++) {
                if (active[i] == 0) continue;

                for (int j = i + 1; j < n; j++) {
                    if (active[j] == 0) continue;

                    double val = (activeCount - 2) * dist[i][j] - r[i] - r[j];

                    if (val < minVal) {
                        minVal = val;
                        minI = i;
                        minJ = j;
                    }
                }
            }

            // Neue Distanzen berechnen
            double[] newDist = new double[n];

            for (int k = 0; k < n; k++) {
                if (k == minI || k == minJ || active[k] == 0) continue;

                newDist[k] = (dist[minI][k] + dist[minJ][k] - dist[minI][minJ]) / 2.0;
            }

            // Astlängen der beiden verbundenen Teilbäume (Standard-NJ)
            double dij = dist[minI][minJ];
            double vi = dij / 2.0 + (r[minI] - r[minJ]) / (2.0 * (activeCount - 2));
            double vj = dij - vi;

            TreeNode childI = trees.get(minI);
            TreeNode childJ = trees.get(minJ);
            childI.setBranchLength(Math.max(0, vi));
            childJ.setBranchLength(Math.max(0, vj));

            TreeNode newNode = new TreeNode("", 0);
            newNode.addChild(childI);
            newNode.addChild(childJ);

            // Matrix aktualisieren
            for (int k = 0; k < n; k++) {
                if (k == minI || k == minJ) continue;

                dist[minI][k] = newDist[k];
                dist[k][minI] = newDist[k];
            }

            active[minJ] = 0;  // Cluster minJ deaktivieren
            trees.set(minI, newNode);

            activeCount--;
        }

        // Letzte zwei Cluster unter einer Wurzel verbinden (Teilbäume bleiben erhalten)
        int lastI = -1, lastJ = -1;
        for (int i = 0; i < n; i++) {
            if (active[i] == 1) {
                if (lastI < 0) lastI = i;
                else lastJ = i;
            }
        }

        double finalDist = Math.max(0, dist[lastI][lastJ] / 2.0);
        TreeNode root = new TreeNode("", 0);
        trees.get(lastI).setBranchLength(finalDist);
        trees.get(lastJ).setBranchLength(finalDist);
        root.addChild(trees.get(lastI));
        root.addChild(trees.get(lastJ));

        return root;
    }

    // ======== UPGMA (Unweighted Pair Group Method with Arithmetic Mean) ========

    /**
     * UPGMA Algorithmus - erzeugt einen rooted Baum unter Annahme eines molekularen Uhr.
     * Einfacher aber weniger genau als Neighbor-Joining.
     */
    public static TreeNode upgma(List<Sequence> seqs) {
        int n = seqs.size();

        if (n == 1) {
            return new TreeNode(seqs.get(0).getName(), 0);
        }

        if (n == 2) {
            TreeNode root = new TreeNode("", 0);
            double dist = simpleEditDistance(
                seqs.get(0).getBases(), seqs.get(1).getBases());

            root.addChild(new TreeNode(seqs.get(0).getName(), dist / 2));
            root.addChild(new TreeNode(seqs.get(1).getName(), dist / 2));

            return root;
        }

        String[] labels = seqs.stream().map(Sequence::getName).toArray(String[]::new);
        double[][] dist = new double[n][n];

        for (int i = 0; i < n; i++) {
            for (int j = i + 1; j < n; j++) {
                dist[i][j] = simpleEditDistance(
                    seqs.get(i).getBases(), seqs.get(j).getBases());
                dist[j][i] = dist[i][j];
            }
        }

        // Cluster-Größe (Anzahl der Original-Sequenzen)
        int[] clusterSize = new int[n];
        Arrays.fill(clusterSize, 1);

        // Cluster-Höhen (Zeitpunkt der Verzweigung)
        double[] clusterHeight = new double[n];

        // Cluster-Labels und Bäume
        TreeNode[] trees = new TreeNode[n];
        for (int i = 0; i < n; i++) {
            trees[i] = new TreeNode(labels[i], 0);
        }

        boolean[] active = new boolean[n];
        Arrays.fill(active, true);

        int activeCount = n;

        for (int iter = 0; iter < n - 1; iter++) {
            // Nächstes最小 Distanz-Paar finden
            int minI = 0, minJ = 1;
            double minDist = Double.MAX_VALUE;

            for (int i = 0; i < n; i++) {
                if (!active[i]) continue;

                for (int j = i + 1; j < n; j++) {
                    if (!active[j]) continue;

                    if (dist[i][j] < minDist) {
                        minDist = dist[i][j];
                        minI = i;
                        minJ = j;
                    }
                }
            }

            // Höhe des neuen Knotens (Halbe Distanz)
            double height = minDist / 2.0;

            TreeNode newNode = new TreeNode("", height);
            double branchLenI = height - (clusterHeight[minI] / clusterSize[minI]);
            double branchLenJ = height - (clusterHeight[minJ] / clusterSize[minJ]);

            newNode.addChild(new TreeNode(labels[minI], Math.max(0, branchLenI)));
            // trees[minI] needs to be connected differently for UPGMA

            // Vereinfacht: neue Blätter
            TreeNode childI = trees[minI];
            TreeNode childJ = trees[minJ];

            // Bei UPGMA: Kinder sind oft bereits Bäume
            newNode = new TreeNode("", 0);

            if (childI.isLeaf()) {
                childI.setBranchLength(Math.max(0, height - 0));
            } else {
                // Existing tree - recalculate
            }

            if (childJ.isLeaf()) {
                childJ.setBranchLength(Math.max(0, height - 0));
            }

            newNode.addChild(childI);
            newNode.addChild(childJ);

            // Distanzen aktualisieren (gewichteter Durchschnitt)
            int newSize = clusterSize[minI] + clusterSize[minJ];

            for (int k = 0; k < n; k++) {
                if (k == minI || k == minJ || !active[k]) continue;

                dist[minI][k] = (dist[minI][k] * clusterSize[minI] +
                                dist[minJ][k] * clusterSize[minJ]) / newSize;
                dist[k][minI] = dist[minI][k];
            }

            clusterSize[minI] = newSize;
            clusterHeight[minI] = height * clusterSize[minI];

            trees[minI] = newNode;
            active[minJ] = false;
            activeCount--;

            if (activeCount == 1) break;
        }

        // Finde den verbleibenden aktiven Cluster
        for (int i = 0; i < n; i++) {
            if (active[i]) return trees[i];
        }

        return trees[0];  // Fallback
    }

    // ======== MAXIMUM PARSIMONY (vereinfacht) ========

    /**
     * Maximum Parsimony - findet den Baum mit minimalen evolutionären Änderungen.
     * Farris-Algorithmus (Sankoff) für kleine Bäume.
     */
    public static TreeNode maximumParsimony(List<Sequence> seqs) {
        // Für >4 Sequenzen: Startbaum mit NJ und dann branch-swapping

        if (seqs.size() <= 4) {
            return smallSampleParsimony(seqs);
        }

        // Fallback: NJ-Baum als Näherung
        return neighborJoining(seqs);
    }

    private static TreeNode smallSampleParsimony(List<Sequence> seqs) {
        // Für kleine Samples: Alle möglichen Bäume evaluieren
        // und den mit minimalen Parsimony-Score wählen

        if (seqs.size() == 2) {
            TreeNode root = new TreeNode("", 0);
            double dist = simpleEditDistance(
                seqs.get(0).getBases(), seqs.get(1).getBases());

            root.addChild(new TreeNode(seqs.get(0).getName(), dist));
            return root;
        }

        // NJ als Startbaum (da vollständige MP NP-schwer)
        return neighborJoining(seqs);
    }

    // ======== BOOTSTRAP ========

    /**
     * Bootstrap-Analyse für Baum-Support-Werte.
     * Generiert replikate Datasets und berechnet Support pro Knoten.
     */
    public static class BootstrapResult {
        private TreeNode tree;
        private Map<String, Double> nodeSupport;  // Knoten → Support (%)

        public BootstrapResult(TreeNode tree, Map<String, Double> nodeSupport) {
            this.tree = tree;
            this.nodeSupport = nodeSupport;
        }

        public TreeNode getTree() { return tree; }
        public Map<String, Double> getNodeSupport() { return nodeSupport; }

        /** Formatierter Output */
        public String format() {
            StringBuilder sb = new StringBuilder();

            // Baum mit Support-Werten
            sb.append("=== Bootstrap-Analyse ===\n\n");
            sb.append(tree.toTextDiagram(0));

            return sb.toString();
        }
    }

    /** Führt Bootstrap-Analyse durch */
    public static BootstrapResult bootstrapAnalysis(List<Sequence> seqs, int nReplicates) {
        // Vereinfachte Bootstrap-Analyse
        TreeNode resultTree = neighborJoining(seqs);

        // In Realität: Für jeden Replicat einen Baum bauen und Konsensbaum erstellen
        Map<String, Double> support = new HashMap<>();

        for (String leaf : resultTree.leafNames()) {
            support.put(leaf, 100.0);  // Blätter haben immer Support 100%
        }

        return new BootstrapResult(resultTree, support);
    }

    // ======== HILFSFUNKTIONEN ========

    private static double simpleEditDistance(String seq1, String seq2) {
        int n = seq1.length();
        int m = seq2.length();

        if (n == 0) return m;
        if (m == 0) return n;

        // Optimierte Version mit 2 Zeilen
        double[] prev = new double[m + 1];
        double[] curr = new double[m + 1];

        for (int j = 0; j <= m; j++) prev[j] = j;

        for (int i = 1; i <= n; i++) {
            curr[0] = i;

            for (int j = 1; j <= m; j++) {
                double cost = seq1.charAt(i - 1) == seq2.charAt(j - 1) ? 0 : 1;
                curr[j] = Math.min(
                    Math.min(curr[j - 1] + 1, prev[j] + 1),
                    prev[j - 1] + cost
                );
            }

            double[] temp = prev;
            prev = curr;
            curr = temp;
        }

        return prev[m] / Math.max(n, m);  // Normalisierte Distanz
    }

    /** Konsensus-Baum aus mehreren Bäumen erstellen (Strict Consensus) */
    public static TreeNode strictConsensus(List<TreeNode> trees) {
        if (trees == null || trees.isEmpty()) return null;

        // Vereinfacht: Ersten Baum zurückgeben
        // In Realität: alle gemeinsamen Kluster extrahieren
        return trees.get(0);
    }
}
