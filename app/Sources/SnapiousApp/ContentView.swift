import SnapiousFFI
import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @ObservedObject var model: AppModel
    @State private var renameTarget: DocumentInfoDto?
    @State private var renameText = ""
    @State private var dropTargeted = false
    @State private var showNcbi = false
    @State private var structure: PdbDto?
    @State private var structureName = ""
    @State private var showWorkflows = false
    @State private var showManage = false
    @State private var showNewSequence = false
    @State private var showTableImport = false
    @State private var showPalette = false
    @State private var showShortcuts = false
    @State private var showInspector = false
    @State private var filterText = ""

    private func openStructure() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = ["pdb", "xyz", "mol", "sdf", "cml", "hin"].compactMap { UTType(filenameExtension: $0) }
        panel.message = "Struktur wählen (PDB, XYZ, MOL, SDF, CML, HIN)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            structure = try structureOpen(path: url.path)
            structureName = url.lastPathComponent
        } catch {
            model.errorMessage = error.localizedDescription
        }
    }

    var body: some View {
        NavigationSplitView {
            sidebar
                .navigationSplitViewColumnWidth(min: 220, ideal: 280, max: 420)
        } detail: {
            DetailView(model: model)
        }
        .navigationTitle(model.projectTitle)
        .toolbar {
            ToolbarItemGroup {
                Button { showNewSequence = true } label: { Label("Neue Sequenz", systemImage: "plus.square") }
                    .help("Sequenz eintippen oder einfügen (⇧⌘N)")
                    .keyboardShortcut("n", modifiers: [.command, .shift])
                    .disabled(model.project == nil)
                Menu {
                    Button("Sequenz- und Alignmentdateien …") { model.chooseFilesToImport() }
                    Button("Tabelle (CSV/TSV) …") { showTableImport = true }
                    Button("Aus NCBI …") { showNcbi = true }
                } label: { Label("Importieren", systemImage: "square.and.arrow.down") }
                    .help("Dateien, Tabellen oder NCBI-Datensätze importieren")
                    .disabled(model.project == nil)
                Button { showPalette = true } label: { Label("Suchen", systemImage: "magnifyingglass") }
                    .help("Befehle, Ansichten und Dokumente suchen (⌘K)")
                    .keyboardShortcut("k")
                Button { showInspector.toggle() } label: { Label("Inspektor", systemImage: "sidebar.right") }
                    .help("Eckdaten, Features und Notiz einblenden")
                    .disabled(model.project == nil)
                Menu {
                    Button("Neues Projekt …") { model.newProject() }
                    Button("Projekt öffnen …") { model.openProject() }
                    Divider()
                    Button("Workflows …") { showWorkflows = true }.disabled(model.project == nil)
                    Button("Verwaltung …") { showManage = true }.disabled(model.project == nil)
                    Button("Struktur öffnen …") { openStructure() }
                    Divider()
                    Button("Tastenkürzel …") { showShortcuts = true }.keyboardShortcut("/")
                } label: { Label("Mehr", systemImage: "ellipsis.circle") }
                    .help("Projekt, Workflows, Verwaltung und weitere Funktionen")
            }
        }
        .sheet(isPresented: $showNcbi) { NcbiView(model: model) }
        .sheet(isPresented: Binding(get: { structure != nil }, set: { if !$0 { structure = nil } })) {
            if let structure { StructureView(pdb: structure, name: structureName) }
        }
        .sheet(isPresented: $showWorkflows) { WorkflowView(model: model) }
        .sheet(isPresented: $showNewSequence) { NewSequenceView(model: model) }
        .sheet(isPresented: $showTableImport) { TableImportView(model: model) }
        .sheet(isPresented: $showPalette) { CommandPaletteView(commands: commands) }
        .sheet(isPresented: $showShortcuts) { ShortcutsView() }
        .inspector(isPresented: $showInspector) {
            InspectorView(model: model).inspectorColumnWidth(min: 220, ideal: 280, max: 400)
        }
        .sheet(isPresented: $showManage) { ManageView(model: model) }
        .onDrop(of: [.fileURL], isTargeted: $dropTargeted) { providers in
            handleDrop(providers)
        }
        .overlay(alignment: .bottom) {
            if let n = model.notice {
                Text(n)
                    .padding(.horizontal, 14).padding(.vertical, 8)
                    .background(.regularMaterial, in: Capsule())
                    .overlay(Capsule().strokeBorder(.separator))
                    .padding(.bottom, 18)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                    .accessibilityAddTraits(.updatesFrequently)
            }
        }
        .animation(.easeInOut(duration: 0.2), value: model.notice)
        .overlay {
            if dropTargeted {
                RoundedRectangle(cornerRadius: 12)
                    .strokeBorder(Theme.accent, style: StrokeStyle(lineWidth: 3, dash: [8]))
                    .padding(8)
                    .allowsHitTesting(false)
            }
        }
        .alert(model.errorMessage.map { friendlyError(message: $0).title } ?? "Fehler", isPresented: Binding(
            get: { model.errorMessage != nil },
            set: { if !$0 { model.errorMessage = nil } }
        )) {
            Button("OK", role: .cancel) {}
        } message: {
            if let m = model.errorMessage {
                Text(friendlyError(message: m).hint + (m.count < 400 ? "\n\n" + m : ""))
            }
        }
        .alert("Dokument umbenennen", isPresented: Binding(
            get: { renameTarget != nil },
            set: { if !$0 { renameTarget = nil } }
        )) {
            TextField("Name", text: $renameText)
            Button("Umbenennen") {
                if let t = renameTarget { model.rename(id: t.id, to: renameText) }
                renameTarget = nil
            }
            Button("Abbrechen", role: .cancel) { renameTarget = nil }
        }
        .frame(minWidth: 820, minHeight: 520)
                        .groupBoxStyle(PanelGroupBoxStyle())
    }

    /// Alle Befehle der Befehlspalette: Aktionen, Reiter und Dokumente.
    private var commands: [AppCommand] {
        var list: [AppCommand] = [
            AppCommand(title: "Neue Sequenz …", keywords: ["einfügen", "tippen", "fasta"], shortcut: "⇧⌘N") { showNewSequence = true },
            AppCommand(title: "Sequenzdateien importieren …", keywords: ["import", "genbank", "fasta", "datei"], shortcut: "⌘I") { model.chooseFilesToImport() },
            AppCommand(title: "Tabelle importieren (CSV/TSV) …", keywords: ["csv", "excel", "tabelle"]) { showTableImport = true },
            AppCommand(title: "Aus NCBI importieren …", keywords: ["genbank", "datenbank", "suchen"]) { showNcbi = true },
            AppCommand(title: "Neues Projekt …", keywords: ["anlegen"], shortcut: "⌘N") { model.newProject() },
            AppCommand(title: "Projekt öffnen …", keywords: ["laden"], shortcut: "⌘O") { model.openProject() },
            AppCommand(title: "Workflows …", keywords: ["abfolge", "automatisieren"]) { showWorkflows = true },
            AppCommand(title: "Verwaltung …", keywords: ["ordner", "version", "einstellungen", "plugins", "suche"]) { showManage = true },
            AppCommand(title: "Struktur öffnen …", keywords: ["pdb", "3d", "molekül", "xyz", "mol"]) { openStructure() },
            AppCommand(title: "Inspektor ein-/ausblenden", keywords: ["notiz", "features", "seitenleiste"]) { showInspector.toggle() },
            AppCommand(title: "Tastenkürzel anzeigen", keywords: ["hilfe", "shortcuts"], shortcut: "⌘/") { showShortcuts = true },
        ]
        if model.selection != nil {
            list.append(AppCommand(title: "Rückgängig", keywords: ["undo", "zurück"], shortcut: "⌘Z") { model.undo() })
            list.append(AppCommand(title: "Wiederholen", keywords: ["redo"], shortcut: "⇧⌘Z") { model.redo() })
            for (i, t) in DetailView.tabNames.enumerated() {
                list.append(AppCommand(title: "Gehe zu: \(t)", keywords: ["ansicht", "reiter", "tab"]) { model.tab = i })
            }
        }
        for d in model.documents {
            list.append(AppCommand(title: "Öffnen: \(d.name)", keywords: [d.docType, "dokument"]) {
                model.multiSelection = [d.id]
                model.selection = d.id
            })
        }
        return list
    }

    private var sidebar: some View {
        List(selection: $model.multiSelection) {
            ForEach(model.documents.filter { filterText.isEmpty || $0.name.localizedCaseInsensitiveContains(filterText) }, id: \.id) { doc in
                docRow(doc)
                    .tag(doc.id)
                    .contextMenu {
                        Button("Umbenennen…") {
                            renameText = doc.name
                            renameTarget = doc
                        }
                        if doc.docType == "sequence" {
                            Button("Annotationen importieren…") { model.importAnnotations(id: doc.id) }
                        }
                        Menu("Exportieren") {
                            ForEach(exportFormats(for: doc.docType), id: \.0) { f in
                                Button(f.1) { model.export(id: doc.id, format: f.0, ext: f.2) }
                            }
                        }
                        Button("Löschen", role: .destructive) { model.delete(id: doc.id) }
                    }
            }
        }
        .listStyle(.sidebar)
        .searchable(text: $filterText, placement: .sidebar, prompt: "Dokumente filtern")
        .overlay {
            if model.project == nil {
                EmptyState(
                    icon: "tray", title: "Kein Projekt",
                    message: "Lege ein neues Projekt an oder öffne ein vorhandenes.",
                    actions: [("Neues Projekt", { model.newProject() }), ("Projekt öffnen", { model.openProject() })]
                )
            } else if model.documents.isEmpty {
                EmptyState(
                    icon: "dna", title: "Noch keine Dokumente",
                    message: "Importiere Sequenzen oder ziehe Dateien in dieses Fenster.",
                    actions: [("Neue Sequenz", { showNewSequence = true }), ("Importieren", { model.chooseFilesToImport() })]
                )
            }
        }
    }


    private func docRow(_ doc: DocumentInfoDto) -> some View {
        Label {
            VStack(alignment: .leading, spacing: 1) {
                Text(doc.name).lineLimit(1)
                Text(typeName(doc.docType)).font(.caption).foregroundStyle(.secondary)
            }
        } icon: {
            Image(systemName: icon(for: doc))
        }
    }

    private func typeName(_ t: String) -> String {
        switch t {
        case "sequence": return "Sequenz"
        case "alignment": return "Alignment"
        default: return t
        }
    }

    /// (Formatname im Kern, Anzeigename, Dateiendung)
    private func exportFormats(for type: String) -> [(String, String, String)] {
        switch type {
        case "sequence": return [("genbank", "GenBank (.gb)", "gb"), ("embl", "EMBL (.embl)", "embl"), ("fasta", "FASTA (.fasta)", "fasta"), ("gff3", "Annotationen (.gff3)", "gff3"), ("svg", "Kartenbild (.svg)", "svg")]
        case "alignment": return [("clustal", "Clustal (.aln)", "aln"), ("phylip", "PHYLIP (.phy)", "phy"), ("nexus", "NEXUS (.nex)", "nex"), ("mega", "MEGA (.meg)", "meg"), ("fasta", "FASTA (.fasta)", "fasta")]
        default: return []
        }
    }

    private func icon(for doc: DocumentInfoDto) -> String {
        switch doc.docType {
        case "sequence": return "text.alignleft"
        case "alignment": return "list.bullet.indent"
        default: return "doc"
        }
    }

    private func handleDrop(_ providers: [NSItemProvider]) -> Bool {
        var accepted = false
        for p in providers where p.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
            accepted = true
            _ = p.loadObject(ofClass: URL.self) { url, _ in
                guard let url else { return }
                Task { @MainActor in model.importFiles([url]) }
            }
        }
        return accepted
    }
}

