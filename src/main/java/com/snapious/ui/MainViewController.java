package com.snapious.ui;

import com.snapious.core.Sequence;
import com.snapious.alignment.Alignment;
import com.snapious.analysis.Phylogeny;
import com.snapious.io.FileIO;

import javax.swing.*;
import javax.swing.border.EmptyBorder;
import javax.swing.filechooser.FileNameExtensionFilter;
import javax.swing.table.DefaultTableModel;
import java.awt.*;
import java.awt.datatransfer.Clipboard;
import java.awt.datatransfer.ClipboardOwner;
import java.awt.datatransfer.StringSelection;
import java.awt.datatransfer.Transferable;
import java.io.File;
import java.nio.file.*;
import java.util.*;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Haupt-Controller für Snapious BioTools UI.
 * Verbindet alle Analyse-Module mit der Benutzeroberfläche (Swing).
 */
public class MainViewController implements ClipboardOwner {

    // ======== UI COMPONENTS ========

    private JFrame mainFrame;
    private JPanel mainPanel;

    // Left panel: Sequence list & controls
    private JList<String> sequenceList;
    private DefaultListModel<String> sequencesModel;
    private JTextField newSeqNameField;
    private JTextArea searchField;

    // Detail panel (left bottom)
    private JLabel detailIdLabel, detailTypeLabel, detailLengthLabel, detailGCLabel, detailBaseDistLabel;
    private JProgressBar gcProgressBar;

    // Center: Dashboard & tools tabs
    private JTextArea quickSeqInput, quickAnalysisOutput;
    private JTabbedPane tabbedPane;

    // Tools panel (right)
    private JTextField matchScoreField, mismatchPenaltyField, gapPenaltyField;
    private JTextArea alignmentOutput, treeOutput, distanceMatrixOutput;
    private JTextField bootstrapField;
    private JComboBox<String> treeMethodCombo;

    // Lists
    private JList<String> orfList, restrictionList;
    private DefaultListModel<String> orfModel, restrictionModel;

    // Status bar
    private JLabel statusTextLabel, statusSeqCountLabel;

    // State
    private Map<String, Sequence> sequenceMap = new LinkedHashMap<>();
    private String selectedSequenceId = null;

    // Align-Parameter
    private int matchScore = 2;
    private int mismatchPenalty = -1;
    private int gapPenalty = -1;

    /**
     * Returns the main Swing panel to be added to a JFrame.
     */
    public JPanel getPanel() {
        return buildUI();
    }

    // ======== UI CONSTRUCTION ========

