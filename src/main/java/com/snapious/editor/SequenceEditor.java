package com.snapious.editor;

import com.snapious.core.Sequence;

import java.util.*;
import java.util.stream.*;

/**
 * Sequence editor for manual sequence manipulation.
 * Supports: SNP (point mutation), insertion, deletion, substitution, reverse complement.
 */
public class SequenceEditor {

    // ======== EDIT OPERATION TYPES ==========

    public enum EditType {
        SNP("Point Mutation (SNP)", "Single base substitution"),
        INSERTION("Insertion", "Base insertion at a position"),
        DELETION("Deletion", "Base removal from the sequence"),
        SUBSTITUTION("Substitution", "Range replacement with a new sequence"),
        REVERSE_COMPLEMENT("Reverse Complement", "Reverse and complement the sequence");

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

    /** Represents a sequence edit with metadata */
    public static class EditResult {
        private String editedSequence;  // edited sequence bases
        private EditType editType;      // type of edit
        private int position;           // start position (0-based)
        private int originalLength;     // length of the original range
        private int newLength;          // length of the new range
        private String description;     // description of the change

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

        /** Length difference (positive = insertion, negative = deletion) */
        public int lengthDifference() { return newLength - originalLength; }

        /** Formatted edit report */
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
     * Perform a single-nucleotide polymorphism (SNP).
     * @param sequence original sequence
     * @param position 0-based position of the mutation
     * @param newBase new base (A, T, G, C)
     * @return EditResult with mutation details
     */
    public static EditResult performSNP(Sequence sequence, int position, char newBase) {
        String seq = sequence.getBases().toUpperCase();
        newBase = Character.toUpperCase(newBase);

        if (position < 0 || position >= seq.length()) {
            throw new IndexOutOfBoundsException("SNP position " + position + " outside the sequence length " + seq.length());
        }

        char oldBase = seq.charAt(position);

        requireValidBases(sequence, String.valueOf(newBase));

        // Create the edited sequence
        StringBuilder sb = new StringBuilder(seq);
        sb.setCharAt(position, newBase);
        String editedSeq = sb.toString();

        // Determine the mutation type (transitions vs. transversions)
        String mutationType = classifyMutation(oldBase, newBase);

        return new EditResult(
            editedSeq,
            EditType.SNP,
            position,
            1,  // one base removed
            1,  // one base inserted
            String.format("%c%d%s (%s)", oldBase, position + 1, newBase, mutationType)
        );
    }

    /** Checks that all characters are in the alphabet of the sequence type (for DNA/RNA additionally N). */
    private static void requireValidBases(Sequence sequence, String bases) {
        String allowed = sequence.getType().getAllowedChars();
        boolean nucleotide = sequence.getType() != Sequence.SequenceType.PROTEIN;
        for (char c : bases.toCharArray()) {
            if (allowed.indexOf(c) < 0 && !(nucleotide && c == 'N')) {
                throw new IllegalArgumentException("Invalid base '" + c + "' for " + sequence.getType()
                        + ". Allowed: " + allowed + (nucleotide ? "N" : ""));
            }
        }
    }

    /** Classify the mutation as transition or transversion */
    private static String classifyMutation(char oldBase, char newBase) {
        boolean isPurine = (oldBase == 'A' || oldBase == 'G');
        boolean isPurineNew = (newBase == 'A' || newBase == 'G');

        if (isPurine == isPurineNew) {
            return "Transition";  // purine → purine or pyrimidine → pyrimidine
        } else {
            return "Transversion";  // purine ↔ pyrimidine
        }
    }

    // ======== INSERTION ==========

    /**
     * Perform an insertion (base insertion).
     */
    public static EditResult performInsertion(Sequence sequence, int position, String insertedBases) {
        String seq = sequence.getBases().toUpperCase();
        String bases = insertedBases.toUpperCase();

        if (position < 0 || position > seq.length()) {
            throw new IndexOutOfBoundsException("Insertion position " + position + " outside the sequence length " + seq.length());
        }

        if (bases.isEmpty()) {
            throw new IllegalArgumentException("Insertion must not be empty");
        }
        requireValidBases(sequence, bases);

        // Create the edited sequence (insert at position)
        StringBuilder sb = new StringBuilder(seq);
        sb.insert(position, bases);
        String editedSeq = sb.toString();

        return new EditResult(
            editedSeq,
            EditType.INSERTION,
            position,
            0,  // no bases removed
            bases.length(),  // new bases inserted
            String.format("+%d bases: %s", bases.length(), bases)
        );
    }

