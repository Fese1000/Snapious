import AppKit
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
                if pinchBase == nil { pinchBase = model.textScale }
                model.textScale = min(max(0.15, (pinchBase ?? 1) * Double(v.magnification)), 4)
            }
            .onEnded { _ in pinchBase = nil }
    }

    var body: some View {
        if let s = model.summary {
            VStack(spacing: 0) {
                toolbar(s)
                Divider()
                HStack(spacing: 0) {
                    if s.seqType == "PROTEIN" || !ring {
                        SequenceView(model: model)
                            .simultaneousGesture(pinch)
                    } else {
                        VStack(spacing: 0) {
                            CircularMap(model: model, length: max(1, Int(s.length)), name: s.name)
                            panSlider
                            MapEditBar(model: model)
                        }
                    }
                    if model.showAnnotations {
                        Divider()
                        AnnotationPanel(model: model)
                    }
                }
            }
        }
    }

    private func toolbar(_ s: DocumentSummary) -> some View {
        HStack(spacing: 12) {
            if s.seqType != "PROTEIN" {
                Picker("", selection: Binding(get: { ring ? "ring" : "linear" }, set: { model.mapStyle = $0; model.setZoom(model.zoom) })) {
                    Text(tr("Plasmid")).tag("ring")
                    Text(tr("Linear")).tag("linear")
                }
                .pickerStyle(.segmented).labelsHidden().frame(width: 150)
                .help(tr("Darstellung: Plasmidring oder lineare Sequenz (bearbeitbar in beiden)"))
            }
            if ring && s.seqType != "PROTEIN" {
                HStack(spacing: 4) {
                    Button { model.setZoom(model.zoom / 1.6) } label: { Image(systemName: "minus.magnifyingglass") }
                        .accessibilityLabel(tr("Herauszoomen")).disabled(model.zoom <= 1.001)
                    Slider(value: Binding(get: { zoomPosition }, set: { model.setZoom(zoom(from: $0)) }), in: 0...1)
                        .frame(width: 140)
                    Button { model.setZoom(model.zoom * 1.6) } label: { Image(systemName: "plus.magnifyingglass") }
                        .disabled(model.zoom >= model.maxZoom - 0.001)
                        .accessibilityLabel(tr("Hineinzoomen"))
                    Text(model.zoom <= 1.001 ? tr("Ganz") : String(format: "%.0f×", model.zoom)).font(Theme.label(11)).foregroundStyle(Theme.inkSoft).frame(width: 40, alignment: .leading)
                }
            } else {
                HStack(spacing: 4) {
                    Button { model.textScale = max(0.15, model.textScale / 1.25) } label: { Image(systemName: "minus.magnifyingglass") }
                        .accessibilityLabel(tr("Herauszoomen")).disabled(model.textScale <= 0.151)
                    Slider(value: Binding(get: { log(model.textScale / 0.15) / log(4 / 0.15) },
                                          set: { model.textScale = 0.15 * exp($0 * log(4 / 0.15)) }), in: 0...1)
                        .frame(width: 140)
                    Button { model.textScale = min(4, model.textScale * 1.25) } label: { Image(systemName: "plus.magnifyingglass") }
                        .accessibilityLabel(tr("Hineinzoomen")).disabled(model.textScale >= 3.99)
                    Text("\(Int(model.textScale * 100)) %").font(Theme.label(11)).foregroundStyle(Theme.inkSoft).frame(width: 44, alignment: .leading)
                }
            }
            Spacer()
            if s.seqType != "PROTEIN" {
                Menu {
                    Picker(tr("Schnittstellen"), selection: $model.siteFilter) {
                        Text(tr("Einzelschneider")).tag("unique")
                        Text(tr("Zweifachschneider")).tag("double")
                        ForEach(enzymeSets(), id: \.name) { Text($0.name).tag("set:\($0.name)") }
                        Text(tr("Alle bekannten Enzyme")).tag("all")
                        Text(tr("Eigene Auswahl")).tag("custom")
                    }
                    .pickerStyle(.inline)
                    Divider()
                    Button(tr("Enzyme einzeln wählen …")) { model.siteFilter = "custom"; showEnzymes = true }
                } label: {
                    Label(tr("Schnittstellen: %@ (%@)", "\(filterTitle)", "\(model.visibleSites.count)"), systemImage: "scissors")
                }
                .menuStyle(.borderlessButton).fixedSize()
                .popover(isPresented: $showEnzymes) { EnzymePicker(model: model) }
            }
            Button { model.showAnnotations.toggle() } label: { Label(tr("Annotationen"), systemImage: "tag") }
                .help(tr("Annotationsliste ein-/ausblenden"))
        }
        .font(Theme.label(13))
        .padding(.horizontal, 12).padding(.vertical, 6)
        .background(.bar)
        .onAppear { model.refreshEnzymeSets() }
    }

    private var filterTitle: String {
        switch model.siteFilter {
        case "unique": return tr("Einzelschneider")
        case "double": return tr("Zweifachschneider")
        case "all": return "alle"
        case "custom": return tr("Auswahl")
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
                Slider(value: Binding(get: { min(model.viewStart, max(0, Double(model.docLength) - model.viewSpan)) }, set: { model.panView(to: $0) }),
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
            TextField(tr("Enzym suchen"), text: $query).textFieldStyle(.roundedBorder)
            HStack {
                Button(tr("Alle schneidenden")) { model.customEnzymes = Set(counts.keys) }
                Button(tr("Keine")) { model.customEnzymes = [] }
                Spacer()
                Text(tr("%@ gewählt", "\(model.customEnzymes.count)")).foregroundStyle(Theme.inkSoft)
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
                Text(lo == hi ? tr("Cursor bei %@", "\(lo + 1)") : tr("Auswahl %@–%@ (%@ bp)", "\(lo + 1)", "\(hi)", "\(hi - lo)")).lineLimit(1).fixedSize()
                TextField(lo == hi ? tr("Basen einfügen") : tr("Ersetzen durch …"), text: $text)
                    .textFieldStyle(.roundedBorder).frame(maxWidth: 260)
                    .font(.system(size: 12, design: .monospaced))
                    .onSubmit { apply() }
                Button(lo == hi ? tr("Einfügen") : tr("Ersetzen")) { apply() }.disabled(text.isEmpty)
                Button(tr("Löschen")) { model.replaceSelection(with: "") }.disabled(lo == hi)
                Button(tr("Kopieren")) { model.copySelection(format: "bases") }.disabled(lo == hi)
                Button(tr("Aufheben")) { model.clearSelection() }
            } else {
                Text(tr("Auf die Karte klicken oder ziehen, um eine Stelle oder einen Bereich zu wählen – dann bearbeiten."))
                    .foregroundStyle(Theme.inkSoft)
            }
            Spacer()
            AminoMenu(model: model)
            Button { model.undo() } label: { Image(systemName: "arrow.uturn.backward") }.accessibilityLabel(tr("Rückgängig"))
                .disabled(model.undoState.undo == 0)
            Button { model.redo() } label: { Image(systemName: "arrow.uturn.forward") }.accessibilityLabel(tr("Wiederholen"))
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
    private var centreBP: Double { zoomed ? model.viewStart + model.viewSpan / 2 : model.ringRotation }

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
            }.onEnded { v in
                if hypot(v.translation.width, v.translation.height) < 3, let idx = featureHit(v.startLocation, size: geo.size) {
                    model.focusFeature(idx, scroll: false)
                }
            })
            .background(ScrollMonitor { dx in rotate(by: dx, size: geo.size) })
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

    /// Ob die Basen am Ring stehen und wie weit Skala und Features dafür nach innen rücken.
    private func innerLayout(_ size: CGSize) -> CGFloat {
        let r = geometry(size).r
        guard zoomed, 2 * Double.pi * Double(r) / Double(length) >= 8, model.summary?.seqType != "PROTEIN" else { return 0 }
        let rows = model.allCodons.isEmpty ? 0 : (model.allCodons.map { $0.row }.max() ?? 0) + 1
        return 20 + CGFloat(rows) * 17
    }

    /// Index der Annotation unter einem Klick auf die Spuren im Ring, sonst `nil`.
    private func featureHit(_ p: CGPoint, size: CGSize) -> Int? {
        let (c, r) = geometry(size)
        let dist = hypot(Double(p.x - c.x), Double(p.y - c.y))
        let pos = anchorBP(at: p, size: size)
        let inner = innerLayout(size)
        for f in model.visibleLanes where f.featureType.lowercased() != "source" {
            let rad = Double(r - 26 - inner - CGFloat(f.lane) * 14)
            guard abs(dist - rad) <= 12 else { continue }
            var d = pos
            let len = Double(length)
            d = d.truncatingRemainder(dividingBy: len); if d < 0 { d += len }
            if d >= Double(f.start) && d < Double(f.end) { return Int(f.featureIndex) }
        }
        return nil
    }

    /// Zwei Finger waagerecht auf dem Trackpad drehen den Ring (der Ring folgt den Fingern).
    private func rotate(by dx: CGFloat, size: CGSize) {
        let r = geometry(size).r
        let bp = -Double(dx) * Double(length) / (2 * .pi * Double(r))
        if zoomed {
            model.panView(to: model.viewStart + bp)
        } else {
            var v = (model.ringRotation + bp).truncatingRemainder(dividingBy: Double(length))
            if v < 0 { v += Double(length) }
            model.ringRotation = v
        }
    }

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
        model.viewStart = model.clampStart(centre - span / 2)
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
            let codons = showBases ? model.allCodons : []
            let aaRows = codons.isEmpty ? 0 : (codons.map { $0.row }.max() ?? 0) + 1
            let inner: CGFloat = showBases ? 20 + CGFloat(aaRows) * 17 : 0
            // Aminosäuren: ein Bogenblock über die drei Basen des Codons, der Buchstabe in der Mitte
            for m in codons where inView(Double(m.lo)) || inView(Double(m.hi)) {
                let rad = r - 34 - CGFloat(m.row) * 17
                let a0 = angle(Double(m.lo)) - .pi / 2, a1 = angle(Double(m.hi + 1)) - .pi / 2
                var arc = Path()
                arc.addArc(center: c, radius: rad, startAngle: .radians(a0), endAngle: .radians(a1), clockwise: false)
                ctx.stroke(arc, with: .color(aminoColor(m).opacity(0.4)), style: StrokeStyle(lineWidth: 14, lineCap: .butt))
                let am = angle(Double(m.lo) + 1.5)
                var g = ctx
                g.translateBy(x: c.x + rad * CGFloat(sin(am)), y: c.y - rad * CGFloat(cos(am)))
                g.rotate(by: .radians(am))
                g.draw(Text(m.letter).font(.system(size: 10, weight: .semibold, design: .monospaced)).foregroundStyle(.primary), at: .zero, anchor: .center)
            }
            // Basen entlang des Rings (obere Reihe: Sequenz, untere: Gegenstrang)
            if showBases {
                let bytes = model.fullBytes
                let lo = Int(model.viewStart) - 1, hi = Int(model.viewStart + model.viewSpan) + 2
                let comp: [UInt8: String] = [65: "T", 84: "A", 67: "G", 71: "C", 85: "A"]
                if lo < hi, !bytes.isEmpty {
                    for k in lo..<hi {
                        let i = ((k % bytes.count) + bytes.count) % bytes.count
                        let a = angle(Double(k) + 0.5)
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
            while zoomed ? Double(t) <= model.viewStart + model.viewSpan : t < length {
                defer { t += step }
                let pos = ((t % length) + length) % length
                guard inView(Double(t)) else { continue }
                var p = Path()
                p.move(to: point(Double(t), r - 4))
                p.addLine(to: point(Double(t), r + 4))
                ctx.stroke(p, with: .color(.secondary), lineWidth: 1)
                ctx.draw(Text(verbatim: "\(pos)").font(.system(size: 9)).foregroundStyle(Theme.inkSoft),
                         at: point(Double(t), r - 16 - inner), anchor: .center)
            }
            // Features (Spuren nach innen)
            for f in model.visibleLanes where f.featureType.lowercased() != "source" {
                let rad = r - 26 - inner - CGFloat(f.lane) * 14
                guard rad > 20 else { continue }
                guard !zoomed || (inView(Double(f.start)) || inView(Double(f.end)) || (Double(f.start) <= centreBP && Double(f.end) >= centreBP)) else { continue }
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

/// Fängt waagerechtes Zwei-Finger-Wischen über der Ansicht ab (SwiftUI hat dafür keine eigene Geste).
struct ScrollMonitor: NSViewRepresentable {
    var onScroll: (CGFloat) -> Void

    func makeNSView(context: Context) -> NSView {
        let v = MonitorView()
        v.onScroll = onScroll
        return v
    }

    func updateNSView(_ v: NSView, context: Context) { (v as? MonitorView)?.onScroll = onScroll }

    final class MonitorView: NSView {
        var onScroll: (CGFloat) -> Void = { _ in }
        private var monitor: Any?

        override func viewDidMoveToWindow() {
            if let m = monitor { NSEvent.removeMonitor(m); monitor = nil }
            guard window != nil else { return }
            monitor = NSEvent.addLocalMonitorForEvents(matching: .scrollWheel) { [weak self] e in
                guard let self, e.window == self.window else { return e }
                let p = self.convert(e.locationInWindow, from: nil)
                guard self.bounds.contains(p), abs(e.scrollingDeltaX) > abs(e.scrollingDeltaY) else { return e }
                self.onScroll(e.scrollingDeltaX)
                return nil
            }
        }

        deinit { if let m = monitor { NSEvent.removeMonitor(m) } }
    }
}
