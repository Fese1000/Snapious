import AppKit
import SnapiousFFI
import SwiftUI

/// Helpers for file selection in the NGS tools.
enum FilePick {
    static func open(_ title: String, directories: Bool = false) -> URL? {
        let p = NSOpenPanel()
        p.title = title
        p.canChooseFiles = !directories
        p.canChooseDirectories = directories
        p.canCreateDirectories = directories
        return p.runModal() == .OK ? p.url : nil
    }

    static func save(_ title: String, name: String) -> URL? {
        let p = NSSavePanel()
        p.title = title
        p.nameFieldStringValue = name
        return p.runModal() == .OK ? p.url : nil
    }
}

/// Split FASTQ into separate files by barcodes at the start of the reads.
struct BarcodeSection: View {
    @State private var fastq: URL?
    @State private var outDir: URL?
    @State private var barcodes = "# Name,Sequenz\nbarcode1,ACGTACGT\nbarcode2,TGCATGCA"
    @State private var mismatch = 0
    @State private var groups: [BarcodeGroupDto] = []
    @State private var message: String?
    @State private var busy = false

    var body: some View {
        GroupBox(tr("Split barcodes (FASTQ by MID at the start of the reads)")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button(tr("Choose FASTQ …")) { fastq = FilePick.open(tr("Choose FASTQ file")) }
                    Text(fastq?.lastPathComponent ?? tr("no file")).foregroundStyle(.secondary)
                    Button(tr("Target folder …")) { outDir = FilePick.open(tr("Folder for the results"), directories: true) }
                    Text(outDir?.lastPathComponent ?? tr("no folder")).foregroundStyle(.secondary)
                }
                TextEditor(text: $barcodes)
                    .font(.system(.body, design: .monospaced))
                    .frame(height: 80)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                HStack {
                    Stepper(tr("Allowed differences: %@", "\(mismatch)"), value: $mismatch, in: 0...3).frame(width: 240)
                    Button(tr("Split")) { run() }.disabled(fastq == nil || outDir == nil || busy)
                    if busy { ProgressView().controlSize(.small) }
                }
                if let message { Text(message).foregroundStyle(.red) }
                ForEach(groups, id: \.name) { g in
                    HStack {
                        Text(g.name).frame(width: 140, alignment: .leading)
                        Text(tr("%@ reads", "\(g.count)")).foregroundStyle(.secondary)
                        Spacer()
                        Button(tr("Show in Finder")) { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: g.path)]) }
                            .buttonStyle(.borderless)
                    }
                }
            }
            .padding(6)
        }
    }

    private func run() {
        guard let fastq, let outDir else { return }
        busy = true
        message = nil
        let text = barcodes, mm = UInt64(mismatch)
        Task.detached {
            do {
                let r = try splitFastqByBarcode(path: fastq.path, barcodes: text, maxMismatch: mm, outDir: outDir.path)
                await MainActor.run { groups = r; busy = false }
            } catch {
                await MainActor.run { message = error.localizedDescription; groups = []; busy = false }
            }
        }
    }
}

/// Convert FASTQ to FASTA and QUAL.
struct FastqConvertSection: View {
    @State private var message: String?

    var body: some View {
        GroupBox(tr("FASTQ → FASTA + QUAL")) {
            VStack(alignment: .leading, spacing: 8) {
                Button(tr("Choose FASTQ and convert …")) { convert() }
                if let message { Text(message).font(.callout) }
            }
            .padding(6)
        }
    }

    private func convert() {
        guard let src = FilePick.open(tr("Choose FASTQ file")),
              let dest = FilePick.save(tr("Name for FASTA (the QUAL file gets the same base)"), name: src.deletingPathExtension().lastPathComponent + ".fasta")
        else { return }
        let qual = dest.deletingPathExtension().appendingPathExtension("qual")
        do {
            let n = try fastqToFastaQual(path: src.path, fastaPath: dest.path, qualPath: qual.path)
            message = tr("%@ reads: %@ and %@ written.", "\(n)", "\(dest.lastPathComponent)", "\(qual.lastPathComponent)")
        } catch {
            message = error.localizedDescription
        }
    }
}

/// Expression (reads, RPKM, TPM) per reference from a SAM/BAM file.
struct ExpressionSection: View {
    @State private var rows: [ExpressionRow] = []
    @State private var message: String?
    @State private var busy = false