    private JPanel buildUI() {
        mainPanel = new JPanel(new BorderLayout(10, 10));
        mainPanel.setBorder(new EmptyBorder(10, 10, 10, 10));
        mainPanel.setBackground(new Color(30, 41, 59));

        // === LEFT PANEL (Sequenzen verwalten) ===
        JPanel leftPanel = new JPanel(new BorderLayout(10, 10));
        leftPanel.setBackground(new Color(30, 41, 59));

        sequencesModel = new DefaultListModel<>();
        sequenceList = new JList<>(sequencesModel);
        sequenceList.setSelectionMode(ListSelectionModel.SINGLE_SELECTION);
        sequenceList.setBackground(new Color(15, 23, 42));
        sequenceList.setForeground(Color.WHITE);
        sequenceList.setFont(new Font("Monospaced", Font.PLAIN, 12));
        sequenceList.addListSelectionListener(e -> {
            if (!e.getValueIsAdjusting()) {
                String id = sequenceList.getSelectedValue();
                if (id != null) handleSequenceSelection(id);
            }
        });
        JScrollPane listScrollPane = new JScrollPane(sequenceList);
        listScrollPane.setPreferredSize(new Dimension(300, 250));

        // Neue Sequenz erstellen
        JPanel newSeqPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        newSeqNameField = new JTextField(15);
        JButton newSeqBtn = new JButton("Neue Sequenz");
        newSeqBtn.addActionListener(e -> handleNewSequence());

        importBtn = new JButton("Importieren");
        importBtn.addActionListener(e -> handleImport());

        saveBtn = new JButton("Speichern");
        saveBtn.addActionListener(e -> handleSave());

        newSeqPanel.add(new JLabel("Name:"));
        newSeqPanel.add(newSeqNameField);
        newSeqPanel.add(newSeqBtn);
        newSeqPanel.add(importBtn);
        newSeqPanel.add(saveBtn);

        leftPanel.add(listScrollPane, BorderLayout.CENTER);
        leftPanel.add(newSeqPanel, BorderLayout.NORTH);

        // Detail-Panel (unten links)
        JPanel detailPanel = new JPanel(new GridLayout(5, 2, 10, 5));
        detailPanel.setBackground(new Color(30, 41, 59));

        detailIdLabel = new JLabel("-");
        detailTypeLabel = new JLabel("-");
        detailLengthLabel = new JLabel("-");
        detailGCLabel = new JLabel("-");
        detailBaseDistLabel = new JLabel("-");

        gcProgressBar = new JProgressBar(0, 100);
        gcProgressBar.setStringPainted(true);
        gcProgressBar.setForeground(new Color(96, 165, 250));

        detailPanel.add(new JLabel("ID:"));
        detailPanel.add(detailIdLabel);
        detailPanel.add(new JLabel("Typ:"));
        detailPanel.add(detailTypeLabel);
        detailPanel.add(new JLabel("Länge:"));
        detailPanel.add(detailLengthLabel);
        detailPanel.add(new JLabel("GC-Gehalt:"));
        detailPanel.add(detailGCLabel);
        detailPanel.add(new JLabel("GC-Fortschritt:"));
        detailPanel.add(gcProgressBar);

        leftPanel.add(detailPanel, BorderLayout.SOUTH);

        // === CENTER PANEL (Tabs) ===
        tabbedPane = new JTabbedPane();
        tabbedPane.setBackground(new Color(30, 41, 59));
        tabbedPane.setForeground(Color.WHITE);

        // Dashboard-Tab
        JPanel dashboardPanel = new JPanel(new BorderLayout(10, 10));
        dashboardPanel.setBackground(new Color(30, 41, 59));

        quickSeqInput = new JTextArea();
        quickSeqInput.setFont(new Font("Monospaced", Font.PLAIN, 12));
        quickSeqInput.setBackground(new Color(15, 23, 42));
        quickSeqInput.setForeground(Color.WHITE);
        quickSeqInput.setLineWrap(true);
        quickSeqInput.setWrapStyleWord(true);

        analyzeBtn = new JButton("Analysieren");
        analyzeBtn.addActionListener(e -> handleQuickAnalysis());

        JPanel inputPanel = new JPanel(new BorderLayout());
        inputPanel.add(new JScrollPane(quickSeqInput), BorderLayout.CENTER);

        quickAnalysisOutput = new JTextArea();
        quickAnalysisOutput.setFont(new Font("Monospaced", Font.PLAIN, 12));
        quickAnalysisOutput.setEditable(false);
        quickAnalysisOutput.setBackground(new Color(15, 23, 42));
        quickAnalysisOutput.setForeground(Color.WHITE);

        dashboardPanel.add(inputPanel, BorderLayout.NORTH);
        dashboardPanel.add(new JScrollPane(quickAnalysisOutput), BorderLayout.CENTER);
        dashboardPanel.add(analyzeBtn, BorderLayout.SOUTH);

        tabbedPane.addTab("Dashboard", dashboardPanel);

        // Alignment-Tab
        JPanel alignPanel = new JPanel(new BorderLayout(10, 10));
        alignPanel.setBackground(new Color(30, 41, 59));

        alignmentOutput = new JTextArea();
        alignmentOutput.setFont(new Font("Monospaced", Font.PLAIN, 12));
        alignmentOutput.setEditable(false);
        alignmentOutput.setBackground(new Color(15, 23, 42));
        alignmentOutput.setForeground(Color.WHITE);

        JPanel alignBtnPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        JButton globalAlignBtn = new JButton("Global Align");
        globalAlignBtn.addActionListener(e -> handleGlobalAlign());
        JButton localAlignBtn = new JButton("Local Align");
        localAlignBtn.addActionListener(e -> handleLocalAlign());

        alignBtnPanel.add(globalAlignBtn);
        alignBtnPanel.add(localAlignBtn);

        alignPanel.add(new JScrollPane(alignmentOutput), BorderLayout.CENTER);
        alignPanel.add(alignBtnPanel, BorderLayout.SOUTH);

        tabbedPane.addTab("Alignment", alignPanel);

        // Tree-Tab
        JPanel treePanel = new JPanel(new BorderLayout(10, 10));
        treePanel.setBackground(new Color(30, 41, 59));

        treeOutput = new JTextArea();
        treeOutput.setFont(new Font("Monospaced", Font.PLAIN, 12));
        treeOutput.setEditable(false);
        treeOutput.setBackground(new Color(15, 23, 42));
        treeOutput.setForeground(Color.WHITE);

        JPanel treeBtnPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        JButton buildTreeBtn = new JButton("Baum erstellen");
        buildTreeBtn.addActionListener(e -> handleParsimonyTree());

        treeMethodCombo = new JComboBox<>(new String[]{"Neighbor-Joining", "UPGMA", "Maximum Parsimony"});

        treeBtnPanel.add(treeMethodCombo);
        treeBtnPanel.add(buildTreeBtn);

        treePanel.add(new JScrollPane(treeOutput), BorderLayout.CENTER);
        treePanel.add(treeBtnPanel, BorderLayout.SOUTH);

        tabbedPane.addTab("Phylogenie", treePanel);

        mainPanel.add(leftPanel, BorderLayout.WEST);
        mainPanel.add(tabbedPane, BorderLayout.CENTER);

        // === RIGHT PANEL (Tools) ===
        JPanel rightPanel = new JPanel(new BorderLayout(10, 10));
        rightPanel.setBackground(new Color(30, 41, 59));

        // Align-Parameter
        JPanel paramPanel = new JPanel(new GridLayout(3, 2, 10, 5));
        paramPanel.setBackground(new Color(30, 41, 59));

        matchScoreField = new JTextField(String.valueOf(matchScore));
        mismatchPenaltyField = new JTextField(String.valueOf(mismatchPenalty));
        gapPenaltyField = new JTextField(String.valueOf(gapPenalty));

        paramPanel.add(new JLabel("Match:"));
        paramPanel.add(matchScoreField);
        paramPanel.add(new JLabel("Mismatch:"));
        paramPanel.add(mismatchPenaltyField);
        paramPanel.add(new JLabel("Gap:"));
        paramPanel.add(gapPenaltyField);

        rightPanel.add(paramPanel, BorderLayout.NORTH);

        // Tools tabs
        JTabbedPane toolsTabs = new JTabbedPane();
        toolsTabs.setBackground(new Color(30, 41, 59));

        // ORF-Tab
        JPanel orfPanel = new JPanel(new BorderLayout());
        orfModel = new DefaultListModel<>();
        orfList = new JList<>(orfModel);
        orfPanel.add(new JScrollPane(orfList), BorderLayout.CENTER);

        JButton findOrfBtn = new JButton("ORF finden");
        findOrfBtn.addActionListener(e -> handleOrfFinder());
        orfPanel.add(findOrfBtn, BorderLayout.SOUTH);

        toolsTabs.addTab("ORF-Finder", orfPanel);

        // Restriktionsenzyme-Tab
        JPanel restrPanel = new JPanel(new BorderLayout());
        restrictionModel = new DefaultListModel<>();
        restrictionList = new JList<>(restrictionModel);
        restrPanel.add(new JScrollPane(restrictionList), BorderLayout.CENTER);

        JButton restrBtn = new JButton("Restriktionsstellen");
        restrBtn.addActionListener(e -> handleRestrictionEnzymes());
        restrPanel.add(restrBtn, BorderLayout.SOUTH);

        toolsTabs.addTab("Restriktionsenzyme", restrPanel);

        // Distance Matrix-Tab
        JPanel matrixPanel = new JPanel(new BorderLayout());
        distanceMatrixOutput = new JTextArea();
        distanceMatrixOutput.setFont(new Font("Monospaced", Font.PLAIN, 12));
        distanceMatrixOutput.setEditable(false);
        distanceMatrixOutput.setBackground(new Color(15, 23, 42));
        distanceMatrixOutput.setForeground(Color.WHITE);

        JButton matrixBtn = new JButton("Distanzmatrix");
        matrixBtn.addActionListener(e -> handleDistanceMatrix());

        matrixPanel.add(new JScrollPane(distanceMatrixOutput), BorderLayout.CENTER);
        matrixPanel.add(matrixBtn, BorderLayout.SOUTH);

        toolsTabs.addTab("Distanzmatrix", matrixPanel);

        rightPanel.add(toolsTabs, BorderLayout.CENTER);
        mainPanel.add(rightPanel, BorderLayout.EAST);

        // === STATUS BAR ===
        JPanel statusBar = new JPanel(new FlowLayout(FlowLayout.LEFT));
        statusBar.setBackground(new Color(15, 23, 42));

        statusTextLabel = new JLabel("Bereit");
        statusSeqCountLabel = new JLabel("Sequenzen: 0");

        helpBtn = new JButton("Hilfe");
        aboutBtn = new JButton("Über");

        helpBtn.addActionListener(e -> handleHelp());
        aboutBtn.addActionListener(e -> handleAbout());

        statusBar.add(statusTextLabel);
        statusBar.add(Box.createHorizontalGlue());
        statusBar.add(statusSeqCountLabel);
        statusBar.add(helpBtn);
        statusBar.add(aboutBtn);

        mainPanel.add(statusBar, BorderLayout.SOUTH);

        return mainPanel;
    }

