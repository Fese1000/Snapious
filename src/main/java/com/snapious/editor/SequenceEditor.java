package com.snapious.editor;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Sequence Editor für manuelle Sequenz-Manipulation.
 * Unterstützt: SNP (Point Mutation), Insertion, Deletion, Substitution, Reverse Complement.
 */
public class SequenceEditor {

    // ======== EDIT OPERATION TYPES ==========

    public enum EditType {
        SNP("Point Mutation (SNP)", "Einzelner Basen-Austausch"),
        INSERTION("Insertion", "Basen-Einfügung an Position"),
        DELETION("Deletion", "Basen-Entfernung aus Sequenz"),
        SUBSTITUTION("Substitution", "Bereichs-Ersatz durch neue Sequenz"),
        REVERSE_COMPLEMENT("Reverse Komplement", "Sequenz umkehren und komplementieren");

        private final String displayName;
        private final String description;

        EditType(String displayName, String description) {
            this.displayName = displayName;
            this.description = description;
        }

        public String getDisplayName() { return displayName; }
        public String getDescription() { return description; }
    }

    // ======== EDIT RESULT ==========

    /** Repräsentiert eine Sequenz-Editierung mit Metadaten */
    public static class EditResult {
        private String editedSequence;  // Editierte Sequenz-Basen
        private EditType editType;      // Art der Editierung
        private int position;           // Start-Position (0-basiert)
        private int originalLength;     // Länge des Original-Bereichs
        private int newLength;          // Länge des neuen Bereichs
        private String description;     // Beschreibung der Änderung

        public EditResult(String editedSequence, EditType editType, int position,
                         int originalLength, int newLength, String description) {
            this.editedSequence = editedSequence;
            this.editType = editType;
            this.position = Math.max(0, position);
            this.originalLength = originalLength;
            this.newLength = newLength;
            this.description = description;
        }

        public String getEditedSequence() { return editedSequence; }
        public EditType getEditType() { return editType; }
        public int getPosition() { return position; }
        public int getOriginalLength() { return originalLength; }
        public int getNewLength() { return newLength; }
        public String getDescription() { return description; }

        /** Länge-Differenz (positiv = Insertion, negativ = Deletion) */
        public int lengthDifference() { return newLength - originalLength; }

        /** Formatierter Edit-Report */
        public String format() {
            return String.format("%s at position %d: %s (len %d → %d)",
                editType.getDisplayName(), position, description,
                originalLength, newLength);
        }

        @Override
        public String toString() { return format(); }
    }

    // ======== SNP (Point Mutation) ==========

    /**
     * Führe einen Single-Nucleotide Polymorphismus (SNP) durch.
     * @param sequence Original-Sequenz
     * @param position 0-basierte Position der Mutation
     * @param newBase Neue Base (A, T, G, C)
     * @return EditResult mit mutation-Details
     */
    public static EditResult performSNP(Sequence sequence, int position, char newBase) {
        String seq = sequence.getBases().toUpperCase();
        newBase = Character.toUpperCase(newBase);

        if (position < 0 || position >= seq.length()) {
            throw new IndexOutOfBoundsException("SNP-Position " + position + " außerhalb der Sequenz-Länge " + seq.length());
        }

        char oldBase = seq.charAt(position);

        requireValidBases(sequence, String.valueOf(newBase));

        // Erzeuge editierte Sequenz
        StringBuilder sb = new StringBuilder(seq);
        sb.setCharAt(position, newBase);
        String editedSeq = sb.toString();

        // Bestimme Mutation-Typ (Transitions vs Transversionen)
        String mutationType = classifyMutation(oldBase, newBase);

        return new EditResult(
            editedSeq,
            EditType.SNP,
            position,
            1,  // Eine Base entfernt
            1,  // Eine Base eingefügt
            String.format("%c%d%s (%s)", oldBase, position + 1, newBase, mutationType)
        );
    }

    /** Prüft, dass alle Zeichen im Alphabet des Sequenztyps liegen (bei DNA/RNA zusätzlich N). */
    private static void requireValidBases(Sequence sequence, String bases) {
        String allowed = sequence.getType().getAllowedChars();
        boolean nucleotide = sequence.getType() != Sequence.SequenceType.PROTEIN;
        for (char c : bases.toCharArray()) {
            if (allowed.indexOf(c) < 0 && !(nucleotide && c == 'N')) {
                throw new IllegalArgumentException("Ungültige Base '" + c + "' für " + sequence.getType()
                        + ". Erlaubt: " + allowed + (nucleotide ? "N" : ""));
            }
        }
    }

