import SnapiousFFI
import SwiftUI

/// RNA folding (ViennaRNA) for the selection or the whole sequence: dot-bracket structure and arc diagram.
struct RnaView: View {
    @ObservedObject var model: AppModel
    @State private var result: RnaFoldDto?
    @State private var message: String?
    @State private var busy = false
    @State private var simple: SimpleFoldDto?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Button(model.hasSelection ? tr("Fold selection") : tr("Fold whole sequence")) { fold() }
                    .disabled(busy)
                Button(tr("Simple folding (no energy)")) { simpleFold() }
                    .help(tr("Largest number of non-crossing base pairs by Nussinov, also for DNA; without ViennaRNA"))
                if busy { ProgressView().controlSize(.small) }
                if let r = result {
                    Text(String(format: tr("Free energy %.2f kcal/mol, %d base pairs"), r.energy, r.pairs.count))
                        .foregroundStyle(Theme.inkSoft)
                }
                Spacer()
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            if let r = result {
                ArcDiagram(length: r.sequence.count, pairs: r.pairs)
                    .frame(height: 220)
                GroupBox(tr("Sequence and structure")) {
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
                GroupBox(tr("Simple folding: %@ base pairs", "\(s.pairs)")) {
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

/// Arc diagram: the sequence as a line, each base pair as a semicircle above it.
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