    // ======== BUTTON REFERENCES (defined in buildUI) ========

    private JButton importBtn, saveBtn, analyzeBtn, helpBtn, aboutBtn;

    // ======== INITIALISIERUNG ========

    public void initialize() {
        updateStats();
        updateStatus();

        // Align-Parameter initialisieren
        try {
            matchScore = Integer.parseInt(matchScoreField.getText());
            mismatchPenalty = Integer.parseInt(mismatchPenaltyField.getText());
            gapPenalty = Integer.parseInt(gapPenaltyField.getText());
        } catch (NumberFormatException e) {
            // Defaults verwenden
        }

        treeMethodCombo.setSelectedItem("Neighbor-Joining");
    }

    // ======== DATEI-OPERATIONEN ========

    private void handleNewSequence() {
        String name = newSeqNameField.getText().trim();
        if (name.isEmpty()) name = "Sequenz_" + (sequenceMap.size() + 1);

        String seqStr = JOptionPane.showInputDialog(mainFrame,
            "Sequenz eingeben (ATCG):", "Neue Sequenz erstellen", JOptionPane.QUESTION_MESSAGE);

        if (seqStr != null) {
            try {
                Sequence seq = new Sequence(
                        "SEQ_" + (sequenceMap.size() + 1),
                        name,
                        seqStr.toUpperCase(),
                        Sequence.SequenceType.DNA
                );
                addSequence(seq);
            } catch (IllegalArgumentException e) {
                showError("Ungültige Sequenz: " + e.getMessage());
            }
        }

        newSeqNameField.setText("");
    }

