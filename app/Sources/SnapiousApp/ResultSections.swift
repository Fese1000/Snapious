import SnapiousFFI
import SwiftUI

/// Ergebnisse externer Programme lesen: BLAST-Tabelle ansehen, Domänen (hmmscan, InterProScan) als Features eintragen.
struct ExternalResultsSection: View {
    @ObservedObject var model: AppModel
    @State private var hits: [BlastTableRow] = []
    @State private var message: String?

    var body: some View {
        GroupBox("Ergebnisse externer Programme lesen") {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button("BLAST-Tabelle ansehen …") { openBlast() }
                    Button("Domänen aus hmmscan eintragen …") { importDomains(format: "hmmer") }
                        .disabled(model.selection == nil)
                    Button("Domänen aus InterProScan eintragen …") { importDomains(format: "interpro") }
                        .disabled(model.selection == nil)
                }
                Text("BLAST: `-outfmt 6`. Domänen: `hmmscan --domtblout` oder InterProScan-TSV; sie werden als Features „domain“ in die gewählte Sequenz eingetragen.")
                    .font(.caption).foregroundStyle(.secondary)
                if let message { Text(message).font(.callout) }
                if !hits.isEmpty {
                    Table(hits) {
                        TableColumn("Treffer") { h in Text(h.dto.subject) }
                        TableColumn("Identität") { h in Text(String(format: "%.1f %%", h.dto.identityPercent)) }.width(80)
                        TableColumn("Länge") { h in Text("\(h.dto.alignmentLength)") }.width(60)
                        TableColumn("Query") { h in Text("\(h.dto.queryStart)–\(h.dto.queryEnd)") }.width(90)
                        TableColumn("E-Wert") { h in Text(String(format: "%.1e", h.dto.evalue)) }.width(80)
                        TableColumn("Bit") { h in Text(String(format: "%.1f", h.dto.bitScore)) }.width(60)
                    }
                    .frame(minHeight: 120, maxHeight: 220)
                }
            }
            .padding(6)
        }
    }

    private func openBlast() {
        guard let url = FilePick.open("BLAST-Tabelle wählen (-outfmt 6)") else { return }
        do {
            hits = try blastTableOpen(path: url.path).enumerated().map { BlastTableRow(index: $0.offset, dto: $0.element) }
            message = "\(hits.count) Treffer."
        } catch {
            hits = []
            message = error.localizedDescription
        }
    }

    private func importDomains(format: String) {
        guard let project = model.project, let id = model.selection, let url = FilePick.open("Domänentabelle wählen") else { return }
        do {
            let n = try project.importDomainTable(id: id, path: url.path, format: format)
            model.reloadCurrent()
            message = "\(n) Domänen als Features eingetragen."
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

/// Statistik für Expressionsdaten: p-Werte korrigieren und Zählwerte normalisieren.
struct StatsSection: View {
    @State private var pText = "0.01\n0.04\n0.03\n0.005"
    @State private var adjusted: [Double] = []
    @State private var countsText = "10 20\n100 200\n0 5"
    @State private var factors: [Double] = []

    private func numbers(_ line: Substring) -> [Double] {
        line.split(whereSeparator: { " \t;,".contains($0) }).compactMap { Double($0.replacingOccurrences(of: ",", with: ".")) }
    }

    var body: some View {
        GroupBox("Statistik (Expression)") {
            VStack(alignment: .leading, spacing: 8) {
                Text("p-Werte korrigieren (Benjamini-Hochberg), ein Wert je Zeile").font(.caption).foregroundStyle(.secondary)
                TextEditor(text: $pText).font(.system(.body, design: .monospaced)).frame(height: 60)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                HStack {
                    Button("Korrigieren") {
                        adjusted = pAdjustBh(p: pText.split(whereSeparator: \.isNewline).compactMap { numbers($0).first })
                    }
                    if !adjusted.isEmpty { Text(adjusted.map { String(format: "%.4g", $0) }.joined(separator: " · ")).font(.callout) }
                }
                Divider()
                Text("Größenfaktoren (Median der Verhältnisse): je Zeile ein Gen, je Spalte eine Probe").font(.caption).foregroundStyle(.secondary)
                TextEditor(text: $countsText).font(.system(.body, design: .monospaced)).frame(height: 70)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                HStack {
                    Button("Faktoren berechnen") {
                        factors = sizeFactors(counts: countsText.split(whereSeparator: \.isNewline).map { numbers($0) }.filter { !$0.isEmpty })
                    }
                    if !factors.isEmpty { Text(factors.map { String(format: "%.3f", $0) }.joined(separator: " · ")).font(.callout) }
                }
            }
            .padding(6)
        }
    }
}

/// Abdeckungs-Kennzahlen und Duplikate aus SAM/BAM gegen die gewählte Sequenz.
struct CoverageReportSection: View {
    @ObservedObject var model: AppModel
    @State private var report: CoverageReportDto?
    @State private var message: String?
    @State private var busy = false

    var body: some View {
        GroupBox("Abdeckung und Duplikate (SAM/BAM gegen die gewählte Sequenz)") {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button("Alignment-Datei wählen …") { run() }.disabled(busy || model.selection == nil)
                    if busy { ProgressView().controlSize(.small) }
                }
                if let message { Text(message).foregroundStyle(.red) }
                if let r = report {
                    Grid(alignment: .leading, horizontalSpacing: 20, verticalSpacing: 3) {
                        GridRow { Text("Mittlere Tiefe").foregroundStyle(.secondary); Text(String(format: "%.1f×", r.mean)) }
                        GridRow { Text("Median / Min / Max").foregroundStyle(.secondary); Text(String(format: "%.1f× / %d× / %d×", r.median, r.min, r.max)) }
                        GridRow { Text("Positionen mit ≥ 1×").foregroundStyle(.secondary); Text(String(format: "%.1f %%", (r.fractions.first ?? 0) * 100)) }
                        GridRow { Text("Positionen mit ≥ 10×").foregroundStyle(.secondary); Text(String(format: "%.1f %%", (r.fractions.dropFirst().first ?? 0) * 100)) }
                        GridRow { Text("Positionen mit ≥ 30×").foregroundStyle(.secondary); Text(String(format: "%.1f %%", (r.fractions.dropFirst(2).first ?? 0) * 100)) }
                        GridRow {
                            Text("Duplikate").foregroundStyle(.secondary)
                            Text("\(r.duplicateReads) von \(r.mappedReads) Reads" + (r.mappedReads > 0 ? String(format: " (%.1f %%)", 100 * Double(r.duplicateReads) / Double(r.mappedReads)) : ""))
                        }
                    }
                }
            }
            .padding(6)
        }
    }

    private func run() {
        guard let project = model.project, let id = model.selection, let url = FilePick.open("SAM- oder BAM-Datei wählen") else { return }
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

/// Fragmentlängen gepaarter Reads als Balkendiagramm.
struct InsertSizeSection: View {
    @State private var report: InsertReportDto?
    @State private var message: String?

    var body: some View {
        GroupBox("Fragmentlängen gepaarter Reads (SAM/BAM)") {
            VStack(alignment: .leading, spacing: 8) {
                Button("Alignment-Datei wählen …") { run() }
                if let message { Text(message).foregroundStyle(.red) }
                if let r = report {
                    Text(String(format: "%d Paare · Mittel %.1f bp · Median %.1f bp · Standardabweichung %.1f bp", r.count, r.mean, r.median, r.stdDev)).font(.callout)
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
        guard let url = FilePick.open("SAM- oder BAM-Datei wählen") else { return }
        do {
            report = try insertSizeReport(path: url.path)
            message = report?.count == 0 ? "Keine gepaarten Reads gefunden." : nil
        } catch {
            report = nil
            message = error.localizedDescription
        }
    }
}

/// Häufigste k-mer einer FASTQ-Datei (zum Erkennen von Adaptern).
struct KmerSection: View {
    @State private var k = 12
    @State private var rows: [KmerDto] = []
    @State private var message: String?
    @State private var busy = false

    var body: some View {
        GroupBox("Häufige k-mer (Adapter und Verunreinigungen erkennen)") {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Stepper("k = \(k)", value: $k, in: 4...32).frame(width: 110)
                    Button("FASTQ wählen …") { run() }.disabled(busy)
                    if busy { ProgressView().controlSize(.small) }
                }
                Text("Gelesen werden die ersten 100 000 Reads. Ein k-mer, das in sehr vielen Reads vorkommt, deutet oft auf einen Adapter hin.")
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
        guard let url = FilePick.open("FASTQ-Datei wählen") else { return }
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
