import AppKit
import SnapiousFFI
import SwiftUI

/// Sequenzierabgleich: eine .ab1-Datei gegen das gewählte Dokument prüfen, Unterschiede und Spur ansehen.
struct ReadCheckView: View {
    @ObservedObject var model: AppModel
    @State private var trace: TraceDto?
    @State private var result: ReadCheckDto?
    @State private var rows: [DiffRow] = []
    @State private var message: String?
    @State private var ownTrace: TraceDto?
    @State private var ownFile = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Button("Sequenzierdatei wählen…") { choose() }
                if let r = result {
                    Text("\(r.readName): \(r.reversed ? "reverse-komplementär, " : "")Referenz \(r.referenceStart + 1)–\(r.referenceEnd), Identität \(String(format: "%.2f", r.identity)) %")
                        .foregroundStyle(Theme.inkSoft)
                }
                Spacer()
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            if let own = ownTrace {
                GroupBox("Spur der Sequenzierdatei „\(ownFile)“ (im Projektordner abgelegt)") {
                    VStack(alignment: .leading, spacing: 4) {
                        TraceCanvas(trace: own).frame(height: 160)
                        Text("Qualität: " + qualitySummary(own)).font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
            if let t = trace {
                TraceCanvas(trace: t)
                    .frame(height: 160)
            }
            if !rows.isEmpty {
                Table(rows) {
                    TableColumn("Art") { r in Text(kindName(r.dto.kind)) }.width(120)
                    TableColumn("Referenz") { r in Text("\(r.dto.referencePosition + 1)") }.width(80)
                    TableColumn("Read") { r in Text("\(r.dto.readPosition + 1)") }.width(70)
                    TableColumn("Erwartet") { r in Text(r.dto.expected.isEmpty ? "–" : r.dto.expected) }.width(80)
                    TableColumn("Gefunden") { r in Text(r.dto.observed.isEmpty ? "–" : r.dto.observed) }.width(80)
                    TableColumn("Qualität") { r in Text("\(r.dto.quality)") }.width(70)
                }
            } else if result != nil {
                Text("Keine Unterschiede.").foregroundStyle(Theme.inkSoft)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear { loadOwnTrace() }
        .onChange(of: model.selection) { _, _ in
            trace = nil
            result = nil
            rows = []
            message = nil
            loadOwnTrace()
        }
    }

    /// Lädt die Spur der verknüpften `.ab1`-Datei des gewählten Dokuments (falls vorhanden).
    private func loadOwnTrace() {
        ownTrace = nil
        ownFile = ""
        guard let project = model.project, let id = model.selection,
              let f = (try? project.externalFiles(docId: id))?.first(where: { $0.kind == "ab1" || $0.kind == "scf" }) else { return }
        ownTrace = try? readTrace(path: f.path)
        ownFile = f.label
    }

    private func qualitySummary(_ t: TraceDto) -> String {
        guard !t.quality.isEmpty else { return "nicht gespeichert" }
        let mean = Double(t.quality.reduce(0) { $0 + Int($1) }) / Double(t.quality.count)
        let good = t.quality.filter { $0 >= 20 }.count
        return String(format: "Mittel %.1f, %d von %d Basen mit Q ≥ 20", mean, good, t.quality.count)
    }

    private func kindName(_ k: String) -> String {
        switch k {
        case "MISMATCH": return "Austausch"
        case "INSERTION": return "Einfügung"
        case "DELETION": return "Löschung"
        default: return "Niedrige Qualität"
        }
    }

    private func choose() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        let panel = NSOpenPanel()
        panel.title = "Sequenzierdatei (.ab1) wählen"
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            trace = try readTrace(path: url.path)
            let r = try project.checkReadFile(id: id, path: url.path)
            result = r
            rows = r.differences.enumerated().map { DiffRow(index: $0.offset, dto: $0.element) }
        } catch {
            trace = nil
            result = nil
            rows = []
            message = error.localizedDescription
        }
    }
}

struct DiffRow: Identifiable {
    let index: Int
    let dto: DifferenceDto
    var id: Int { index }
}

/// Vier Farbkanäle mit den Basenaufrufen darunter; niedrige Qualität wird blasser gezeichnet.
struct TraceCanvas: View {
    let trace: TraceDto
    /// Horizontale Vergrößerung (1 = 3 Messpunkte je Pixel) und Höhe der Peaks.
    @State private var zoom = 1.0
    @State private var height = 1.0
    @State private var pinchStart: Double?
    @State private var viewWidth: CGFloat = 600