    // ======== DELETION ==========

    /**
     * Perform a deletion (base removal).
     */
    public static EditResult performDeletion(Sequence sequence, int startPosition, int endPosition) {
        String seq = sequence.getBases().toUpperCase();

        if (startPosition < 0 || endPosition > seq.length() || startPosition >= endPosition) {
            throw new IndexOutOfBoundsException("Deletion range invalid: " + startPosition + "-" + endPosition);
        }

        String deletedBases = seq.substring(startPosition, endPosition);
        int deletionLength = endPosition - startPosition;

        // Create the edited sequence (remove the range)
        StringBuilder sb = new StringBuilder(seq);
        sb.delete(startPosition, endPosition);
        String editedSeq = sb.toString();

        return new EditResult(
            editedSeq,
            EditType.DELETION,
            startPosition,
            deletionLength,  // bases removed
            0,  // no new bases
            String.format("-%d bp: %s", deletionLength, deletedBases)
        );
    }

    // ======== SUBSTITUTION ==========

    /**
     * Perform a substitution (range replacement).
     */
    public static EditResult performSubstitution(Sequence sequence, int startPosition, int endPosition, String replacement) {
        String seq = sequence.getBases().toUpperCase();
        String repl = replacement.toUpperCase();

        if (startPosition < 0 || endPosition > seq.length() || startPosition >= endPosition) {
            throw new IndexOutOfBoundsException("Substitution range invalid: " + startPosition + "-" + endPosition);
        }

        requireValidBases(sequence, repl);

        String replacedBases = seq.substring(startPosition, endPosition);
        int originalLength = endPosition - startPosition;

        // Create the edited sequence (replace the range)
        StringBuilder sb = new StringBuilder(seq);
        sb.replace(startPosition, endPosition, repl);
        String editedSeq = sb.toString();

        return new EditResult(
            editedSeq,
            EditType.SUBSTITUTION,
            startPosition,
            originalLength,  // old bases removed
            repl.length(),   // new bases inserted
            String.format("%s → %s", replacedBases, repl)
        );
    }

    // ======== REVERSE COMPLEMENT ==========

    /**
     * Compute the reverse complement of a sequence.
     */
    public static EditResult performReverseComplement(Sequence sequence) {
        Sequence revComp = sequence.reverseComplement();

        return new EditResult(
            revComp.getBases(),
            EditType.REVERSE_COMPLEMENT,
            0,  // no specific position
            sequence.getBases().length(),  // original length
            revComp.getBases().length(),   // new length (identical)
            "Reverse complement computed"
        );
    }

    // ======== BATCH EDIT ==========

    /**
     * Apply several edits in the given order. Positions of each operation
     * refer to the sequence after the previous operations. Failed operations
     * are logged as an EditResult with "Error: ..." and the sequence remains unchanged.
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
                    "Error: " + e.getMessage()
                ));
            }
        }

        return results;
    }

    // ======== EDIT OPERATION (FLUENT API) ==========

    /** Represents an edit operation for the fluent API */
    public static class EditOperation {
        private EditType type;
        private int position;
        private String bases;  // for insertion/substitution
        private int endPosition;  // for deletion/substitution

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
                default: throw new IllegalArgumentException("Unknown EditType: " + type);
            }
        }

        public EditType getType() { return type; }
        public int getPosition() { return position; }

        /** Create an EditResult from a Sequence */
        public String describe() {
            return type.getDisplayName() + " @ pos=" + position;
        }
    }

    // ======== HISTORY ==========

    /** Represents a step in the edit history */
    public static class EditHistoryEntry {
        private String sequenceName;
        private String basesBefore;  // previous state
        private String basesAfter;   // subsequent state
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

