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
    private let groupSize = 10

    private var charWidth: CGFloat {
        ("M" as NSString).size(withAttributes: [.font: font]).width
    }

    private var laneCount: Int {
        (model.lanes.map { Int($0.lane) }.max() ?? -1) + 1
    }

    /// Platz für die Aminosäurezeile unter den Basen (nur wenn CDS-Features vorhanden sind).
    private var codonRow: CGFloat { model.codons.isEmpty ? 0 : CGFloat((model.codons.map { $0.row }.max() ?? 0) + 1) * 14 }

    private var rowHeight: CGFloat {
        20 + codonRow + CGFloat(laneCount) * laneHeight + 8
    }

    var body: some View {
        VStack(spacing: 0) {
            GeometryReader { geo in
                let bpl = basesPerLine(width: geo.size.width)
                let total = model.fullBytes.count
                let lines = max(1, (total + bpl - 1) / bpl)
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        ForEach(0..<lines, id: \.self) { line in
                            lineView(line: line, bpl: bpl, total: total)
                                .frame(height: rowHeight)
                        }
                    }
                    .frame(width: geo.size.width, alignment: .leading)
                    .contentShape(Rectangle())
                    .gesture(dragGesture(bpl: bpl, total: total))
                }
            }
            coverageStrip
            statusBar
        }
        .focusable()
        .focused($focused)
        .focusEffectDisabled()
        .onPasteCommand(of: [.plainText]) { providers in
            _ = providers.first?.loadObject(ofClass: NSString.self) { obj, _ in
                if let t = obj as? String { Task { @MainActor in model.replaceSelection(with: t) } }
            }
        }
        .onKeyPress(phases: .down) { press in
            if press.modifiers.contains(.command) { return .ignored }
            switch press.key {
            case .delete: model.backspace(); return .handled
            case .leftArrow: model.moveCaret(by: -1); return .handled
            case .rightArrow: model.moveCaret(by: 1); return .handled
            default: break
            }
            let ch = press.characters.uppercased()
            let allowed = model.summary?.seqType == "PROTEIN" ? "ACDEFGHIKLMNPQRSTVWYXBZUOJ*" : "ACGTUNRYSWKMBDHV"
            if ch.count == 1, allowed.contains(ch), model.selStart != nil {
                model.replaceSelection(with: ch)
                return .handled
            }
            return .ignored
        }
    }

    /// Lesetiefe als Fläche über die ganze Sequenz (nur wenn eine SAM/BAM-Datei gewählt wurde).
    @ViewBuilder
    private var coverageStrip: some View {
        if !model.coverage.isEmpty {
            let maxDepth = max(model.coverage.max() ?? 1, 1)
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text("Coverage: \(model.coverageLabel)").font(Theme.label(11))
                    Text(String(format: "max %.0f×, Ø %.1f×", maxDepth, model.coverage.reduce(0, +) / Double(model.coverage.count)))
                        .font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
                    Spacer()
                    Button { model.coverage = [] } label: { Image(systemName: "xmark.circle") }.accessibilityLabel("Coverage ausblenden").buttonStyle(.plain)
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
                lineView(line: line, bpl: bpl, total: total).frame(height: rowHeight)
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
                let a = position(at: value.startLocation, bpl: bpl, total: total)
                let b = position(at: value.location, bpl: bpl, total: total)
                model.select(from: a, to: b)
            }
    }

    // MARK: Zeichnen

    private func lineView(line: Int, bpl: Int, total: Int) -> some View {
        let start = line * bpl
        let end = min(total, start + bpl)
        return Canvas { ctx, size in
            let cw = charWidth
            // Auswahl
            if let a = model.selStart, let b = model.selEnd, a != b {
                let lo = max(min(a, b), start), hi = min(max(a, b), end)
                if lo < hi {
                    let x0 = x(forColumn: lo - start)
                    let x1 = x(forColumn: hi - 1 - start) + cw
                    ctx.fill(Path(CGRect(x: x0, y: 0, width: x1 - x0, height: 20)), with: .color(Theme.accent.opacity(0.3)))
                }
            }
            // Positionszahl
            ctx.draw(Text("\(start + 1)").font(.system(size: 11, design: .monospaced)).foregroundStyle(Theme.inkSoft),
                     at: CGPoint(x: marginWidth - 8, y: 10), anchor: .trailing)
            // Basen
            let bases = model.fullBytes
            for i in start..<end {
                let c = i - start
                let ch = String(UnicodeScalar(bases[i]))
                ctx.draw(Text(ch).font(.system(size: 13, design: .monospaced)).foregroundStyle(color(of: bases[i])),
                         at: CGPoint(x: x(forColumn: c), y: 10), anchor: .leading)
            }
            // Aminosäuren: der Buchstabe steht unter der mittleren Codonbase
            if codonRow > 0 {
                for m in model.codons where m.hi >= start && m.lo < end {
                    let mid = m.lo + 1
                    guard mid >= start && mid < end else { continue }
                    let color: Color = m.isStop ? .red : (m.isStart ? .green : .secondary)
                    ctx.draw(Text(m.letter).font(.system(size: 10, design: .monospaced)).foregroundStyle(color),
                             at: CGPoint(x: x(forColumn: mid - start) + cw / 2, y: 26 + CGFloat(m.row) * 14), anchor: .center)
                }
            }
            // Annotationen
            for f in model.lanes {
                let s = Int(f.start), e = Int(f.end)
                if e <= start || s >= end { continue }
                let lo = max(s, start), hi = min(e, end)
                let x0 = x(forColumn: lo - start)
                let x1 = x(forColumn: hi - 1 - start) + cw
                let y = 22 + codonRow + CGFloat(f.lane) * laneHeight
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

    /// Auswahl der angezeigten Aminosäure-Leserahmen.
    private var frameMenu: some View {
        Menu {
            Toggle("Aminosäuren der CDS-Features", isOn: Binding(
                get: { model.showCdsTranslation },
                set: { model.setFrames(model.aaFrames, cds: $0) }))
            Divider()
            ForEach([1, 2, 3, -1, -2, -3], id: \.self) { f in
                Toggle(f > 0 ? "Rahmen +\(f)" : "Rahmen −\(-f) (Gegenstrang)", isOn: Binding(
                    get: { model.aaFrames.contains(f) },
                    set: { on in
                        var list = model.aaFrames.filter { $0 != f }
                        if on { list.append(f) }
                        let order = [1, 2, 3, -1, -2, -3]
                        model.setFrames(list.sorted { order.firstIndex(of: $0)! < order.firstIndex(of: $1)! }, cds: model.showCdsTranslation)
                    }))
            }
            Divider()
            Button("Alle 6 Rahmen") { model.setFrames([1, 2, 3, -1, -2, -3], cds: model.showCdsTranslation) }
            Button("Aus") { model.setFrames([], cds: false) }
        } label: {
            Label("Aminosäuren", systemImage: "textformat.abc")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .help("Aminosäuren unter der Sequenz anzeigen")
    }

    private var statusBar: some View {
        HStack(spacing: 16) {
            if let s = model.selStats, model.hasSelection {
                Text("\(s.length) bp").lineLimit(1).fixedSize()
                Text(String(format: "GC %.1f %%", s.gcPercent)).lineLimit(1).fixedSize()
                Text(String(format: "Tm %.1f °C", s.tmCelsius)).lineLimit(1).fixedSize()
                Spacer()
                Button("Kopieren") { model.copySelection(format: "bases") }
                Button("FASTA") { model.copySelection(format: "fasta") }
                Button("Rev.-kompl.") { model.copySelection(format: "reverse_complement") }
                Button("Feature…") { showFeature = true }
                    .popover(isPresented: $showFeature) {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Auswahl als Feature eintragen").font(Theme.heading())
                            TextField("Name", text: $featureName).textFieldStyle(.roundedBorder).frame(width: 220)
                            Picker("Typ", selection: $featureType) {
                                ForEach(["misc_feature", "gene", "CDS", "promoter", "terminator", "rep_origin", "primer_bind", "protein_bind", "misc_binding"], id: \.self) { Text($0).tag($0) }
                            }
                            Picker("Strang", selection: $featureStrand) {
                                Text("+").tag("FORWARD")
                                Text("−").tag("REVERSE")
                                Text("ohne").tag("NONE")
                            }
                            .pickerStyle(.segmented)
                            HStack {
                                Spacer()
                                Button("Eintragen") {
                                    model.addFeatureFromSelection(name: featureName.isEmpty ? featureType : featureName, type: featureType, strand: featureStrand)
                                    showFeature = false
                                    featureName = ""
                                }
                                .keyboardShortcut(.defaultAction)
                            }
                        }
                        .padding(14)
                    }
                Button("Aufheben") { model.clearSelection() }
            } else {
                TextField("Motiv suchen (IUPAC)", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 200)
                    .onSubmit { model.findMotif(searchText) }
                Button { model.nextMotif(-1) } label: { Image(systemName: "chevron.up") }.accessibilityLabel("Voriger Treffer")
                    .disabled(model.motifHits.isEmpty)
                Button { model.nextMotif(1) } label: { Image(systemName: "chevron.down") }.accessibilityLabel("Nächster Treffer")
                    .disabled(model.motifHits.isEmpty)
                if let m = model.motifMessage { Text(m).foregroundStyle(Theme.inkSoft) }
                Spacer()
            }
            if model.summary?.seqType != "PROTEIN" { frameMenu }
            Button { model.undo() } label: { Image(systemName: "arrow.uturn.backward") }.accessibilityLabel("Rückgängig")
                .keyboardShortcut("z", modifiers: .command)
                .disabled(model.undoState.undo == 0)
                .help("Rückgängig")
            Button { model.redo() } label: { Image(systemName: "arrow.uturn.forward") }.accessibilityLabel("Wiederholen")
                .keyboardShortcut("z", modifiers: [.command, .shift])
                .disabled(!model.undoState.redo)
                .help("Wiederholen")
        }
        .font(Theme.label(13))
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
