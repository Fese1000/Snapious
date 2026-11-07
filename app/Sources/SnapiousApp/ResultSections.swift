import SnapiousFFI
import SwiftUI

/// Read results of external programs: view a BLAST table, add domains (hmmscan, InterProScan) as features.
struct ExternalResultsSection: View {
    @ObservedObject var model: AppModel
    @State private var hits: [BlastTableRow] = []
    @State private var message: String?

    var body: some View {
        GroupBox(tr("Read results of external programs")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button(tr("View BLAST table …")) { openBlast() }
                    Button(tr("Add domains from hmmscan …")) { importDomains(format: "hmmer") }
                        .disabled(model.selection == nil)
                    Button(tr("Add domains from InterProScan …")) { importDomains(format: "interpro") }
                        .disabled(model.selection == nil)
                }
                Text(tr("BLAST: `-outfmt 6`. Domains: `hmmscan --domtblout` or InterProScan TSV; they are added as “domain” features to the selected sequence."))
                    .font(.caption).foregroundStyle(.secondary)
                if let message { Text(message).font(.callout) }
                if !hits.isEmpty {
                    Table(hits) {
                        TableColumn(tr("Hits")) { h in Text(h.dto.subject) }
                        TableColumn(tr("Identity")) { h in Text(String(format: "%.1f %%", h.dto.identityPercent)) }.width(80)
                        TableColumn(tr("Length")) { h in Text("\(h.dto.alignmentLength)") }.width(60)
                        TableColumn("Query") { h in Text("\(h.dto.queryStart)–\(h.dto.queryEnd)") }.width(90)
                        TableColumn(tr("E-value")) { h in Text(String(format: "%.1e", h.dto.evalue)) }.width(80)
                        TableColumn("Bit") { h in Text(String(format: "%.1f", h.dto.bitScore)) }.width(60)
                    }
                    .frame(minHeight: 120, maxHeight: 220)
                }
            }
            .padding(6)
        }
    }

    private func openBlast() {
        guard let url = FilePick.open(tr("Choose BLAST table (-outfmt 6)")) else { return }
        do {
            hits = try blastTableOpen(path: url.path).enumerated().map { BlastTableRow(index: $0.offset, dto: $0.element) }
            message = tr("%@ hits.", "\(hits.count)")
        } catch {
            hits = []
            message = error.localizedDescription
        }
    }

    private func importDomains(format: String) {
        guard let project = model.project, let id = model.selection, let url = FilePick.open(tr("Choose domain table")) else { return }
        do {
            let n = try project.importDomainTable(id: id, path: url.path, format: format)
            model.reloadCurrent()
            message = tr("%@ domains added as features.", "\(n)")
        } catch {
            message = error.localizedDescription
        }
    }
}

struct BlastTableRow: Identifiable {
    let index: Int
    let dto: BlastHitDto
    var id: Int { index }
}

/// Statistics for expression data: correct p-values and normalize counts.
struct StatsSection: View {
    @State private var pText = "0.01\n0.04\n0.03\n0.005"
    @State private var adjusted: [Double] = []
    @State private var countsText = "10 20\n100 200\n0 5"
    @State private var factors: [Double] = []

    private func numbers(_ line: Substring) -> [Double] {
        line.split(whereSeparator: { " \t;,".contains($0) }).compactMap { Double($0.replacingOccurrences(of: ",", with: ".")) }
    }

    var body: some View {
        GroupBox(tr("Statistics (expression)")) {
            VStack(alignment: .leading, spacing: 8) {
                Text(tr("Correct p-values (Benjamini-Hochberg), one value per line")).font(.caption).foregroundStyle(.secondary)
                TextEditor(text: $pText).font(.system(.body, design: .monospaced)).frame(height: 60)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                HStack {
                    Button(tr("Correct")) {
                        adjusted = pAdjustBh(p: pText.split(whereSeparator: \.isNewline).compactMap { numbers($0).first })
                    }
                    if !adjusted.isEmpty { Text(adjusted.map { String(format: "%.4g", $0) }.joined(separator: " · ")).font(.callout) }
                }
                Divider()
                Text(tr("Size factors (median of ratios): one gene per row, one sample per column")).font(.caption).foregroundStyle(.secondary)
                TextEditor(text: $countsText).font(.system(.body, design: .monospaced)).frame(height: 70)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                HStack {
                    Button(tr("Calculate factors")) {
                        factors = sizeFactors(counts: countsText.split(whereSeparator: \.isNewline).map { numbers($0) }.filter { !$0.isEmpty })
                    }
                    if !factors.isEmpty { Text(factors.map { String(format: "%.3f", $0) }.joined(separator: " · ")).font(.callout) }
                }
            }
            .padding(6)
        }
    }
}

/// Coverage metrics and duplicates from SAM/BAM against the selected sequence.
struct CoverageReportSection: View {
    @ObservedObject var model: AppModel
    @State private var report: CoverageReportDto?
    @State private var message: String?
    @State private var busy = false

