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
 * Visualisiert Phylogenie-Bäume auf einem Swing Canvas.
 * Unterstützt Newick-String-Eingabe und TreeNode-Objekte.
 */
public class TreeVisualizer extends JComponent {

    private Phylogeny.TreeNode tree;

    // Layout-Parameter
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

        // Layout berechnen
        LayoutInfo layout = calculateLayout(tree, width, height);

        // Linien zeichnen
        drawBranches(gc, layout, tree);

        // Knoten zeichnen
        drawNodes(gc, layout, tree);

        // Labels zeichnen
        drawLabels(gc, layout, tree);
    }

    /** Zeichnet einen Phylogenie-Baum */
    public void drawTree(Phylogeny.TreeNode root) {
        this.tree = root;
        repaint();
    }

    /** Zeichnet einen Newick-String */
    public void drawNewick(String newickTree) {
        Phylogeny.TreeNode root = parseNewick(newickTree);

        if (root != null) {
            drawTree(root);
        }
    }

    // ======== LAYOUT-BERECHNUNG ========

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

    /** Berechnet Positionen aller Knoten für rechteckiges Layout */
    private LayoutInfo calculateLayout(Phylogeny.TreeNode root, double width, double height) {
        LayoutInfo layout = new LayoutInfo();

        // Alle Blätter finden und horizontal verteilen
        List<Phylogeny.TreeNode> leaves = new ArrayList<>();
        collectLeaves(root, leaves);

        if (leaves.isEmpty()) return layout;

        // Horizontale Positionen berechnen
        double marginX = 120;  // Platz für Labels links
        double marginY = 40;   // Oben/unten
        double usableWidth = width - marginX * 2;
        double usableHeight = height - marginY * 2;

        double leafSpacing = (leaves.size() > 1) ? usableHeight / (leaves.size() - 1) : 0;

        for (int i = 0; i < leaves.size(); i++) {
            Phylogeny.TreeNode leaf = leaves.get(i);

            double x, y;

            if (leaves.size() == 1) {
                // Einzelnes Blatt - Mitte rechts
                x = marginX + usableWidth;
                y = height / 2;
            } else {
                // Mehrere Blätter - gleichmäßig verteilen
                x = marginX + usableWidth;
                y = marginY + (i * leafSpacing);

                // Anpassen falls zu wenig Blätter
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

        // Interne Knoten positionieren (rekursiv)
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

        // Kinder-Positionen holen
        List<Point2D> childPositions = new ArrayList<>();

        for (Phylogeny.TreeNode child : node.getChildren()) {
            Point2D pos = layout.get(child);

            if (pos != null) {
                childPositions.add(pos);
            } else {
                // Rekursiv positionieren falls noch nicht geschehen
                positionInternalNodes(child, layout);

                pos = layout.get(child);
                if (pos != null) childPositions.add(pos);
            }
        }

        // Eltern-Position = mittlerer Y, X aus Branch-Länge
        if (!childPositions.isEmpty()) {
            double avgY = childPositions.stream().mapToDouble(p -> p.getY()).average().orElse(0);

            // Branch-Länge aus dem Baum
            double branchLen = node.getBranchLength();

            // X-Position basierend auf max Branch-Länge der Kinder
            double maxX = childPositions.stream().mapToDouble(p -> p.getX()).max().orElse(0);

            double parentX = Math.max(50, maxX - branchLen * 100);

            layout.put(node, new Point2D.Double(parentX, avgY));
        }
    }

    // ======== ZEICHENFUNKTIONEN ========

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

                // Rekursiv Kinder zeichnen
                drawBranches(gc, layout, child);
            }
        }
    }

    private void drawNodes(Graphics2D gc, LayoutInfo layout, Phylogeny.TreeNode node) {
        Point2D pos = layout.get(node);

        if (pos != null) {
            // Blatt-Knoten
            if (node.isLeaf()) {
                gc.setColor(highlightColor);
                gc.fillOval((int) (pos.getX() - nodeRadius), (int) (pos.getY() - nodeRadius),
                        (int) (nodeRadius * 2), (int) (nodeRadius * 2));

                gc.setColor(Color.WHITE);
                gc.setStroke(new BasicStroke(1.5f));
                gc.drawOval((int) (pos.getX() - nodeRadius), (int) (pos.getY() - nodeRadius),
                        (int) (nodeRadius * 2), (int) (nodeRadius * 2));

            } else {
                // Interne Knoten - kleinerer Kreis
                gc.setColor(internalNodeColor);
                gc.fillOval((int) (pos.getX() - nodeRadius / 2), (int) (pos.getY() - nodeRadius / 2),
                        (int) nodeRadius, (int) nodeRadius);

                gc.setColor(lineColor);
                gc.setStroke(new BasicStroke(1.5f));
                gc.drawOval((int) (pos.getX() - nodeRadius / 2), (int) (pos.getY() - nodeRadius / 2),
                        (int) nodeRadius, (int) nodeRadius);

                // Rekursiv Kinder zeichnen
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

            // Label rechts vom Knoten
            double textX = pos.getX() + nodeRadius + 5;

            // Y-Position anpassen für vertikales Zentrieren
            gc.drawString(label, (int) textX, (int) (pos.getY() + 4));

            // Support-Wert anzeigen falls vorhanden
            if (node.getSupport() < 100) {
                gc.setColor(highlightColor);
                gc.setFont(new Font("Monospaced", Font.PLAIN, 10));

                String supportLabel = String.format("[%.0f%%]", node.getSupport());
                gc.drawString(supportLabel, (int) textX, (int) (pos.getY() - 8));

                gc.setFont(new Font("Monospaced", Font.PLAIN, 12));
            }
        }

        // Rekursiv Kinder labels zeichnen
        if (!node.isLeaf()) {
            for (Phylogeny.TreeNode child : node.getChildren()) {
                drawLabels(gc, layout, child);
            }
        }
    }

    // ======== NEWICK-PARSER ========

    /** Parsed Newick-String zu TreeNode */
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
            // Interner Knoten
            pos[0]++;  // ( überspringen

            Phylogeny.TreeNode node = new Phylogeny.TreeNode("", 0);

            // Erstes Kind
            Phylogeny.TreeNode child1 = parseNode(chars, pos);
            node.addChild(child1);

            skipWhitespace(chars, pos);

            if (chars[pos[0]] == ',') {
                pos[0]++;  // Komma überspringen

                // Zweites Kind
                Phylogeny.TreeNode child2 = parseNode(chars, pos);
                node.addChild(child2);

            } else if (chars[pos[0]] == ')') {
                // Nur ein Kind - unwahrscheinlich aber möglich
            }

            skipWhitespace(chars, pos);

            // Label falls vorhanden
            String label = parseLabel(chars, pos);

            if (label != null && !label.isEmpty()) {
                node.setName(label);
            }

            skipWhitespace(chars, pos);

            // Branch-Länge falls vorhanden
            if (pos[0] < chars.length && chars[pos[0]] == ':') {
                pos[0]++;  // : überspringen

                String lengthStr = parseNumber(chars, pos);
                if (lengthStr != null) {
                    try {
                        node.setBranchLength(Double.parseDouble(lengthStr));
                    } catch (NumberFormatException e) {
                        // Ignorieren
                    }
                }
            }

            skipWhitespace(chars, pos);

            if (pos[0] < chars.length && chars[pos[0]] == ')') {
                pos[0]++;  // ) überspringen (falls vorhanden)
            }

            return node;

        } else {
            // Blatt-Knoten
            String label = parseLabel(chars, pos);

            skipWhitespace(chars, pos);

            Phylogeny.TreeNode node;

            if (label == null || label.isEmpty()) {
                node = new Phylogeny.TreeNode("(unbenannt)", 0);
            } else {
                node = new Phylogeny.TreeNode(label, 0);
            }

            // Branch-Länge falls vorhanden
            if (pos[0] < chars.length && chars[pos[0]] == ':') {
                pos[0]++;  // : überspringen

                String lengthStr = parseNumber(chars, pos);
                if (lengthStr != null) {
                    try {
                        node.setBranchLength(Double.parseDouble(lengthStr));
                    } catch (NumberFormatException e) {
                        // Ignorieren
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

    // ======== GETTER/SETTER ========

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

    /** Canvas-Größe aktualisieren und neu zeichnen */
    public void resize() {
        if (tree != null) {
            drawTree(tree);
        }
    }
}
