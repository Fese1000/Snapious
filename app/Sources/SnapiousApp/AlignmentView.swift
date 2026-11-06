import SnapiousFFI
import SwiftUI

/// Alignment-Ansicht: Namen links, Zeilen mit Farben, darüber Identität und Konsens.
struct AlignmentView: View {
    @ObservedObject var model: AppModel

    @State private var variants: [VariantDto] = []
    @State private var showVariants = false
    @State private var matrix: IdentityMatrixDto?
    @State private var logo: [LogoColumnDto] = []
    @State private var showLogo = false
    @State private var showStrip = false
    @State private var showApply = false
    @State private var showSimilarity = false

    private let font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
    private let rowHeight: CGFloat = 16

    private var charWidth: CGFloat {
        ("M" as NSString).size(withAttributes: [.font: font]).width
    }

    var body: some View {
        if let a = model.alignment {
            VStack(spacing: 0) {
                HStack {
                    Text(a.name).font(Theme.heading())
                    Text("\(a.rowNames.count) Zeilen, \(a.rows.first?.count ?? 0) Spalten").foregroundStyle(Theme.inkSoft)
                    Spacer()
                    Menu("Bearbeiten") {
                        Button("Reine Lückenspalten entfernen") { model.stripSelectedAlignment(threshold: 1.0) }
                        Button("Spalten mit ≥ 50 % Lücken entfernen") { model.stripSelectedAlignment(threshold: 0.5) }
                        Button("Alle Spalten mit Lücken entfernen") { model.stripSelectedAlignment(threshold: 0.0001) }
                        Button("Weitere Spalten entfernen …") { showStrip = true }
                        Divider()
                        Button("Varianten gegen erste Zeile …") { loadVariants() }
                        Button("Identitäts-Heatmap …") { matrix = try? model.project?.alignmentIdentityMatrix(id: a.id) }
                        Button("Sequenzlogo …") { loadLogo() }
                        Button("Konsens als Sequenz speichern") { model.consensusOfSelectedAlignment() }
                        Button("Sequenz mit Varianten anlegen …") { showApply = true }
                        Button("Ähnlichkeitsverlauf …") { showSimilarity = true }
                    }
                }
                .padding(8)
                Divider()
                ScrollView([.horizontal, .vertical]) {
                    canvas(a)
                }
            }
            .sheet(isPresented: $showStrip) { ColumnStripSheet(model: model) }
            .sheet(isPresented: $showSimilarity) { SimilarityGraphSheet(model: model, alignmentID: a.id) }
            .sheet(isPresented: $showApply) { ApplyVariantsSheet(model: model, rowNames: a.rowNames) }
            .sheet(isPresented: Binding(get: { matrix != nil }, set: { if !$0 { matrix = nil } })) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Paarweise Identität (%)").font(Theme.heading())
                    if let matrix { IdentityHeatmapView(matrix: matrix) }
                    HStack { Spacer(); Button("Schließen") { matrix = nil } }
                }
                .padding(16)
                .frame(minWidth: 560, minHeight: 380)
            }
            .sheet(isPresented: $showLogo) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Sequenzlogo (Bit)").font(Theme.heading())
                    SequenceLogoView(columns: logo, protein: a.seqType == "PROTEIN")
                    HStack { Spacer(); Button("Schließen") { showLogo = false } }
                }
                .padding(16)
                .frame(minWidth: 560, minHeight: 280)
            }
            .sheet(isPresented: $showVariants) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Varianten gegen \(a.rowNames.first ?? "Referenz"): \(variants.count)").font(Theme.heading())
                    Table(Array(variants.enumerated()).map { VariantRow(id: $0.offset, dto: $0.element) }) {
                        TableColumn("Zeile") { v in Text(v.dto.rowName) }
                        TableColumn("Spalte") { v in Text("\(v.dto.column + 1)") }.width(70)
                        TableColumn("Ref-Pos.") { v in Text("\(v.dto.referencePosition + 1)") }.width(70)
                        TableColumn("Ref") { v in Text(v.dto.referenceBase) }.width(40)
                        TableColumn("Alt") { v in Text(v.dto.altBase) }.width(40)
                        TableColumn("Art") { v in Text(v.dto.kind) }.width(100)
                    }
                    HStack { Spacer(); Button("Schließen") { showVariants = false } }
                }
                .padding(16)
                .frame(minWidth: 560, minHeight: 360)
            }
        }
    }

    private func loadLogo() {
        guard let project = model.project, let id = model.selection else { return }
        logo = (try? project.alignmentLogo(id: id)) ?? []
        showLogo = true
    }

    private func loadVariants() {
        guard let project = model.project, let id = model.selection else { return }
        variants = (try? project.alignmentVariants(id: id, reference: 0)) ?? []
        showVariants = true
    }

    /// Für Bildschirmfotos ohne Fenster.
    func snapshotCanvas() -> some View {
        Group { if let a = model.alignment { canvas(a) } }
    }

    private func canvas(_ a: AlignmentDto) -> some View {
        let nameWidth = (a.rowNames.map { $0.count }.max() ?? 4)
        let nameX = CGFloat(nameWidth) * charWidth + 16
        let columns = a.rows.first?.count ?? 0
        let width = nameX + CGFloat(columns) * charWidth + 16
        let height = CGFloat(a.rows.count + 3) * rowHeight + 16
        return Canvas { ctx, _ in
            let cw = charWidth
            // Identität als Balken
            for (i, v) in a.identity.enumerated() {
                let h = CGFloat(v) * (rowHeight - 2)
                ctx.fill(Path(CGRect(x: nameX + CGFloat(i) * cw, y: rowHeight - h, width: cw - 1, height: h)),
                         with: .color(.green.opacity(0.7)))
            }
            ctx.draw(Text("Identität").font(.system(size: 10)).foregroundStyle(Theme.inkSoft), at: CGPoint(x: 4, y: rowHeight / 2), anchor: .leading)
            // Konsens
            let cy = rowHeight * 1.5
            ctx.draw(Text("Konsens").font(.system(size: 10)).foregroundStyle(Theme.inkSoft), at: CGPoint(x: 4, y: cy), anchor: .leading)
            for (i, ch) in a.consensus.enumerated() {
                ctx.draw(Text(String(ch)).font(.system(size: 12, design: .monospaced)),
                         at: CGPoint(x: nameX + CGFloat(i) * cw, y: cy), anchor: .leading)
            }
            // Zeilen
            for (r, row) in a.rows.enumerated() {
                let y = rowHeight * CGFloat(r + 3)
                ctx.draw(Text(a.rowNames[r]).font(.system(size: 11)), at: CGPoint(x: 4, y: y + rowHeight / 2), anchor: .leading)
                for (i, ch) in row.enumerated() {
                    let x = nameX + CGFloat(i) * cw
                    if let bg = background(ch, protein: a.seqType == "PROTEIN") {
                        ctx.fill(Path(CGRect(x: x, y: y, width: cw, height: rowHeight)), with: .color(bg))
                    }
                    ctx.draw(Text(String(ch)).font(.system(size: 12, design: .monospaced)),
                             at: CGPoint(x: x, y: y + rowHeight / 2), anchor: .leading)
                }
            }
        }
        .frame(width: width, height: height)
    }

    private func background(_ c: Character, protein: Bool) -> Color? {
        if c == "-" || c == "." { return nil }
        if protein {
            switch c {
            case "A", "V", "L", "I", "M", "F", "W", "P": return .blue.opacity(0.25)
            case "K", "R", "H": return .red.opacity(0.25)
            case "D", "E": return .purple.opacity(0.25)
            case "S", "T", "N", "Q", "C", "Y", "G": return .green.opacity(0.25)
            default: return nil
            }
        }
        switch c {
        case "A", "a": return .green.opacity(0.3)
        case "C", "c": return .blue.opacity(0.3)
        case "G", "g": return .orange.opacity(0.3)
        case "T", "t", "U", "u": return .red.opacity(0.3)
        default: return nil
        }
    }
}

struct VariantRow: Identifiable {
    let id: Int
    let dto: VariantDto
}
