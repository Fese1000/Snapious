import SwiftUI
import UniformTypeIdentifiers

/// "New sequence" dialog: paste text (raw or FASTA) or type it in, choose kind and shape.
struct NewSequenceView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var text = ""
    @State private var kind = "AUTO"
    @State private var circular = false

    /// Number of letters in the text (without header lines), for display only.
    private var letterCount: Int {
        text.split(separator: "\n", omittingEmptySubsequences: false)
            .filter { !$0.hasPrefix(">") }
            .reduce(0) { $0 + $1.filter { $0.isLetter }.count }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("New sequence")).font(.title2.bold())
            Form {
                TextField(tr("Name"), text: $name, prompt: Text(tr("optional, otherwise from the FASTA header")))
                Picker(tr("Kind"), selection: $kind) {
                    Text(tr("Detect automatically")).tag("AUTO")
                    Text("DNA").tag("DNA")
                    Text("RNA").tag("RNA")
                    Text(tr("Protein")).tag("PROTEIN")
                }
                Toggle(tr("Circular (plasmid)"), isOn: $circular)
                    .disabled(kind == "PROTEIN")
            }
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(tr("Sequence")).font(.headline)
                    Spacer()
                    Text(tr("%@ characters", "\(letterCount)")).foregroundStyle(.secondary)
                    Button(tr("From clipboard")) {
                        if let s = NSPasteboard.general.string(forType: .string) { text = s }
                    }
                }
                TextEditor(text: $text)
                    .font(.system(.body, design: .monospaced))
                    .frame(minHeight: 180)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                Text(tr("Raw letters; line numbers and spaces are removed. FASTA with several entries creates several sequences."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            HStack {
                Spacer()
                Button(tr("Cancel"), role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(tr("Create")) {
                    if model.createSequence(name: name, text: text, kind: kind, circular: circular) { dismiss() }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(letterCount == 0)
            }
        }
        .padding(20)
        .frame(width: 560)
    }
}
