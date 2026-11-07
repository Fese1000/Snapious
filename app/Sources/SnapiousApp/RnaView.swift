import SnapiousFFI
import SwiftUI

/// RNA-Faltung (ViennaRNA) für die Auswahl oder die ganze Sequenz: Punkt-Klammer-Struktur und Bogendiagramm.
struct RnaView: View {
    @ObservedObject var model: AppModel
    @State private var result: RnaFoldDto?
    @State private var message: String?
    @State private var busy = false
    @State private var simple: SimpleFoldDto?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Button(model.hasSelection ? tr("Auswahl falten") : tr("Ganze Sequenz falten")) { fold() }
                    .disabled(busy)
                Button(tr("Einfache Faltung (ohne Energie)")) { simpleFold() }
                    .help(tr("Größte Zahl nicht kreuzender Basenpaare nach Nussinov, auch für DNA; ohne ViennaRNA"))
                if busy { ProgressView().controlSize(.small) }
                if let r = result {
                    Text(String(format: tr("Freie Energie %.2f kcal/mol, %d Basenpaare"), r.energy, r.pairs.count))
                        .foregroundStyle(Theme.inkSoft)
                }
                Spacer()
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            if let r = result {
                ArcDiagram(length: r.sequence.count, pairs: r.pairs)
                    .frame(height: 220)
                GroupBox(tr("Sequenz und Struktur")) {
                    ScrollView(.horizontal) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(r.sequence)
                            Text(r.structure)
                        }
                        .font(Theme.mono(13))
                        .textSelection(.enabled)
                    }
                }
            }
            if let s = simple {
                GroupBox(tr("Einfache Faltung: %@ Basenpaare", "\(s.pairs)")) {
                    ScrollView(.horizontal) {
                        Text(s.structure).font(Theme.mono(13)).textSelection(.enabled)
                    }
                }
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: model.selection) { _, _ in
            result = nil
            simple = nil
            message = nil
        }
    }

    private func simpleFold() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        var start: UInt64 = 0
        var end: UInt64 = 0
        if model.hasSelection, let a = model.selStart, let b = model.selEnd {
            start = UInt64(min(a, b))
            end = UInt64(max(a, b))
        }
        do { simple = try project.simpleFold(id: id, start: start, end: end) } catch { message = error.localizedDescription }
    }

    private func fold() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        var start: UInt64 = 0
        var end: UInt64 = 0
        if model.hasSelection, let a = model.selStart, let b = model.selEnd {
            start = UInt64(min(a, b))
            end = UInt64(max(a, b))
        }
        busy = true
        Task.detached {
            do {
                let r = try project.foldRegion(id: id, start: start, end: end)
                await MainActor.run { result = r; busy = false }
            } catch {
                await MainActor.run { result = nil; message = error.localizedDescription; busy = false }
            }
        }
    }
}

/// Bogendiagramm: die Sequenz als Linie, jedes Basenpaar als Halbkreis darüber.
struct ArcDiagram: View {
    let length: Int
    let pairs: [BasePairDto]

    var body: some View {
        Canvas { ctx, size in
            let left: CGFloat = 12, right = size.width - 12
            let baseY = size.height - 14
            func x(_ i: UInt64) -> CGFloat {
                left + (right - left) * CGFloat(Double(i) + 0.5) / CGFloat(max(1, length))
            }
            ctx.stroke(Path { $0.move(to: CGPoint(x: left, y: baseY)); $0.addLine(to: CGPoint(x: right, y: baseY)) },
                       with: .color(.secondary), lineWidth: 2)
            for p in pairs {
                let a = x(p.first), b = x(p.second)
                let r = (b - a) / 2
                var arc = Path()
                arc.addArc(center: CGPoint(x: (a + b) / 2, y: baseY), radius: r, startAngle: .degrees(180), endAngle: .degrees(0), clockwise: false)
                ctx.stroke(arc, with: .color(Theme.accent.opacity(0.7)), lineWidth: 1)
            }
        }
    }
}
