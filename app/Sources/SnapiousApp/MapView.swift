import SnapiousFFI
import SwiftUI

/// Eine Ansicht für Karte und Sequenz: ganz herausgezoomt Plasmidring oder lineare Karte, hineingezoomt ein
/// Ausschnitt, ab etwa neun Pixeln je Base die Sequenz mit Buchstaben. Schnittstellen nach Filter, Bearbeiten überall.
struct MapView: View {
    @ObservedObject var model: AppModel
    @State private var showEnzymes = false

    private var ring: Bool {
        switch model.mapStyle {
        case "ring": return true
        case "linear": return false
        default: return model.summary?.topology == "CIRCULAR"
        }
    }

    @State private var pinchBase: Double?

    /// Trackpad: zwei Finger spreizen/zusammenziehen zoomt in allen Darstellungen.
    private var pinch: some Gesture {
        MagnifyGesture()
            .onChanged { v in
                if pinchBase == nil { pinchBase = model.zoom }
                model.setZoom((pinchBase ?? 1) * Double(v.magnification))
            }
            .onEnded { _ in pinchBase = nil }
    }

    var body: some View {
        if let s = model.summary {
            VStack(spacing: 0) {
                toolbar(s)
                Divider()
                GeometryReader { geo in
                    let pixelsPerBase = (geo.size.width - 80) / model.viewSpan + 0.02
                    let _ = { model.viewWidth = Double(geo.size.width) }()
                    if s.seqType == "PROTEIN" || (!ring && pixelsPerBase >= 9) {
                        SequenceView(model: model)
                            .simultaneousGesture(pinch)
                    } else if ring {
                        VStack(spacing: 0) {
                            CircularMap(model: model, length: max(1, Int(s.length)), name: s.name)
                            panSlider
                            MapEditBar(model: model)
                        }
                    } else {
                        VStack(spacing: 0) {
                            LinearMap(model: model, length: max(1, Int(s.length)))
                            panSlider
                            MapEditBar(model: model)
                        }
                    }
                }
            }
        }
    }

    private func toolbar(_ s: DocumentSummary) -> some View {
        HStack(spacing: 12) {
            if s.seqType != "PROTEIN" {
                Picker("", selection: Binding(get: { ring ? "ring" : "linear" }, set: { model.mapStyle = $0; model.setZoom(model.zoom) })) {
                    Text("Plasmid").tag("ring")
                    Text("Linear").tag("linear")
                }
                .pickerStyle(.segmented).labelsHidden().frame(width: 150)
                .help("Darstellung: Plasmidring oder lineare Karte (bearbeitbar in beiden)")
            }
            HStack(spacing: 4) {
                Button { model.setZoom(model.zoom / 1.6) } label: { Image(systemName: "minus.magnifyingglass") }
                    .accessibilityLabel("Herauszoomen").disabled(model.zoom <= 1.001)
                Slider(value: Binding(get: { zoomPosition }, set: { model.setZoom(zoom(from: $0)) }), in: 0...1)
                    .frame(width: 140)
                Button { model.setZoom(model.zoom * 1.6) } label: { Image(systemName: "plus.magnifyingglass") }
                    .disabled(model.zoom >= model.maxZoom - 0.001)
                    .accessibilityLabel("Hineinzoomen")
                Text(model.zoom <= 1.001 ? "Ganz" : String(format: "%.0f×", model.zoom)).font(Theme.label(11)).foregroundStyle(Theme.inkSoft).frame(width: 40, alignment: .leading)
            }
            Spacer()
            if s.seqType != "PROTEIN" {
                Menu {
                    Picker("Schnittstellen", selection: $model.siteFilter) {
                        Text("Einzelschneider").tag("unique")
                        Text("Zweifachschneider").tag("double")
                        ForEach(enzymeSets(), id: \.name) { Text($0.name).tag("set:\($0.name)") }
                        Text("Alle bekannten Enzyme").tag("all")
                        Text("Eigene Auswahl").tag("custom")
                    }
                    .pickerStyle(.inline)
                    Divider()
                    Button("Enzyme einzeln wählen …") { model.siteFilter = "custom"; showEnzymes = true }
                } label: {
                    Label("Schnittstellen: \(filterTitle) (\(model.visibleSites.count))", systemImage: "scissors")
                }
                .menuStyle(.borderlessButton).fixedSize()
                .popover(isPresented: $showEnzymes) { EnzymePicker(model: model) }
            }
        }
        .font(Theme.label(13))
        .padding(.horizontal, 12).padding(.vertical, 6)
        .background(.bar)
        .onAppear { model.refreshEnzymeSets() }
    }

