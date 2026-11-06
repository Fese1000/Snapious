import SnapiousFFI
import SwiftUI

/// Automatische Erkennung: offene Leserahmen und bekannte Sequenzmotive; Treffer lassen sich als Features eintragen.
struct AnalysisView: View {
    @ObservedObject var model: AppModel
    @State private var minCodons = 100
    @State private var results: [FoundRow] = []
    @State private var chosen = Set<Int>()
    @State private var message: String?
    @State private var kind = ""
    @State private var gc: [GcPointDto] = []
    @State private var cpg: [RegionDto] = []
    @State private var showExtract = false
    @State private var extractType = ""
    @State private var flank = 0
    @State private var showFinder = false
    @State private var finderRef = ""
    @State private var finderName = "Treffer"
    @State private var finderType = "misc_feature"
    @State private var finderMismatch = 1

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Stepper("Mindestens \(minCodons) Codons", value: $minCodons, in: 10...2000, step: 10)
                    .frame(width: 260)
                Button("ORFs suchen") { run(orfs: true) }
                Divider().frame(height: 18)
                Button("Bekannte Motive suchen") { run(orfs: false) }
                Divider().frame(height: 18)
                Button("Ähnliche Stellen suchen …") { showFinder = true }
                    .popover(isPresented: $showFinder) {
                        VStack(alignment: .leading, spacing: 10) {
                            Text("Feature per Ähnlichkeit finden").font(.headline)
                            Text("Sucht eine kurze Sequenz (z. B. Promotor, Primerstelle) auf beiden Strängen, auch mit Fehlpaarungen. Die Treffer erscheinen unten und lassen sich eintragen.")
                                .font(.caption).foregroundStyle(.secondary)
                            TextField("Sequenz, z. B. TTGACA", text: $finderRef).textFieldStyle(.roundedBorder).font(.system(.body, design: .monospaced))
                            TextField("Name des Features", text: $finderName).textFieldStyle(.roundedBorder)
                            Picker("Typ", selection: $finderType) {
                                ForEach(["misc_feature", "promoter", "terminator", "primer_bind", "protein_bind", "rep_origin", "gene", "CDS"], id: \.self) { Text($0).tag($0) }
                            }
                            Stepper("Erlaubte Unterschiede: \(finderMismatch)", value: $finderMismatch, in: 0...6)
                            HStack {
                                Spacer()
                                Button("Suchen") { findSimilar(); showFinder = false }
                                    .keyboardShortcut(.defaultAction)
                                    .disabled(finderRef.trimmingCharacters(in: .whitespaces).count < 4)
                            }
                        }
                        .padding(14)
                        .frame(width: 340)
                    }
                Button("Features extrahieren …") { showExtract = true }
                    .popover(isPresented: $showExtract) {
                        VStack(alignment: .leading, spacing: 10) {
                            Text("Features als neue Sequenzen").font(.headline)
                            Picker("Feature-Typ", selection: $extractType) {
                                Text("Alle Typen").tag("")
                                ForEach(Array(Set(model.features.map { $0.featureType })).sorted(), id: \.self) { Text($0).tag($0) }
                            }
                            Stepper("Rand: \(flank) Basen", value: $flank, in: 0...5000, step: flank >= 100 ? 100 : 10)
                            Text("Features auf dem Gegenstrang werden revers-komplementiert.").font(.caption).foregroundStyle(.secondary)
                            HStack {
                                Spacer()
                                Button("Anlegen") {
                                    let n = model.extractFeatures(type: extractType, flank: flank)
                                    if n > 0 { message = "\(n) neue Sequenz(en) angelegt." }
                                    showExtract = false
                                }
                                .keyboardShortcut(.defaultAction)
                            }
                        }
                        .padding(14)
                        .frame(width: 320)
                    }
                Spacer()
            }
            if let message { Text(message).foregroundStyle(Theme.inkSoft) }
            HStack {
                Button("GC-Verlauf und CpG-Inseln") { stats() }
                if !cpg.isEmpty {
                    Text("CpG-Inseln: " + cpg.map { "\($0.start + 1)–\($0.end)" }.joined(separator: ", "))
                        .font(Theme.label(11)).foregroundStyle(Theme.inkSoft).lineLimit(2)
                }
            }
            if !gc.isEmpty {
                Canvas { ctx, size in
                    let maxPos = Double(gc.last?.position ?? 1) + 1
                    var path = Path()
                    for (i, p) in gc.enumerated() {
                        let x = size.width * CGFloat(Double(p.position) / maxPos)
                        let y = size.height * CGFloat(1 - p.percent / 100)
                        if i == 0 { path.move(to: CGPoint(x: x, y: y)) } else { path.addLine(to: CGPoint(x: x, y: y)) }
                    }
                    ctx.stroke(Path(CGRect(origin: .zero, size: size)), with: .color(.secondary.opacity(0.4)), lineWidth: 1)
                    ctx.stroke(path, with: .color(Theme.accent), lineWidth: 1.5)
                    ctx.draw(Text("GC %").font(.system(size: 9)).foregroundStyle(Theme.inkSoft), at: CGPoint(x: 4, y: 8), anchor: .leading)
                }
                .frame(height: 90)
            }
            Table(results, selection: $chosen) {
                TableColumn("Typ") { r in Text(r.info.featureType) }.width(100)
                TableColumn("Name") { r in Text(r.info.name) }
                TableColumn("Bereich") { r in Text("\(r.info.start + 1)–\(r.info.end)") }.width(110)
                TableColumn("Strang") { r in Text(r.info.strand == "FORWARD" ? "+" : (r.info.strand == "REVERSE" ? "−" : "·")) }.width(60)
            }
            HStack {
                Text(kind.isEmpty ? "\(results.count) Treffer" : "\(results.count) Treffer (\(kind))").foregroundStyle(Theme.inkSoft)
                Spacer()
                Button("Alle eintragen") { add(results.map { $0.info }) }.disabled(results.isEmpty)
                Button("Auswahl eintragen") { add(results.filter { chosen.contains($0.id) }.map { $0.info }) }
                    .disabled(chosen.isEmpty)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: model.selection) { _, _ in
            results = []
            chosen = []
            message = nil
            gc = []
            cpg = []
        }
    }

    private func run(orfs: Bool) {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        do {
            let found = orfs ? try project.findOrfs(id: id, minCodons: UInt64(minCodons)) : try project.detectCommonFeatures(id: id)
            results = found.enumerated().map { FoundRow(id: $0.offset, info: $0.element) }
            chosen = []
            kind = orfs ? "ORFs" : "Motive"
            if found.isEmpty { message = "Nichts gefunden." }
        } catch {
            results = []
            message = error.localizedDescription
        }
    }

    private func findSimilar() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        do {
            let hits = try project.findSimilarFeatures(id: id, reference: finderRef, maxMismatch: UInt64(finderMismatch))
            results = hits.enumerated().map { i, h in
                let label = h.mismatches == 0 ? finderName : "\(finderName) (\(h.mismatches) Unterschied\(h.mismatches == 1 ? "" : "e"))"
                return FoundRow(id: i, info: FeatureInfo(featureType: finderType, name: label, start: h.start, end: h.end, strand: h.reverse ? "REVERSE" : "FORWARD"))
            }
            chosen = []
            kind = "Treffer"
            if hits.isEmpty { message = "Nichts gefunden." }
        } catch {
            results = []
            message = error.localizedDescription
        }
    }

    private func stats() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        do {
            let length = Int(model.summary?.length ?? 0)
            let window = UInt64(max(10, min(500, length / 40)))
            gc = try project.gcProfile(id: id, window: window, step: max(1, window / 4))
            cpg = try project.cpgIslandRegions(id: id)
        } catch {
            gc = []
            cpg = []
            message = error.localizedDescription
        }
    }

    private func add(_ infos: [FeatureInfo]) {
        guard let project = model.project, let id = model.selection, !infos.isEmpty else { return }
        do {
            try project.addFeatures(id: id, features: infos)
            model.reloadCurrent()
            message = "\(infos.count) Feature(s) eingetragen (Rückgängig mit Strg+Z in der Sequenzansicht)."
        } catch {
            message = error.localizedDescription
        }
    }
}

struct FoundRow: Identifiable, Hashable {
    let id: Int
    let info: FeatureInfo
    static func == (a: FoundRow, b: FoundRow) -> Bool { a.id == b.id }
    func hash(into h: inout Hasher) { h.combine(id) }
}