    private void handleImport() {
        try {
            JFileChooser fileChooser = new JFileChooser();
            fileChooser.addChoosableFileFilter(new FileNameExtensionFilter("FASTA Dateien", "fasta", "fa"));
            fileChooser.addChoosableFileFilter(new FileNameExtensionFilter("FASTQ Dateien", "fastq", "fq"));
            fileChooser.addChoosableFileFilter(new FileNameExtensionFilter("GenBank Dateien", "gb", "genbank"));

            int result = fileChooser.showOpenDialog(mainFrame);
            if (result == JFileChooser.APPROVE_OPTION) {
                File file = fileChooser.getSelectedFile();
                List<Sequence> loaded = FileIO.loadSequences(file.toPath());

                for (int i = 0; i < loaded.size(); i++) {
                    Sequence seq = loaded.get(i);
                    if (seq.getId().startsWith("SEQ_")) {
                        seq.setId(seq.getId() + "_file" + (sequenceMap.size() + 1));
                    }
                    addSequence(seq);
                }

                statusTextLabel.setText("✓ " + loaded.size() + " Sequenz(en) importiert: " + file.getName());
            }
        } catch (Exception e) {
            showError("Import fehlgeschlagen: " + e.getMessage());
        }
    }

    private void handleSave() {
        if (sequenceMap.isEmpty()) return;

        try {
            JFileChooser fileChooser = new JFileChooser();
            fileChooser.addChoosableFileFilter(new FileNameExtensionFilter("FASTA Dateien", "fasta"));

            int result = fileChooser.showSaveDialog(mainFrame);
            if (result == JFileChooser.APPROVE_OPTION) {
                File file = fileChooser.getSelectedFile();
                FileIO.writeFASTA(new ArrayList<>(sequenceMap.values()), file.toPath());
                statusTextLabel.setText("✓ Gespeichert: " + file.getName());
            }
        } catch (Exception e) {
            showError("Speichern fehlgeschlagen: " + e.getMessage());
        }
    }