    private var filterTitle: String {
        switch model.siteFilter {
        case "unique": return "Einzelschneider"
        case "double": return "Zweifachschneider"
        case "all": return "alle"
        case "custom": return "Auswahl"
        default: return String(model.siteFilter.dropFirst(4))
        }
    }

    // Zoom auf einer logarithmischen Skala
    private var maxZoom: Double { max(1.01, model.maxZoom) }
    private var zoomPosition: Double { log(model.zoom) / log(maxZoom) }
    private func zoom(from v: Double) -> Double { exp(v * log(maxZoom)) }

    @ViewBuilder
    private var panSlider: some View {
        if model.zoom > 1.001 {
            HStack {
                Text("\(Int(model.viewStart) + 1)").font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
                Slider(value: Binding(get: { model.viewStart }, set: { model.panView(to: $0) }),
                       in: 0...max(1, Double(model.docLength) - model.viewSpan))
                Text("\(Int(model.viewStart + model.viewSpan))").font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            }
            .padding(.horizontal, 12)
        }
    }
}

/// Auswahl der Enzyme einzeln: Suchfeld, Häkchen, Zahl der Schnittstellen in dieser Sequenz.
struct EnzymePicker: View {
    @ObservedObject var model: AppModel
    @State private var query = ""

    var body: some View {
        let counts = model.siteCounts
        let list = model.enzymes.filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) }
            .sorted { (counts[$0.name] ?? 0) > 0 && (counts[$1.name] ?? 0) == 0 || $0.name < $1.name && ((counts[$0.name] ?? 0) > 0) == ((counts[$1.name] ?? 0) > 0) }
        VStack(alignment: .leading, spacing: 8) {
            TextField("Enzym suchen", text: $query).textFieldStyle(.roundedBorder)
            HStack {
                Button("Alle schneidenden") { model.customEnzymes = Set(counts.keys) }
                Button("Keine") { model.customEnzymes = [] }
                Spacer()
                Text("\(model.customEnzymes.count) gewählt").foregroundStyle(Theme.inkSoft)
            }
            .font(Theme.label(12))
            List(list, id: \.name) { e in
                Toggle(isOn: Binding(get: { model.customEnzymes.contains(e.name) },
                                     set: { if $0 { model.customEnzymes.insert(e.name) } else { model.customEnzymes.remove(e.name) } })) {
                    HStack {
                        Text(e.name)
                        Text(e.recognition).font(.system(size: 11, design: .monospaced)).foregroundStyle(Theme.inkSoft)
                        Spacer()
                        Text("\(counts[e.name] ?? 0)×").foregroundStyle(Theme.inkSoft)
                    }
                }
            }
            .frame(width: 340, height: 320)
        }
        .padding(12)
    }
}

/// Bearbeiten in der Kartenansicht: Auswahl (oder Cursor), Text einfügen/ersetzen, löschen, rückgängig.
struct MapEditBar: View {
    @ObservedObject var model: AppModel
    @State private var text = ""

    var body: some View {
        HStack(spacing: 10) {
            if let a = model.selStart, let b = model.selEnd {
                let lo = min(a, b), hi = max(a, b)
                Text(lo == hi ? "Cursor bei \(lo + 1)" : "Auswahl \(lo + 1)–\(hi) (\(hi - lo) bp)").lineLimit(1).fixedSize()
                TextField(lo == hi ? "Basen einfügen" : "Ersetzen durch …", text: $text)
                    .textFieldStyle(.roundedBorder).frame(maxWidth: 260)
                    .font(.system(size: 12, design: .monospaced))
                    .onSubmit { apply() }
                Button(lo == hi ? "Einfügen" : "Ersetzen") { apply() }.disabled(text.isEmpty)
                Button("Löschen") { model.replaceSelection(with: "") }.disabled(lo == hi)
                Button("Kopieren") { model.copySelection(format: "bases") }.disabled(lo == hi)
                Button("Aufheben") { model.clearSelection() }
            } else {
                Text("Auf die Karte klicken oder ziehen, um eine Stelle oder einen Bereich zu wählen – dann bearbeiten.")
                    .foregroundStyle(Theme.inkSoft)
            }
            Spacer()
            Button { model.undo() } label: { Image(systemName: "arrow.uturn.backward") }.accessibilityLabel("Rückgängig")
                .disabled(model.undoState.undo == 0)
            Button { model.redo() } label: { Image(systemName: "arrow.uturn.forward") }.accessibilityLabel("Wiederholen")
                .disabled(!model.undoState.redo)
        }
        .font(Theme.label(13))
        .padding(.horizontal, 12).padding(.vertical, 6)
        .background(.bar)
    }

