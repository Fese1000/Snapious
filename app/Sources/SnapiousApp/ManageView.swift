import AppKit
import SnapiousFFI
import SwiftUI

/// Projektverwaltung: Ordner, Favoriten und Notizen, Suche, Versionen, Workflow-Aufzeichnung, Export, Einstellungen und Plugins.
struct ManageView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 0) {
            TabView {
                FoldersTab(model: model).tabItem { Text("Ordner") }
                SearchTab(model: model).tabItem { Text("Suche") }
                DocumentTab(model: model).tabItem { Text("Dokument") }
                SettingsTab().tabItem { Text("Einstellungen") }
            }
            .padding(10)
            Divider()
            HStack {
                Spacer()
                Button("Schließen") { dismiss() }.keyboardShortcut(.defaultAction)
            }
            .padding(10)
        }
        .frame(minWidth: 640, minHeight: 460)
    }
}

/// Ordnerbaum (flach mit Einrückung), Favoriten und Verschieben.
struct FoldersTab: View {
    @ObservedObject var model: AppModel
    @State private var folders: [FolderDto] = []
    @State private var selectedFolder = ""
    @State private var docs: [DocumentInfoDto] = []
    @State private var newName = ""
    @State private var message: String?

    private func depth(_ f: FolderDto) -> Int {
        var d = 0
        var cur = f
        while !cur.parentId.isEmpty, let p = folders.first(where: { $0.id == cur.parentId }), d < 20 { d += 1; cur = p }
        return d
    }

    private var ordered: [FolderDto] {
        var out: [FolderDto] = []
        func add(_ parent: String) {
            for f in folders.filter({ $0.parentId == parent }) { out.append(f); add(f.id) }
        }
        add("")
        return out
    }

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Ordner").font(Theme.heading())
                List {
                    row(id: "", name: "Ohne Ordner (Wurzel)", level: 0)
                    ForEach(ordered, id: \.id) { f in row(id: f.id, name: f.name, level: depth(f) + 1) }
                }
                .frame(minHeight: 220)
                HStack {
                    TextField("Neuer Ordner", text: $newName).textFieldStyle(.roundedBorder)
                    Button("Anlegen") { create() }.disabled(newName.trimmingCharacters(in: .whitespaces).isEmpty)
                }
                HStack {
                    Button("Umbenennen") { rename() }.disabled(selectedFolder.isEmpty || newName.isEmpty)
                    Button("Löschen") { delete(contents: false) }.disabled(selectedFolder.isEmpty)
                    Button("Mit Inhalt löschen") { delete(contents: true) }.disabled(selectedFolder.isEmpty)
                }
                if let message { Text(message).foregroundStyle(Theme.danger).font(Theme.label(11)) }
            }
            .frame(width: 280)
            VStack(alignment: .leading, spacing: 6) {
                Text("Dokumente im Ordner").font(Theme.heading())
                List(docs, id: \.id) { d in
                    HStack {
                        Text(d.name)
                        Spacer()
                        Menu("Verschieben") {
                            Button("Wurzel") { move(d, to: "") }
                            ForEach(folders, id: \.id) { f in Button(f.name) { move(d, to: f.id) } }
                        }
                        .fixedSize()
                        Button { toggleFavorite(d) } label: { Image(systemName: "star") }.accessibilityLabel("Favorit umschalten").buttonStyle(.plain)
                    }
                }
                .frame(minHeight: 260)
                Text("Der Stern markiert ein Dokument als Favorit.").font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            }
        }
        .onAppear { reload() }
    }

    private func row(id: String, name: String, level: Int) -> some View {
        Text(String(repeating: "   ", count: level) + name)
            .padding(.vertical, 1)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(selectedFolder == id ? Theme.accent.opacity(0.25) : Color.clear)
            .contentShape(Rectangle())
            .onTapGesture { selectedFolder = id; reload() }
    }

    private func reload() {
        guard let p = model.project else { return }
        folders = (try? p.folders()) ?? []
        docs = (try? p.documentsInFolder(folderId: selectedFolder)) ?? []
    }

    private func perform(_ f: (Project) throws -> Void) {
        guard let p = model.project else { return }
        do { try f(p); message = nil } catch { message = error.localizedDescription }
        reload()
    }

    private func create() { perform { _ = try $0.createFolder(name: newName, parentId: selectedFolder) }; newName = "" }
    private func rename() { perform { try $0.renameFolder(id: selectedFolder, name: newName) }; newName = "" }
    private func delete(contents: Bool) {
        perform { try $0.deleteFolder(id: selectedFolder, deleteContents: contents) }
        selectedFolder = ""
        try? model.refresh()
        reload()
    }
    private func move(_ d: DocumentInfoDto, to folder: String) { perform { try $0.moveDocument(docId: d.id, folderId: folder) } }
    private func toggleFavorite(_ d: DocumentInfoDto) {
        perform { p in
            let isFav = try p.favorites().contains { $0.id == d.id }
            try p.setFavorite(docId: d.id, favorite: !isFav)
        }
    }
}