struct DetailView: View {
    @ObservedObject var model: AppModel
    @State private var featFilter = ""
    @State private var featType = ""
    @State private var featSort = "start"
    @State private var featAsc = true

    static let tabNames = ["Übersicht", "Sequenz & Karte", "Verdau", "CRISPR", "Klonieren", "Primer", "Analyse", "Protein", "Sequenzierung", "RNA", "Werkzeuge", "NGS"]
    /// Aufgabengruppen: Name und Indizes der Reiter.
    private static let groups: [(String, [Int])] = [
        ("Ansehen", [0, 1]),
        ("Bauen", [2, 3, 4, 5]),
        ("Prüfen", [6, 7, 8, 9]),
        ("Daten", [10, 11]),
    ]

    private var group: Int { Self.groups.firstIndex { $0.1.contains(model.tab) } ?? 0 }

    /// Zwei Zeilen: oben die Aufgabengruppe, darunter die Reiter dieser Gruppe.
    private var tabBar: some View {
        VStack(spacing: 4) {
            Picker("", selection: Binding(get: { group }, set: { model.tab = Self.groups[$0].1[0] })) {
                ForEach(Array(Self.groups.enumerated()), id: \.offset) { Text($0.element.0).tag($0.offset) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(maxWidth: 420)
            HStack(spacing: 2) {
                ForEach(Self.groups[group].1, id: \.self) { i in
                    Button { model.tab = i } label: {
                        Text(Self.tabNames[i])
                            .fontWeight(model.tab == i ? .semibold : .regular)
                            .padding(.horizontal, 12).padding(.vertical, 4)
                            .background(model.tab == i ? Theme.accent.opacity(0.22) : Color.clear, in: RoundedRectangle(cornerRadius: 6))
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .padding(.vertical, 6)
        .frame(maxWidth: .infinity)
    }

    var body: some View {
        if model.project == nil {
            WelcomeView(model: model)
        } else if model.multiSelection.count >= 2 {
            MultiSelectionView(model: model)
        } else if model.selectedType == "alignment", model.alignment != nil {
            AlignmentView(model: model)
        } else if let s = model.summary {
            VStack(spacing: 0) {
                tabBar
                Divider()
                switch model.tab {
                case 0: overview(s)
                case 1: MapView(model: model)
                case 2: DigestView(model: model)
                case 3: GuideView(model: model)
                case 4: CloneView(model: model)
                case 5: PrimerView(model: model)
                case 6: AnalysisView(model: model)
                case 7: ProteinView(model: model)
                case 8: ReadCheckView(model: model)
                case 9: RnaView(model: model)
                case 10: ToolsView(model: model)
                default: NgsView(model: model)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                    } else {
            EmptyState(icon: "dna", title: "Kein Dokument ausgewählt", message: "Wähle links ein Dokument aus.")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                        }
    }

    private func overview(_ s: DocumentSummary) -> some View {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    header(s)
                    facts(s)
                    if !model.features.isEmpty { featureTable }
                    previewBox(s)
                }
                .padding(24)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
                }

    private func header(_ s: DocumentSummary) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(s.name).font(.largeTitle.bold())
            if !s.description.isEmpty {
                Text(s.description).font(Theme.label(13)).foregroundStyle(Theme.inkSoft)
            }
            Text([s.seqType == "PROTEIN" ? "Protein" : s.seqType, s.topology == "CIRCULAR" ? "zirkulär" : "linear", s.organism]
                    .filter { !$0.isEmpty }.joined(separator: " · "))
                .foregroundStyle(.secondary)
        }
    }

    private func facts(_ s: DocumentSummary) -> some View {
        VStack(spacing: Theme.gap) {
            HStack(spacing: Theme.gap) {
                StatTile(title: "Länge", value: "\(s.length) \(s.seqType == "PROTEIN" ? "aa" : "bp")")
                if s.seqType != "PROTEIN" {
                    StatTile(title: "GC-Gehalt", value: String(format: "%.1f %%", s.gcPercent), tint: Theme.amber)
                }
                StatTile(title: "Features", value: "\(s.featureCount)", tint: Theme.good)
            }
            if !s.accession.isEmpty {
                Card(padding: 12) {
                    HStack {
                        Text("Accession").font(Theme.label()).foregroundStyle(Theme.inkSoft)
                        Text(s.accession).font(Theme.mono(12)).textSelection(.enabled)
                    }
                }
            }
        }
    }

    private var featureTable: some View {
        let order = featureTableOrder(features: model.features, typeFilter: featType, text: featFilter, column: featSort, ascending: featAsc)
        let rows = order.map { IndexedFeature(index: Int($0), info: model.features[Int($0)]) }
        return VStack(alignment: .leading, spacing: 8) {
            SectionHeader(text: "Features", detail: rows.count == model.features.count ? "\(rows.count)" : "\(rows.count) von \(model.features.count)")
            HStack {
                TextField("Name filtern", text: $featFilter).textFieldStyle(.roundedBorder).frame(width: 170)
                Picker("Typ", selection: $featType) {
                    Text("Alle Typen").tag("")
                    ForEach(Array(Set(model.features.map { $0.featureType })).sorted(), id: \.self) { Text($0).tag($0) }
                }
                .frame(width: 170)
                Picker("Sortieren", selection: $featSort) {
                    Text("Beginn").tag("start")
                    Text("Ende").tag("end")
                    Text("Länge").tag("length")
                    Text("Name").tag("name")
                    Text("Typ").tag("type")
                }
                .frame(width: 160)
                Button { featAsc.toggle() } label: { Image(systemName: featAsc ? "arrow.up" : "arrow.down") }.accessibilityLabel(featAsc ? "Aufsteigend sortiert" : "Absteigend sortiert")
                    .help(featAsc ? "Aufsteigend" : "Absteigend")
                Spacer()
                Button("Als CSV …") { exportFeatureCsv() }
            }
            Table(rows) {
                TableColumn("Typ") { f in Text(f.info.featureType) }
                TableColumn("Name") { f in Text(f.info.name) }
                TableColumn("Bereich") { f in Text(verbatim: "\(f.info.start + 1)–\(f.info.end)") }
                TableColumn("Länge") { f in Text("\(f.info.end - f.info.start)") }.width(70)
                TableColumn("Strang") { f in Text(strand(f.info.strand)) }
                TableColumn("") { f in
                    Button { model.removeFeature(index: f.index) } label: { Image(systemName: "trash") }
                        .accessibilityLabel("Feature löschen")
                        .buttonStyle(.borderless)
                        .help("Feature löschen")
                }.width(30)
            }
            .frame(minHeight: 120, maxHeight: 240)
        }
    }

    /// Alle Features des Dokuments als CSV speichern.
    private func exportFeatureCsv() {
        guard let project = model.project, let id = model.selection, let url = FilePick.save("Features als CSV speichern", name: "features.csv") else { return }
        do { try project.exportText(id: id, kind: "features_csv").write(to: url, atomically: true, encoding: .utf8) } catch { model.errorMessage = error.localizedDescription }
    }

    private func strand(_ s: String) -> String {
        switch s {
        case "FORWARD": return "+"
        case "REVERSE": return "−"
        default: return "·"
        }
    }

    private func previewBox(_ s: DocumentSummary) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(text: "Ganze Sequenz")
            Card(padding: 12) {
                let full = String(model.fullBases.prefix(200_000))
                Text(s.seqType == "PROTEIN" || full.count > 30_000 ? AttributedString(grouped(full)) : coloredBases(grouped(full)))
                    .font(Theme.mono(13))
                    .textSelection(.enabled)
            }
            if s.length > 200_000 {
                Text("… weitere \(s.length - 200_000) Basen (nicht angezeigt)")
                    .font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            }
        }
    }

    private func coloredBases(_ text: String) -> AttributedString {
        var out = AttributedString()
        for ch in text {
            var piece = AttributedString(String(ch))
            piece.foregroundColor = Theme.base(ch)
            out += piece
        }
        return out
    }

    /// Zeilen zu 60 Zeichen, in Gruppen zu 10 (Vorschau; die richtige Sequenzansicht folgt in Phase 1).
    private func grouped(_ bases: String) -> String {
        var lines: [String] = []
        var i = bases.startIndex
        while i < bases.endIndex {
            let end = bases.index(i, offsetBy: 60, limitedBy: bases.endIndex) ?? bases.endIndex
            let line = bases[i..<end]
            var parts: [String] = []
            var j = line.startIndex
            while j < line.endIndex {
                let e = line.index(j, offsetBy: 10, limitedBy: line.endIndex) ?? line.endIndex
                parts.append(String(line[j..<e]))
                j = e
            }
            lines.append(parts.joined(separator: " "))
            i = end
        }
        return lines.joined(separator: "\n")
    }
}

extension FeatureInfo: Identifiable {
    public var id: String { "\(featureType)-\(name)-\(start)-\(end)-\(strand)" }
}

/// Aktionen für mehrere gewählte Dokumente: ausrichten oder zusammenbauen.
struct MultiSelectionView: View {
    @ObservedObject var model: AppModel
    @State private var minOverlap = 20
    @State private var maxMismatch = 2
    @State private var mapRows: [ReadMapDto] = []
    @State private var circular = true
    @State private var enzyme = "BsaI"
    @State private var aligner = "builtin"
    @State private var blastProgram = "blastn"
    @State private var blastHits: [BlastRow] = []
    @State private var blastMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("\(model.multiSelection.count) Dokumente gewählt").font(.title2.bold())
            ReadConsensusSection(model: model)
            ConcatSection(model: model)
            CompareAnnotationsSection(model: model)
            PairedReadsSection(model: model)
            LengthHistogramSection(model: model)
            GroupBox("Ausrichten") {
                HStack {
                    Picker("Verfahren", selection: $aligner) {
                        Text("Eingebaut").tag("builtin")
                        Text("MAFFT").tag("mafft")
                        Text("MUSCLE").tag("muscle")
                        Text("Clustal Omega").tag("clustalo")
                    }
                    .frame(width: 240)
                    Spacer()
                    Button("Ausrichten") { model.alignSelected(backend: aligner) }
                }
                .padding(6)
            }
            GroupBox("BLAST (lokal): erste gewählte Sequenz gegen die übrigen") {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Picker("Programm", selection: $blastProgram) {
                            Text("blastn (DNA)").tag("blastn")
                            Text("blastp (Protein)").tag("blastp")
                        }
                        .frame(width: 220)
                        Button("Suchen") { runBlast() }
                        if let blastMessage { Text(blastMessage).foregroundStyle(Theme.danger) }
                    }
                    if !blastHits.isEmpty {
                        Table(blastHits) {
                            TableColumn("Treffer") { h in Text(h.dto.subject) }
                            TableColumn("Identität") { h in Text(String(format: "%.1f %%", h.dto.identityPercent)) }.width(80)
                            TableColumn("Länge") { h in Text("\(h.dto.alignmentLength)") }.width(60)
                            TableColumn("Query") { h in Text("\(h.dto.queryStart)–\(h.dto.queryEnd)") }.width(90)
                            TableColumn("Subject") { h in Text("\(h.dto.subjectStart)–\(h.dto.subjectEnd)" + (h.dto.reverseStrand ? " (−)" : "")) }.width(110)
                            TableColumn("E-Wert") { h in Text(String(format: "%.1e", h.dto.evalue)) }.width(80)
                            TableColumn("Bit") { h in Text(String(format: "%.1f", h.dto.bitScore)) }.width(60)
                        }
                        .frame(minHeight: 120, maxHeight: 220)
                    }
                }
                .padding(6)
            }
            GroupBox("Reads: Contigs bilden oder auf die erste Sequenz abbilden") {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Stepper("Mindestüberlappung \(minOverlap) bp", value: $minOverlap, in: 5...200)
                            .frame(width: 280)
                        Button("Contigs bilden") { model.assembleReadsSelected(minOverlap: minOverlap) }
                    }
                    HStack {
                        Stepper("Fehlpaarungen ≤ \(maxMismatch)", value: $maxMismatch, in: 0...20)
                            .frame(width: 280)
                        Button("Auf Referenz abbilden") { mapRows = model.mapReadsSelected(maxMismatches: maxMismatch) }
                    }
                    if !mapRows.isEmpty {
                        ForEach(Array(mapRows.enumerated()), id: \.offset) { _, r in
                            Text(r.mapped
                                 ? "\(r.name): Position \(r.position + 1)\(r.reverse ? " (−)" : "") · \(r.mismatches) Fehlpaarung(en)"
                                 : "\(r.name): nicht abgebildet")
                                .font(Theme.label(13))
                        }
                    }
                }
                .padding(6)
            }
            GroupBox("Zusammenbauen (Reihenfolge wie in der Liste)") {
                VStack(alignment: .leading, spacing: 8) {
                    Toggle("Zirkuläres Konstrukt", isOn: $circular)
                    HStack {
                        Button("Gibson") { model.assembleSelected(kind: "gibson", circular: circular) }
                        Button("In-Fusion") { model.assembleSelected(kind: "infusion", circular: circular) }
                        Button("SLIC") { model.assembleSelected(kind: "slic", circular: circular) }
                    }
                    HStack {
                        Picker("Enzym", selection: $enzyme) {
                            ForEach(["BsaI", "BsmBI", "Esp3I", "BbsI", "SapI"], id: \.self) { Text($0).tag($0) }
                        }
                        .frame(width: 180)
                        Button("Golden Gate") { model.assembleSelected(kind: "goldengate", circular: circular, enzyme: enzyme) }
                    }
                    if let notice = model.assemblyNotice {
                        Text(notice).font(Theme.label(13)).foregroundStyle(Theme.amber)
                    }
                }
                .padding(6)
            }
            Spacer()
        }
        .padding(20)
        .onChange(of: model.multiSelection) { _, _ in
            blastHits = []
            blastMessage = nil
        }
    }

    private func runBlast() {
        blastMessage = nil
        guard let project = model.project else { return }
        let ids = model.documents.filter { model.multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        guard ids.count >= 2 else { return }
        do {
            let hits = try project.blastSearch(queryId: ids[0], subjectIds: Array(ids.dropFirst()), program: blastProgram, maxHits: 25)
            blastHits = hits.enumerated().map { BlastRow(index: $0.offset, dto: $0.element) }
            if hits.isEmpty { blastMessage = "Keine Treffer." }
        } catch {
            blastHits = []
            blastMessage = error.localizedDescription
        }
    }
}

struct BlastRow: Identifiable {
    let index: Int
    let dto: BlastHitDto
    var id: Int { index }
}

struct IndexedFeature: Identifiable {
    let index: Int
    let info: FeatureInfo
    var id: Int { index }
}
