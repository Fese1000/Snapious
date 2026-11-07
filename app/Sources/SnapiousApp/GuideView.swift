import SnapiousFFI
import SwiftUI

/// CRISPR gRNA search for the selected document.
struct GuideView: View {
    @ObservedObject var model: AppModel
    @State private var nuclease = "spcas9"
    @State private var guides: [GuideRow] = []
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Picker(tr("Nuclease"), selection: $nuclease) {
                    Text("SpCas9 (NGG)").tag("spcas9")
                    Text("SaCas9 (NNGRRT)").tag("sacas9")
                    Text("Cas12a (TTTV)").tag("cas12a")
                }
                .frame(width: 280)
                Button(tr("Search")) { search() }
                Text(tr("%@ hits", "\(guides.count)")).foregroundStyle(Theme.inkSoft)
                Spacer()
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            Table(guides) {
                TableColumn(tr("Range")) { g in Text("\(g.dto.start + 1)–\(g.dto.end)") }.width(110)
                TableColumn(tr("Strand")) { g in Text(g.dto.forward ? "+" : "−") }.width(50)
                TableColumn(tr("Protospacer")) { g in Text(g.dto.protospacer).font(Theme.mono(13)) }
                TableColumn("PAM", value: \.dto.pam).width(70)
                TableColumn("GC") { g in Text(String(format: "%.0f %%", g.dto.gcPercent)) }.width(60)
                TableColumn(tr("Score")) { g in Text(String(format: "%.0f", g.dto.score)) }.width(70)
                TableColumn(tr("Notes")) { g in Text(g.dto.warnings.joined(separator: "; ")).foregroundStyle(Theme.inkSoft) }
            }
        }
        .padding(8)
        .onChange(of: model.selection) { _, _ in
            guides = []
            message = nil
        }
    }

    private func search() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        do {
            let result = try project.findGuides(id: id, nuclease: nuclease)
            guides = result.enumerated().map { GuideRow(index: $0.offset, dto: $0.element) }
        } catch {
            guides = []
            message = error.localizedDescription
        }
    }
}

struct GuideRow: Identifiable {
    let index: Int
    let dto: GuideDto
    var id: Int { index }
}