    private var samples: Int { trace.channels.first?.values.count ?? 400 }
    private var baseWidth: CGFloat { CGFloat(max(400, samples / 3)) }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Button { zoomBy(1 / 1.5) } label: { Image(systemName: "minus.magnifyingglass") }
                    .accessibilityLabel("Herauszoomen")
                Slider(value: Binding(get: { log2(zoom) }, set: { zoom = pow(2, $0) }), in: -2...4).frame(width: 140)
                Button { zoomBy(1.5) } label: { Image(systemName: "plus.magnifyingglass") }
                    .accessibilityLabel("Hineinzoomen")
                Text(String(format: "%.0f %%", zoom * 100)).font(.caption).foregroundStyle(.secondary).frame(width: 48, alignment: .leading)
                Button("Anpassen") { zoom = max(0.25, Double(viewWidth / baseWidth)) }
                    .help("Die ganze Spur ins Fenster einpassen")
                Button("1:1") { zoom = 1 }
                Divider().frame(height: 14)
                Image(systemName: "arrow.up.and.down").foregroundStyle(.secondary).accessibilityLabel("Höhe der Peaks")
                Slider(value: $height, in: 0.5...6).frame(width: 90)
                Spacer()
                Text("Zoomen auch mit zwei Fingern").font(.caption).foregroundStyle(.secondary)
            }
            .buttonStyle(.borderless)
            ScrollView(.horizontal) {
                canvas
                    .frame(width: baseWidth * CGFloat(zoom), height: 160)
            }
            .background(GeometryReader { g in Color.clear.onAppear { viewWidth = g.size.width }.onChange(of: g.size.width) { _, w in viewWidth = w } })
            .gesture(MagnifyGesture()
                .onChanged { v in
                    if pinchStart == nil { pinchStart = zoom }
                    zoom = min(16, max(0.25, (pinchStart ?? zoom) * Double(v.magnification)))
                }
                .onEnded { _ in pinchStart = nil })
        }
    }

    private func zoomBy(_ f: Double) { zoom = min(16, max(0.25, zoom * f)) }

    private var canvas: some View {
        Canvas { ctx, size in
            let colors: [String: Color] = ["A": .green, "C": .blue, "G": .black, "T": .red]
            let maxValue = Double(trace.channels.flatMap { $0.values }.max() ?? 1) / height
            let n = Double(samples)
            for ch in trace.channels {
                var p = Path()
                for (i, v) in ch.values.enumerated() {
                    let x = size.width * CGFloat(Double(i) / n)
                    let y = (size.height - 22) * CGFloat(1 - min(1, Double(max(0, v)) / max(1, maxValue))) + 2
                    if i == 0 { p.move(to: CGPoint(x: x, y: y)) } else { p.addLine(to: CGPoint(x: x, y: y)) }
                }
                ctx.stroke(p, with: .color(colors[ch.base] ?? .gray), lineWidth: 1)
            }
            let bases = Array(trace.bases)
            for (i, pos) in trace.peakPositions.enumerated() where i < bases.count {
                let x = size.width * CGFloat(Double(pos) / n)
                let q = i < trace.quality.count ? Int(trace.quality[i]) : 40
                ctx.draw(Text(String(bases[i])).font(.system(size: 11, design: .monospaced))
                            .foregroundStyle((colors[String(bases[i])] ?? .primary).opacity(q < 20 ? 0.35 : 1)),
                         at: CGPoint(x: x, y: size.height - 8), anchor: .center)
                // alle zehn Basen die Nummer, sobald genug Platz ist
                if (i + 1) % 10 == 0 && size.width / n * 30 > 2 {
                    ctx.draw(Text("\(i + 1)").font(.system(size: 8)).foregroundStyle(.secondary), at: CGPoint(x: x, y: 6), anchor: .center)
                }
            }
        }
    }
}