/// Suche in den Dokumenten, Dubletten und Stapelumbenennung.
struct SearchTab: View {
    @ObservedObject var model: AppModel
    @State private var kind = "name"
    @State private var text = ""
    @State private var text2 = ""
    @State private var results: [DocumentInfoDto] = []
    @State private var pattern = "{name}_{n:02}"
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Picker("Suche nach", selection: $kind) {
                    Text("Name").tag("name")
                    Text("Typ").tag("type")
                    Text("Feature-Text").tag("feature")
                    Text("Metadaten").tag("metadata")
                    Text("Motiv (IUPAC)").tag("motif")
                }.frame(width: 240)
                TextField(kind == "metadata" ? "Schlüssel" : "Suchtext", text: $text).textFieldStyle(.roundedBorder)
                if kind == "metadata" { TextField("Wert (leer = nur Schlüssel)", text: $text2).textFieldStyle(.roundedBorder) }
                Button("Suchen") { search() }
                Button("Dubletten") { duplicates() }
            }
            List(results, id: \.id) { d in
                HStack { Text(d.name); Spacer(); Text(d.docType).foregroundStyle(Theme.inkSoft) }
                    .contentShape(Rectangle())
                    .onTapGesture { model.selection = d.id }
            }
            .frame(minHeight: 200)
            HStack {
                TextField("Muster", text: $pattern).textFieldStyle(.roundedBorder).frame(width: 240)
                Button("Treffer umbenennen") { rename() }.disabled(results.isEmpty)
                Text("{name} {n} {n:03} {type}").font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            }
            if let message { Text(message).font(Theme.label(11)).foregroundStyle(message.hasPrefix("Fehler") ? .red : .secondary) }
        }
    }

    private func search() {
        guard let p = model.project else { return }
        do { results = try p.searchDocuments(kind: kind, text: text, text2: text2); message = "\(results.count) Treffer" }
        catch { message = "Fehler: \(error.localizedDescription)" }
    }

    private func duplicates() {
        guard let p = model.project else { return }
        do {
            let groups = try p.findDuplicates()
            results = groups.flatMap { $0 }
            message = groups.isEmpty ? "Keine Dubletten." : "\(groups.count) Gruppe(n) mit gleicher Sequenz."
        } catch { message = "Fehler: \(error.localizedDescription)" }
    }

    private func rename() {
        guard let p = model.project else { return }
        do {
            let n = try p.batchRename(ids: results.map { $0.id }, pattern: pattern)
            try model.refresh()
            message = "\(n) Dokument(e) umbenannt."
            results = []
        } catch { message = "Fehler: \(error.localizedDescription)" }
    }
}

