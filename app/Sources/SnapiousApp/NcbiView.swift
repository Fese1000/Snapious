import SnapiousFFI
import SwiftUI

/// Suche in NCBI Nucleotide und Import als GenBank. Läuft im Hintergrund, damit die Oberfläche nicht einfriert.
struct NcbiView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var term = ""
    @State private var result: NcbiSearchDto?
    @State private var chosen: String?
    @State private var busy = false
    @State private var message: String?
    @State private var source = "ncbi"

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Picker("Datenbank", selection: $source) {
                Text("NCBI Nucleotide").tag("ncbi")
                Text("UniProt (Proteine)").tag("uniprot")
            }
            .pickerStyle(.segmented)
            .frame(width: 360)
            .onChange(of: source) { _, _ in result = nil; chosen = nil; message = nil }
            HStack {
                TextField(source == "ncbi" ? "Suchbegriff, z. B. pUC19 oder L09137" : "Suchbegriff, z. B. hemoglobin alpha human", text: $term)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { search() }
                Button("Suchen") { search() }.disabled(term.trimmingCharacters(in: .whitespaces).isEmpty || busy)
                if busy { ProgressView().controlSize(.small) }
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            if let r = result {
                Text("\(r.total) Treffer, \(r.records.count) angezeigt").foregroundStyle(Theme.inkSoft)
                Table(r.records, selection: $chosen) {
                    TableColumn("Zugang") { x in Text(x.accession) }.width(110)
                    TableColumn("Titel") { x in Text(x.title) }
                    TableColumn("Organismus") { x in Text(x.organism) }.width(160)
                    TableColumn("Länge") { x in Text(x.length > 0 ? "\(x.length) bp" : "–") }.width(90)
                }
            } else {
                Spacer()
            }
            HStack {
                Spacer()
                Button("Schließen") { dismiss() }
                Button("Importieren") { importChosen() }
                    .disabled(chosen == nil || busy)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(minWidth: 760, minHeight: 440)
    }

    private func search() {
        guard let project = model.project else { return }
        let query = term.trimmingCharacters(in: .whitespaces)
        busy = true
        message = nil
        let src = source
        Task.detached {
            do {
                let r = src == "ncbi" ? try project.ncbiSearch(term: query, maxResults: 25) : try project.uniprotSearch(query: query, maxResults: 25)
                await MainActor.run { result = r; chosen = nil; busy = false }
            } catch {
                await MainActor.run { message = error.localizedDescription; busy = false }
            }
        }
    }

    private func importChosen() {
        guard let project = model.project, let id = chosen else { return }
        busy = true
        message = nil
        let src = source
        Task.detached {
            do {
                let infos = src == "ncbi" ? try project.ncbiImport(id: id) : try project.uniprotImport(accession: id)
                await MainActor.run {
                    busy = false
                    model.reload(selecting: infos.last?.id)
                    dismiss()
                }
            } catch {
                await MainActor.run { message = error.localizedDescription; busy = false }
            }
        }
    }
}

extension NcbiRecordDto: Identifiable {}
