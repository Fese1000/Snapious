import SwiftUI
import UniformTypeIdentifiers

/// Dialog „Neue Sequenz“: Text einfügen (roh oder FASTA) oder eintippen, Art und Form wählen.
struct NewSequenceView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var text = ""
    @State private var kind = "AUTO"
    @State private var circular = false

    /// Zahl der Buchstaben im Text (ohne Kopfzeilen), nur zur Anzeige.
    private var letterCount: Int {
        text.split(separator: "\n", omittingEmptySubsequences: false)
            .filter { !$0.hasPrefix(">") }
            .reduce(0) { $0 + $1.filter { $0.isLetter }.count }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Neue Sequenz").font(.title2.bold())
            Form {
                TextField("Name", text: $name, prompt: Text("optional, sonst aus der FASTA-Kopfzeile"))
                Picker("Art", selection: $kind) {
                    Text("Automatisch erkennen").tag("AUTO")
                    Text("DNA").tag("DNA")
                    Text("RNA").tag("RNA")
                    Text("Protein").tag("PROTEIN")
                }
                Toggle("Zirkulär (Plasmid)", isOn: $circular)
                    .disabled(kind == "PROTEIN")
            }
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text("Sequenz").font(.headline)
                    Spacer()
                    Text("\(letterCount) Zeichen").foregroundStyle(.secondary)
                    Button("Aus Zwischenablage") {
                        if let s = NSPasteboard.general.string(forType: .string) { text = s }
                    }
                }
                TextEditor(text: $text)
                    .font(.system(.body, design: .monospaced))
                    .frame(minHeight: 180)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                Text("Rohe Buchstaben, Zeilennummern und Leerzeichen werden entfernt. FASTA mit mehreren Einträgen legt mehrere Sequenzen an.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            HStack {
                Spacer()
                Button("Abbrechen", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button("Anlegen") {
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
