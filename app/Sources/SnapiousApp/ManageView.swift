import AppKit
import SnapiousFFI
import SwiftUI

/// Project management: folders, favorites and notes, search, versions, workflow recording, export, settings and plugins.
struct ManageView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 0) {
            TabView {
                FoldersTab(model: model).tabItem { Text(tr("Folder")) }
                SearchTab(model: model).tabItem { Text(tr("Find")) }
                DocumentTab(model: model).tabItem { Text(tr("Document")) }
                SettingsTab().tabItem { Text(tr("Settings")) }
            }
            .padding(10)
            Divider()
            HStack {
                Spacer()
                Button(tr("Close")) { dismiss() }.keyboardShortcut(.defaultAction)
            }
            .padding(10)
        }
        .frame(minWidth: 640, minHeight: 460)
    }
}

/// Folder tree (flat with indentation), favorites and moving.
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
                Text(tr("Folder")).font(Theme.heading())
                List {
                    row(id: "", name: tr("No folder (root)"), level: 0)
                    ForEach(ordered, id: \.id) { f in row(id: f.id, name: f.name, level: depth(f) + 1) }
                }
                .frame(minHeight: 220)
                HStack {
                    TextField(tr("New folder"), text: $newName).textFieldStyle(.roundedBorder)
                    Button(tr("Create")) { create() }.disabled(newName.trimmingCharacters(in: .whitespaces).isEmpty)
                }
                HStack {
                    Button(tr("Rename")) { rename() }.disabled(selectedFolder.isEmpty || newName.isEmpty)
                    Button(tr("Delete")) { delete(contents: false) }.disabled(selectedFolder.isEmpty)
                    Button(tr("Delete with contents")) { delete(contents: true) }.disabled(selectedFolder.isEmpty)
                }
                if let message { Text(message).foregroundStyle(Theme.danger).font(Theme.label(11)) }
            }
            .frame(width: 280)
            VStack(alignment: .leading, spacing: 6) {
                Text(tr("Documents in folder")).font(Theme.heading())
                List(docs, id: \.id) { d in
                    HStack {
                        Text(d.name)
                        Spacer()
                        Menu(tr("Move")) {
                            Button(tr("Root")) { move(d, to: "") }
                            ForEach(folders, id: \.id) { f in Button(f.name) { move(d, to: f.id) } }
                        }
                        .fixedSize()
                        Button { toggleFavorite(d) } label: { Image(systemName: "star") }.accessibilityLabel(tr("Toggle favorite")).buttonStyle(.plain)
                    }
                }
                .frame(minHeight: 260)
                Text(tr("The star marks a document as a favorite.")).font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
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

/// Search in documents, duplicates and batch renaming.
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
                Picker(tr("Search for"), selection: $kind) {
                    Text(tr("Name")).tag("name")
                    Text(tr("Type")).tag("type")
                    Text(tr("Feature text")).tag("feature")
                    Text(tr("Metadata")).tag("metadata")
                    Text(tr("Motif (IUPAC)")).tag("motif")
                }.frame(width: 240)
                TextField(kind == "metadata" ? tr("Key") : tr("Search text"), text: $text).textFieldStyle(.roundedBorder)
                if kind == "metadata" { TextField(tr("Value (empty = key only)"), text: $text2).textFieldStyle(.roundedBorder) }
                Button(tr("Search")) { search() }
                Button(tr("Duplicates")) { duplicates() }
            }
            List(results, id: \.id) { d in
                HStack { Text(d.name); Spacer(); Text(d.docType).foregroundStyle(Theme.inkSoft) }
                    .contentShape(Rectangle())
                    .onTapGesture { model.selection = d.id }
            }
            .frame(minHeight: 200)
            HStack {
                TextField(tr("Pattern"), text: $pattern).textFieldStyle(.roundedBorder).frame(width: 240)
                Button(tr("Rename hits")) { rename() }.disabled(results.isEmpty)
                Text("{name} {n} {n:03} {type}").font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            }
            if let message { Text(message).font(Theme.label(11)).foregroundStyle(message.hasPrefix(tr("Error")) ? .red : .secondary) }
        }
    }

    private func search() {
        guard let p = model.project else { return }
        do { results = try p.searchDocuments(kind: kind, text: text, text2: text2); message = tr("%@ hits", "\(results.count)") }
        catch { message = tr("Error: %@", "\(error.localizedDescription)") }
    }

    private func duplicates() {
        guard let p = model.project else { return }
        do {
            let groups = try p.findDuplicates()
            results = groups.flatMap { $0 }
            message = groups.isEmpty ? tr("No duplicates.") : tr("%@ group(s) with identical sequence.", "\(groups.count)")
        } catch { message = tr("Error: %@", "\(error.localizedDescription)") }
    }

    private func rename() {
        guard let p = model.project else { return }
        do {
            let n = try p.batchRename(ids: results.map { $0.id }, pattern: pattern)
            try model.refresh()
            message = tr("%@ document(s) renamed.", "\(n)")
            results = []
        } catch { message = tr("Error: %@", "\(error.localizedDescription)") }
    }
}

