package com.snapious.analysis;

import com.snapious.core.Sequence;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.ThreadLocalRandom;

class PhylogenyTest {

    private Sequence makeSeq(String name, String bases) {
        return new Sequence(bases, name, Sequence.SequenceType.DNA);
    }

    @Test
    void testTreeNodeLeaf() {
        var node = new Phylogeny.TreeNode("Human", 0.1);

        assertTrue(node.isLeaf());
        assertEquals("Human", node.getName());
        assertEquals(0.1, node.getBranchLength(), 0.001);
        assertEquals(1, node.leafCount());

        List<String> leaves = node.leafNames();
        assertEquals(1, leaves.size());
        assertEquals("Human", leaves.get(0));
    }

    @Test
    void testTreeNodeInternal() {
        var child1 = new Phylogeny.TreeNode("Human", 0.1);
        var child2 = new Phylogeny.TreeNode("Mouse", 0.2);

        var parent = new Phylogeny.TreeNode("", 0.05);
        parent.addChild(child1);
        parent.addChild(child2);

        assertFalse(parent.isLeaf());
        assertEquals(2, parent.leafCount());

        List<String> leaves = parent.leafNames();
        assertEquals(2, leaves.size());
        assertTrue(leaves.contains("Human"));
        assertTrue(leaves.contains("Mouse"));
    }

    @Test
    void testTreeNodeToNewick() {
        var child1 = new Phylogeny.TreeNode("Human", 0.1);
        var child2 = new Phylogeny.TreeNode("Mouse", 0.2);

        var parent = new Phylogeny.TreeNode("", 0.05);
        parent.addChild(child1);
        parent.addChild(child2);

        String newick = parent.toNewick();

        assertNotNull(newick);
        assertTrue(newick.contains("Human"));
        assertTrue(newick.contains("Mouse"));
    }

    @Test
    void testTreeNodeToTextDiagram() {
        var child1 = new Phylogeny.TreeNode("Human", 0.1);
        var child2 = new Phylogeny.TreeNode("Mouse", 0.2);

        var parent = new Phylogeny.TreeNode("", 0.05);
        parent.addChild(child1);
        parent.addChild(child2);

        String diagram = parent.toTextDiagram(0);

        assertNotNull(diagram);
        assertTrue(diagram.contains("Human"));
        assertTrue(diagram.contains("Mouse"));
    }

    @Test
    void testJukesCantorIdentical() {
        double dist = Phylogeny.jukesCantorDistance("ATCG", "ATCG");

        assertEquals(0.0, dist, 0.001);
    }

    @Test
    void testJukesCantorDifferent() {
        double dist = Phylogeny.jukesCantorDistance("AAAA", "TTTT");

        assertTrue(dist > 0);
    }

    @Test
    void testJukesCantorSaturated() {
        // Sehr unterschiedliche Sequenzen sollten saturieren
        StringBuilder sb1 = new StringBuilder();
        StringBuilder sb2 = new StringBuilder();

        for (int i = 0; i < 1000; i++) {
            sb1.append("A");
        }

        for (int i = 0; i < 1000; i++) {
            sb2.append(i % 4 == 0 ? "A" : "G");
        }

        double dist = Phylogeny.jukesCantorDistance(sb1.toString(), sb2.toString());

        assertTrue(dist > 0);
    }

    @Test
    void testKimura2ParamDistance() {
        double dist = Phylogeny.kimura2ParamDistance("ATCG", "ATCG");

        assertEquals(0.0, dist, 0.001);
    }

    @Test
    void testKimura2ParamWithTransitions() {
        // A <-> G ist eine Transition
        double dist = Phylogeny.kimura2ParamDistance("AATT", "GGTT");

        assertTrue(dist > 0);
    }

    @Test
    void testPercentIdentity() {
        assertEquals(100.0, Phylogeny.percentIdentity("ATCG", "ATCG"));

        double identity = Phylogeny.percentIdentity("ATCG", "AATG");

        assertTrue(identity < 100);
        assertTrue(identity >= 0);
    }

    @Test
    void testDistanceMatrixIdentical() {
        List<Sequence> seqs = List.of(
            makeSeq("A", "ATCG"),
            makeSeq("B", "ATCG"),
            makeSeq("C", "ATCG")
        );

        var dm = Phylogeny.createDistanceMatrix(seqs, "Edit-Distance");

        assertEquals(3, dm.size());
        assertEquals(0, dm.getDistance(0, 1), 0.001);
        assertEquals(0, dm.getDistance(1, 2), 0.001);
    }

    @Test
    void testDistanceMatrixDifferent() {
        List<Sequence> seqs = List.of(
            makeSeq("A", "AAAA"),
            makeSeq("B", "TTTT")
        );

        var dm = Phylogeny.createDistanceMatrix(seqs, "Edit-Distance");

        assertTrue(dm.getDistance(0, 1) > 0);
    }

    @Test
    void testDistanceMatrixFormat() {
        List<Sequence> seqs = List.of(
            makeSeq("Human", "ATCG"),
            makeSeq("Mouse", "AATG")
        );

        var dm = Phylogeny.createDistanceMatrix(seqs, "Edit-Distance");
        String formatted = dm.format();

        assertNotNull(formatted);
        assertTrue(formatted.contains("Human"));
        assertTrue(formatted.contains("Mouse"));
    }