    /** Klassifiziere Mutation als Transition oder Transversion */
    private static String classifyMutation(char oldBase, char newBase) {
        boolean isPurine = (oldBase == 'A' || oldBase == 'G');
        boolean isPurineNew = (newBase == 'A' || newBase == 'G');

        if (isPurine == isPurineNew) {
            return "Transition";  // Purin → Purin oder Pyrimidin → Pyrimidin
        } else {
            return "Transversion";  // Purin ↔ Pyrimidin
        }
    }

    // ======== INSERTION ==========

    /**
     * Führe eine Insertion (Basen-Einfügung) durch.
     */
    public static EditResult performInsertion(Sequence sequence, int position, String insertedBases) {
        String seq = sequence.getBases().toUpperCase();
        String bases = insertedBases.toUpperCase();

        if (position < 0 || position > seq.length()) {
            throw new IndexOutOfBoundsException("Insertion-Position " + position + " außerhalb der Sequenz-Länge " + seq.length());
        }

        if (bases.isEmpty()) {
            throw new IllegalArgumentException("Insertion darf nicht leer sein");
        }
        requireValidBases(sequence, bases);

        // Erzeuge editierte Sequenz (Einfügen an Position)
        StringBuilder sb = new StringBuilder(seq);
        sb.insert(position, bases);
        String editedSeq = sb.toString();

        return new EditResult(
            editedSeq,
            EditType.INSERTION,
            position,
            0,  // Keine Basen entfernt
            bases.length(),  // Neue Basen eingefügt
            String.format("+%d Basen: %s", bases.length(), bases)
        );
    }

    // ======== DELETION ==========

    /**
     * Führe eine Deletion (Basen-Entfernung) durch.
     */
    public static EditResult performDeletion(Sequence sequence, int startPosition, int endPosition) {
        String seq = sequence.getBases().toUpperCase();

        if (startPosition < 0 || endPosition > seq.length() || startPosition >= endPosition) {
            throw new IndexOutOfBoundsException("Deletion-Range ungültig: " + startPosition + "-" + endPosition);
        }

        String deletedBases = seq.substring(startPosition, endPosition);
        int deletionLength = endPosition - startPosition;

        // Erzeuge editierte Sequenz (Entfernen des Bereichs)
        StringBuilder sb = new StringBuilder(seq);
        sb.delete(startPosition, endPosition);
        String editedSeq = sb.toString();

        return new EditResult(
            editedSeq,
            EditType.DELETION,
            startPosition,
            deletionLength,  // Basen entfernt
            0,  // Keine neuen Basen
            String.format("-%d bp: %s", deletionLength, deletedBases)
        );
    }

    // ======== SUBSTITUTION ==========

    /**
     * Führe eine Substitution (Bereichs-Ersatz) durch.
     */
    public static EditResult performSubstitution(Sequence sequence, int startPosition, int endPosition, String replacement) {
        String seq = sequence.getBases().toUpperCase();
        String repl = replacement.toUpperCase();

        if (startPosition < 0 || endPosition > seq.length() || startPosition >= endPosition) {
            throw new IndexOutOfBoundsException("Substitution-Range ungültig: " + startPosition + "-" + endPosition);
        }

        requireValidBases(sequence, repl);

        String replacedBases = seq.substring(startPosition, endPosition);
        int originalLength = endPosition - startPosition;

        // Erzeuge editierte Sequenz (Bereich ersetzen)
        StringBuilder sb = new StringBuilder(seq);
        sb.replace(startPosition, endPosition, repl);
        String editedSeq = sb.toString();

        return new EditResult(
            editedSeq,
            EditType.SUBSTITUTION,
            startPosition,
            originalLength,  // Alte Basen entfernt
            repl.length(),   // Neue Basen eingefügt
            String.format("%s → %s", replacedBases, repl)
        );
    }

    // ======== REVERSE COMPLEMENT ==========

