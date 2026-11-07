import AppKit
import SnapiousFFI
import SwiftUI

/// Sequenzansicht: Zeilen mit Positionszahl, Basen in Zehnergruppen und Annotationsleisten darunter.
/// Die Spuren der Features kommen aus dem Rust-Kern (`featureLanes`); hier wird nur gezeichnet.
struct SequenceView: View {
    @ObservedObject var model: AppModel
    @FocusState private var focused: Bool
    @State private var searchText = ""
    @State private var showFeature = false
    @State private var featureName = ""
    @State private var featureType = "misc_feature"
    @State private var featureStrand = "FORWARD"

    private let font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
    private let marginWidth: CGFloat = 76
    private let laneHeight: CGFloat = 12
    private let groupSize = 9

    private var charWidth: CGFloat {
        ("M" as NSString).size(withAttributes: [.font: font]).width
    }

    private var laneCount: Int {
        (model.visibleLanes.map { Int($0.lane) }.max() ?? -1) + 1
    }

    /// Platz für die Aminosäurezeile unter den Basen (nur wenn CDS-Features vorhanden sind).
    /// Platz für die Aminosäurezeilen; bei aktiver Markierungsübersetzung bleibt eine Zeile frei, damit sich beim Markieren nichts verschiebt.
    private var codonRow: CGFloat {
        let base = model.codons.isEmpty ? 0 : (model.codons.map { $0.row }.max() ?? 0) + 1
        let extra = model.translateSelection != "off" && model.summary?.seqType != "PROTEIN" ? 1 : 0
        return CGFloat(base + extra) * 14
    }

    private var allCodons: [CodonMark] { model.allCodons }

    /// Platz über den Basen für die Namen der Schnittstellen (zwei versetzte Zeilen).
    private var siteRow: CGFloat { model.summary?.seqType == "PROTEIN" || model.visibleSites.isEmpty ? 0 : 2 * (siteFont + 2) + 4 }

    /// Schriftgröße der Enzymnamen: beim Herauszoomen wächst sie mit, damit sie auf dem Bildschirm lesbar bleiben.
    private var siteFont: CGFloat { scale < 1 ? min(13 / scale, 70) : 9 }

    /// Vergrößerungsfaktor der linearen Ansicht (Pinch, Knöpfe, Regler).
    private var scale: CGFloat { CGFloat(model.textScale) }

    private var rowHeight: CGFloat {
        siteRow + 20 + codonRow + CGFloat(laneCount) * laneHeight + 8
    }