    @Test
    void testDistanceMatrixToJson() {
        List<Sequence> seqs = List.of(
            makeSeq("A", "AT"),
            makeSeq("B", "CG")
        );

        var dm = Phylogeny.createDistanceMatrix(seqs, "Edit-Distance");
        String json = dm.toJson();

        assertNotNull(json);
        assertTrue(json.contains("labels"));
        assertTrue(json.contains("matrix"));
    }

    @Test
    void testNeighborJoiningTwoSequences() {
        List<Sequence> seqs = List.of(
            makeSeq("Human", "ATCG"),
            makeSeq("Mouse", "AATG")
        );

        var tree = Phylogeny.neighborJoining(seqs);

        assertNotNull(tree);
        List<String> leaves = tree.leafNames();
        assertEquals(2, leaves.size());
    }

    @Test
    void testNeighborJoiningSingleSequence() {
        List<Sequence> seqs = List.of(makeSeq("Only", "ATCG"));

        var tree = Phylogeny.neighborJoining(seqs);

        assertNotNull(tree);
        assertEquals(1, tree.leafCount());
    }

    @Test
    void testNeighborJoiningMultipleSequences() {
        List<Sequence> seqs = List.of(
            makeSeq("Human", "AAAAGGGG"),
            makeSeq("Mouse", "AAATGGGC"),
            makeSeq("Chicken", "GGGAAAAA")
        );

        var tree = Phylogeny.neighborJoining(seqs);

        assertNotNull(tree);
        assertEquals(3, tree.leafCount());

        String newick = tree.toNewick();
        assertNotNull(newick);
    }

    @Test
    void testUPGMA() {
        List<Sequence> seqs = List.of(
            makeSeq("A", "ATCG"),
            makeSeq("B", "ATCG"),
            makeSeq("C", "GCTA")
        );

        var tree = Phylogeny.upgma(seqs);

        assertNotNull(tree);
    }

    @Test
    void testUPGMATwoSequences() {
        List<Sequence> seqs = List.of(
            makeSeq("A", "ATCG"),
            makeSeq("B", "GCTA")
        );

        var tree = Phylogeny.upgma(seqs);

        assertNotNull(tree);
        assertEquals(2, tree.leafCount());
    }

    @Test
    void testMaximumParsimony() {
        List<Sequence> seqs = List.of(
            makeSeq("A", "ATCG"),
            makeSeq("B", "AATG"),
            makeSeq("C", "GGTT")
        );

        var tree = Phylogeny.maximumParsimony(seqs);

        assertNotNull(tree);
    }

    @Test
    void testBootstrapAnalysis() {
        List<Sequence> seqs = List.of(
            makeSeq("Human", "AAAAGGGGCCCC"),
            makeSeq("Mouse", "AAATGGGCCCGA"),
            makeSeq("Chicken", "GGGAAAAACCCC")
        );

        var result = Phylogeny.bootstrapAnalysis(seqs, 10);

        assertNotNull(result);
        assertNotNull(result.getTree());

        String formatted = result.format();
        assertNotNull(formatted);
    }

    @Test
    void testTreeNodeSupport() {
        var node = new Phylogeny.TreeNode("Test", 0.1);

        assertEquals(100, node.getSupport());

        node.setSupport(75);
        assertEquals(75, node.getSupport());

        // Boundary: sollte auf 0-100 gekappt werden
        node.setSupport(150);
        assertEquals(100, node.getSupport());

        node.setSupport(-10);
        assertEquals(0, node.getSupport());
    }

    @Test
    void testTreeNodeSetBranchLength() {
        var node = new Phylogeny.TreeNode("Test", 0.1);

        assertEquals(0.1, node.getBranchLength(), 0.001);

        node.setBranchLength(0.5);
        assertEquals(0.5, node.getBranchLength(), 0.001);

        // Negative sollte auf 0 gekappt werden
        node.setBranchLength(-1);
        assertEquals(0, node.getBranchLength(), 0.001);
    }

    @Test
    void testLargeTreePerformance() {
        List<Sequence> seqs = java.util.stream.IntStream.range(0, 10)
            .mapToObj(i -> makeSeq("Seq" + i, generateRandomDNA(50)))
            .toList();

        long startTime = System.currentTimeMillis();
        var tree = Phylogeny.neighborJoining(seqs);
        long elapsed = System.currentTimeMillis() - startTime;

        assertTrue(elapsed < 10000, "Tree sollte innerhalb von 10s gebaut werden");
        assertNotNull(tree);

        String newick = tree.toNewick();
        assertNotNull(newick);
    }

    @Test
    void testStrictConsensus() {
        var seqs = List.of(
            makeSeq("A", "ATCG"),
            makeSeq("B", "GCTA")
        );

        var tree1 = Phylogeny.neighborJoining(seqs);
        List<Phylogeny.TreeNode> trees = List.of(tree1);

        var consensus = Phylogeny.strictConsensus(trees);

        assertNotNull(consensus);
    }

    // ======== HILFSFUNKTIONEN ========

    private String generateRandomDNA(int length) {
        StringBuilder sb = new StringBuilder(length);
        String bases = "ATCG";

        for (int i = 0; i < length; i++) {
            sb.append(bases.charAt(ThreadLocalRandom.current().nextInt(4)));
        }

        return sb.toString();
    }
}
