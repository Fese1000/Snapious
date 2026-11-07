import SnapiousFFI
import SwiftUI

/// Similarity profile of an alignment: per-column identity (gray) and smoothed (accent color).
struct SimilarityGraphSheet: View {
    @ObservedObject var model: AppModel
    let alignmentID: String
    @Environment(\.dismiss) private var dismiss
    @State private var window = 15
    @State private var raw: [Double] = []
    @State private var smooth: [Double] = []
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(tr("Similarity profile")).font(.title2.bold())
            Text(tr("Mean pairwise identity per column (gaps do not count). 1 = all identical."))
                .font(.callout).foregroundStyle(.secondary)
            Stepper(tr("Smoothing window: %@ columns", "\(window)"), value: $window, in: 1...201, step: 2)
                .onChange(of: window) { _, _ in load() }
            Canvas { ctx, size in
                guard raw.count > 1 else { return }
                func path(_ v: [Double]) -> Path {
                    var p = Path()
                    for (i, y) in v.enumerated() {
                        let pt = CGPoint(x: size.width * CGFloat(i) / CGFloat(v.count - 1), y: size.height * (1 - CGFloat(y)))
                        if i == 0 { p.move(to: pt) } else { p.addLine(to: pt) }
                    }
                    return p
                }
                ctx.stroke(path(raw), with: .color(.gray.opacity(0.5)), lineWidth: 1)
                ctx.stroke(path(smooth), with: .color(.accentColor), lineWidth: 2)
            }
            .frame(height: 180)
            .background(.quaternary.opacity(0.3), in: RoundedRectangle(cornerRadius: 6))
            if let message { Text(message).foregroundStyle(.red) }
            if !smooth.isEmpty {
                Text(String(format: tr("Mean %.1f %% · lowest smoothed value %.1f %%"),
                            100 * smooth.reduce(0, +) / Double(smooth.count), 100 * (smooth.min() ?? 0)))
                    .font(.callout).foregroundStyle(.secondary)
            }
            HStack { Spacer(); Button(tr("Close")) { dismiss() }.keyboardShortcut(.defaultAction) }
        }
        .padding(20)
        .frame(width: 560)
        .onAppear { load() }
    }

    private func load() {
        guard let project = model.project else { return }
        do {
            raw = try project.alignmentSimilarity(id: alignmentID, window: 1)
            smooth = try project.alignmentSimilarity(id: alignmentID, window: UInt64(window))
            message = nil
        } catch {
            message = error.localizedDescription
        }
    }
}
