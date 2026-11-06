import AppKit
import SnapiousFFI
import SwiftUI

/// Ein Befehl der Befehlspalette (⌘K).
struct AppCommand: Identifiable {
    let id = UUID()
    let title: String
    var keywords: [String] = []
    var shortcut: String?
    let run: () -> Void
}

/// Befehlspalette: tippen, mit ↑↓ wählen, Return führt aus.
struct CommandPaletteView: View {
    let commands: [AppCommand]
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var selected = 0
    @FocusState private var focused: Bool

    private var ranked: [AppCommand] {
        let order = commandRank(items: commands.map { CommandItemDto(title: $0.title, keywords: $0.keywords) }, query: query)
        return order.prefix(40).map { commands[Int($0)] }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
                TextField("Befehl, Ansicht oder Dokument …", text: $query)
                    .textFieldStyle(.plain)
                    .font(.title3)
                    .focused($focused)
                    .onSubmit { runSelected() }
                    .onChange(of: query) { _, _ in selected = 0 }
                    .onKeyPress(.downArrow) { selected = min(selected + 1, max(ranked.count - 1, 0)); return .handled }
                    .onKeyPress(.upArrow) { selected = max(selected - 1, 0); return .handled }
            }
            .padding(12)
            Divider()
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach(Array(ranked.enumerated()), id: \.element.id) { i, c in
                            HStack {
                                Text(c.title)
                                Spacer()
                                if let s = c.shortcut { Text(s).foregroundStyle(.secondary).font(.callout) }
                            }
                            .padding(.horizontal, 12).padding(.vertical, 6)
                            .background(i == selected ? Color.accentColor.opacity(0.25) : Color.clear)
                            .contentShape(Rectangle())
                            .onTapGesture { selected = i; runSelected() }
                            .id(i)
                        }
                        if ranked.isEmpty {
                            Text("Nichts gefunden").foregroundStyle(.secondary).padding(20)
                        }
                    }
                }
                .onChange(of: selected) { _, v in proxy.scrollTo(v) }
            }
        }
        .frame(width: 520, height: 380)
        .onAppear { focused = true }
    }

    private func runSelected() {
        let list = ranked
        guard selected < list.count else { return }
        let c = list[selected]
        dismiss()
        DispatchQueue.main.async { c.run() }
    }
}

/// Willkommensseite ohne Projekt: neues Projekt, öffnen und zuletzt benutzte Projekte.
struct WelcomeView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "dna").font(.system(size: 54, weight: .light)).foregroundStyle(Color.accentColor)
            Text("Snapious").font(.largeTitle.bold())
            Text("Sequenzen ansehen, bearbeiten und klonieren").foregroundStyle(.secondary)
            HStack(spacing: 12) {
                Button { model.newProject() } label: { Label("Neues Projekt", systemImage: "doc.badge.plus").frame(width: 150) }
                    .buttonStyle(.borderedProminent).controlSize(.large)
                Button { model.openProject() } label: { Label("Projekt öffnen", systemImage: "folder").frame(width: 150) }
                    .controlSize(.large)
            }
            if !model.recentProjects.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Zuletzt geöffnet").font(.headline)
                    ForEach(model.recentProjects.prefix(6), id: \.self) { p in
                        Button { model.open(path: p) } label: {
                            HStack {
                                Image(systemName: "clock").foregroundStyle(.secondary)
                                VStack(alignment: .leading) {
                                    Text(URL(fileURLWithPath: p).deletingPathExtension().lastPathComponent)
                                    Text(p).font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
                                }
                                Spacer()
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(14)
                .frame(width: 380, alignment: .leading)
                .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 8))
            }
        }
        .padding(30)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// Inspektor rechts: Eckdaten, Features und Notiz des gewählten Dokuments.
struct InspectorView: View {
    @ObservedObject var model: AppModel
    @State private var note = ""
    @State private var loadedFor: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if let s = model.summary {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(s.name).font(.headline)
                        Text("\(s.length) \(s.seqType == "PROTEIN" ? "aa" : "bp") · \(s.topology == "CIRCULAR" ? "zirkulär" : "linear")")
                            .foregroundStyle(.secondary)
                        if s.seqType != "PROTEIN" { Text(String(format: "GC %.1f %%", s.gcPercent)).foregroundStyle(.secondary) }
                    }
                    Divider()
                    Text("Features (\(model.features.count))").font(.headline)
                    ForEach(Array(model.features.prefix(60).enumerated()), id: \.offset) { _, f in
                        Button {
                            model.select(from: Int(f.start), to: Int(f.end))
                            model.tab = 1
                        } label: {
                            HStack {
                                Text(f.name.isEmpty ? f.featureType : f.name).lineLimit(1)
                                Spacer()
                                Text("\(f.start + 1)–\(f.end)").font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        .buttonStyle(.plain)
                    }
                    if model.features.count > 60 { Text("… und \(model.features.count - 60) weitere").font(.caption).foregroundStyle(.secondary) }
                    Divider()
                    Text("Notiz").font(.headline)
                    TextEditor(text: $note)
                        .frame(minHeight: 100)
                        .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                    Button("Notiz speichern") { saveNote() }
                } else {
                    Text("Wähle ein Dokument.").foregroundStyle(.secondary)
                }
            }
            .padding(14)
        }
        .onAppear { loadNote() }
        .onChange(of: model.selection) { _, _ in loadNote() }
    }

    private func loadNote() {
        guard let project = model.project, let id = model.selection else { note = ""; loadedFor = nil; return }
        note = (try? project.note(docId: id)) ?? ""
        loadedFor = id
    }

    private func saveNote() {
        guard let project = model.project, let id = model.selection else { return }
        do { try project.setNote(docId: id, text: note) } catch { model.errorMessage = error.localizedDescription }
    }
}

/// Übersicht der Tastenkürzel (⌘/).
struct ShortcutsView: View {
    @Environment(\.dismiss) private var dismiss
    static let all: [(String, String)] = [
        ("Befehlspalette", "cmd+k"),
        ("Neue Sequenz", "cmd+shift+n"),
        ("Neues Projekt", "cmd+n"),
        ("Projekt öffnen", "cmd+o"),
        ("Importieren", "cmd+i"),
        ("Rückgängig (Sequenzansicht)", "cmd+z"),
        ("Wiederholen (Sequenzansicht)", "cmd+shift+z"),
        ("Tastenkürzel anzeigen", "cmd+/"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Tastenkürzel").font(.title2.bold())
            ForEach(Self.all, id: \.0) { item in
                HStack {
                    Text(item.0)
                    Spacer()
                    Text(Self.pretty(item.1)).font(.system(.body, design: .monospaced)).foregroundStyle(.secondary)
                }
            }
            let conflicts = shortcutConflictPairs(keys: Self.all.map { $0.1 })
            if !conflicts.isEmpty {
                Text("Doppelt belegt: " + conflicts.map { "\(Self.all[Int($0[0])].0) / \(Self.all[Int($0[1])].0)" }.joined(separator: "; "))
                    .foregroundStyle(.orange)
            }
            Text("In der Sequenzansicht: Klicken setzt den Cursor, Tippen fügt Basen ein, Rücktaste löscht, ⌘V fügt Text ein.")
                .font(.caption).foregroundStyle(.secondary)
            HStack { Spacer(); Button("Schließen") { dismiss() }.keyboardShortcut(.defaultAction) }
        }
        .padding(20)
        .frame(width: 420)
    }

    /// Darstellung im macOS-Stil (⇧⌘K).
    static func pretty(_ keys: String) -> String { formatShortcut(keys: keys) }
}