    // ======== SEQUENZ-OPERATIONEN ========

    private void addSequence(Sequence seq) {
        sequenceMap.put(seq.getId(), seq);
        sequencesModel.addElement(seq.getName() + " (" + seq.length() + " bp, " + seq.getType() + ")");
        updateStats();
    }

    private void handleSequenceSelection(String id) {
        selectedSequenceId = id;
        updateDetailPanel(id);
    }

    private void handleCopySequence() {
        if (selectedSequenceId == null) return;

        Sequence seq = sequenceMap.get(selectedSequenceId);
        if (seq != null) {
            Clipboard clipboard = Toolkit.getDefaultToolkit().getSystemClipboard();
            StringSelection content = new StringSelection(Sequence.toFASTA(seq));
            clipboard.setContents(content, this);
            statusTextLabel.setText("✓ Sequenz in Zwischenablage kopiert");
        }
    }

    // ======== SEQUENZ-ANALYSEN ========

    private void handleQuickAnalysis() {
        String seqStr = quickSeqInput.getText().trim().toUpperCase();
        if (seqStr.isEmpty()) {
            quickAnalysisOutput.setText("Bitte Sequenz eingeben");
            return;
        }

        try {
            Sequence seq = new Sequence("quick", "Schnellanalyse", seqStr, Sequence.SequenceType.DNA);

            StringBuilder sb = new StringBuilder();
            sb.append("=== Schnellanalyse ===\n\n");
            sb.append("Länge: ").append(seq.length()).append("\n");
            sb.append("GC-Gehalt: ").append(String.format("%.1f%%", seq.gcContent() * 100)).append("\n");
            sb.append("AT-Gehalt: ").append(String.format("%.1f%%", seq.atContent() * 100)).append("\n\n");

            // Basen-Verteilung
            sb.append("Basen-Verteilung:\n");
            for (Map.Entry<Character, Double> entry : seq.baseFrequencies().entrySet()) {
                sb.append("  ").append(entry.getKey()).append(": ")
                  .append(String.format("%.1f%%", entry.getValue() * 100)).append("\n");
            }

            // ORFs finden
            List<Sequence> orfs = seq.findORFs(30);
            sb.append("\nGefundene ORFs: ").append(orfs.size()).append("\n");
            for (Sequence orf : orfs) {
                sb.append("  ").append(orf.getName()).append(": ").append(orf.length()).append(" bp\n");
            }

            quickAnalysisOutput.setText(sb.toString());
        } catch (Exception e) {
            quickAnalysisOutput.setText("Analyse fehlgeschlagen: " + e.getMessage());
        }
    }

    private void handleOrfFinder() {
        if (selectedSequenceId == null) return;

        Sequence seq = sequenceMap.get(selectedSequenceId);
        if (seq == null) return;

        List<Sequence> orfs = seq.findORFs(30);  // Mindestlänge 10 Aminosäuren

        orfModel.removeAllElements();
        for (Sequence orf : orfs) {
            orfModel.addElement(orf.getName() + " (Länge: " + orf.length() + ")");
        }

        statusTextLabel.setText(String.format("✓ %d ORFs gefunden", orfs.size()));
    }