    // ======== MAIN FUNCTIONS FOR UI ==========

    /** Generate editing suggestions for a sequence */
    public static List<String> getEditingSuggestions(Sequence sequence) {
        List<String> suggestions = new ArrayList<>();

        String seq = sequence.getBases().toUpperCase();
        int len = seq.length();

        // Check for common editing targets
        suggestions.add("=== Sequence analysis for editing ===");
        suggestions.add(String.format("Length: %d bp", len));

        // Find restriction sites (common editing target)
        // Use a common restriction enzyme motif as an example (4-cutter EcoRI: GAATTC)
        List<Integer> sites = sequence.findRestrictionSites("GAATTC");

        if (!sites.isEmpty()) {
            suggestions.add(String.format("\nEcoRI Restriction sites (%d found):", sites.size()));

            for (int i = 0; i < Math.min(5, sites.size()); i++) {
                suggestions.add(String.format("  Position %d", sites.get(i)));
            }

            suggestions.add("  → These sites can be removed or introduced by SNP");
        }

        // Find ORFs (common editing target, minimum length 30 codons = 90 bp)
        List<Sequence> orfs = sequence.findORFs(30);

        if (!orfs.isEmpty()) {
            suggestions.add(String.format("\nOpen reading frames (%d found, min. 90 bp):", orfs.size()));

            for (int i = 0; i < Math.min(3, orfs.size()); i++) {
                Sequence orf = orfs.get(i);
                suggestions.add(String.format("  ORF #%d: %d bp", i + 1, orf.getBases().length()));
            }

            suggestions.add("  → ORFs can be extended or shortened by mutation");
        }

        // Check for common mutation signatures (CpG sites)
        long cpgSites = 0;
        for (int i = seq.indexOf("CG"); i >= 0; i = seq.indexOf("CG", i + 1)) cpgSites++;
        suggestions.add(String.format("\nCpG sites: %d", cpgSites));

        if (cpgSites > len * 0.05) {
            suggestions.add("  → High CpG density: methylation could affect editing");
        }

        return suggestions;
    }

    /** Generate the full editing report */
    public static String generateEditReport(Sequence sequence, List<EditResult> edits) {
        StringBuilder sb = new StringBuilder();

        sb.append(String.format("=== Editing Report for: %s ===\n\n", sequence.getName()));
        sb.append(String.format("Original length: %d bp\n", sequence.getBases().length()));
        sb.append(String.format("Number of edits: %d\n", edits.size()));

        // Summary of the changes
        long insertions = edits.stream().filter(e -> e.lengthDifference() > 0).count();
        long deletions = edits.stream().filter(e -> e.lengthDifference() < 0).count();
        long snps = edits.stream().filter(e -> e.getEditType() == EditType.SNP).count();

        sb.append(String.format("  SNPs: %d\n", snps));
        sb.append(String.format("  Insertions: %d\n", insertions));
        sb.append(String.format("  Deletions: %d\n", deletions));

        // Detailed list
        sb.append("\nEdits:\n");

        for (int i = 0; i < edits.size(); i++) {
            EditResult edit = edits.get(i);
            sb.append(String.format("%d. %s\n", i + 1, edit.format()));
        }

        // Final sequence statistics
        String finalSeq = edits.isEmpty() ? sequence.getBases() : edits.get(edits.size() - 1).getEditedSequence();
        int diff = edits.stream().mapToInt(EditResult::lengthDifference).sum();
        sb.append(String.format("\nFinal length: %d bp (difference: %+d)\n", finalSeq.length(), diff));

        return sb.toString();
    }

    /** Undo the last edit (simplified implementation) */
    public static Sequence undoEdit(Sequence edited, String previousBases) {
        return new Sequence(edited.getId(), edited.getName(), previousBases, edited.getType());
    }

    /** Redo the next edit (simplified implementation) */
    public static Sequence redoEdit(Sequence undone, String nextBases) {
        return new Sequence(undone.getId(), undone.getName(), nextBases, undone.getType());
    }
}