    private func apply() {
        model.replaceSelection(with: text)
        text = ""
    }
}

private func niceStep(length: Int, maxTicks: Int) -> Int {
    var exp = 1
    while true {
        for m in [1, 2, 5] {
            let step = m * exp
            if length <= step * max(1, maxTicks) { return step }
        }
        exp *= 10
    }
}

private func featureColor(_ type: String) -> Color { Theme.feature(type) }

struct CircularMap: View {
    @ObservedObject var model: AppModel
    let length: Int
    let name: String

    private var zoomed: Bool { model.zoom > 1.001 }
    /// Position, die oben in der Mitte steht (bei Vergrößerung die Mitte des Ausschnitts).
    private var centreBP: Double { zoomed ? model.viewStart + model.viewSpan / 2 : 0 }

    /// Bei Vergrößerung wächst der Ring um den Faktor `zoom`; sein oberer Rand bleibt im Bild.
    private func geometry(_ size: CGSize) -> (c: CGPoint, r: CGFloat) {
        let r0 = max(40, min(size.width, size.height) / 2 - 95)
        if !zoomed { return (CGPoint(x: size.width / 2, y: size.height / 2), r0) }
        let r = r0 * CGFloat(model.zoom)
        return (CGPoint(x: size.width / 2, y: 130 + r), r)
    }

    /// Position (zwischen den Basen) unter einem Klickpunkt auf dem Ring.
    private func position(at p: CGPoint, size: CGSize) -> Int {
        let c = geometry(size).c
        let a = atan2(Double(p.x - c.x), Double(c.y - p.y))
        var pos = (centreBP + a / (2 * .pi) * Double(length)).rounded()
        pos = pos.truncatingRemainder(dividingBy: Double(length))
        if pos < 0 { pos += Double(length) }
        return Int(pos)
    }

    var body: some View {
        GeometryReader { geo in
            Canvas { ctx, size in
                draw(ctx, size)
            }
            .contentShape(Rectangle())
            .gesture(DragGesture(minimumDistance: 0).onChanged { v in
                focused = true
                model.select(from: position(at: v.startLocation, size: geo.size), to: position(at: v.location, size: geo.size))
            })
            .simultaneousGesture(MagnifyGesture().onChanged { v in
                if pinchBase == nil {
                    pinchBase = model.zoom
                    pinchAnchor = anchorBP(at: v.startLocation, size: geo.size)
                }
                zoom(to: (pinchBase ?? 1) * Double(v.magnification), anchor: pinchAnchor, at: v.startLocation, size: geo.size)
            }.onEnded { _ in pinchBase = nil })
        }
        .padding(8)
        .editKeys(model, focus: $focused)
    }

    @State private var pinchBase: Double?
    @State private var pinchAnchor = 0.0
    @FocusState private var focused: Bool

    /// Position (Basen), die unter einem Punkt der Ansicht liegt (nicht gerundet).
    private func anchorBP(at p: CGPoint, size: CGSize) -> Double {
        let c = geometry(size).c
        let a = atan2(Double(p.x - c.x), Double(c.y - p.y))
        return centreBP + a / (2 * .pi) * Double(length)
    }

    /// Zoomt so, dass die Base `anchor` unter dem Mauszeiger/den Fingern `p` bleibt.
    private func zoom(to z: Double, anchor: Double, at p: CGPoint, size: CGSize) {
        let maxZ = model.maxZoom
        let nz = min(max(1, z), maxZ)
        let r0 = max(40, min(size.width, size.height) / 2 - 95)
        let r = r0 * nz
        let c = nz > 1.001 ? CGPoint(x: size.width / 2, y: 130 + r) : CGPoint(x: size.width / 2, y: size.height / 2)
        let a = atan2(Double(p.x - c.x), Double(c.y - p.y))
        let centre = anchor - a / (2 * .pi) * Double(length)
        let span = max(1, Double(length) / nz)
        model.zoom = nz
        model.viewStart = min(max(0, centre - span / 2), max(0, Double(length) - span))
    }