    var body: some View {
        GroupBox(tr("Coverage and duplicates (SAM/BAM against the selected sequence)")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button(tr("Choose alignment file …")) { run() }.disabled(busy || model.selection == nil)
                    if busy { ProgressView().controlSize(.small) }
                }
                if let message { Text(message).foregroundStyle(.red) }
                if let r = report {
                    Grid(alignment: .leading, horizontalSpacing: 20, verticalSpacing: 3) {
                        GridRow { Text(tr("Mean depth")).foregroundStyle(.secondary); Text(String(format: "%.1f×", r.mean)) }
                        GridRow { Text(tr("Median / min / max")).foregroundStyle(.secondary); Text(String(format: "%.1f× / %d× / %d×", r.median, r.min, r.max)) }
                        GridRow { Text(tr("Positions with ≥ 1×")).foregroundStyle(.secondary); Text(String(format: "%.1f %%", (r.fractions.first ?? 0) * 100)) }
                        GridRow { Text(tr("Positions with ≥ 10×")).foregroundStyle(.secondary); Text(String(format: "%.1f %%", (r.fractions.dropFirst().first ?? 0) * 100)) }
                        GridRow { Text(tr("Positions with ≥ 30×")).foregroundStyle(.secondary); Text(String(format: "%.1f %%", (r.fractions.dropFirst(2).first ?? 0) * 100)) }
                        GridRow {
                            Text(tr("Duplicate reads")).foregroundStyle(.secondary)
                            Text(tr("%@ of %@ reads", "\(r.duplicateReads)", "\(r.mappedReads)") + (r.mappedReads > 0 ? String(format: " (%.1f %%)", 100 * Double(r.duplicateReads) / Double(r.mappedReads)) : ""))
                        }
                    }
                }
            }
            .padding(6)
        }
    }

    private func run() {
        guard let project = model.project, let id = model.selection, let url = FilePick.open(tr("Choose SAM or BAM file")) else { return }
        busy = true
        message = nil
        Task.detached {
            do {
                let r = try project.coverageReport(referenceId: id, alignmentPath: url.path)
                await MainActor.run { report = r; busy = false }
            } catch {
                await MainActor.run { message = error.localizedDescription; report = nil; busy = false }
            }
        }
    }
}

/// Fragment lengths of paired reads as a bar chart.
struct InsertSizeSection: View {
    @State private var report: InsertReportDto?
    @State private var message: String?

    var body: some View {
        GroupBox(tr("Fragment lengths of paired reads (SAM/BAM)")) {
            VStack(alignment: .leading, spacing: 8) {
                Button(tr("Choose alignment file …")) { run() }
                if let message { Text(message).foregroundStyle(.red) }
                if let r = report {
                    Text(String(format: tr("%d pairs · mean %.1f bp · median %.1f bp · standard deviation %.1f bp"), r.count, r.mean, r.median, r.stdDev)).font(.callout)
                    let maxCount = max(r.bins.map { Int($0.count) }.max() ?? 1, 1)
                    Canvas { ctx, size in
                        guard !r.bins.isEmpty else { return }
                        let w = size.width / CGFloat(r.bins.count)
                        for (i, b) in r.bins.enumerated() {
                            let h = size.height * CGFloat(b.count) / CGFloat(maxCount)
                            ctx.fill(Path(CGRect(x: CGFloat(i) * w + 0.5, y: size.height - h, width: max(w - 1, 1), height: h)), with: .color(.accentColor.opacity(0.8)))
                        }
                    }
                    .frame(height: 80)
                    HStack {
                        Text("\(r.bins.first?.start ?? 0) bp").font(.caption)
                        Spacer()
                        Text("\((r.bins.last?.start ?? 0) + 25) bp").font(.caption)
                    }
                    .foregroundStyle(.secondary)
                }
            }
            .padding(6)
        }
    }

    private func run() {
        guard let url = FilePick.open(tr("Choose SAM or BAM file")) else { return }
        do {
            report = try insertSizeReport(path: url.path)
            message = report?.count == 0 ? tr("No paired reads found.") : nil
        } catch {
            report = nil
            message = error.localizedDescription
        }
    }
}

/// Most frequent k-mers of a FASTQ file (for detecting adapters).
struct KmerSection: View {
    @State private var k = 12
    @State private var rows: [KmerDto] = []
    @State private var message: String?
    @State private var busy = false

    var body: some View {
        GroupBox(tr("Frequent k-mers (detect adapters and contaminants)")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Stepper("k = \(k)", value: $k, in: 4...32).frame(width: 110)
                    Button(tr("Choose FASTQ …")) { run() }.disabled(busy)
                    if busy { ProgressView().controlSize(.small) }
                }
                Text(tr("The first 100,000 reads are read. A k-mer that occurs in very many reads often indicates an adapter."))
                    .font(.caption).foregroundStyle(.secondary)
                if let message { Text(message).foregroundStyle(.red) }
                ForEach(rows, id: \.kmer) { r in
                    HStack {
                        Text(r.kmer).font(.system(.body, design: .monospaced))
                        Text("\(r.count)×").foregroundStyle(.secondary)
                    }
                }
            }
            .padding(6)
        }
    }

    private func run() {
        guard let url = FilePick.open(tr("Choose FASTQ file")) else { return }
        busy = true
        message = nil
        let kk = UInt64(k)
        Task.detached {
            do {
                let r = try fastqTopKmers(path: url.path, k: kk, top: 10, maxReads: 100_000)
                await MainActor.run { rows = r; busy = false }
            } catch {
                await MainActor.run { message = error.localizedDescription; rows = []; busy = false }
            }
        }
    }
}
