package com.snapious.visualization;

import com.snapious.analysis.Phylogeny;

import javax.swing.*;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.BasicStroke;
import java.awt.RenderingHints;
import java.awt.geom.Point2D;
import java.util.List;
import java.util.ArrayList;
import java.util.Map;
import java.util.HashMap;

/**
 * Visualizes phylogenetic trees on a Swing canvas.
 * Supports Newick string input and TreeNode objects.
 */
public class TreeVisualizer extends JComponent {

    private Phylogeny.TreeNode tree;

    // Layout parameters
    private double nodeRadius = 6;
    private Color textColor = new Color(248, 250, 252);
    private Color lineColor = new Color(96, 165, 250);
    private Color highlightColor = new Color(245, 158, 11);
    private Color internalNodeColor = new Color(71, 85, 105);

    public TreeVisualizer() {
        setBackground(new Color(15, 23, 42));
    }

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g);

        Graphics2D gc = (Graphics2D) g;
        gc.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);

        double width = getWidth();
        double height = getHeight();

        if (tree == null) return;

        // Compute layout
        LayoutInfo layout = calculateLayout(tree, width, height);

        // Draw lines
        drawBranches(gc, layout, tree);

        // Draw nodes
        drawNodes(gc, layout, tree);

        // Draw labels
        drawLabels(gc, layout, tree);
    }

    /** Draws a phylogenetic tree */
    public void drawTree(Phylogeny.TreeNode root) {
        this.tree = root;
        repaint();
    }

    /** Draws a Newick string */
    public void drawNewick(String newickTree) {
        Phylogeny.TreeNode root = parseNewick(newickTree);

        if (root != null) {
            drawTree(root);
        }
    }

    // ======== LAYOUT CALCULATION ========

    private static class LayoutInfo {
        Map<Phylogeny.TreeNode, Point2D> positions;

        LayoutInfo() {
            this.positions = new HashMap<>();
        }

        void put(Phylogeny.TreeNode node, Point2D pos) {
            positions.put(node, pos);
        }

        Point2D get(Phylogeny.TreeNode node) {
            return positions.get(node);
        }

        Map<Phylogeny.TreeNode, Point2D> getAll() {
            return positions;
        }
    }

    /** Computes positions of all nodes for a rectangular layout */
    private LayoutInfo calculateLayout(Phylogeny.TreeNode root, double width, double height) {
        LayoutInfo layout = new LayoutInfo();

        // Find all leaves and distribute them horizontally
        List<Phylogeny.TreeNode> leaves = new ArrayList<>();
        collectLeaves(root, leaves);

        if (leaves.isEmpty()) return layout;

        // Compute horizontal positions
        double marginX = 120;  // space for labels on the left
        double marginY = 40;   // top/bottom
        double usableWidth = width - marginX * 2;
        double usableHeight = height - marginY * 2;

        double leafSpacing = (leaves.size() > 1) ? usableHeight / (leaves.size() - 1) : 0;

        for (int i = 0; i < leaves.size(); i++) {
            Phylogeny.TreeNode leaf = leaves.get(i);

            double x, y;

            if (leaves.size() == 1) {
                // Single leaf - middle right
                x = marginX + usableWidth;
                y = height / 2;
            } else {
                // Several leaves - distribute evenly
                x = marginX + usableWidth;
                y = marginY + (i * leafSpacing);

                // Adjust if there are too few leaves
                if (leaves.size() < 4) {
                    double extraSpace = usableHeight - (leaves.size() - 1) * leafSpacing;
                    y = marginY + extraSpace / 2 + (i * leafSpacing);

                    double clampedY = Math.max(marginY,
                            Math.min(height - marginY - 30, y));

                    layout.put(leaf, new Point2D.Double(x, clampedY));
                } else {
                    layout.put(leaf, new Point2D.Double(x, y));
                }
            }

            if (leaves.size() > 1 && leaves.size() >= 4) {
                layout.put(leaf, new Point2D.Double(x, y));
            }
        }

        // Position internal nodes (recursively)
        positionInternalNodes(root, layout);

        return layout;
    }

    private void collectLeaves(Phylogeny.TreeNode node, List<Phylogeny.TreeNode> leaves) {
        if (node.isLeaf()) {
            leaves.add(node);
            return;
        }

        for (Phylogeny.TreeNode child : node.getChildren()) {
            collectLeaves(child, leaves);
        }
    }

    private void positionInternalNodes(Phylogeny.TreeNode node, LayoutInfo layout) {
        if (node.isLeaf()) return;

        // Get child positions
        List<Point2D> childPositions = new ArrayList<>();

        for (Phylogeny.TreeNode child : node.getChildren()) {
            Point2D pos = layout.get(child);

            if (pos != null) {
                childPositions.add(pos);
            } else {
                // Position recursively if not yet done
                positionInternalNodes(child, layout);

                pos = layout.get(child);
                if (pos != null) childPositions.add(pos);
            }
        }

        // Parent position = mean Y, X from branch length
        if (!childPositions.isEmpty()) {
            double avgY = childPositions.stream().mapToDouble(p -> p.getY()).average().orElse(0);

            // Branch length from the tree
            double branchLen = node.getBranchLength();

            // X position based on the max branch length of the children
            double maxX = childPositions.stream().mapToDouble(p -> p.getX()).max().orElse(0);

            double parentX = Math.max(50, maxX - branchLen * 100);

            layout.put(node, new Point2D.Double(parentX, avgY));
        }
    }

    // ======== DRAWING FUNCTIONS ========

    private void drawBranches(Graphics2D gc, LayoutInfo layout, Phylogeny.TreeNode node) {
        if (node.isLeaf()) return;

        Point2D parentPos = layout.get(node);

        for (Phylogeny.TreeNode child : node.getChildren()) {
            Point2D childPos = layout.get(child);

            if (parentPos != null && childPos != null) {
                gc.setColor(lineColor);
                gc.setStroke(new BasicStroke(2));

                gc.drawLine((int) parentPos.getX(), (int) parentPos.getY(),
                            (int) childPos.getX(), (int) childPos.getY());

                // Draw children recursively
                drawBranches(gc, layout, child);
            }
        }
    }

    private void drawNodes(Graphics2D gc, LayoutInfo layout, Phylogeny.TreeNode node) {
        Point2D pos = layout.get(node);

        if (pos != null) {
            // Leaf node
            if (node.isLeaf()) {
                gc.setColor(highlightColor);
                gc.fillOval((int) (pos.getX() - nodeRadius), (int) (pos.getY() - nodeRadius),
                        (int) (nodeRadius * 2), (int) (nodeRadius * 2));

                gc.setColor(Color.WHITE);
                gc.setStroke(new BasicStroke(1.5f));
                gc.drawOval((int) (pos.getX() - nodeRadius), (int) (pos.getY() - nodeRadius),
                        (int) (nodeRadius * 2), (int) (nodeRadius * 2));

            } else {
                // Internal nodes - smaller circle
                gc.setColor(internalNodeColor);
                gc.fillOval((int) (pos.getX() - nodeRadius / 2), (int) (pos.getY() - nodeRadius / 2),
                        (int) nodeRadius, (int) nodeRadius);

                gc.setColor(lineColor);
                gc.setStroke(new BasicStroke(1.5f));
                gc.drawOval((int) (pos.getX() - nodeRadius / 2), (int) (pos.getY() - nodeRadius / 2),
                        (int) nodeRadius, (int) nodeRadius);

                // Draw children recursively
                for (Phylogeny.TreeNode child : node.getChildren()) {
                    drawNodes(gc, layout, child);
                }
            }
        }
    }

    private void drawLabels(Graphics2D gc, LayoutInfo layout, Phylogeny.TreeNode node) {
        Point2D pos = layout.get(node);

        if (pos != null && node.getName() != null && !node.getName().isEmpty()) {
            gc.setColor(textColor);
            gc.setFont(new Font("Monospaced", Font.PLAIN, 12));

            String label = node.getName();

            // Label to the right of the node
            double textX = pos.getX() + nodeRadius + 5;

            // Adjust Y position for vertical centering
            gc.drawString(label, (int) textX, (int) (pos.getY() + 4));

            // Show support value if present
            if (node.getSupport() < 100) {
                gc.setColor(highlightColor);
                gc.setFont(new Font("Monospaced", Font.PLAIN, 10));

                String supportLabel = String.format("[%.0f%%]", node.getSupport());
                gc.drawString(supportLabel, (int) textX, (int) (pos.getY() - 8));

                gc.setFont(new Font("Monospaced", Font.PLAIN, 12));
            }
        }

        // Draw child labels recursively
        if (!node.isLeaf()) {
            for (Phylogeny.TreeNode child : node.getChildren()) {
                drawLabels(gc, layout, child);
            }
        }
    }

    // ======== NEWICK PARSER ========

    /** Parses a Newick string into a TreeNode */
    public Phylogeny.TreeNode parseNewick(String newick) {
        if (newick == null || newick.trim().isEmpty()) return null;

        String cleaned = newick.trim();
        if (!cleaned.endsWith(";")) {
            cleaned += ";";
        }

        return parseNode(cleaned.toCharArray(), new int[]{0});
    }

    private Phylogeny.TreeNode parseNode(char[] chars, int[] pos) {
        skipWhitespace(chars, pos);

        if (chars[pos[0]] == '(') {
            // Internal node
            pos[0]++;  // skip (

            Phylogeny.TreeNode node = new Phylogeny.TreeNode("", 0);

            // First child
            Phylogeny.TreeNode child1 = parseNode(chars, pos);
            node.addChild(child1);

            skipWhitespace(chars, pos);

            if (chars[pos[0]] == ',') {
                pos[0]++;  // skip comma

                // Second child
                Phylogeny.TreeNode child2 = parseNode(chars, pos);
                node.addChild(child2);

            } else if (chars[pos[0]] == ')') {
                // Only one child - unlikely but possible
            }

            skipWhitespace(chars, pos);

            // Label if present
            String label = parseLabel(chars, pos);

            if (label != null && !label.isEmpty()) {
                node.setName(label);
            }

            skipWhitespace(chars, pos);

            // Branch length if present
            if (pos[0] < chars.length && chars[pos[0]] == ':') {
                pos[0]++;  // skip :

                String lengthStr = parseNumber(chars, pos);
                if (lengthStr != null) {
                    try {
                        node.setBranchLength(Double.parseDouble(lengthStr));
                    } catch (NumberFormatException e) {
                        // Ignore
                    }
                }
            }

            skipWhitespace(chars, pos);

            if (pos[0] < chars.length && chars[pos[0]] == ')') {
                pos[0]++;  // skip ) (if present)
            }

            return node;

        } else {
            // Leaf node
            String label = parseLabel(chars, pos);

            skipWhitespace(chars, pos);

            Phylogeny.TreeNode node;

            if (label == null || label.isEmpty()) {
                node = new Phylogeny.TreeNode("(unnamed)", 0);
            } else {
                node = new Phylogeny.TreeNode(label, 0);
            }

            // Branch length if present
            if (pos[0] < chars.length && chars[pos[0]] == ':') {
                pos[0]++;  // skip :

                String lengthStr = parseNumber(chars, pos);
                if (lengthStr != null) {
                    try {
                        node.setBranchLength(Double.parseDouble(lengthStr));
                    } catch (NumberFormatException e) {
                        // Ignore
                    }
                }
            }

            return node;
        }
    }

    private String parseLabel(char[] chars, int[] pos) {
        skipWhitespace(chars, pos);

        if (pos[0] >= chars.length || chars[pos[0]] == ',' ||
            chars[pos[0]] == ')' || chars[pos[0]] == ';' ||
            chars[pos[0]] == ':') {
            return null;
        }

        StringBuilder sb = new StringBuilder();

        while (pos[0] < chars.length) {
            char c = chars[pos[0]];

            if (c == ',' || c == ')' || c == ';' || c == ':') {
                break;
            }

            sb.append(c);
            pos[0]++;
        }

        String label = sb.toString().trim();
        return label.isEmpty() ? null : label;
    }

    private String parseNumber(char[] chars, int[] pos) {
        skipWhitespace(chars, pos);

        if (pos[0] >= chars.length || !isDigitOrMinus(chars[pos[0]])) {
            return null;
        }

        StringBuilder sb = new StringBuilder();

        while (pos[0] < chars.length) {
            char c = chars[pos[0]];

            if (!isDigitOrMinus(c) && c != '.' && c != 'e' && c != 'E' && c != '+') {
                break;
            }

            sb.append(c);
            pos[0]++;
        }

        return sb.toString().trim();
    }

    private void skipWhitespace(char[] chars, int[] pos) {
        while (pos[0] < chars.length && Character.isWhitespace(chars[pos[0]])) {
            pos[0]++;
        }
    }

    private boolean isDigitOrMinus(char c) {
        return Character.isDigit(c) || c == '-';
    }

    // ======== GETTERS/SETTERS ========

    public void setNodeRadius(double radius) {
        this.nodeRadius = Math.max(2, radius);
    }

    public void setTextColor(Color color) {
        this.textColor = color;
    }

    public void setLineColor(Color color) {
        this.lineColor = color;
    }

    public void setHighlightColor(Color color) {
        this.highlightColor = color;
    }

    /** Update canvas size and redraw */
    public void resize() {
        if (tree != null) {
            drawTree(tree);
        }
    }
}