    var body: some View {
        GroupBox(tr("Expression from SAM/BAM (reads, RPKM, TPM per reference)")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button(tr("Choose alignment file …")) { choose() }.disabled(busy)
                    if busy { ProgressView().controlSize(.small) }
                    if !rows.isEmpty { Button(tr("Save as CSV …")) { saveCsv() } }
                }
                if let message { Text(message).foregroundStyle(.red) }
                if !rows.isEmpty {
                    Table(rows) {
                        TableColumn(tr("Reference")) { r in Text(r.dto.reference) }
                        TableColumn(tr("Length")) { r in Text("\(r.dto.length)") }.width(80)
                        TableColumn(tr("Reads")) { r in Text("\(r.dto.count)") }.width(80)
                        TableColumn("RPKM") { r in Text(String(format: "%.2f", r.dto.rpkm)) }.width(100)
                        TableColumn("TPM") { r in Text(String(format: "%.2f", r.dto.tpm)) }.width(100)
                    }
                    .frame(minHeight: 120, maxHeight: 240)
                }
            }
            .padding(6)
        }
    }

    private func choose() {
        guard let url = FilePick.open(tr("Choose SAM or BAM file")) else { return }
        busy = true
        message = nil
        Task.detached {
            do {
                let r = try expressionFromAlignment(path: url.path)
                await MainActor.run { rows = r.enumerated().map { ExpressionRow(index: $0.offset, dto: $0.element) }; busy = false }
            } catch {
                await MainActor.run { message = error.localizedDescription; rows = []; busy = false }
            }
        }
    }

    private func saveCsv() {
        guard let url = FilePick.save(tr("Save expression as CSV"), name: "expression.csv") else { return }
        var text = "Reference,Length,Reads,RPKM,TPM\n"
        for r in rows {
            text += "\"\(r.dto.reference.replacingOccurrences(of: "\"", with: "\"\""))\",\(r.dto.length),\(r.dto.count),\(r.dto.rpkm),\(r.dto.tpm)\n"
        }
        do { try text.write(to: url, atomically: true, encoding: .utf8) } catch { message = error.localizedDescription }
    }
}

struct ExpressionRow: Identifiable {
    let index: Int
    let dto: ExpressionRowDto
    var id: Int { index }
}

/// Match paired reads by name (several selected reads).
struct PairedReadsSection: View {
    @ObservedObject var model: AppModel
    @State private var report: PairReportDto?

    var body: some View {
        GroupBox(tr("Check paired reads (names with /1, /2 or 1:, 2:)")) {
            VStack(alignment: .leading, spacing: 8) {
                Button(tr("Find pairs")) { find() }
                if let r = report {
                    Text(tr("%@ pairs, %@ without partner", "\(r.pairs.count)", "\(r.unpaired.count)")).font(.headline)
                    ForEach(Array(r.pairs.prefix(20).enumerated()), id: \.offset) { _, p in
                        Text("\(p[0])  ↔  \(p[1])").font(.callout).foregroundStyle(.secondary)
                    }
                    if r.pairs.count > 20 { Text(tr("… and %@ more pairs", "\(r.pairs.count - 20)")).font(.caption).foregroundStyle(.secondary) }
                    if !r.unpaired.isEmpty {
                        Text(tr("Without partner: ") + r.unpaired.prefix(10).joined(separator: ", ") + (r.unpaired.count > 10 ? " …" : ""))
                            .font(.callout).foregroundStyle(.orange)
                    }
                }
            }
            .padding(6)
        }
        .onChange(of: model.multiSelection) { _, _ in report = nil }
    }

    private func find() {
        guard let project = model.project else { return }
        let ids = model.documents.filter { model.multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        do { report = try project.pairReadsReport(ids: ids) } catch { model.errorMessage = error.localizedDescription }
    }
}

/// Length distribution of the selected sequences as a bar chart.
struct LengthHistogramSection: View {
    @ObservedObject var model: AppModel
    @State private var width = 100
    @State private var bins: [HistogramBinDto] = []

    var body: some View {
        GroupBox(tr("Length distribution of the selected sequences")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Stepper(tr("Bin width %@ bp", "\(width)"), value: $width, in: 1...100_000, step: width >= 100 ? 100 : 10).frame(width: 220)
                    Button(tr("Show")) { load() }
                }
                if !bins.isEmpty {
                    let maxCount = max(bins.map { Int($0.count) }.max() ?? 1, 1)
                    Canvas { ctx, size in
                        let w = size.width / CGFloat(bins.count)
                        for (i, b) in bins.enumerated() {
                            let h = size.height * CGFloat(b.count) / CGFloat(maxCount)
                            let rect = CGRect(x: CGFloat(i) * w + 1, y: size.height - h, width: max(w - 2, 1), height: h)
                            ctx.fill(Path(rect), with: .color(.accentColor.opacity(0.8)))
                        }
                    }
                    .frame(height: 90)
                    HStack {
                        Text("\(bins.first?.start ?? 0) bp").font(.caption)
                        Spacer()
                        Text(tr("%@ bp  ·  highest bin: %@ sequences", "\(Int(bins.last?.start ?? 0) + width)", "\(maxCount)")).font(.caption)
                    }
                    .foregroundStyle(.secondary)
                }
            }
            .padding(6)
        }
        .onChange(of: model.multiSelection) { _, _ in bins = [] }
    }

    private func load() {
        guard let project = model.project else { return }
        let ids = model.documents.filter { model.multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        do { bins = try project.lengthHistogram(ids: ids, binWidth: UInt64(width)) } catch { model.errorMessage = error.localizedDescription }
    }
}
