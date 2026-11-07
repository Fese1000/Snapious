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
        panel.message = tr("Choose structure (PDB, XYZ, MOL, SDF, CML, HIN)")
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
                Button { showNewSequence = true } label: { Label(tr("New sequence"), systemImage: "plus.square") }
                    .help(tr("Type or paste a sequence (⇧⌘N)"))
                    .keyboardShortcut("n", modifiers: [.command, .shift])
                    .disabled(model.project == nil)
                Menu {
                    Button(tr("Sequence and alignment files …")) { model.chooseFilesToImport() }
                    Button(tr("Table (CSV/TSV) …")) { showTableImport = true }
                    Button(tr("From NCBI …")) { showNcbi = true }
                } label: { Label(tr("Import"), systemImage: "square.and.arrow.down") }
                    .help(tr("Import files, tables or NCBI records"))
                    .disabled(model.project == nil)
                Button { showPalette = true } label: { Label(tr("Search"), systemImage: "magnifyingglass") }
                    .help(tr("Search commands, views and documents (⌘K)"))
                    .keyboardShortcut("k")
                Button { showInspector.toggle() } label: { Label(tr("Inspector"), systemImage: "sidebar.right") }
                    .help(tr("Show facts, features and note"))
                    .disabled(model.project == nil)
                Menu {
                    Button(tr("New project …")) { model.newProject() }
                    Button(tr("Open project …")) { model.openProject() }
                    Divider()
                    Button(tr("Workflows …")) { showWorkflows = true }.disabled(model.project == nil)
                    Button(tr("Management …")) { showManage = true }.disabled(model.project == nil)
                    Button(tr("Open structure …")) { openStructure() }
                    Divider()
                    Button(tr("Keyboard shortcuts …")) { showShortcuts = true }.keyboardShortcut("/")
                } label: { Label(tr("More"), systemImage: "ellipsis.circle") }
                    .help(tr("Project, workflows, management and more"))
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
        .alert(model.errorMessage.map { friendlyError(message: $0).title } ?? tr("Error"), isPresented: Binding(
            get: { model.errorMessage != nil },
            set: { if !$0 { model.errorMessage = nil } }
        )) {
            Button(tr("OK"), role: .cancel) {}
        } message: {
            if let m = model.errorMessage {
                Text(friendlyError(message: m).hint + (m.count < 400 ? "\n\n" + m : ""))
            }
        }
        .alert(tr("Rename document"), isPresented: Binding(
            get: { renameTarget != nil },
            set: { if !$0 { renameTarget = nil } }
        )) {
            TextField(tr("Name"), text: $renameText)
            Button(tr("Rename")) {
                if let t = renameTarget { model.rename(id: t.id, to: renameText) }
                renameTarget = nil
            }
            Button(tr("Cancel"), role: .cancel) { renameTarget = nil }
        }
        .frame(minWidth: 820, minHeight: 520)
                        .groupBoxStyle(PanelGroupBoxStyle())
    }

    /// All commands of the command palette: actions, tabs and documents.
    private var commands: [AppCommand] {
        var list: [AppCommand] = [
            AppCommand(title: tr("New sequence …"), keywords: [tr("paste"), "tippen", "fasta"], shortcut: "⇧⌘N") { showNewSequence = true },
            AppCommand(title: tr("Import sequence files …"), keywords: ["import", "genbank", "fasta", "datei"], shortcut: "⌘I") { model.chooseFilesToImport() },
            AppCommand(title: tr("Import table (CSV/TSV) …"), keywords: ["csv", "excel", "tabelle"]) { showTableImport = true },
            AppCommand(title: tr("Import from NCBI …"), keywords: ["genbank", "datenbank", "suchen"]) { showNcbi = true },
            AppCommand(title: tr("New project …"), keywords: ["anlegen"], shortcut: "⌘N") { model.newProject() },
            AppCommand(title: tr("Open project …"), keywords: ["laden"], shortcut: "⌘O") { model.openProject() },
            AppCommand(title: tr("Workflows …"), keywords: ["abfolge", "automatisieren"]) { showWorkflows = true },
            AppCommand(title: tr("Management …"), keywords: ["ordner", "version", "einstellungen", "plugins", "suche"]) { showManage = true },
            AppCommand(title: tr("Open structure …"), keywords: ["pdb", "3d", tr("molecule"), "xyz", "mol"]) { openStructure() },
            AppCommand(title: tr("Show/hide inspector"), keywords: ["notiz", "features", "seitenleiste"]) { showInspector.toggle() },
            AppCommand(title: tr("Show keyboard shortcuts"), keywords: ["hilfe", "shortcuts"], shortcut: "⌘/") { showShortcuts = true },
        ]
        if model.selection != nil {
            list.append(AppCommand(title: tr("Undo"), keywords: ["undo", tr("back")], shortcut: "⌘Z") { model.undo() })
            list.append(AppCommand(title: tr("Redo"), keywords: ["redo"], shortcut: "⇧⌘Z") { model.redo() })
            for (i, t) in DetailView.tabNames.enumerated() {
                list.append(AppCommand(title: tr("Go to: %@", "\(t)"), keywords: ["ansicht", "reiter", "tab"]) { model.tab = i })
            }
        }
        for d in model.documents {
            list.append(AppCommand(title: tr("Open: %@", "\(d.name)"), keywords: [d.docType, "dokument"]) {
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
                        Button(tr("Rename…")) {
                            renameText = doc.name
                            renameTarget = doc
                        }
                        if doc.docType == "sequence" {
                            Button(tr("Import annotations…")) { model.importAnnotations(id: doc.id) }
                        }
                        Menu(tr("Export")) {
                            ForEach(exportFormats(for: doc.docType), id: \.0) { f in
                                Button(f.1) { model.export(id: doc.id, format: f.0, ext: f.2) }
                            }
                        }
                        Button(tr("Delete"), role: .destructive) { model.delete(id: doc.id) }
                    }
            }
        }
        .listStyle(.sidebar)
        .searchable(text: $filterText, placement: .sidebar, prompt: tr("Filter documents"))
        .overlay {
            if model.project == nil {
                EmptyState(
                    icon: "tray", title: tr("No project"),
                    message: tr("Create a new project or open an existing one."),
                    actions: [(tr("New project"), { model.newProject() }), (tr("Open project"), { model.openProject() })]
                )
            } else if model.documents.isEmpty {
                EmptyState(
                    icon: "dna", title: tr("No documents yet"),
                    message: tr("Import sequences or drag files into this window."),
                    actions: [(tr("New sequence"), { showNewSequence = true }), (tr("Import"), { model.chooseFilesToImport() })]
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
        case "sequence": return tr("Sequence")
        case "alignment": return tr("Alignment")
        default: return t
        }
    }

    /// (format name in the core, display name, file extension)
    private func exportFormats(for type: String) -> [(String, String, String)] {
        switch type {
        case "sequence": return [("genbank", "GenBank (.gb)", "gb"), ("embl", "EMBL (.embl)", "embl"), ("fasta", "FASTA (.fasta)", "fasta"), ("gff3", tr("Annotations (.gff3)"), "gff3"), ("svg", tr("Map image (.svg)"), "svg")]
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

    static var tabNames: [String] { [tr("Overview"), tr("Sequence & map"), tr("Digest"), "CRISPR", tr("Cloning"), tr("Primers"), tr("Analysis"), tr("Protein"), tr("Sequencing"), "RNA", tr("Tools"), "NGS"] }
    /// Task groups: name and indices of the tabs.
    private static var groups: [(String, [Int])] { [
        (tr("View"), [0, 1]),
        (tr("Build"), [2, 3, 4, 5]),
        (tr("Check"), [6, 7, 8, 9]),
        (tr("Data"), [10, 11]),
    ] }

    private var group: Int { Self.groups.firstIndex { $0.1.contains(model.tab) } ?? 0 }

    /// Two rows: the task group on top, below it the tabs of that group.
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
        content
            .alert(tr("Save changes to “%@”?", "\(model.selectedName)"),
                   isPresented: Binding(get: { model.pendingLeave != nil }, set: { if !$0 { model.pendingLeave = nil } })) {
                Button(tr("Save")) { model.resolveLeave("save") }
                Button(tr("Discard"), role: .destructive) { model.resolveLeave("discard") }
                Button(tr("Cancel"), role: .cancel) { model.resolveLeave("cancel") }
            } message: {
                Text(tr("You edited the sequence. Discard reverts all changes since it was opened."))
            }
    }

    @ViewBuilder
    private var content: some View {
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
            EmptyState(icon: "dna", title: tr("No document selected"), message: tr("Select a document on the left."))
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
            Text([s.seqType == "PROTEIN" ? tr("Protein") : s.seqType, s.topology == "CIRCULAR" ? tr("circular") : "linear", s.organism]
                    .filter { !$0.isEmpty }.joined(separator: " · "))
                .foregroundStyle(.secondary)
        }
    }

    private func facts(_ s: DocumentSummary) -> some View {
        VStack(spacing: Theme.gap) {
            HStack(spacing: Theme.gap) {
                StatTile(title: tr("Length"), value: "\(s.length) \(s.seqType == "PROTEIN" ? "aa" : "bp")")
                if s.seqType != "PROTEIN" {
                    StatTile(title: tr("GC content"), value: String(format: "%.1f %%", s.gcPercent), tint: Theme.amber)
                }
                StatTile(title: tr("Features"), value: "\(s.featureCount)", tint: Theme.good)
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
            SectionHeader(text: tr("Features"), detail: rows.count == model.features.count ? "\(rows.count)" : tr("%@ of %@", "\(rows.count)", "\(model.features.count)"))
            HStack {
                TextField(tr("Filter by name"), text: $featFilter).textFieldStyle(.roundedBorder).frame(width: 170)
                Picker(tr("Type"), selection: $featType) {
                    Text(tr("All types")).tag("")
                    ForEach(Array(Set(model.features.map { $0.featureType })).sorted(), id: \.self) { Text($0).tag($0) }
                }
                .frame(width: 170)
                Picker(tr("Sort"), selection: $featSort) {
                    Text(tr("Start")).tag("start")
                    Text(tr("End")).tag("end")
                    Text(tr("Length")).tag("length")
                    Text(tr("Name")).tag("name")
                    Text(tr("Type")).tag("type")
                }
                .frame(width: 160)
                Button { featAsc.toggle() } label: { Image(systemName: featAsc ? "arrow.up" : "arrow.down") }.accessibilityLabel(featAsc ? tr("Sorted ascending") : tr("Sorted descending"))
                    .help(featAsc ? tr("Ascending") : tr("Descending"))
                Spacer()
                Button(tr("As CSV …")) { exportFeatureCsv() }
            }
            Table(rows) {
                TableColumn(tr("Type")) { f in Text(f.info.featureType) }
                TableColumn(tr("Name")) { f in Text(f.info.name) }
                TableColumn(tr("Range")) { f in Text(verbatim: "\(f.info.start + 1)–\(f.info.end)") }
                TableColumn(tr("Length")) { f in Text("\(f.info.end - f.info.start)") }.width(70)
                TableColumn(tr("Strand")) { f in Text(strand(f.info.strand)) }
                TableColumn("") { f in
                    Button { model.removeFeature(index: f.index) } label: { Image(systemName: "trash") }
                        .accessibilityLabel(tr("Delete feature"))
                        .buttonStyle(.borderless)
                        .help(tr("Delete feature"))
                }.width(30)
            }
            .frame(minHeight: 120, maxHeight: 240)
        }
    }

    /// Save all features of the document as CSV.
    private func exportFeatureCsv() {
        guard let project = model.project, let id = model.selection, let url = FilePick.save(tr("Save features as CSV"), name: "features.csv") else { return }
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
            SectionHeader(text: tr("Whole sequence"))
            Card(padding: 12) {
                let full = String(model.fullBases.prefix(200_000))
                Text(s.seqType == "PROTEIN" || full.count > 30_000 ? AttributedString(grouped(full)) : coloredBases(grouped(full)))
                    .font(Theme.mono(13))
                    .textSelection(.enabled)
            }
            if s.length > 200_000 {
                Text(tr("… %@ more bases (not shown)", "\(s.length - 200_000)"))
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

    /// Lines of 60 characters, in groups of 10 (preview; the real sequence view follows in phase 1).
    private func grouped(_ bases: String) -> String {
        var lines: [String] = []
        var i = bases.startIndex
        while i < bases.endIndex {
            let end = bases.index(i, offsetBy: 54, limitedBy: bases.endIndex) ?? bases.endIndex
            let line = bases[i..<end]
            var parts: [String] = []
            var j = line.startIndex
            while j < line.endIndex {
                let e = line.index(j, offsetBy: 9, limitedBy: line.endIndex) ?? line.endIndex
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

/// Actions for several selected documents: align or assemble.
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
            Text(tr("%@ documents selected", "\(model.multiSelection.count)")).font(.title2.bold())
            ReadConsensusSection(model: model)
            ConcatSection(model: model)
            CompareAnnotationsSection(model: model)
            PairedReadsSection(model: model)
            LengthHistogramSection(model: model)
            GroupBox(tr("Align")) {
                HStack {
                    Picker(tr("Algorithm"), selection: $aligner) {
                        Text(tr("Built-in")).tag("builtin")
                        Text("MAFFT").tag("mafft")
                        Text("MUSCLE").tag("muscle")
                        Text("Clustal Omega").tag("clustalo")
                    }
                    .frame(width: 240)
                    Spacer()
                    Button(tr("Align")) { model.alignSelected(backend: aligner) }
                }
                .padding(6)
            }
            GroupBox(tr("BLAST (local): first selected sequence against the others")) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Picker(tr("Program"), selection: $blastProgram) {
                            Text("blastn (DNA)").tag("blastn")
                            Text("blastp (Protein)").tag("blastp")
                        }
                        .frame(width: 220)
                        Button(tr("Search")) { runBlast() }
                        if let blastMessage { Text(blastMessage).foregroundStyle(Theme.danger) }
                    }
                    if !blastHits.isEmpty {
                        Table(blastHits) {
                            TableColumn(tr("Hits")) { h in Text(h.dto.subject) }
                            TableColumn(tr("Identity")) { h in Text(String(format: "%.1f %%", h.dto.identityPercent)) }.width(80)
                            TableColumn(tr("Length")) { h in Text("\(h.dto.alignmentLength)") }.width(60)
                            TableColumn("Query") { h in Text("\(h.dto.queryStart)–\(h.dto.queryEnd)") }.width(90)
                            TableColumn("Subject") { h in Text("\(h.dto.subjectStart)–\(h.dto.subjectEnd)" + (h.dto.reverseStrand ? " (−)" : "")) }.width(110)
                            TableColumn(tr("E-value")) { h in Text(String(format: "%.1e", h.dto.evalue)) }.width(80)
                            TableColumn("Bit") { h in Text(String(format: "%.1f", h.dto.bitScore)) }.width(60)
                        }
                        .frame(minHeight: 120, maxHeight: 220)
                    }
                }
                .padding(6)
            }
            GroupBox(tr("Reads: build contigs or map onto the first sequence")) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Stepper(tr("Minimum overlap %@ bp", "\(minOverlap)"), value: $minOverlap, in: 5...200)
                            .frame(width: 280)
                        Button(tr("Build contigs")) { model.assembleReadsSelected(minOverlap: minOverlap) }
                    }
                    HStack {
                        Stepper(tr("Mismatches ≤ %@", "\(maxMismatch)"), value: $maxMismatch, in: 0...20)
                            .frame(width: 280)
                        Button(tr("Map to reference")) { mapRows = model.mapReadsSelected(maxMismatches: maxMismatch) }
                    }
                    if !mapRows.isEmpty {
                        ForEach(Array(mapRows.enumerated()), id: \.offset) { _, r in
                            Text(r.mapped
                                 ? tr("%@: position %@%@ · %@ mismatch(es)", "\(r.name)", "\(r.position + 1)", "\(r.reverse ? " (−)" : "")", "\(r.mismatches)")
                                 : tr("%@: not mapped", "\(r.name)"))
                                .font(Theme.label(13))
                        }
                    }
                }
                .padding(6)
            }
            GroupBox(tr("Assemble (order as in the list)")) {
                VStack(alignment: .leading, spacing: 8) {
                    Toggle(tr("Circular construct"), isOn: $circular)
                    HStack {
                        Button("Gibson") { model.assembleSelected(kind: "gibson", circular: circular) }
                        Button("In-Fusion") { model.assembleSelected(kind: "infusion", circular: circular) }
                        Button("SLIC") { model.assembleSelected(kind: "slic", circular: circular) }
                    }
                    HStack {
                        Picker(tr("Enzyme"), selection: $enzyme) {
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
            if hits.isEmpty { blastMessage = tr("No hits.") }
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