    private void handleRestrictionEnzymes() {
        if (selectedSequenceId == null) return;

        Sequence seq = sequenceMap.get(selectedSequenceId);
        if (seq == null) return;

        // Häufige Restriktionsenzyme
        String[] enzymes = {
            "GAATTC",  // EcoRI
            "GGCC",    // BamHI
            "AAGCTT",  // HindIII
            "CGCG",    // SmaI
            "GTCGAC"   // SalI
        };

        restrictionModel.removeAllElements();

        for (String enzyme : enzymes) {
            List<Integer> sites = seq.findRestrictionSites(enzyme);
            if (!sites.isEmpty()) {
                restrictionModel.addElement(
                    enzyme + ": " + sites.size() + " Stellen [" + String.join(", ",
                        sites.stream().map(String::valueOf).collect(Collectors.toList())) + "]"
                );
            }
        }

        statusTextLabel.setText("✓ Restriktionsstellen analysiert");
    }

    // ======== ALIGNMENT ========

    private void handleGlobalAlign() {
        if (sequenceMap.size() < 2) {
            showError("Zwei Sequenzen für Alignment nötig");
            return;
        }

        List<Sequence> seqs = new ArrayList<>(sequenceMap.values());
        Alignment aligner = new Alignment(matchScore, mismatchPenalty, gapPenalty);

        try {
            Alignment.AlignResult result = aligner.needlemanWunsch(
                    seqs.get(0).getBases(), seqs.get(1).getBases());

            alignmentOutput.setText(result.format());
            statusTextLabel.setText(String.format("✓ Global Alignment: Score=%.1f", result.getScore()));
        } catch (Exception e) {
            showError("Alignment fehlgeschlagen: " + e.getMessage());
        }
    }

    private void handleLocalAlign() {
        if (sequenceMap.size() < 2) {
            showError("Zwei Sequenzen für Alignment nötig");
            return;
        }

        List<Sequence> seqs = new ArrayList<>(sequenceMap.values());
        Alignment aligner = new Alignment(matchScore, mismatchPenalty, gapPenalty);

        try {
            Alignment.AlignResult result = aligner.smithWaterman(
                    seqs.get(0).getBases(), seqs.get(1).getBases());

            alignmentOutput.setText(result.format());
            statusTextLabel.setText(String.format("✓ Lokal Alignment: Score=%.1f", result.getScore()));
        } catch (Exception e) {
            showError("Alignment fehlgeschlagen: " + e.getMessage());
        }
    }

    // ======== PHYLOGENIE ========

    private void handleParsimonyTree() {
        if (sequenceMap.size() < 3) {
            showError("Mindestens drei Sequenzen für Phylogenie nötig");
            return;
        }

        try {
            List<Sequence> seqs = new ArrayList<>(sequenceMap.values());
            Phylogeny.TreeNode tree = Phylogeny.neighborJoining(seqs);

            // Newick-String generieren
            String treeText = tree.toNewick() + ";";

            treeOutput.setText(treeText);
            statusTextLabel.setText("✓ Phylogenetischer Baum erstellt");
        } catch (Exception e) {
            showError("Baum-Erstellung fehlgeschlagen: " + e.getMessage());
        }
    }

    private void handleDistanceMatrix() {
        if (sequenceMap.size() < 2) return;

        List<Sequence> seqs = new ArrayList<>(sequenceMap.values());
        StringBuilder matrix = new StringBuilder();

        // Header
        matrix.append(String.format("%-20s", ""));
        for (Sequence seq : seqs) {
            matrix.append(String.format("%-15s", seq.getName().substring(0, Math.min(15, seq.getName().length()))));
        }
        matrix.append("\n");

        // Matrix
        for (int i = 0; i < seqs.size(); i++) {
            matrix.append(String.format("%-20s", seqs.get(i).getName().substring(0, Math.min(15, seqs.get(i).getName().length()))));

            for (int j = 0; j < seqs.size(); j++) {
                double dist = i == j ? 0.0 : Alignment.editDistance(
                        seqs.get(i).getBases(), seqs.get(j).getBases());

                matrix.append(String.format("%-15.2f", dist));
            }
            matrix.append("\n");
        }

        distanceMatrixOutput.setText(matrix.toString());
    }