/// Notiz, Versionen, Workflow-Aufzeichnung und Export für das gewählte Dokument.
struct DocumentTab: View {
    @ObservedObject var model: AppModel
    @State private var note = ""
    @State private var versions: [VersionDto] = []
    @State private var workflowName = ""
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let id = model.selection, let name = model.documents.first(where: { $0.id == id })?.name {
                Text(name).font(Theme.heading())
                GroupBox("Notiz") {
                    VStack(alignment: .leading) {
                        TextEditor(text: $note).frame(height: 70).font(.body)
                        Button("Notiz speichern") { try? model.project?.setNote(docId: id, text: note); message = "Notiz gespeichert." }
                    }.padding(4)
                }
                GroupBox("Versionen") {
                    VStack(alignment: .leading) {
                        if versions.isEmpty { Text("Noch keine Versionen (entstehen beim Bearbeiten).").foregroundStyle(Theme.inkSoft) }
                        ForEach(versions.prefix(8), id: \.version) { v in
                            HStack {
                                Text("Version \(v.version) · \(Date(timeIntervalSince1970: TimeInterval(v.time)).formatted(date: .abbreviated, time: .shortened)) · \(v.summary)")
                                    .font(Theme.label(13))
                                Spacer()
                                Button("Wiederherstellen") { restore(id, v) }
                            }
                        }
                    }.padding(4)
                }
                GroupBox("Workflow aus dem Verlauf") {
                    HStack {
                        TextField("Name des Workflows", text: $workflowName).textFieldStyle(.roundedBorder)
                        Button("Aufzeichnen") { record(id) }.disabled(workflowName.isEmpty)
                    }.padding(4)
                }
                GroupBox("Export") {
                    HStack {
                        Button("Text mit Positionen …") { export(id, kind: "text", ext: "txt") }
                        Button("Bericht (Markdown) …") { export(id, kind: "report", ext: "md") }
                        Button("Features als CSV …") { export(id, kind: "features_csv", ext: "csv") }
                        Button("Features als TSV …") { export(id, kind: "features_tsv", ext: "tsv") }
                    }.padding(4)
                }
                if let message { Text(message).font(Theme.label(11)).foregroundStyle(Theme.inkSoft) }
            } else {
                Text("Wähle links ein Dokument aus.").foregroundStyle(Theme.inkSoft)
            }
            Spacer()
        }
        .onAppear { load() }
    }

    private func load() {
        guard let p = model.project, let id = model.selection else { return }
        note = (try? p.note(docId: id)) ?? ""
        versions = (try? p.versions(docId: id)) ?? []
    }

    private func restore(_ id: String, _ v: VersionDto) {
        do {
            try model.project?.restoreVersion(docId: id, version: v.version)
            try model.refresh()
            model.selection = id
            message = "Version \(v.version) wiederhergestellt."
            load()
        } catch { message = "Fehler: \(error.localizedDescription)" }
    }

    private func record(_ id: String) {
        do {
            let n = try model.project?.recordWorkflow(docId: id, name: workflowName) ?? 0
            message = "Workflow mit \(n) Schritt(en) gespeichert (Dialog „Workflows“)."
        } catch { message = "Fehler: \(error.localizedDescription)" }
    }

    private func export(_ id: String, kind: String, ext: String) {
        guard let p = model.project else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "export.\(ext)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let text = try p.exportText(id: id, kind: kind)
            try text.write(to: url, atomically: true, encoding: .utf8)
            message = "Gespeichert: \(url.lastPathComponent)"
        } catch { message = "Fehler: \(error.localizedDescription)" }
    }
}

/// Einstellungen und Plugins.
struct SettingsTab: View {
    @State private var s = settingsLoad()
    @State private var plugins: [String] = []
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            GroupBox("Programm") {
                Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
                    GridRow { Text("NCBI-E-Mail"); TextField("", text: $s.ncbiEmail).textFieldStyle(.roundedBorder) }
                    GridRow { Text("NCBI-API-Schlüssel"); SecureField("", text: $s.ncbiApiKey).textFieldStyle(.roundedBorder) }
                    GridRow { Text("Standardorganismus"); TextField("", text: $s.defaultOrganism).textFieldStyle(.roundedBorder) }
                    GridRow { Text("Rückgängig-Schritte"); Stepper("\(s.undoLimit)", value: Binding(get: { Int(s.undoLimit) }, set: { s.undoLimit = UInt64($0) }), in: 10...1000, step: 10) }
                }.padding(4)
                HStack {
                    Button("Speichern") { do { try settingsSave(s: s); message = "Gespeichert (nur für dich lesbar)." } catch { message = error.localizedDescription } }
                    if let message { Text(message).font(Theme.label(11)).foregroundStyle(Theme.inkSoft) }
                }.padding(4)
            }
            GroupBox("Plugins (Programme mit plugin.json im Ordner)") {
                VStack(alignment: .leading, spacing: 4) {
                    if plugins.isEmpty { Text("Keine Plugins gefunden.").foregroundStyle(Theme.inkSoft) }
                    ForEach(plugins, id: \.self) { Text($0).font(Theme.label(13)) }
                    HStack {
                        Button("Plugin-Ordner öffnen") { NSWorkspace.shared.open(URL(fileURLWithPath: pluginsDirectory())) }
                        Button("Neu laden") { plugins = pluginsList() }
                    }
                    Text("Achtung: Plugins laufen ohne Schutzraum mit deinen Rechten.").font(Theme.label(11)).foregroundStyle(Theme.amber)
                }.padding(4)
            }
            Spacer()
        }
        .onAppear { plugins = pluginsList() }
    }
}
