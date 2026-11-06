import SnapiousFFI
import SwiftUI

/// Primerdesign für einen Zielbereich: Standardvorgaben (18–27 nt, Tm 57–63 °C, GC-Klammer).
struct PrimerView: View {
    @ObservedObject var model: AppModel
    @State private var startText = ""
    @State private var endText = ""
    @State private var pairs: [PairRow] = []
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Zielbereich (1-basiert)")
                TextField("von", text: $startText).frame(width: 80).textFieldStyle(.roundedBorder)
                Text("–")
                TextField("bis", text: $endText).frame(width: 80).textFieldStyle(.roundedBorder)
                Button("Aus Auswahl übernehmen") {
                    if model.hasSelection, let a = model.selStart, let b = model.selEnd {
                        startText = "\(min(a, b) + 1)"
                        endText = "\(max(a, b))"
                    }
                }
                .disabled(!model.hasSelection)
                Button("Primer suchen") { search() }
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            Table(pairs) {
                TableColumn("Nr.") { p in Text("\(p.index + 1)") }.width(40)
                TableColumn("Vorwärts 5'→3'") { p in Text(p.dto.forwardSequence).font(Theme.mono(13)) }
                TableColumn("Tm") { p in Text(String(format: "%.1f", p.dto.forwardTm)) }.width(50)
                TableColumn("Rückwärts 5'→3'") { p in Text(p.dto.reverseSequence).font(Theme.mono(13)) }
                TableColumn("Tm ") { p in Text(String(format: "%.1f", p.dto.reverseTm)) }.width(50)
                TableColumn("Produkt") { p in Text("\(p.dto.productSize) bp") }.width(80)
                TableColumn("Strafe") { p in Text(String(format: "%.2f", p.dto.penalty)) }.width(60)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: model.selection) { _, _ in
            pairs = []
            message = nil
        }
    }

    private func search() {
        message = nil
        guard let project = model.project, let id = model.selection,
              let a = UInt64(startText), let b = UInt64(endText), a >= 1, b >= a
        else {
            message = "Bitte einen gültigen Zielbereich angeben."
            return
        }
        do {
            let list = try project.pickPrimers(id: id, targetStart: a - 1, targetEnd: b, maxResults: 20)
            pairs = list.enumerated().map { PairRow(index: $0.offset, dto: $0.element) }
            if list.isEmpty { message = "Keine Primerpaare mit diesen Vorgaben gefunden." }
        } catch {
            pairs = []
            message = error.localizedDescription
        }
    }
}

struct PairRow: Identifiable {
    let index: Int
    let dto: PrimerPairDto
    var id: Int { index }
}
