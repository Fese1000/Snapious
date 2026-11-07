import AppKit
import SnapiousFFI
import SwiftUI

/// NGS tools: quality report and trimming for FASTQ files, variants from SAM/BAM against the selected sequence.
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
                GroupBox(tr("Quality (FASTQ, also .gz)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Button(tr("Check FASTQ …")) { chooseFastq(forTrim: false) }.disabled(busy)
                            Button(tr("Trim FASTQ …")) { chooseFastq(forTrim: true) }.disabled(busy)
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
                GroupBox(tr("Mapping: map reads onto the selected sequence")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Picker(tr("Program"), selection: $mapper) {
                                Text("minimap2").tag("minimap2")
                                Text("BWA").tag("bwa")
                            }.frame(width: 190)
                            Picker(tr("Preset"), selection: $preset) {
                                Text(tr("Short reads (sr)")).tag("sr")
                                Text("Nanopore (map-ont)").tag("map-ont")
                                Text("PacBio (map-pb)").tag("map-pb")
                            }.frame(width: 250).disabled(mapper == "bwa")
                            Stepper(tr("Threads %@", "\(threads)"), value: $threads, in: 1...16).frame(width: 130)
                        }
                        HStack {
                            Button(tr("Choose reads and map …")) { chooseReadsAndMap() }.disabled(busy || model.selection == nil)
                            if busy { ProgressView().controlSize(.small) }
                        }
                        if let mapMessage { Text(mapMessage).font(Theme.label(13)) }
                    }
                    .padding(6)
                }
                GroupBox(tr("Linked files of this sequence")) {
                    VStack(alignment: .leading, spacing: 6) {
                        if files.isEmpty { Text(tr("None yet.")).foregroundStyle(Theme.inkSoft) }
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
                                    Button(tr("Show coverage")) { model.showCoverage(path: f.path, label: f.label) }
                                }
                                Button { remove(f) } label: { Image(systemName: "trash") }.accessibilityLabel(tr("Remove link")).buttonStyle(.plain)
                            }
                        }
                        Button(tr("Link file …")) { linkFile() }
                    }
                    .padding(6)
                }
                GroupBox(tr("Variants (SAM or BAM against the selected sequence)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        Button(tr("Choose alignment file …")) { chooseAlignment() }.disabled(busy)
                        if let variantMessage { Text(variantMessage).font(Theme.label(13)) }
                        ForEach(Array(snvs.prefix(200).enumerated()), id: \.offset) { _, s in
                            Text(tr("Position %@: %@ → %@ · depth %@ · %@ % · Q %@", "\(s.position + 1)", "\(s.refBase)", "\(s.altBase)", "\(s.depth)", "\(String(format: "%.0f", s.altFraction * 100))", "\(String(format: "%.0f", s.quality))"))
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
        panel.message = tr("Link SAM, BAM, FASTQ or VCF (the file is not copied)")
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
        panel.message = tr("Choose one or two FASTQ files (paired reads: both)")
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
                    mapMessage = tr("Done: %@", "\(f.path)")
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
        panel.message = tr("Choose FASTQ file")
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
                        trimMessage = tr("%@ → %@ reads, %@ bases removed, adapters in %@ reads.", "\(s.inputReads)", "\(s.outputReads)", "\(s.basesRemoved)", "\(s.adaptersFound)")
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
        panel.message = tr("Choose SAM or BAM file")
        guard panel.runModal() == .OK, let url = panel.url else { return }
        busy = true
        variantMessage = nil
        let vcf = url.deletingPathExtension().appendingPathExtension("variants.vcf").path
        Task.detached {
            do {
                let v = try project.callVariants(referenceId: id, alignmentPath: url.path, vcfPath: vcf)
                await MainActor.run {
                    snvs = v
                    variantMessage = tr("%@ variant(s); VCF: %@", "\(v.count)", "\(vcf)")
                    busy = false
                }
            } catch {
                await MainActor.run { message = error.localizedDescription; busy = false }
            }
        }
    }
}

/// Summary with traffic lights and quality chart (mean and quartiles per position).
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
            row(tr("%@ reads, length %@–%@ (avg %@)", "\(qc.reads)", "\(qc.minLength)", "\(qc.maxLength)", "\(String(format: "%.1f", qc.meanLength))"), .secondary)
            row(tr("Mean quality %@", "\(String(format: "%.1f", meanOfMeans))"), light(meanOfMeans >= 28, meanOfMeans >= 20))
            row(tr("GC content %@ %", "\(String(format: "%.1f", qc.gcPercent))"), light(qc.gcPercent > 35 && qc.gcPercent < 65, qc.gcPercent > 25 && qc.gcPercent < 75))
            row(tr("Duplication %@ %", "\(String(format: "%.1f", qc.duplicationPercent))"), light(qc.duplicationPercent < 20, qc.duplicationPercent < 50))
            row(tr("Adapters %@ %", "\(String(format: "%.1f", qc.adapterPercent))"), light(qc.adapterPercent < 1, qc.adapterPercent < 5))
            chart.frame(height: 160)
                .background(Theme.panel, in: RoundedRectangle(cornerRadius: 6))
            if !qc.overrepresented.isEmpty {
                Text(tr("Frequent sequences: ") + qc.overrepresented.prefix(3).joined(separator: ", "))
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

    /// Quality per position: band between lower and upper quartile, line for the mean; background green/yellow/red by Phred 28/20.
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
            ctx.draw(Text(tr("Phred per position")).font(.system(size: 9)).foregroundStyle(Theme.inkSoft), at: CGPoint(x: 6, y: 8), anchor: .leading)
        }
    }
}
