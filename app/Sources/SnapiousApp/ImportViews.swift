import AppKit
import SnapiousFFI
import SwiftUI

/// "Import table" dialog: choose CSV/TSV, map columns for name and sequence.
struct TableImportView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var url: URL?
    @State private var sep = ","
    @State private var preview: [[String]] = []
    @State private var hasHeader = true
    @State private var nameCol = 0
    @State private var seqCol = 1
    @State private var kind = "AUTO"
    @State private var circular = false
    @State private var message: String?

    private var columnCount: Int { preview.map { $0.count }.max() ?? 0 }

    private func columnTitle(_ i: Int) -> String {
        if hasHeader, let h = preview.first, i < h.count, !h[i].isEmpty { return "\(i + 1): \(h[i])" }
        return tr("Column %@", "\(i + 1)")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("Import table")).font(.title2.bold())
            HStack {
                Button(tr("Choose file …")) { choose() }
                Text(url?.lastPathComponent ?? tr("no file")).foregroundStyle(.secondary)
                Spacer()
                Picker(tr("Delimiter"), selection: $sep) {
                    Text(tr("Comma")).tag(",")
                    Text(tr("Semicolon")).tag(";")
                    Text(tr("Tab")).tag("\t")
                }
                .frame(width: 220)
                .onChange(of: sep) { _, _ in loadPreview() }
            }
            if !preview.isEmpty {
                ScrollView(.horizontal) {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(preview.enumerated()), id: \.offset) { r, row in
                            HStack(spacing: 12) {
                                ForEach(0..<columnCount, id: \.self) { c in
                                    Text(c < row.count ? row[c] : "").lineLimit(1).frame(width: 130, alignment: .leading)
                                        .font(r == 0 && hasHeader ? .callout.bold() : .callout)
                                }
                            }
                        }
                    }
                    .padding(6)
                }
                .frame(height: 120)
                .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                Form {
                    Toggle(tr("First row is the header"), isOn: $hasHeader)
                    Picker(tr("Name"), selection: $nameCol) { ForEach(0..<columnCount, id: \.self) { Text(columnTitle($0)).tag($0) } }
                    Picker(tr("Sequence"), selection: $seqCol) { ForEach(0..<columnCount, id: \.self) { Text(columnTitle($0)).tag($0) } }
                    Picker(tr("Kind"), selection: $kind) {
                        Text(tr("Detect automatically")).tag("AUTO")
                        Text("DNA").tag("DNA")
                        Text("RNA").tag("RNA")
                        Text(tr("Protein")).tag("PROTEIN")
                    }
                    Toggle(tr("Circular"), isOn: $circular).disabled(kind == "PROTEIN")
                }
            }
            if let message { Text(message).foregroundStyle(.red) }
            HStack {
                Spacer()
                Button(tr("Cancel"), role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
                Button(tr("Import")) { run() }.keyboardShortcut(.defaultAction).disabled(url == nil || preview.isEmpty)
            }
        }
        .padding(20)
        .frame(width: 640)
    }

    private func choose() {
        let p = NSOpenPanel()
        p.title = tr("Choose table (CSV, TSV)")
        guard p.runModal() == .OK, let u = p.url else { return }
        url = u
        sep = Self.guessSeparator(u)
        loadPreview()
    }

    /// Guess the delimiter from the first line: the most frequent of tab, semicolon and comma (otherwise comma).
    private static func guessSeparator(_ url: URL) -> String {
        guard let h = try? FileHandle(forReadingFrom: url), let data = try? h.read(upToCount: 4096),
              let text = String(data: data, encoding: .utf8) else { return "," }
        let first = text.split(whereSeparator: \.isNewline).first.map(String.init) ?? ""
        let counts = ["\t": first.filter { $0 == "\t" }.count, ";": first.filter { $0 == ";" }.count, ",": first.filter { $0 == "," }.count]
        let best = counts.max { $0.value < $1.value }
        return (best?.value ?? 0) > 0 ? best!.key : ","
    }

    private func loadPreview() {
        guard let url else { return }
        do {
            preview = try tablePreview(path: url.path, sep: sep, maxRows: 6)
            message = nil
            nameCol = 0
            seqCol = min(1, max(columnCount - 1, 0))
        } catch {
            preview = []
            message = error.localizedDescription
        }
    }

    private func run() {
        guard let url, let project = model.project else { return }
        do {
            let infos = try project.importTable(path: url.path, sep: sep, nameCol: UInt64(nameCol), seqCol: UInt64(seqCol), hasHeader: hasHeader, kind: kind, circular: circular)
            model.reload(selecting: infos.first?.id)
            dismiss()
        } catch {
            message = error.localizedDescription
        }
    }
}