/// Note, versions, workflow recording and export for the selected document.
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
                GroupBox(tr("Note")) {
                    VStack(alignment: .leading) {
                        TextEditor(text: $note).frame(height: 70).font(.body)
                        Button(tr("Save note")) { try? model.project?.setNote(docId: id, text: note); message = tr("Note saved.") }
                    }.padding(4)
                }
                GroupBox(tr("Versions")) {
                    VStack(alignment: .leading) {
                        if versions.isEmpty { Text(tr("No versions yet (created when editing).")).foregroundStyle(Theme.inkSoft) }
                        ForEach(versions.prefix(8), id: \.version) { v in
                            HStack {
                                Text(tr("Version %@ · %@ · %@", "\(v.version)", "\(Date(timeIntervalSince1970: TimeInterval(v.time)).formatted(date: .abbreviated, time: .shortened))", "\(v.summary)"))
                                    .font(Theme.label(13))
                                Spacer()
                                Button(tr("Restore")) { restore(id, v) }
                            }
                        }
                    }.padding(4)
                }
                GroupBox(tr("Workflow from history")) {
                    HStack {
                        TextField(tr("Workflow name"), text: $workflowName).textFieldStyle(.roundedBorder)
                        Button(tr("Record")) { record(id) }.disabled(workflowName.isEmpty)
                    }.padding(4)
                }
                GroupBox(tr("Exports")) {
                    HStack {
                        Button(tr("Text with positions …")) { export(id, kind: "text", ext: "txt") }
                        Button(tr("Report (Markdown) …")) { export(id, kind: "report", ext: "md") }
                        Button(tr("Features as CSV …")) { export(id, kind: "features_csv", ext: "csv") }
                        Button(tr("Features as TSV …")) { export(id, kind: "features_tsv", ext: "tsv") }
                    }.padding(4)
                }
                if let message { Text(message).font(Theme.label(11)).foregroundStyle(Theme.inkSoft) }
            } else {
                Text(tr("Select a document on the left.")).foregroundStyle(Theme.inkSoft)
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
            message = tr("Version %@ restored.", "\(v.version)")
            load()
        } catch { message = tr("Error: %@", "\(error.localizedDescription)") }
    }

    private func record(_ id: String) {
        do {
            let n = try model.project?.recordWorkflow(docId: id, name: workflowName) ?? 0
            message = tr("Workflow with %@ step(s) saved (see the “Workflows” dialog).", "\(n)")
        } catch { message = tr("Error: %@", "\(error.localizedDescription)") }
    }

    private func export(_ id: String, kind: String, ext: String) {
        guard let p = model.project else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "export.\(ext)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let text = try p.exportText(id: id, kind: kind)
            try text.write(to: url, atomically: true, encoding: .utf8)
            message = tr("Saved: %@", "\(url.lastPathComponent)")
        } catch { message = tr("Error: %@", "\(error.localizedDescription)") }
    }
}

/// Settings and plugins.
struct SettingsTab: View {
    @State private var s = settingsLoad()
    @State private var plugins: [String] = []
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            GroupBox(tr("Program")) {
                Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
                    GridRow { Text(tr("NCBI email")); TextField("", text: $s.ncbiEmail).textFieldStyle(.roundedBorder) }
                    GridRow { Text(tr("NCBI API key")); SecureField("", text: $s.ncbiApiKey).textFieldStyle(.roundedBorder) }
                    GridRow { Text(tr("Default organism")); TextField("", text: $s.defaultOrganism).textFieldStyle(.roundedBorder) }
                    GridRow { Text(tr("Undo steps")); Stepper("\(s.undoLimit)", value: Binding(get: { Int(s.undoLimit) }, set: { s.undoLimit = UInt64($0) }), in: 10...1000, step: 10) }
                }.padding(4)
                HStack {
                    Button(tr("Save")) { do { try settingsSave(s: s); message = tr("Saved (readable only by you).") } catch { message = error.localizedDescription } }
                    if let message { Text(message).font(Theme.label(11)).foregroundStyle(Theme.inkSoft) }
                }.padding(4)
            }
            GroupBox(tr("Plugins (programs with plugin.json in the folder)")) {
                VStack(alignment: .leading, spacing: 4) {
                    if plugins.isEmpty { Text(tr("No plugins found.")).foregroundStyle(Theme.inkSoft) }
                    ForEach(plugins, id: \.self) { Text($0).font(Theme.label(13)) }
                    HStack {
                        Button(tr("Open plugin folder")) { NSWorkspace.shared.open(URL(fileURLWithPath: pluginsDirectory())) }
                        Button(tr("Reload")) { plugins = pluginsList() }
                    }
                    Text(tr("Caution: plugins run without a sandbox with your permissions.")).font(Theme.label(11)).foregroundStyle(Theme.amber)
                }.padding(4)
            }
            Spacer()
        }
        .onAppear { plugins = pluginsList() }
    }
}
