import AppKit
import SnapiousFFI
import SwiftUI

/// NGS-Werkzeuge: Qualitätsbericht und Trimming für FASTQ-Dateien, Varianten aus SAM/BAM gegen die gewählte Sequenz.
struct NgsView: View {
    @ObservedObject var model: AppModel
    @State private var qc: QcDto?
    @State private var qcName = ""
    @State private var trimMessage: String?
    @State private var snvs: [SnvDto] = []
    @State private var variantMessage: String?
    @State private var busy = false
    @State private var files: [ExternalFileDto] = []
    @State private var mapper = "minimap2"
    @State private var preset = "sr"
    @State private var threads = 4
    @State private var mapMessage: String?
    @State private var message: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if let message { Text(message).foregroundStyle(Theme.danger) }
                GroupBox(tr("Qualität (FASTQ, auch .gz)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Button(tr("FASTQ prüfen …")) { chooseFastq(forTrim: false) }.disabled(busy)
                            Button(tr("FASTQ trimmen …")) { chooseFastq(forTrim: true) }.disabled(busy)
                            if busy { ProgressView().controlSize(.small) }
                        }
                        if let trimMessage { Text(trimMessage).font(Theme.label(13)) }
                        if let qc { QcSummary(qc: qc, name: qcName) }
                    }
                    .padding(6)
                }
                CoverageReportSection(model: model)
                InsertSizeSection()
                KmerSection()
                BarcodeSection()
                FastqConvertSection()
                ExpressionSection()
                GroupBox(tr("Mapping: Reads auf die gewählte Sequenz abbilden")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Picker(tr("Programm"), selection: $mapper) {
                                Text("minimap2").tag("minimap2")
                                Text("BWA").tag("bwa")
                            }.frame(width: 190)
                            Picker(tr("Preset"), selection: $preset) {
                                Text(tr("Kurze Reads (sr)")).tag("sr")
                                Text("Nanopore (map-ont)").tag("map-ont")
                                Text("PacBio (map-pb)").tag("map-pb")
                            }.frame(width: 250).disabled(mapper == "bwa")
                            Stepper(tr("Threads %@", "\(threads)"), value: $threads, in: 1...16).frame(width: 130)
                        }
                        HStack {
                            Button(tr("Reads wählen und abbilden …")) { chooseReadsAndMap() }.disabled(busy || model.selection == nil)
                            if busy { ProgressView().controlSize(.small) }
                        }
                        if let mapMessage { Text(mapMessage).font(Theme.label(13)) }
                    }
                    .padding(6)
                }
                GroupBox(tr("Verknüpfte Dateien dieser Sequenz")) {
                    VStack(alignment: .leading, spacing: 6) {
                        if files.isEmpty { Text(tr("Noch keine.")).foregroundStyle(Theme.inkSoft) }
                        ForEach(files, id: \.id) { f in
                            HStack {
                                Text(f.kind.uppercased()).font(.caption.monospaced()).padding(.horizontal, 5)
                                    .background(.quaternary, in: RoundedRectangle(cornerRadius: 3))
                                VStack(alignment: .leading) {
                                    Text(f.label)
                                    Text(f.path).font(Theme.label(11)).foregroundStyle(Theme.inkSoft).lineLimit(1).truncationMode(.middle)
                                }
                                Spacer()
                                if f.kind == "sam" || f.kind == "bam" {
                                    Button(tr("Coverage zeigen")) { model.showCoverage(path: f.path, label: f.label) }
                                }
                                Button { remove(f) } label: { Image(systemName: "trash") }.accessibilityLabel(tr("Verknüpfung entfernen")).buttonStyle(.plain)
                            }
                        }
                        Button(tr("Datei verknüpfen …")) { linkFile() }
                    }
                    .padding(6)
                }
                GroupBox(tr("Varianten (SAM oder BAM gegen die gewählte Sequenz)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        Button(tr("Alignment-Datei wählen …")) { chooseAlignment() }.disabled(busy)
                        if let variantMessage { Text(variantMessage).font(Theme.label(13)) }
                        ForEach(Array(snvs.prefix(200).enumerated()), id: \.offset) { _, s in
                            Text(tr("Position %@: %@ → %@ · Tiefe %@ · %@ % · Q %@", "\(s.position + 1)", "\(s.refBase)", "\(s.altBase)", "\(s.depth)", "\(String(format: "%.0f", s.altFraction * 100))", "\(String(format: "%.0f", s.quality))"))
                                .font(.system(.callout, design: .monospaced))
                        }
                    }
                    .padding(6)
                }
            }
            .padding(12)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear { reload() }
        .onChange(of: model.selection) { _, _ in
            snvs = []
            variantMessage = nil
            reload()
        }
    }

    private func reload() {
        guard let project = model.project, let id = model.selection else { files = []; return }
        files = (try? project.externalFiles(docId: id)) ?? []
    }

    private func remove(_ f: ExternalFileDto) {
        try? model.project?.removeExternalFile(id: f.id)
        reload()
    }

    private func linkFile() {
        guard let project = model.project, let id = model.selection else { return }
        let panel = NSOpenPanel()
        panel.message = tr("SAM, BAM, FASTQ oder VCF verknüpfen (die Datei wird nicht kopiert)")
        guard panel.runModal() == .OK, let url = panel.url else { return }
        var kind = url.pathExtension.lowercased()
        if kind == "gz" { kind = URL(fileURLWithPath: url.deletingPathExtension().path).pathExtension.lowercased() }
        if kind == "fq" { kind = "fastq" }
        do {
            _ = try project.addExternalFile(path: url.path, kind: kind.isEmpty ? "datei" : kind, docId: id, label: "")
            reload()
        } catch {
            message = error.localizedDescription
        }
    }

    private func chooseReadsAndMap() {
        guard let project = model.project, let id = model.selection else { return }
        let panel = NSOpenPanel()
        panel.message = tr("Eine oder zwei FASTQ-Dateien wählen (gepaarte Reads: beide)")
        panel.allowsMultipleSelection = true
        guard panel.runModal() == .OK, !panel.urls.isEmpty else { return }
        let paths = Array(panel.urls.prefix(2)).map { $0.path }
        let outDir = panel.urls[0].deletingLastPathComponent().appendingPathComponent("snapious-mapping").path
        let m = mapper, p = preset, t = threads
        busy = true
        mapMessage = nil
        message = nil
        Task.detached {
            do {
                let f = try project.mapReadsToReference(referenceId: id, readPaths: paths, mapper: m, preset: p,
                                                        threads: UInt64(t), outputDir: outDir)
                await MainActor.run {
                    mapMessage = tr("Fertig: %@", "\(f.path)")
                    busy = false
                    reload()
                    model.showCoverage(path: f.path, label: f.label)
                }
            } catch {
                await MainActor.run { message = error.localizedDescription; busy = false }
            }
        }
    }

    private func chooseFastq(forTrim: Bool) {
        let panel = NSOpenPanel()
        panel.message = tr("FASTQ-Datei wählen")
        guard panel.runModal() == .OK, let url = panel.url else { return }
        busy = true
        message = nil
        trimMessage = nil
        let path = url.path
        var out = ""
        if forTrim {
            let save = NSSavePanel()
            save.nameFieldStringValue = url.deletingPathExtension().lastPathComponent + ".trimmed.fastq"
            guard save.runModal() == .OK, let o = save.url else { busy = false; return }
            out = o.path
        }
        let name = url.lastPathComponent
        let output = out
        Task.detached {
            do {
                if forTrim {
                    let s = try trimFastq(input: path, output: output)
                    await MainActor.run {
                        trimMessage = tr("%@ → %@ Reads, %@ Basen entfernt, Adapter in %@ Reads.", "\(s.inputReads)", "\(s.outputReads)", "\(s.basesRemoved)", "\(s.adaptersFound)")
                        busy = false
                    }
                } else {
                    let r = try qcFastq(path: path, maxReads: 200_000)
                    await MainActor.run { qc = r; qcName = name; busy = false }
                }
            } catch {
                await MainActor.run { message = error.localizedDescription; busy = false }
            }
        }
    }

    private func chooseAlignment() {
        guard let project = model.project, let id = model.selection else { return }
        let panel = NSOpenPanel()
        panel.message = tr("SAM- oder BAM-Datei wählen")
        guard panel.runModal() == .OK, let url = panel.url else { return }
        busy = true
        variantMessage = nil
        let vcf = url.deletingPathExtension().appendingPathExtension("variants.vcf").path
        Task.detached {
            do {
                let v = try project.callVariants(referenceId: id, alignmentPath: url.path, vcfPath: vcf)
                await MainActor.run {
                    snvs = v
                    variantMessage = tr("%@ Variante(n); VCF: %@", "\(v.count)", "\(vcf)")
                    busy = false
                }
            } catch {
                await MainActor.run { message = error.localizedDescription; busy = false }
            }
        }
    }
}