    /**
     * Berechne Reverse-Komplement einer Sequenz.
     */
    public static EditResult performReverseComplement(Sequence sequence) {
        Sequence revComp = sequence.reverseComplement();

        return new EditResult(
            revComp.getBases(),
            EditType.REVERSE_COMPLEMENT,
            0,  // Keine spezifische Position
            sequence.getBases().length(),  // Original-Länge
            revComp.getBases().length(),   // Neue Länge (identisch)
            "Reverse-Komplement berechnet"
        );
    }

    // ======== BATCH EDIT ==========

    /**
     * Wende mehrere Editierungen in der übergebenen Reihenfolge an. Positionen jeder Operation
     * beziehen sich auf die Sequenz nach den vorherigen Operationen. Fehlgeschlagene Operationen
     * werden als EditResult mit "Fehler: ..." protokolliert und die Sequenz bleibt unverändert.
     */
    public static List<EditResult> batchEdits(Sequence sequence, List<EditOperation> operations) {
        String currentSeq = sequence.getBases().toUpperCase();
        List<EditResult> results = new ArrayList<>();

        for (EditOperation op : operations) {
            Sequence tempSeq = new Sequence(sequence.getId(), sequence.getName(), currentSeq, sequence.getType());

            try {
                EditResult result = op.apply(tempSeq);
                results.add(result);
                currentSeq = result.getEditedSequence();

            } catch (IllegalArgumentException | IndexOutOfBoundsException e) {
                results.add(new EditResult(
                    currentSeq,
                    op.getType(),
                    op.getPosition(),
                    0, 0,
                    "Fehler: " + e.getMessage()
                ));
            }
        }

        return results;
    }

    // ======== EDIT OPERATION (FLUENT API) ==========

    /** Repräsentiert eine Editier-Operation für fluent API */
    public static class EditOperation {
        private EditType type;
        private int position;
        private String bases;  // Für Insertion/Substitution
        private int endPosition;  // Für Deletion/Substitution

        public static EditOperation snp(int position, char newBase) {
            EditOperation op = new EditOperation();
            op.type = EditType.SNP;
            op.position = position;
            op.bases = String.valueOf(newBase);
            return op;
        }

        public static EditOperation insertion(int position, String bases) {
            EditOperation op = new EditOperation();
            op.type = EditType.INSERTION;
            op.position = position;
            op.bases = bases.toUpperCase();
            return op;
        }

        public static EditOperation deletion(int startPosition, int endPosition) {
            EditOperation op = new EditOperation();
            op.type = EditType.DELETION;
            op.position = startPosition;
            op.endPosition = endPosition;
            return op;
        }

        public static EditOperation substitution(int startPosition, int endPosition, String replacement) {
            EditOperation op = new EditOperation();
            op.type = EditType.SUBSTITUTION;
            op.position = startPosition;
            op.endPosition = endPosition;
            op.bases = replacement.toUpperCase();
            return op;
        }

        public EditResult apply(Sequence sequence) {
            switch (type) {
                case SNP: return performSNP(sequence, position, bases.charAt(0));
                case INSERTION: return performInsertion(sequence, position, bases);
                case DELETION: return performDeletion(sequence, position, endPosition);
                case SUBSTITUTION: return performSubstitution(sequence, position, endPosition, bases);
                default: throw new IllegalArgumentException("Unbekannter EditType: " + type);
            }
        }

        public EditType getType() { return type; }
        public int getPosition() { return position; }

        /** Erzeuge EditResult aus Sequence */
        public String describe() {
            return type.getDisplayName() + " @ pos=" + position;
        }
    }

    // ======== HISTORIE ==========

    /** Repräsentiert einen Schritt in der Editier-Historie */
    public static class EditHistoryEntry {
        private String sequenceName;
        private String basesBefore;  // Vorheriger Zustand
        private String basesAfter;   // Nachheriger Zustand
        private EditResult editResult;

        public EditHistoryEntry(String sequenceName, String basesBefore, String basesAfter, EditResult editResult) {
            this.sequenceName = sequenceName;
            this.basesBefore = basesBefore;
            this.basesAfter = basesAfter;
            this.editResult = editResult;
        }

        public String getSequenceName() { return sequenceName; }
        public String getBasesBefore() { return basesBefore; }
        public String getBasesAfter() { return basesAfter; }
        public EditResult getEditResult() { return editResult; }

        @Override
        public String toString() {
            return String.format("EditHistory[%s: %s]", sequenceName, editResult.format());
        }
    }

    // ======== HAUPTFUNKTIONEN FÜR UI ==========