    private func draw(_ ctx: GraphicsContext, _ size: CGSize) {
            var ctx = ctx
            let (c, r) = geometry(size)
            func angle(_ p: Double) -> Double {
                var d = (p - centreBP).truncatingRemainder(dividingBy: Double(length))
                if d > Double(length) / 2 { d -= Double(length) } else if d < -Double(length) / 2 { d += Double(length) }
                return 2 * .pi * d / Double(length)
            }
            func inView(_ p: Double) -> Bool { !zoomed || abs(angle(p)) <= 2 * .pi * (model.viewSpan / Double(length)) / 2 + 0.02 }
            func point(_ p: Double, _ rad: CGFloat) -> CGPoint {
                CGPoint(x: c.x + rad * CGFloat(sin(angle(p))), y: c.y - rad * CGFloat(cos(angle(p))))
            }
            // Auswahl
            if let a = model.selStart, let b = model.selEnd {
                let lo = Double(min(a, b)), hi = Double(max(a, b))
                var sel = Path()
                if lo == hi {
                    sel.move(to: point(lo, r - 30)); sel.addLine(to: point(lo, r + 12))
                    ctx.stroke(sel, with: .color(Theme.accent), lineWidth: 2)
                } else {
                    sel.addArc(center: c, radius: r, startAngle: .radians(angle(lo) - .pi / 2), endAngle: .radians(angle(hi) - .pi / 2), clockwise: false)
                    ctx.stroke(sel, with: .color(Theme.accent.opacity(0.5)), lineWidth: 16)
                }
            }
            // Rückgrat
            let arcPerBase = 2 * Double.pi * Double(r) / Double(length)
            let showBases = zoomed && arcPerBase >= 8 && model.summary?.seqType != "PROTEIN"
            if zoomed {
                var back = Path()
                let half = 2 * .pi * model.viewSpan / Double(length) / 2 + 0.05
                back.addArc(center: c, radius: r, startAngle: .radians(-.pi / 2 - half), endAngle: .radians(-.pi / 2 + half), clockwise: false)
                ctx.stroke(back, with: .color(.secondary.opacity(showBases ? 0.35 : 1)), lineWidth: showBases ? 1 : 2)
            } else {
                ctx.stroke(Path(ellipseIn: CGRect(x: c.x - r, y: c.y - r, width: 2 * r, height: 2 * r)),
                           with: .color(.secondary), lineWidth: 2)
            }
            // Basen entlang des Rings (obere Reihe: Sequenz, untere: Gegenstrang)
            if showBases {
                let bytes = model.fullBytes
                let lo = max(0, Int(model.viewStart) - 1), hi = min(bytes.count, Int(model.viewStart + model.viewSpan) + 2)
                let comp: [UInt8: String] = [65: "T", 84: "A", 67: "G", 71: "C", 85: "A"]
                if lo < hi {
                    for i in lo..<hi {
                        let a = angle(Double(i) + 0.5)
                        let ch = Character(UnicodeScalar(bytes[i]))
                        for (rad, text, dim) in [(r + 0, String(ch), false), (r - 16, comp[bytes[i] & 0xDF] ?? "N", true)] {
                            var g = ctx
                            g.translateBy(x: c.x + rad * CGFloat(sin(a)), y: c.y - rad * CGFloat(cos(a)))
                            g.rotate(by: .radians(a))
                            g.draw(Text(text).font(.system(size: 13, weight: .medium, design: .monospaced))
                                    .foregroundStyle(Theme.base(text.first ?? "N").opacity(dim ? 0.55 : 1)), at: .zero, anchor: .center)
                        }
                    }
                }
            }
            // Skala
            let step = niceStep(length: zoomed ? Int(model.viewSpan) : length, maxTicks: 12)
            var t = zoomed ? (Int(model.viewStart) / step) * step : 0
            while t < length && (!zoomed || Double(t) <= model.viewStart + model.viewSpan) {
                defer { t += step }
                guard inView(Double(t)) else { continue }
                var p = Path()
                p.move(to: point(Double(t), r - 4))
                p.addLine(to: point(Double(t), r + 4))
                ctx.stroke(p, with: .color(.secondary), lineWidth: 1)
                ctx.draw(Text(verbatim: "\(t)").font(.system(size: 9)).foregroundStyle(Theme.inkSoft),
                         at: point(Double(t), r - 16 - (showBases ? 20 : 0)), anchor: .center)
                t += step
            }
            // Features (Spuren nach innen)
            for f in model.lanes where f.featureType.lowercased() != "source" {
                let rad = r - 26 - (showBases ? 20 : 0) - CGFloat(f.lane) * 14
                guard rad > 20 else { continue }
                guard !zoomed || (inView(Double(f.start)) || inView(Double(f.end)) || (Double(f.start) <= model.viewStart && Double(f.end) >= model.viewStart + model.viewSpan)) else { continue }
                let a0 = angle(Double(f.start)) - .pi / 2
                let a1 = a0 + 2 * .pi * Double(f.end - f.start) / Double(length)
                var arc = Path()
                arc.addArc(center: c, radius: rad, startAngle: .radians(a0), endAngle: .radians(a1), clockwise: false)
                ctx.stroke(arc, with: .color(featureColor(f.featureType)), lineWidth: 10)
                let mid = Double(f.start + f.end) / 2
                if Double(f.end - f.start) / Double(length) > 0.04 {
                    ctx.draw(Text(f.name).font(.system(size: 9)).foregroundStyle(.white), at: point(mid, rad), anchor: .center)
                }
            }
            // Enzymmarken außen, Beschriftungen auseinandergeschoben (nur Vorwärtsdurchgang)
            let sites = model.visibleSites
            var lastAngle = -10.0 // vor dem ersten Eintrag
            for s in sites where inView(Double(s.position)) {
                var a = angle(Double(s.position))
                let gap = 0.09 / (zoomed ? min(model.zoom, 12) : 1)
                if lastAngle > -9, a < lastAngle + gap { a = lastAngle + gap }
                lastAngle = a
                var tick = Path()
                tick.move(to: point(Double(s.position), r))
                tick.addLine(to: point(Double(s.position), r + (showBases ? 20 : 10)))
                ctx.stroke(tick, with: .color(.primary), lineWidth: 1)
                let lp = CGPoint(x: c.x + (r + (showBases ? 30 : 20)) * CGFloat(sin(a)), y: c.y - (r + (showBases ? 30 : 20)) * CGFloat(cos(a)))
                let anchor: UnitPoint = sin(a) >= 0 ? .leading : .trailing
                ctx.draw(Text(verbatim: "\(s.enzyme) (\(s.position + 1))").font(.system(size: 10)), at: lp, anchor: anchor)
            }
            // Mitte
            ctx.draw(Text(name).font(Theme.heading()), at: CGPoint(x: c.x, y: c.y - 8), anchor: .center)
            ctx.draw(Text(verbatim: "\(length) bp").font(.subheadline).foregroundStyle(Theme.inkSoft),
                     at: CGPoint(x: c.x, y: c.y + 12), anchor: .center)
    }
}

