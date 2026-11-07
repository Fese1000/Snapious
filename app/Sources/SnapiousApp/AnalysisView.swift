import SnapiousFFI
import SwiftUI

/// Automatic detection: open reading frames and known sequence motifs; hits can be added as features.
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
    @State private var finderName = tr("Hits")
    @State private var finderType = "misc_feature"
    @State private var finderMismatch = 1

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Stepper(tr("At least %@ codons", "\(minCodons)"), value: $minCodons, in: 10...2000, step: 10)
                    .frame(width: 260)
                Button(tr("Find ORFs")) { run(orfs: true) }
                Divider().frame(height: 18)
                Button(tr("Find known motifs")) { run(orfs: false) }
                Divider().frame(height: 18)
                Button(tr("Find similar sites …")) { showFinder = true }
                    .popover(isPresented: $showFinder) {
                        VStack(alignment: .leading, spacing: 10) {
                            Text(tr("Find feature by similarity")).font(.headline)
                            Text(tr("Searches a short sequence (e.g. promoter, primer site) on both strands, also with mismatches. Hits appear below and can be added."))
                                .font(.caption).foregroundStyle(.secondary)
                            TextField(tr("Sequence, e.g. TTGACA"), text: $finderRef).textFieldStyle(.roundedBorder).font(.system(.body, design: .monospaced))
                            TextField(tr("Feature name"), text: $finderName).textFieldStyle(.roundedBorder)
                            Picker(tr("Type"), selection: $finderType) {
                                ForEach(["misc_feature", "promoter", "terminator", "primer_bind", "protein_bind", "rep_origin", "gene", "CDS"], id: \.self) { Text($0).tag($0) }
                            }
                            Stepper(tr("Allowed differences: %@", "\(finderMismatch)"), value: $finderMismatch, in: 0...6)
                            HStack {
                                Spacer()
                                Button(tr("Search")) { findSimilar(); showFinder = false }
                                    .keyboardShortcut(.defaultAction)
                                    .disabled(finderRef.trimmingCharacters(in: .whitespaces).count < 4)
                            }
                        }
                        .padding(14)
                        .frame(width: 340)
                    }
                Button(tr("Extract features …")) { showExtract = true }
                    .popover(isPresented: $showExtract) {
                        VStack(alignment: .leading, spacing: 10) {
                            Text(tr("Features as new sequences")).font(.headline)
                            Picker(tr("Feature type"), selection: $extractType) {
                                Text(tr("All types")).tag("")
                                ForEach(Array(Set(model.features.map { $0.featureType })).sorted(), id: \.self) { Text($0).tag($0) }
                            }
                            Stepper(tr("Margin: %@ bases", "\(flank)"), value: $flank, in: 0...5000, step: flank >= 100 ? 100 : 10)
                            Text(tr("Features on the reverse strand are reverse-complemented.")).font(.caption).foregroundStyle(.secondary)
                            HStack {
                                Spacer()
                                Button(tr("Create")) {
                                    let n = model.extractFeatures(type: extractType, flank: flank)
                                    if n > 0 { message = tr("%@ new sequence(s) created.", "\(n)") }
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
                Button(tr("GC profile and CpG islands")) { stats() }
                if !cpg.isEmpty {
                    Text(tr("CpG islands: ") + cpg.map { "\($0.start + 1)–\($0.end)" }.joined(separator: ", "))
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
                TableColumn(tr("Type")) { r in Text(r.info.featureType) }.width(100)
                TableColumn(tr("Name")) { r in Text(r.info.name) }
                TableColumn(tr("Range")) { r in Text("\(r.info.start + 1)–\(r.info.end)") }.width(110)
                TableColumn(tr("Strand")) { r in Text(r.info.strand == "FORWARD" ? "+" : (r.info.strand == "REVERSE" ? "−" : "·")) }.width(60)
            }
            HStack {
                Text(kind.isEmpty ? tr("%@ hits", "\(results.count)") : tr("%@ hits (%@)", "\(results.count)", "\(kind)")).foregroundStyle(Theme.inkSoft)
                Spacer()
                Button(tr("Add all")) { add(results.map { $0.info }) }.disabled(results.isEmpty)
                Button(tr("Add selection")) { add(results.filter { chosen.contains($0.id) }.map { $0.info }) }
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
            kind = orfs ? tr("ORFs") : tr("Motifs")
            if found.isEmpty { message = tr("Nothing found.") }
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
                let label = h.mismatches == 0 ? finderName : tr("%@ (%@ difference%@)", "\(finderName)", "\(h.mismatches)", "\(h.mismatches == 1 ? "" : "e")")
                return FoundRow(id: i, info: FeatureInfo(featureType: finderType, name: label, start: h.start, end: h.end, strand: h.reverse ? "REVERSE" : "FORWARD"))
            }
            chosen = []
            kind = tr("Hits")
            if hits.isEmpty { message = tr("Nothing found.") }
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
            message = tr("%@ feature(s) added (undo with ⌘Z in the sequence view).", "\(infos.count)")
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