    // ======== INFORMATION & HILFE ========

    private void handleHelp() {
        String content = """
            Funktionen:

            • Sequenz-Erstellung & Import (FASTA, FASTQ, GenBank)
            • Basale Operationen: Komplementär, Transkription, Translation
            • ORF-Finder & Restriktionsstellen-Analyse
            • Pairwise-Alignment (Needleman-Wunsch, Smith-Waterman)
            • Phylogenetische Bäume (Neighbor-Joining)
            • Konsensus-Sequenz-Erstellung

            Navigation:
            • Linkes Panel: Sequenzen verwalten
            • Zentrale Area: Tabs für verschiedene Ansichten
            • Rechtes Panel: Kontext-sensitive Werkzeuge""";

        JOptionPane.showMessageDialog(mainFrame, content, "Hilfe - Snapious BioTools", JOptionPane.INFORMATION_MESSAGE);
    }

    private void handleAbout() {
        JOptionPane.showMessageDialog(mainFrame,
            "Snapious BioTools v1.0\n" +
            "Komplette Bioinformatik-Suite\n\n" +
            "Entwickelt als vereinfachte, moderne Alternative zu Geneious Prime.\n" +
            "Java 21 (Swing)\n\n" +
            "© 2026 Snapious",
            "Über Snapious BioTools",
            JOptionPane.INFORMATION_MESSAGE);
    }

    // ======== UI UPDATE HELPERS ========

    private void updateDetailPanel(String sequenceId) {
        if (sequenceId == null || !sequenceMap.containsKey(sequenceId)) {
            detailIdLabel.setText("-");
            detailTypeLabel.setText("-");
            detailLengthLabel.setText("-");
            detailGCLabel.setText("-");
            detailBaseDistLabel.setText("-");
            gcProgressBar.setValue(0);
            return;
        }

        Sequence seq = sequenceMap.get(sequenceId);

        detailIdLabel.setText(seq.getId());
        detailTypeLabel.setText(String.valueOf(seq.getType()));
        detailLengthLabel.setText(String.valueOf(seq.length()) + " bp");

        double gc = seq.gcContent();
        detailGCLabel.setText(String.format("%.1f%%", gc * 100));
        gcProgressBar.setValue((int) (gc * 100));

        // Basen-Verteilung
        StringBuilder baseDist = new StringBuilder();
        for (Map.Entry<Character, Double> entry : seq.baseFrequencies().entrySet()) {
            baseDist.append(entry.getKey()).append("=")
                    .append(String.format("%.0f%%", entry.getValue() * 100)).append(" ");
        }
        detailBaseDistLabel.setText(baseDist.toString());
    }

    private void updateStats() {
        int totalSeqs = sequenceMap.size();
        long totalBases = sequenceMap.values().stream()
                .mapToLong(Sequence::length)
                .sum();

        statusSeqCountLabel.setText("Sequenzen: " + totalSeqs);
    }

    private void updateStatus() {
        statusTextLabel.setText("Bereit");
        statusSeqCountLabel.setText("Sequenzen: " + sequenceMap.size());
    }

    // ======== UTILITY METHODS ========

    private void showError(String message) {
        JOptionPane.showMessageDialog(mainFrame, message, "Fehler", JOptionPane.ERROR_MESSAGE);
        statusTextLabel.setText("Fehler: " + message);
    }

    // ClipboardOwner interface (required for clipboard operations)
    @Override
    public void lostOwnership(Clipboard clipboard, Transferable contents) {
        // Nothing to do
    }

    /**
     * Creates and shows the main application window.
     */
    public JFrame createMainWindow() {
        mainFrame = new JFrame("Snapious BioTools - Komplette Bioinformatik-Suite");
        mainFrame.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        mainFrame.setSize(1400, 900);

        // Try to set system look and feel
        try {
            UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName());
        } catch (Exception e) {
            // Fallback: default look and feel
        }

        mainFrame.setContentPane(getPanel());
        initialize();

        mainFrame.setLocationRelativeTo(null);
        return mainFrame;
    }
}
