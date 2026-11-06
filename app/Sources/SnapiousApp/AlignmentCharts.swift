import SnapiousFFI
import SwiftUI

/// Identitäts-Heatmap: paarweise Identität aller Zeilen in Prozent, mit Zahlen.
struct IdentityHeatmapView: View {
    let matrix: IdentityMatrixDto

    private let cell: CGFloat = 44
    private let labelWidth: CGFloat = 110

    private func color(_ percent: Double) -> Color {
        // 0 % blau-kühl, 100 % warm; unterhalb von 50 % kaum Sättigung
        let t = max(0, min(1, (percent - 50) / 50))
        return Color(hue: 0.62 - 0.62 * t, saturation: 0.15 + 0.7 * t, brightness: 0.85)
    }

    var body: some View {
        ScrollView([.horizontal, .vertical]) { grid }
    }

    /// Ohne Scrollbereich (für Bildschirmfotos ohne Fenster).
    var grid: some View {
        let n = matrix.names.count
        return Group {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 2) {
                    Color.clear.frame(width: labelWidth, height: 20)
                    ForEach(0..<n, id: \.self) { j in
                        Text(matrix.names[j]).font(.caption2).lineLimit(1)
                            .frame(width: cell, height: 20)
                    }
                }
                ForEach(0..<n, id: \.self) { i in
                    HStack(spacing: 2) {
                        Text(matrix.names[i]).font(Theme.label(11)).lineLimit(1)
                            .frame(width: labelWidth, height: cell, alignment: .trailing)
                        ForEach(0..<n, id: \.self) { j in
                            let v = matrix.values[i * n + j]
                            Text(String(format: "%.0f", v))
                                .font(.caption.monospacedDigit())
                                .foregroundStyle(.black)
                                .frame(width: cell, height: cell)
                                .background(color(v), in: RoundedRectangle(cornerRadius: 3))
                        }
                    }
                }
            }
            .padding(12)
        }
    }
}

/// Sequenzlogo: Symbolhöhe = Informationsgehalt in Bit, je Spalte gestapelt (niedrigstes unten).
struct SequenceLogoView: View {
    let columns: [LogoColumnDto]
    let protein: Bool

    private let colWidth: CGFloat = 16
    private let height: CGFloat = 160

    private func color(_ symbol: String) -> Color {
        switch symbol.uppercased() {
        case "A": return .green
        case "C": return .blue
        case "G": return .orange
        case "T", "U": return .red
        default: return .purple
        }
    }

    var body: some View {
        ScrollView(.horizontal) { chart }
    }

    /// Ohne Scrollbereich (für Bildschirmfotos ohne Fenster).
    var chart: some View {
        let maxBits = protein ? log2(20.0) : 2.0
        return Group {
            Canvas { ctx, size in
                let unit = (size.height - 22) / maxBits
                for (i, col) in columns.enumerated() {
                    var y = size.height - 22
                    for (k, symbol) in col.symbols.enumerated() {
                        let h = col.heights[k] * unit
                        if h < 1 { continue }
                        let text = Text(symbol).font(.system(size: 100, weight: .bold, design: .monospaced))
                            .foregroundStyle(color(symbol))
                        // Symbol auf die Höhe `h` strecken
                        var c = ctx
                        c.translateBy(x: CGFloat(i) * colWidth, y: y - h)
                        c.scaleBy(x: colWidth / 60, y: h / 100)
                        c.draw(c.resolve(text), at: .zero, anchor: .topLeading)
                        y -= h
                    }
                    if i % 5 == 4 {
                        ctx.draw(Text("\(i + 1)").font(.system(size: 9)).foregroundStyle(Theme.inkSoft),
                                 at: CGPoint(x: CGFloat(i) * colWidth + colWidth / 2, y: size.height - 8))
                    }
                }
            }
            .frame(width: max(CGFloat(columns.count) * colWidth + 24, 200), height: height + 22)
            .padding(12)
        }
    }
}