struct LinearMap: View {
    @ObservedObject var model: AppModel
    let length: Int
    private let left: CGFloat = 30
    @State private var pinchBase: Double?
    @State private var pinchAnchor = 0.0
    @FocusState private var focused: Bool

    private func bp(at x: CGFloat, width: CGFloat) -> Int {
        let t = Double((x - left) / max(1, width - 2 * left))
        return min(length, max(0, Int((model.viewStart + t * model.viewSpan).rounded())))
    }

    var body: some View {
        GeometryReader { geo in
            Canvas { ctx, size in draw(ctx, size) }
                .contentShape(Rectangle())
                .gesture(DragGesture(minimumDistance: 0).onChanged { v in
                    focused = true
                    model.select(from: bp(at: v.startLocation.x, width: geo.size.width), to: bp(at: v.location.x, width: geo.size.width))
                })
                .simultaneousGesture(MagnifyGesture().onChanged { v in
                    if pinchBase == nil {
                        pinchBase = model.zoom
                        let t = Double((v.startLocation.x - left) / max(1, geo.size.width - 2 * left))
                        pinchAnchor = model.viewStart + t * model.viewSpan
                    }
                    let t = Double((v.startLocation.x - left) / max(1, geo.size.width - 2 * left))
                    let nz = min(max(1, (pinchBase ?? 1) * Double(v.magnification)), model.maxZoom)
                    let span = max(1, Double(length) / nz)
                    model.zoom = nz
                    model.viewStart = min(max(0, pinchAnchor - t * span), max(0, Double(length) - span))
                }.onEnded { _ in pinchBase = nil })
        }
        .padding(8)
        .editKeys(model, focus: $focused)
    }