    var body: some View {
        VStack(spacing: 0) {
            GeometryReader { geo in
                let bpl = basesPerLine(width: geo.size.width / scale)
                let total = model.fullBytes.count
                let lines = max(1, (total + bpl - 1) / bpl)
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            ForEach(0..<lines, id: \.self) { line in
                                lineView(line: line, bpl: bpl, total: total)
                                    .frame(height: rowHeight * scale)
                                    .id(line)
                            }
                        }
                        .frame(width: geo.size.width, alignment: .leading)
                        .contentShape(Rectangle())
                        .gesture(dragGesture(bpl: bpl, total: total))
                    }
                    .onAppear { proxy.scrollTo(Int(model.viewStart + model.viewSpan / 2) / bpl, anchor: .center) }
                    .onChange(of: model.scrollBP) { _, v in
                        withAnimation(.easeInOut(duration: 0.15)) { proxy.scrollTo(v / bpl, anchor: .center) }
                    }
                    .onChange(of: model.viewStart) { _, _ in
                        withAnimation(.easeInOut(duration: 0.15)) { proxy.scrollTo(Int(model.viewStart + model.viewSpan / 2) / bpl, anchor: .center) }
                    }
                }
            }
            coverageStrip
            statusBar
        }
        .editKeys(model, focus: $focused)
    }

    /// Lesetiefe als Fläche über die ganze Sequenz (nur wenn eine SAM/BAM-Datei gewählt wurde).
    @ViewBuilder
    private var coverageStrip: some View {
        if !model.coverage.isEmpty {
            let maxDepth = max(model.coverage.max() ?? 1, 1)
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(tr("Coverage: %@", "\(model.coverageLabel)")).font(Theme.label(11))
                    Text(String(format: tr("max %.0f×, Ø %.1f×"), maxDepth, model.coverage.reduce(0, +) / Double(model.coverage.count)))
                        .font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
                    Spacer()
                    Button { model.coverage = [] } label: { Image(systemName: "xmark.circle") }.accessibilityLabel(tr("Coverage ausblenden")).buttonStyle(.plain)
                }
                Canvas { ctx, size in
                    let n = model.coverage.count
                    var area = Path()
                    area.move(to: CGPoint(x: 0, y: size.height))
                    for (i, v) in model.coverage.enumerated() {
                        let x = size.width * (CGFloat(i) + 0.5) / CGFloat(n)
                        area.addLine(to: CGPoint(x: x, y: size.height * (1 - CGFloat(v / maxDepth))))
                    }
                    area.addLine(to: CGPoint(x: size.width, y: size.height))
                    area.closeSubpath()
                    ctx.fill(area, with: .color(.teal.opacity(0.55)))
                    ctx.stroke(Path(CGRect(origin: .zero, size: size)), with: .color(.secondary.opacity(0.3)), lineWidth: 1)
                }
                .frame(height: 54)
            }
            .padding(.horizontal, 8).padding(.vertical, 4)
            .background(.bar)
        }
    }

    /// Für Bildschirmfotos ohne Fenster: die ersten `lines` Zeilen ohne ScrollView und GeometryReader.
    func snapshot(width: CGFloat, lines: Int) -> some View {
        let bpl = basesPerLine(width: width)
        let total = model.fullBytes.count
        return VStack(alignment: .leading, spacing: 0) {
            ForEach(0..<lines, id: \.self) { line in
                lineView(line: line, bpl: bpl, total: total).frame(height: rowHeight * scale)
            }
        }
        .frame(width: width, alignment: .leading)
    }

    // MARK: Geometrie

    private func basesPerLine(width: CGFloat) -> Int {
        let groupWidth = CGFloat(groupSize + 1) * charWidth
        let groups = max(1, Int((width - marginWidth - 16) / groupWidth))
        return groups * groupSize
    }

    private func x(forColumn c: Int) -> CGFloat {
        marginWidth + (CGFloat(c) + CGFloat(c / groupSize)) * charWidth
    }

    private func position(at point: CGPoint, bpl: Int, total: Int) -> Int {
        let line = max(0, Int(point.y / rowHeight))
        let rel = (point.x - marginWidth) / charWidth
        // Spalte aus der x-Lage zurückrechnen (jede Gruppe hat eine Lücke von einem Zeichen)
        let groupWidth = Double(groupSize + 1)
        let g = max(0, Int(rel / groupWidth))
        let inGroup = rel - Double(g) * groupWidth
        let col = g * groupSize + min(groupSize, max(0, Int((inGroup + 0.5).rounded(.down))))
        return min(total, line * bpl + min(col, bpl))
    }

    private func dragGesture(bpl: Int, total: Int) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                focused = true
                let s = scale
                let a = position(at: CGPoint(x: value.startLocation.x / s, y: value.startLocation.y / s), bpl: bpl, total: total)
                let b = position(at: CGPoint(x: value.location.x / s, y: value.location.y / s), bpl: bpl, total: total)
                model.select(from: a, to: b)
            }
            .onEnded { value in
                guard hypot(value.translation.width, value.translation.height) < 3 else { return }
                let s = scale
                let pos = position(at: CGPoint(x: value.startLocation.x / s, y: value.startLocation.y / s), bpl: bpl, total: total)
                let line = max(0, Int(value.startLocation.y / s / rowHeight))
                let localY = value.startLocation.y / s - CGFloat(line) * rowHeight - siteRow - 22 - codonRow
                guard localY >= 0 else { return }
                let lane = Int(localY / laneHeight)
                if let f = model.visibleLanes.first(where: { Int($0.lane) == lane && Int($0.start) <= pos && pos < Int($0.end) && $0.featureType.lowercased() != "source" }) {
                    model.focusFeature(Int(f.featureIndex), scroll: false)
                }
            }
    }

    // MARK: Zeichnen

    private func lineView(line: Int, bpl: Int, total: Int) -> some View {
        let start = line * bpl
        let end = min(total, start + bpl)
        return Canvas { gc, size in
            var ctx = gc
            ctx.scaleBy(x: scale, y: scale)
            let cw = charWidth
            let tiny = cw * scale < 6
            let codons = allCodons
            // Auswahl
            if let a = model.selStart, let b = model.selEnd, a != b {
                let lo = max(min(a, b), start), hi = min(max(a, b), end)
                if lo < hi {
                    let x0 = x(forColumn: lo - start)
                    let x1 = x(forColumn: hi - 1 - start) + cw
                    ctx.fill(Path(CGRect(x: x0, y: siteRow, width: x1 - x0, height: 20)), with: .color(Theme.accent.opacity(0.3)))
                }
            }
            // Positionszahl
            ctx.draw(Text("\(start + 1)").font(.system(size: 11, design: .monospaced)).foregroundStyle(Theme.inkSoft),
                     at: CGPoint(x: marginWidth - 8, y: siteRow + 10), anchor: .trailing)
            // Basen
            let bases = model.fullBytes
            for i in start..<end {
                let c = i - start
                if tiny {
                    ctx.fill(Path(CGRect(x: x(forColumn: c), y: siteRow + 3, width: cw, height: 14)), with: .color(color(of: bases[i])))
                    continue
                }
                let ch = String(UnicodeScalar(bases[i]))
                ctx.draw(Text(ch).font(.system(size: 13, design: .monospaced)).foregroundStyle(color(of: bases[i])),
                         at: CGPoint(x: x(forColumn: c), y: siteRow + 10), anchor: .leading)
            }
            // Aminosäuren: ein farbiger Block über die drei Basen des Codons, der Buchstabe in der Mitte
            if codonRow > 0 {
                for m in codons where m.hi >= start && m.lo < end {
                    let y = siteRow + 22 + CGFloat(m.row) * 14
                    let x0 = x(forColumn: max(m.lo, start) - start)
                    let x1 = x(forColumn: min(m.hi, end - 1) - start) + cw
                    let rect = CGRect(x: x0 + 0.5, y: y, width: max(2, x1 - x0 - 1), height: 13)
                    ctx.fill(Path(roundedRect: rect, cornerRadius: 3), with: .color(aminoColor(m).opacity(0.38)))
                    ctx.stroke(Path(roundedRect: rect, cornerRadius: 3), with: .color(aminoColor(m).opacity(0.8)), lineWidth: 0.6)
                    let mid = m.lo + 1
                    if mid >= start && mid < end {
                        ctx.draw(Text(m.letter).font(.system(size: 10, weight: .semibold, design: .monospaced)).foregroundStyle(.primary),
                                 at: CGPoint(x: x(forColumn: mid - start) + cw / 2, y: y + 6.5), anchor: .center)
                    }
                }
            }
            // Schnittstellen: Strich durch die Basenzeile, Name darüber (abwechselnd in zwei Zeilen)
            if siteRow > 0 {
                var n = 0
                for site in model.visibleSites where site.position >= start && site.position <= end {
                    let col = site.position - start
                    let gap: CGFloat = (col > 0 && col % groupSize == 0) ? charWidth / 2 : 0
                    let sx: CGFloat = x(forColumn: col) - gap
                    let step = siteFont + 2
                    ctx.stroke(Path { $0.move(to: CGPoint(x: sx, y: siteRow - 4 - CGFloat(n % 2) * step)); $0.addLine(to: CGPoint(x: sx, y: siteRow + 18)) },
                               with: .color(.red.opacity(0.8)), lineWidth: max(1, 1 / scale))
                    ctx.draw(Text(site.enzyme).font(.system(size: siteFont, weight: scale < 1 ? .semibold : .regular)).foregroundStyle(.red), at: CGPoint(x: sx + 2, y: siteRow - 6 - CGFloat(n % 2) * step), anchor: .bottomLeading)
                    n += 1
                }
            }
            // Annotationen
            for f in model.visibleLanes {
                let s = Int(f.start), e = Int(f.end)
                if e <= start || s >= end { continue }
                let lo = max(s, start), hi = min(e, end)
                let x0 = x(forColumn: lo - start)
                let x1 = x(forColumn: hi - 1 - start) + cw
                let y = siteRow + 22 + codonRow + CGFloat(f.lane) * laneHeight
                let rect = CGRect(x: x0, y: y, width: x1 - x0, height: laneHeight - 2)
                ctx.fill(Path(roundedRect: rect, cornerRadius: 3), with: .color(featureColor(f.featureType)))
                if rect.width > 40 {
                    let arrow = f.strand == "FORWARD" ? " ▶" : (f.strand == "REVERSE" ? "◀ " : "")
                    let label = f.strand == "REVERSE" ? arrow + f.name : f.name + arrow
                    ctx.draw(Text(label).font(.system(size: 9)).foregroundStyle(.white),
                             at: CGPoint(x: rect.midX, y: rect.midY), anchor: .center)
                }
            }
        }
    }

    /// Basenfarben nur bei Nukleotiden; Aminosäuren bleiben in der Textfarbe.
    private func color(of b: UInt8) -> Color { model.summary?.seqType == "PROTEIN" ? .primary : Theme.base(Character(UnicodeScalar(b))) }

    private func featureColor(_ type: String) -> Color { Theme.feature(type) }

    // MARK: Statusleiste

    private var statusBar: some View {
        HStack(spacing: 16) {
            if let s = model.selStats, model.hasSelection {
                Text("\(s.length) bp").lineLimit(1).fixedSize()
                Text(String(format: "GC %.1f %%", s.gcPercent)).lineLimit(1).fixedSize()
                Text(String(format: tr("Tm %.1f °C"), s.tmCelsius)).lineLimit(1).fixedSize()
                Spacer()
                Button(tr("Kopieren")) { model.copySelection(format: "bases") }
                Button("FASTA") { model.copySelection(format: "fasta") }
                Button(tr("Rev.-kompl.")) { model.copySelection(format: "reverse_complement") }
                Button(tr("Feature…")) { showFeature = true }
                    .popover(isPresented: $showFeature) {
                        VStack(alignment: .leading, spacing: 8) {
                            Text(tr("Auswahl als Feature eintragen")).font(Theme.heading())
                            TextField(tr("Name"), text: $featureName).textFieldStyle(.roundedBorder).frame(width: 220)
                            Picker(tr("Typ"), selection: $featureType) {
                                ForEach(["misc_feature", "gene", "CDS", "promoter", "terminator", "rep_origin", "primer_bind", "protein_bind", "misc_binding"], id: \.self) { Text($0).tag($0) }
                            }
                            Picker(tr("Strang"), selection: $featureStrand) {
                                Text("+").tag("FORWARD")
                                Text("−").tag("REVERSE")
                                Text("ohne").tag("NONE")
                            }
                            .pickerStyle(.segmented)
                            HStack {
                                Spacer()
                                Button(tr("Eintragen")) {
                                    model.addFeatureFromSelection(name: featureName.isEmpty ? featureType : featureName, type: featureType, strand: featureStrand)
                                    showFeature = false
                                    featureName = ""
                                }
                                .keyboardShortcut(.defaultAction)
                            }
                        }
                        .padding(14)
                    }
                Button(tr("Aufheben")) { model.clearSelection() }
            } else {
                TextField(tr("Motiv suchen (IUPAC)"), text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 200)
                    .onSubmit { model.findMotif(searchText) }
                Button { model.nextMotif(-1) } label: { Image(systemName: "chevron.up") }.accessibilityLabel(tr("Voriger Treffer"))
                    .disabled(model.motifHits.isEmpty)
                Button { model.nextMotif(1) } label: { Image(systemName: "chevron.down") }.accessibilityLabel(tr("Nächster Treffer"))
                    .disabled(model.motifHits.isEmpty)
                if let m = model.motifMessage { Text(m).foregroundStyle(Theme.inkSoft) }
                Spacer()
            }
            if model.summary?.seqType != "PROTEIN" { AminoMenu(model: model) }
            Button { model.undo() } label: { Image(systemName: "arrow.uturn.backward") }.accessibilityLabel(tr("Rückgängig"))
                .keyboardShortcut("z", modifiers: .command)
                .disabled(model.undoState.undo == 0)
                .help(tr("Rückgängig"))
            Button { model.redo() } label: { Image(systemName: "arrow.uturn.forward") }.accessibilityLabel(tr("Wiederholen"))
                .keyboardShortcut("z", modifiers: [.command, .shift])
                .disabled(!model.undoState.redo)
                .help(tr("Wiederholen"))
        }
        .font(Theme.label(13))
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