    /** Generiere Editier-Vorschläge für eine Sequenz */
    public static List<String> getEditingSuggestions(Sequence sequence) {
        List<String> suggestions = new ArrayList<>();

        String seq = sequence.getBases().toUpperCase();
        int len = seq.length();

        // Prüfe auf häufige Editier-Ziele
        suggestions.add("=== Sequenz-Analyse für Editing ===");
        suggestions.add(String.format("Länge: %d bp", len));

        // Finde Restriction Sites (häufig Editier-Ziel)
        // Verwende ein häufiges Restriktionsenzym-Motif als Beispiel (4-cutter EcoRI: GAATTC)
        List<Integer> sites = sequence.findRestrictionSites("GAATTC");

        if (!sites.isEmpty()) {
            suggestions.add(String.format("\nEcoRI Restriktions Sites (%d gefunden):", sites.size()));

            for (int i = 0; i < Math.min(5, sites.size()); i++) {
                suggestions.add(String.format("  Position %d", sites.get(i)));
            }

            suggestions.add("  → Diese Sites können durch SNP entfernt oder eingefügt werden");
        }

        // Finde ORFs (häufig Editier-Ziel, minimiale Länge 30 Codons = 90 bp)
        List<Sequence> orfs = sequence.findORFs(30);

        if (!orfs.isEmpty()) {
            suggestions.add(String.format("\nOffene Leserahmen (%d gefunden, min. 90 bp):", orfs.size()));

            for (int i = 0; i < Math.min(3, orfs.size()); i++) {
                Sequence orf = orfs.get(i);
                suggestions.add(String.format("  ORF #%d: %d bp", i + 1, orf.getBases().length()));
            }

            suggestions.add("  → ORFs können durch Mutation verlängert oder verkürzt werden");
        }

        // Prüfe auf häufige Mutations-Signaturen (CpG-Stellen)
        long cpgSites = 0;
        for (int i = seq.indexOf("CG"); i >= 0; i = seq.indexOf("CG", i + 1)) cpgSites++;
        suggestions.add(String.format("\nCpG-Stellen: %d", cpgSites));

        if (cpgSites > len * 0.05) {
            suggestions.add("  → Hohe CpG-Dichte: Methylierung könnte Editing beeinflussen");
        }

        return suggestions;
    }

    /** Generiere vollständigen Editier-Bericht */
    public static String generateEditReport(Sequence sequence, List<EditResult> edits) {
        StringBuilder sb = new StringBuilder();

        sb.append(String.format("=== Editier-Bericht für: %s ===\n\n", sequence.getName()));
        sb.append(String.format("Original-Länge: %d bp\n", sequence.getBases().length()));
        sb.append(String.format("Anzahl Editierungen: %d\n", edits.size()));

        // Zusammenfassung der Änderungen
        long insertions = edits.stream().filter(e -> e.lengthDifference() > 0).count();
        long deletions = edits.stream().filter(e -> e.lengthDifference() < 0).count();
        long snps = edits.stream().filter(e -> e.getEditType() == EditType.SNP).count();

        sb.append(String.format("  SNPs: %d\n", snps));
        sb.append(String.format("  Insertionen: %d\n", insertions));
        sb.append(String.format("  Deletionen: %d\n", deletions));

        // Detaillierte Liste
        sb.append("\nEditierungen:\n");

        for (int i = 0; i < edits.size(); i++) {
            EditResult edit = edits.get(i);
            sb.append(String.format("%d. %s\n", i + 1, edit.format()));
        }

        // Finale Sequenz-Statistik
        String finalSeq = edits.isEmpty() ? sequence.getBases() : edits.get(edits.size() - 1).getEditedSequence();
        int diff = edits.stream().mapToInt(EditResult::lengthDifference).sum();
        sb.append(String.format("\nFinale Länge: %d bp (Differenz: %+d)\n", finalSeq.length(), diff));

        return sb.toString();
    }

    /** Undo letzte Editierung (vereinfachte Implementierung) */
    public static Sequence undoEdit(Sequence edited, String previousBases) {
        return new Sequence(edited.getId(), edited.getName(), previousBases, edited.getType());
    }

    /** Redo nächste Editierung (vereinfachte Implementierung) */
    public static Sequence redoEdit(Sequence undone, String nextBases) {
        return new Sequence(undone.getId(), undone.getName(), nextBases, undone.getType());
    }
}