/// Zusammenfassung mit Ampeln und Qualitätsdiagramm (Mittelwert und Quartile je Position).
struct QcSummary: View {
    let qc: QcDto
    let name: String

    private func light(_ ok: Bool, _ warn: Bool) -> Color { ok ? .green : (warn ? .orange : .red) }

    private var meanOfMeans: Double {
        qc.meanQuality.isEmpty ? 0 : qc.meanQuality.reduce(0, +) / Double(qc.meanQuality.count)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(name).font(Theme.heading())
            row(tr("%@ Reads, Länge %@–%@ (Ø %@)", "\(qc.reads)", "\(qc.minLength)", "\(qc.maxLength)", "\(String(format: "%.1f", qc.meanLength))"), .secondary)
            row(tr("Mittlere Qualität %@", "\(String(format: "%.1f", meanOfMeans))"), light(meanOfMeans >= 28, meanOfMeans >= 20))
            row(tr("GC-Gehalt %@ %", "\(String(format: "%.1f", qc.gcPercent))"), light(qc.gcPercent > 35 && qc.gcPercent < 65, qc.gcPercent > 25 && qc.gcPercent < 75))
            row(tr("Duplikation %@ %", "\(String(format: "%.1f", qc.duplicationPercent))"), light(qc.duplicationPercent < 20, qc.duplicationPercent < 50))
            row(tr("Adapter %@ %", "\(String(format: "%.1f", qc.adapterPercent))"), light(qc.adapterPercent < 1, qc.adapterPercent < 5))
            chart.frame(height: 160)
                .background(Theme.panel, in: RoundedRectangle(cornerRadius: 6))
            if !qc.overrepresented.isEmpty {
                Text(tr("Häufige Sequenzen: ") + qc.overrepresented.prefix(3).joined(separator: ", "))
                    .font(Theme.label(11)).foregroundStyle(Theme.inkSoft).lineLimit(2)
            }
        }
    }

    private func row(_ text: String, _ color: Color) -> some View {
        HStack(spacing: 6) {
            Circle().fill(color).frame(width: 9, height: 9)
            Text(text).font(Theme.label(13))
        }
    }

    /// Qualität je Position: Band zwischen unterem und oberem Quartil, Linie für den Mittelwert; Hintergrund grün/gelb/rot nach Phred 28/20.
    var chart: some View {
        Canvas { ctx, size in
            let n = max(qc.meanQuality.count, 1)
            let maxQ = 42.0
            func x(_ i: Int) -> CGFloat { size.width * (CGFloat(i) + 0.5) / CGFloat(n) }
            func y(_ q: Double) -> CGFloat { size.height * (1 - CGFloat(min(q, maxQ) / maxQ)) }
            ctx.fill(Path(CGRect(x: 0, y: y(maxQ), width: size.width, height: y(28) - y(maxQ))), with: .color(.green.opacity(0.12)))
            ctx.fill(Path(CGRect(x: 0, y: y(28), width: size.width, height: y(20) - y(28))), with: .color(.orange.opacity(0.12)))
            ctx.fill(Path(CGRect(x: 0, y: y(20), width: size.width, height: y(0) - y(20))), with: .color(.red.opacity(0.12)))
            guard qc.meanQuality.count > 1 else { return }
            var band = Path()
            for i in 0..<qc.q75.count { i == 0 ? band.move(to: CGPoint(x: x(i), y: y(qc.q75[i]))) : band.addLine(to: CGPoint(x: x(i), y: y(qc.q75[i]))) }
            for i in stride(from: qc.q25.count - 1, through: 0, by: -1) { band.addLine(to: CGPoint(x: x(i), y: y(qc.q25[i]))) }
            band.closeSubpath()
            ctx.fill(band, with: .color(Theme.accent.opacity(0.25)))
            var line = Path()
            for (i, q) in qc.meanQuality.enumerated() { i == 0 ? line.move(to: CGPoint(x: x(i), y: y(q))) : line.addLine(to: CGPoint(x: x(i), y: y(q))) }
            ctx.stroke(line, with: .color(Theme.accent), lineWidth: 1.6)
            ctx.draw(Text(tr("Phred je Position")).font(.system(size: 9)).foregroundStyle(Theme.inkSoft), at: CGPoint(x: 6, y: 8), anchor: .leading)
        }
    }
}