    private func draw(_ ctx: GraphicsContext, _ size: CGSize) {
        var ctx = ctx
        let right = size.width - 30
        let y0: CGFloat = 90
        let lo = model.viewStart, span = model.viewSpan
        func x(_ p: Double) -> CGFloat { left + (right - left) * CGFloat((p - lo) / span) }
        func inView(_ p: Double) -> Bool { p >= lo - 1 && p <= lo + span + 1 }
        // Auswahl
        if let a = model.selStart, let b = model.selEnd {
            let s0 = Double(min(a, b)), s1 = Double(max(a, b))
            if s0 == s1 {
                if inView(s0) { ctx.stroke(Path { $0.move(to: CGPoint(x: x(s0), y: y0 - 50)); $0.addLine(to: CGPoint(x: x(s0), y: y0 + 130)) }, with: .color(Theme.accent), lineWidth: 2) }
            } else {
                let r = CGRect(x: max(left, x(s0)), y: y0 - 50, width: max(2, min(right, x(s1)) - max(left, x(s0))), height: 180)
                ctx.fill(Path(r), with: .color(Theme.accent.opacity(0.18)))
            }
        }
        let x0 = max(left, x(0)), x1 = min(right, x(Double(length)))
        ctx.stroke(Path { $0.move(to: CGPoint(x: x0, y: y0)); $0.addLine(to: CGPoint(x: x1, y: y0)) }, with: .color(.secondary), lineWidth: 2)
        let step = niceStep(length: Int(span), maxTicks: 10)
        var t = (Int(lo) / step) * step
        while Double(t) <= lo + span {
            if t >= 0 && t <= length {
                ctx.stroke(Path { $0.move(to: CGPoint(x: x(Double(t)), y: y0 - 4)); $0.addLine(to: CGPoint(x: x(Double(t)), y: y0 + 4)) }, with: .color(.secondary), lineWidth: 1)
                ctx.draw(Text(verbatim: "\(t)").font(.system(size: 9)).foregroundStyle(Theme.inkSoft), at: CGPoint(x: x(Double(t)), y: y0 + 14), anchor: .center)
            }
            t += step
        }
        for f in model.lanes where f.featureType.lowercased() != "source" {
            guard Double(f.end) >= lo, Double(f.start) <= lo + span else { continue }
            let y = y0 + 30 + CGFloat(f.lane) * 18
            let fx0 = max(left, x(Double(f.start))), fx1 = min(right, x(Double(f.end)))
            let rect = CGRect(x: fx0, y: y, width: max(2, fx1 - fx0), height: 14)
            ctx.fill(Path(roundedRect: rect, cornerRadius: 3), with: .color(Theme.feature(f.featureType)))
            if rect.width > 40 {
                ctx.draw(Text(f.name).font(.system(size: 9)).foregroundStyle(.white), at: CGPoint(x: rect.midX, y: rect.midY), anchor: .center)
            }
        }
        var lastX: CGFloat = -100
        var row = 0
        for s in model.visibleSites where inView(Double(s.position)) {
            let px = x(Double(s.position))
            guard px >= left, px <= right else { continue }
            row = px < lastX + 70 ? (row + 1) % 5 : 0
            lastX = px
            ctx.stroke(Path { $0.move(to: CGPoint(x: px, y: y0)); $0.addLine(to: CGPoint(x: px, y: y0 - 8 - CGFloat(row) * 12)) }, with: .color(.primary), lineWidth: 1)
            ctx.draw(Text(s.enzyme).font(.system(size: 10)), at: CGPoint(x: px + 2, y: y0 - 12 - CGFloat(row) * 12), anchor: .bottomLeading)
        }
    }
}
