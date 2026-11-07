import AppKit
import Foundation
import SnapiousFFI
import SwiftUI

/// An amino acid via its three codon bases `[lo, hi]` (0-based).
struct CodonMark {
    let lo: Int
    let hi: Int
    let letter: String
    let isStart: Bool
    let isStop: Bool
    /// Row below the bases (0 = first amino acid row).
    var row: Int = 0
}

/// A cut site of an enzyme (start of the recognition sequence, 0-based).
struct EnzymeSite: Hashable {
    let enzyme: String
    let position: Int
}

/// App state: open project, document list, selection and error messages.
/// All domain logic lives in the Rust core; this layer only calls it and displays results.
@MainActor
final class AppModel: ObservableObject {
    @Published var project: Project?
    @Published var documents: [DocumentInfoDto] = []
    @Published var selection: String? {
        didSet {
            if !guardBypass, oldValue != selection, let old = oldValue, isDirty(old) {
                pendingLeave = .selection(selection)
                guardBypass = true
                selection = old
                multiSelection = [old]
                guardBypass = false
                return
            }
            if let id = selection, baselines[id] == nil { baselines[id] = undoCount(of: id) }
            let wanted: Set<String> = selection.map { [$0] } ?? []
            if multiSelection.count <= 1 && multiSelection != wanted { multiSelection = wanted }
            coverage = []
            loadSelection()
        }
    }
    /// Multi-selection in the document list (e.g. for an alignment); with exactly one entry this is also `selection`.
    @Published var multiSelection: Set<String> = [] {
        didSet {
            if multiSelection.count == 1, let only = multiSelection.first, selection != only {
                selection = only
            } else if multiSelection.isEmpty, selection != nil {
                selection = nil
            }
        }
    }
    // MARK: Unsaved changes
    enum Leave { case tab(Int); case selection(String?) }
    /// Target the user wanted to navigate to while changes to the sequence are open (triggers the confirmation prompt).
    @Published var pendingLeave: Leave?
    var guardBypass = false
    /// Number of undo steps on opening or at the last "Save", per document.
    var baselines: [String: Int] = [:]
    /// Steps per user action (insert + annotation = 2), so that undo reverts the action as a whole.
    /// Multi-step actions: (number of steps afterwards, number of steps of the action).
    private var undoGroups: [String: [(after: Int, steps: Int)]] = [:]
    private var redoGroups: [String: [(after: Int, steps: Int)]] = [:]
    static weak var current: AppModel?

    func undoCount(of id: String) -> Int {
        guard let project, let s = try? project.editState(id: id), s.count == 2 else { return 0 }
        return Int(s[0])
    }

    func isDirty(_ id: String) -> Bool {
        guard project != nil else { return false }
        return undoCount(of: id) != (baselines[id] ?? undoCount(of: id))
    }

    var currentDirty: Bool { selection.map { isDirty($0) } ?? false }
    var selectedName: String { summary?.name ?? tr("Sequence") }

    /// Changes are already stored in the project; "Save" adopts them as the new baseline.
    func commitEdits() {
        if let id = selection { baselines[id] = undoCount(of: id); undoGroups[id] = []; redoGroups[id] = [] }
    }

    /// Discards all changes since opening or the last save.
    func discardEdits() {
        guard let project, let id = selection else { return }
        let base = baselines[id] ?? 0
        while undoCount(of: id) > base, (try? project.editUndo(id: id)) == true {}
        undoGroups[id] = []; redoGroups[id] = []
        reloadCurrent()
    }

    /// Answer to the prompt: `save`, `discard` or `cancel`.
    func resolveLeave(_ choice: String) {
        guard let target = pendingLeave else { return }
        pendingLeave = nil
        if choice == "cancel" { return }
        if choice == "discard" { discardEdits() } else { commitEdits() }
        guardBypass = true
        switch target {
        case .tab(let t): tab = t
        case .selection(let id): selection = id; multiSelection = id.map { [$0] } ?? []
        }
        guardBypass = false
    }

    /// The selected alignment (if the selected document is one).
    @Published var alignment: AlignmentDto?
    @Published var summary: DocumentSummary?
    @Published var features: [FeatureInfo] = []
    /// Read depth of the selected sequence (averaged to at most 1000 values) from a linked SAM/BAM file.
    @Published var coverage: [Double] = []
    @Published var coverageLabel = ""
    @Published var preview: String = ""
    @Published var lanes: [PlacedFeatureDto] = []
    /// Translations of the CDS features for the sequence view: amino acids per codon, with the span `[lo, hi]` of the codon bases.
    @Published var codons: [CodonMark] = []
    /// Displayed reading frames: 1, 2, 3 forward, −1, −2, −3 reverse strand.
    @Published var aaFrames: [Int] = []
    /// Show amino acids of the CDS features.
    @Published var showCdsTranslation = true
    /// Enzymes of the default database (for the digest view).
    @Published var enzymes: [EnzymeInfoDto] = []
    /// Single-cutter enzymes with their position (one row per cut site), sorted by position.
    @Published var uniqueSites: [EnzymeSite] = []
    /// All cut sites of all enzymes in the default database (basis for the map view filter).
    @Published var allSites: [EnzymeSite] = []
    /// Enzyme filter of the map: `unique`, `double`, `all`, `custom` or `set:<name>`.
    @Published var siteFilter: String = UserDefaults.standard.string(forKey: "siteFilter") ?? "set:Häufige Klonierung" {
        didSet { UserDefaults.standard.set(siteFilter, forKey: "siteFilter") }
    }
    @Published var customEnzymes: Set<String> = Set(UserDefaults.standard.stringArray(forKey: "customEnzymes") ?? []) {
        didSet { UserDefaults.standard.set(Array(customEnzymes), forKey: "customEnzymes") }
    }
    /// Map display: `auto` (by topology), `ring` or `linear`.
    @Published var mapStyle = "auto"
    /// Zoom (1 = whole sequence) and left edge of the visible section in bases.
    @Published var zoom = 1.0
    @Published var viewStart = 0.0
    /// Rotation of the whole ring (position at the top) as long as it is not zoomed in.
    @Published var ringRotation = 0.0
    /// Whole sequence for the sequence view (up to `fullLimit` bases).
    @Published var fullBases: String = "" {
        didSet { fullBytes = Array(fullBases.utf8) }
    }
    /// The same bases as bytes (fast access while drawing).
    private(set) var fullBytes: [UInt8] = []
    /// Selection in the sequence view: positions between bases, `nil` = nothing selected.
    @Published var selStart: Int?
    @Published var selEnd: Int?
    @Published var selStats: SelectionStatsDto?
    @Published var errorMessage: String?
    /// Selected tab of the detail view (0 = overview … 12 = NGS).
    /// Interface language: `en` or `de` (setting, applies immediately).
    @Published var language: String = L10n.language {
        didSet { L10n.language = language }
    }
    @Published var tab = 0 {
        didSet {
            if !guardBypass, oldValue == 1, tab != 1, let id = selection, isDirty(id) {
                pendingLeave = .tab(tab)
                guardBypass = true
                tab = oldValue
                guardBypass = false
            }
        }
    }
    /// Short feedback at the bottom of the window (disappears after a few seconds).
    @Published var notice: String?
    private var noticeToken = 0
    /// Recently opened projects (newest first), stored in the user settings.
    @Published var recentProjects: [String] = (UserDefaults.standard.stringArray(forKey: "recentProjects") ?? []).filter { FileManager.default.fileExists(atPath: $0) }
    /// Notes from the last assembly (Gibson/Golden Gate), `nil` if none.
    @Published var assemblyNotice: String?

    private static let lastProjectKey = "lastProjectPath"
    /// Number of bases shown in the preview.
    static let previewLength: UInt64 = 600
    static let fullLimit: UInt64 = 2_000_000

    init() {
        Self.current = self
        if let path = UserDefaults.standard.string(forKey: Self.lastProjectKey),
           FileManager.default.fileExists(atPath: path) {
            open(path: path)
        }
    }

    var projectTitle: String {
        guard let p = project else { return tr("No project open") }
        return URL(fileURLWithPath: p.path()).deletingPathExtension().lastPathComponent
    }

    // MARK: Project

    func open(path: String) {
        run {
            let p = try Project.open(path: path)
            self.project = p
            self.enzymes = p.enzymeList()
            UserDefaults.standard.set(path, forKey: Self.lastProjectKey)
            self.recentProjects = recentAdd(list: self.recentProjects, path: path, max: 8)
            UserDefaults.standard.set(self.recentProjects, forKey: "recentProjects")
            self.selection = nil
            try self.refresh()
        }
    }

    func newProject() {
        let panel = NSSavePanel()
        panel.title = tr("New Snapious project")
        panel.nameFieldStringValue = "Projekt.snapious"
        panel.canCreateDirectories = true
        // Always start in the Documents folder (otherwise the panel remembers the last used folder)
        panel.directoryURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        guard panel.runModal() == .OK, let url = panel.url else { return }
        open(path: url.path)
    }

    func openProject() {
        let panel = NSOpenPanel()
        panel.title = tr("Open Snapious project")
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        open(path: url.path)
    }

    // MARK: Documents

    func importFiles(_ urls: [URL]) {
        guard let project else {
            errorMessage = tr("Please create or open a project first.")
            return
        }
        run {
            var last: String?
            for url in urls {
                let neu = try project.importFile(path: url.path)
                last = neu.last?.id
            }
            try self.refresh()
            if let last { self.selection = last }
            self.announce(urls.count == 1 ? tr("“%@” imported", "\(urls[0].lastPathComponent)") : tr("%@ files imported", "\(urls.count)"))
        }
    }

    func chooseFilesToImport() {
        let panel = NSOpenPanel()
        panel.title = tr("Import (FASTA, FASTQ, GenBank, EMBL, SnapGene, PIR, GCG, MEGA, NEXUS, Clustal, PHYLIP)")
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK else { return }
        importFiles(panel.urls)
    }

    /// Saves a document to a file via a dialog (`format` as in `exportDocument`, `ext` is the file extension).
    func export(id: String, format: String, ext: String) {
        guard let project, let info = documents.first(where: { $0.id == id }) else { return }
        let panel = NSSavePanel()
        panel.title = tr("Export as %@", "\(format)")
        panel.nameFieldStringValue = "\(info.name).\(ext)"
        panel.directoryURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        guard panel.runModal() == .OK, let url = panel.url else { return }
        run { try project.exportDocument(id: id, path: url.path, format: format) }
    }

    /// Reads annotations (GFF3 or BED) from a file and adds them to the document as features.
    func importAnnotations(id: String) {
        guard let project else { return }
        let panel = NSOpenPanel()
        panel.title = tr("Import annotations (GFF3 or BED)")
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        run {
            let n = try project.importAnnotations(id: id, path: url.path)
            if self.selection == id { self.reloadCurrent() }
            self.assemblyNotice = tr("%@ annotation(s) imported", "\(n)")
        }
    }

    func delete(id: String) {
        guard let project else { return }
        run {
            try project.deleteDocument(id: id)
            if self.selection == id { self.selection = nil }
            try self.refresh()
        }
    }

    func rename(id: String, to name: String) {
        guard let project, !name.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        run {
            try project.renameDocument(id: id, name: name)
            try self.refresh()
            self.loadSelection()
        }
    }

    /// Reload the list and select a document (e.g. after saving a construct).
    func reload(selecting id: String?) {
        run {
            try self.refresh()
            if let id { self.selection = id }
        }
    }

    func refresh() throws {
        documents = try project?.documents() ?? []
    }

    /// Kind of the selected document (`sequence`, `alignment`) or `nil`.
    var selectedType: String? {
        documents.first { $0.id == selection }?.docType
    }

    /// Amino acids of all CDS features (only for short documents, to keep the view fast).
    private func loadCodons(project: Project, id: String) -> [CodonMark] {
        guard (summary?.length ?? 0) <= 500_000, summary?.seqType != "PROTEIN" else { return [] }
        var marks: [CodonMark] = []
        var row = 0
        if showCdsTranslation {
            var any = false
            for (index, f) in features.enumerated() where f.featureType.uppercased() == "CDS" {
                guard let cells = try? project.translateFeature(id: id, featureIndex: UInt64(index)) else { continue }
                any = true
                let reverse = f.strand == "REVERSE"
                for c in cells {
                    let bp = Int(c.basePosition)
                    let lo = reverse ? max(0, bp - 2) : bp
                    marks.append(CodonMark(lo: lo, hi: lo + 2, letter: c.aminoAcid, isStart: c.isStart, isStop: c.isStop, row: row))
                }
            }
            if any { row += 1 }
        }
        for frame in aaFrames {
            if let cells = try? project.translateFrame(id: id, frame: Int32(frame)) {
                for c in cells {
                    let lo = Int(c.basePosition)
                    marks.append(CodonMark(lo: lo, hi: lo + 2, letter: c.aminoAcid, isStart: c.isStart, isStop: c.isStop, row: row))
                }
            }
            row += 1
        }
        return marks
    }

    /// Selects the displayed reading frames and reloads the amino acids.
    func setFrames(_ frames: [Int], cds: Bool) {
        aaFrames = frames
        showCdsTranslation = cds
        guard let project, let id = selection, selectedType == "sequence" else { return }
        codons = loadCodons(project: project, id: id)
    }

    /// Creates sequences from pasted text and selects the first one. `true` on success.
    func createSequence(name: String, text: String, kind: String, circular: Bool) -> Bool {
        guard let project else { return false }
        var ok = false
        run {
            let infos = try project.createSequence(name: name, text: text, kind: kind, circular: circular)
            try self.refresh()
            self.multiSelection = Set(infos.prefix(1).map { $0.id })
            self.selection = infos.first?.id
            self.announce(infos.count == 1 ? tr("Sequence “%@” created", "\(infos[0].name)") : tr("%@ sequences created", "\(infos.count)"))
            ok = true
        }
        return ok
    }

    /// Assembles the fragments in the given order (`gibson`, `infusion`, `slic`, `goldengate`);
    /// returns the assembly notes (`nil` on error; the error is then in `errorMessage`).
    func assembleFragments(ids: [String], kind: String, circular: Bool, enzyme: String, name: String) -> [String]? {
        guard let project, ids.count >= 2 else { return nil }
        var warnings: [String]?
        run {
            let result: AssemblyDto
            if kind == "goldengate" {
                result = try project.assembleGoldenGate(ids: ids, enzyme: enzyme, circular: circular)
            } else {
                result = try project.assembleOverlap(ids: ids, method: kind, circular: circular)
            }
            if !name.trimmingCharacters(in: .whitespaces).isEmpty {
                try project.renameDocument(id: result.document.id, name: name.trimmingCharacters(in: .whitespaces))
            }
            try self.refresh()
            self.multiSelection = [result.document.id]
            self.selection = result.document.id
            self.announce(tr("Construct “%@” created", "\(result.document.name)"))
            warnings = result.warnings
        }
        return warnings
    }

    private func loadSelection() {
        alignment = nil
        if let project, let id = selection, selectedType == "alignment" {
            summary = nil
            features = []
            preview = ""
            lanes = []
            uniqueSites = []
            fullBases = ""
            clearSelection()
            do { alignment = try project.alignment(id: id) } catch { errorMessage = message(for: error) }
            return
        }
        guard let project, let id = selection else {
            summary = nil
            features = []
            preview = ""
            lanes = []
            uniqueSites = []
            fullBases = ""
            clearSelection()
            return
        }
        do {
            let previous = summary?.id
            summary = try project.summary(id: id)
            if previous != summary?.id { zoom = 1; viewStart = 0; mapStyle = "auto" }
            features = try project.features(id: id)
            preview = try project.bases(id: id, start: 0, end: Self.previewLength)
            lanes = try project.featureLanes(id: id).filter { $0.featureType.lowercased() != "source" }
            codons = loadCodons(project: project, id: id)
            uniqueSites = try project.enzymeSites(id: id, filter: "unique")
                .flatMap { e in e.positions.map { EnzymeSite(enzyme: e.enzyme, position: Int($0)) } }
                .sorted { $0.position < $1.position }
            if setMembers.isEmpty { refreshEnzymeSets() }
            allSites = try project.enzymeSites(id: id, filter: "all")
                .flatMap { e in e.positions.map { EnzymeSite(enzyme: e.enzyme, position: Int($0)) } }
                .sorted { $0.position < $1.position }
            fullBases = try project.bases(id: id, start: 0, end: Self.fullLimit)
            clearSelection()
        } catch {
            summary = nil
            features = []
            preview = ""
            errorMessage = error.localizedDescription
        }
    }

    // MARK: Alignment and tree

    /// Aligns the sequences selected in the list and selects the new alignment.
    func alignSelected(backend: String = "builtin") {
        guard let project, multiSelection.count >= 2 else { return }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        run {
            let info = try project.alignDocuments(ids: ids, name: tr("Alignment"), backend: backend)
            try self.refresh()
            self.selection = info.id
        }
    }

    /// Assembles the sequences selected in the list: `kind` is `gibson`, `infusion`, `slic` or `goldengate` (the latter with `enzyme`).
    /// Reports assembly notes via `errorMessage`-like text in `assemblyNotice`.
    func assembleSelected(kind: String, circular: Bool, enzyme: String = "BsaI") {
        guard let project, multiSelection.count >= 2 else { return }
        // Order as in the list
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        run {
            let result: AssemblyDto
            if kind == "goldengate" {
                result = try project.assembleGoldenGate(ids: ids, enzyme: enzyme, circular: circular)
            } else {
                result = try project.assembleOverlap(ids: ids, method: kind, circular: circular)
            }
            try self.refresh()
            self.selection = result.document.id
            self.assemblyNotice = result.warnings.isEmpty ? nil : result.warnings.joined(separator: "\n")
        }
    }

    /// Loads the read depth from a SAM/BAM file for the selected sequence and shows it below the sequence view.
    func showCoverage(path: String, label: String) {
        guard let project, let id = selection else { return }
        // Large SAM/BAM files take time: read in the background so the UI does not freeze
        Task.detached {
            do {
                let depth = try project.coverageDepth(referenceId: id, alignmentPath: path, bins: 1000)
                await MainActor.run {
                    guard self.selection == id else { return }
                    self.coverage = depth
                    self.coverageLabel = label
                }
            } catch {
                await MainActor.run {
                    self.coverage = []
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }

    /// Assembles the selected sequences into contigs and selects the first.
    func assembleReadsSelected(minOverlap: Int) {
        guard let project, multiSelection.count >= 2 else { return }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        run {
            let infos = try project.assembleReads(ids: ids, minOverlap: UInt64(minOverlap))
            try self.refresh()
            if let first = infos.first { self.selection = first.id }
            self.assemblyNotice = tr("%@ contig(s) from %@ reads", "\(infos.count)", "\(ids.count)")
        }
    }

    /// Maps the other selected sequences onto the first selected one (reference).
    func mapReadsSelected(maxMismatches: Int) -> [ReadMapDto] {
        guard let project else { return [] }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        guard let ref = ids.first, ids.count >= 2 else { return [] }
        do {
            return try project.mapReadsTo(referenceId: ref, readIds: Array(ids.dropFirst()), maxMismatches: UInt64(maxMismatches))
        } catch {
            self.assemblyNotice = error.localizedDescription
            return []
        }
    }

    /// New alignment without columns in which at least the fraction `threshold` of rows has a gap.
    func stripSelectedAlignment(threshold: Double) {
        guard let project, let id = selection, selectedType == "alignment" else { return }
        run {
            let info = try project.alignmentStrip(id: id, threshold: threshold)
            try self.refresh()
            self.selection = info.id
        }
    }

    /// Consensus sequence of the selected alignment as a new document.
    func consensusOfSelectedAlignment() {
        guard let project, let id = selection, selectedType == "alignment" else { return }
        run {
            let info = try project.alignmentConsensus(id: id)
            try self.refresh()
            self.selection = info.id
        }
    }

    /// Consensus from the sequencing reads selected in the list (alignment and consensus sequence are created).
    @discardableResult
    func consensusOfSelectedReads(minQuality: Int) -> Bool {
        guard let project else { return false }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        var ok = false
        run {
            let info = try project.consensusFromReads(ids: ids, minQuality: UInt8(max(0, min(60, minQuality))), name: "")
            try self.refresh()
            self.multiSelection = [info.id]
            self.selection = info.id
            self.announce(tr("Consensus from %@ reads created (alignment of the reads too)", "\(ids.count)"))
            ok = true
        }
        return ok
    }

    // MARK: Edit features

    /// Adds the selection as a feature.
    func addFeatureFromSelection(name: String, type: String, strand: String) {
        guard let project, let id = selection, let a = selStart, let b = selEnd, a != b else { return }
        let info = FeatureInfo(featureType: type, name: name, start: UInt64(min(a, b)), end: UInt64(max(a, b)), strand: strand)
        run {
            try project.addFeatures(id: id, features: [info])
            self.reloadCurrent()
        }
    }

    /// Removes the feature with this index from the document.
    func removeFeature(index: Int) {
        guard let project, let id = selection else { return }
        run {
            try project.removeFeature(id: id, index: UInt64(index))
            self.reloadCurrent()
        }
    }

    // MARK: Search

    @Published var motifHits: [MotifHitDto] = []
    @Published var motifIndex = 0
    @Published var motifMessage: String?

    /// Searches for a motif (IUPAC) in the selected document and selects the first hit.
    func findMotif(_ motif: String) {
        motifMessage = nil
        guard let project, let id = selection, !motif.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        do {
            motifHits = try project.findMotifHits(id: id, motif: motif)
            motifIndex = -1
            if motifHits.isEmpty { motifMessage = tr("Not found") } else { nextMotif(1) }
        } catch {
            motifHits = []
            motifMessage = error.localizedDescription
        }
    }

    /// Jumps to the next (+1) or previous (−1) hit and selects it.
    func nextMotif(_ step: Int) {
        guard !motifHits.isEmpty else { return }
        motifIndex = ((motifIndex + step) % motifHits.count + motifHits.count) % motifHits.count
        let h = motifHits[motifIndex]
        select(from: Int(h.start), to: Int(min(h.end, UInt64(fullBytes.count))))
        motifMessage = tr("Hit %@ of %@", "\(motifIndex + 1)", "\(motifHits.count)") + (h.reverse ? tr(" (reverse strand)") : "")
    }

    // MARK: Map: enzyme filter and zoom

    /// Number of cut sites per enzyme in the current sequence.
    var siteCounts: [String: Int] {
        Dictionary(allSites.map { ($0.enzyme, 1) }, uniquingKeysWith: +)
    }

    /// The cut sites selected according to `siteFilter`.
    var visibleSites: [EnzymeSite] {
        let counts = siteCounts
        switch siteFilter {
        case "all": return allSites
        case "double": return allSites.filter { counts[$0.enzyme] == 2 }
        case "custom": return allSites.filter { customEnzymes.contains($0.enzyme) }
        case let f where f.hasPrefix("set:"):
            let name = String(f.dropFirst(4))
            let members = setMembers[name] ?? []
            return allSites.filter { members.contains($0.enzyme) }
        default: return allSites.filter { counts[$0.enzyme] == 1 }
        }
    }

    /// Members of the enzyme sets (name → enzymes), so drawing does not read the file every time.
    var setMembers: [String: Set<String>] = [:]

    func refreshEnzymeSets() {
        setMembers = Dictionary(enzymeSets().map { ($0.name, Set($0.enzymes)) }, uniquingKeysWith: { a, _ in a })
        objectWillChange.send()
    }

    /// Length of the document in bases.
    var docLength: Int { Int(summary?.length ?? 0) }

    /// Visible span in bases at the current zoom.
    var viewSpan: Double { max(1, Double(docLength) / zoom) }

    /// Width of the map view in points (set by the view).
    var viewWidth: Double = 1000
    /// Font size of the linear sequence view as a factor (1 = normal).
    @Published var textScale = 1.0

    /// Maximum zoom: in linear mode the level at which bases appear as text (nine points per base); in the ring up to eight bases.
    var maxZoom: Double {
        let len = Double(max(1, docLength))
        let ringMode = mapStyle == "ring" || (mapStyle == "auto" && summary?.topology == "CIRCULAR")
        if ringMode || summary?.seqType == "PROTEIN" { return max(1, len / 8) }
        return max(1, len / 8)
    }

    func setZoom(_ z: Double) {
        let len = Double(max(1, docLength))
        var centre = zoom <= 1.001 && mapStyle != "linear" ? ringRotation : viewStart + viewSpan / 2
        if let a = selStart, let b = selEnd { centre = Double(a + b) / 2 }
        zoom = min(max(1, z), maxZoom)
        viewStart = clampStart(centre - viewSpan / 2)
    }

    /// In the plasmid ring the section wraps across the seam (rotate endlessly); otherwise it stays within the sequence.
    var ringMode: Bool { summary?.seqType != "PROTEIN" && (mapStyle == "ring" || (mapStyle == "auto" && summary?.topology == "CIRCULAR")) }

    func clampStart(_ v: Double) -> Double {
        let len = Double(max(1, docLength))
        if ringMode {
            var w = v.truncatingRemainder(dividingBy: len)
            if w < 0 { w += len }
            return w
        }
        return min(max(0, v), max(0, len - viewSpan))
    }

    func panView(to start: Double) {
        viewStart = clampStart(start)
    }

    // MARK: Edit

    /// Replaces the selection (or inserts at the cursor) with `bases`; empty text deletes the selection.
    func replaceSelection(with bases: String) {
        guard let project, let id = selection else { return }
        let allowed = summary?.seqType == "PROTEIN" ? "ACDEFGHIKLMNPQRSTVWYXBZUOJ*" : "ACGTUNRYSWKMBDHV"
        let cleaned = bases.uppercased().filter { allowed.contains($0) }
        guard let a = selStart, let b = selEnd else { return }
        let lo = min(a, b), hi = max(a, b)
        if cleaned.isEmpty && lo == hi { return }
        // Continuing to type directly after an automatically created "New" annotation extends it
        let before = undoCount(of: id)
        let joined = features.first { $0.name == Self.newName && $0.featureType == "misc_feature" && Int($0.end) == lo && lo == hi }
        run {
            if lo == hi {
                try project.editInsert(id: id, position: UInt64(lo), bases: cleaned)
            } else if cleaned.isEmpty {
                try project.editDelete(id: id, start: UInt64(lo), end: UInt64(hi))
            } else {
                try project.editReplace(id: id, start: UInt64(lo), end: UInt64(hi), bases: cleaned)
            }
            if !cleaned.isEmpty, self.summary?.seqType != "PROTEIN" {
                // Mark the new part as an annotation (one undo step together with the insertion)
                var start = lo
                if let j = joined, let idx = try? project.features(id: id).firstIndex(where: { $0.name == Self.newName && Int($0.end) == lo }) {
                    start = Int(j.start)
                    try project.removeFeature(id: id, index: UInt64(idx))
                }
                try project.addFeatures(id: id, features: [FeatureInfo(featureType: "misc_feature", name: Self.newName, start: UInt64(start), end: UInt64(lo + cleaned.count), strand: "NONE")])
            }
            let after = self.undoCount(of: id)
            if after - before > 1 { self.undoGroups[id, default: []].append((after, after - before)) }
            self.redoGroups[id] = []
            self.reloadCurrent()
            let caret = lo + cleaned.count
            self.selStart = caret
            self.selEnd = caret
            self.selStats = nil
        }
    }

    /// Deletes the character before the cursor (Backspace) or the selection.
    func backspace() {
        guard let project, let id = selection, let a = selStart, let b = selEnd else { return }
        let lo = min(a, b), hi = max(a, b)
        if lo == hi {
            guard lo > 0 else { return }
            run {
                try project.editDelete(id: id, start: UInt64(lo - 1), end: UInt64(lo))
                self.reloadCurrent()
                self.selStart = lo - 1
                self.selEnd = lo - 1
            }
        } else {
            replaceSelection(with: "")
        }
    }

    /// Moves the cursor by `delta` bases (clears the selection).
    func moveCaret(by delta: Int) {
        let total = fullBytes.count
        let base = (delta < 0 ? min(selStart ?? 0, selEnd ?? 0) : max(selStart ?? 0, selEnd ?? 0))
        let target = max(0, min(total, base + (hasSelection ? 0 : delta)))
        selStart = target
        selEnd = target
        selStats = nil
    }

    static let newName = "Neu"

    // MARK: Annotations

    /// Annotation types hidden in map and sequence.
    /// Translate selected bases: `off`, `forward` (from the first selected base) or `reverse` (reverse strand of the selection).
    @Published var translateSelection = "off"
    @Published var hiddenTypes: Set<String> = []
    /// Sidebar with the annotation list.
    @Published var showAnnotations = true
    @Published var selectedFeature: Int?
    /// Position the linear view should scroll to.
    @Published var scrollBP = 0

    var visibleLanes: [PlacedFeatureDto] { lanes.filter { !hiddenTypes.contains($0.featureType) } }

    /// Selects an annotation: marks the range and scrolls there or moves the ring section.
    func focusFeature(_ index: Int, scroll: Bool = true) {
        guard features.indices.contains(index) else { return }
        let f = features[index]
        selectedFeature = index
        select(from: Int(f.start), to: Int(f.end))
        let mid = Double(f.start + f.end) / 2
        guard scroll else { return }
        scrollBP = Int(mid)
        if zoom > 1.001 { panView(to: mid - viewSpan / 2) }
    }

    /// Create a new annotation (one undo step).
    func addAnnotation(_ info: FeatureInfo) {
        guard let project, let id = selection else { return }
        run {
            try project.addFeatures(id: id, features: [info])
            self.reloadCurrent()
            self.selectedFeature = self.features.firstIndex { $0.name == info.name && $0.start == info.start && $0.end == info.end }
        }
    }

    /// Edit an annotation (remove + recreate, as one undo step).
    func updateAnnotation(index: Int, to info: FeatureInfo) {
        guard let project, let id = selection else { return }
        let before = undoCount(of: id)
        run {
            try project.removeFeature(id: id, index: UInt64(index))
            try project.addFeatures(id: id, features: [info])
            let after = self.undoCount(of: id)
            if after - before > 1 { self.undoGroups[id, default: []].append((after, after - before)) }
            self.redoGroups[id] = []
            self.reloadCurrent()
            self.selectedFeature = self.features.firstIndex { $0.name == info.name && $0.start == info.start && $0.end == info.end }
        }
    }

    func undo() {
        guard let id = selection else { return }
        let c = undoCount(of: id)
        var n = 1
        if let g = undoGroups[id]?.last, g.after == c { n = g.steps; undoGroups[id]?.removeLast(); redoGroups[id, default: []].append(g) }
        step { p, i in var any = false; for _ in 0..<n { any = try p.editUndo(id: i) || any }; return any }
    }

    func redo() {
        guard let id = selection else { return }
        var n = 1
        if let g = redoGroups[id]?.last, undoCount(of: id) == g.after - g.steps { n = g.steps; redoGroups[id]?.removeLast(); undoGroups[id, default: []].append(g) }
        step { p, i in var any = false; for _ in 0..<n { any = try p.editRedo(id: i) || any }; return any }
    }

    /// Text of the selection (for copy/cut via keyboard).
    func selectionString() -> String? {
        guard let a = selStart, let b = selEnd, a != b else { return nil }
        let lo = max(0, min(a, b)), hi = min(fullBytes.count, max(a, b))
        guard lo < hi else { return nil }
        return String(decoding: fullBytes[lo..<hi], as: UTF8.self)
    }

    /// Delete key: deletes the selection or the base after the cursor.
    func forwardDelete() {
        guard let a = selStart, let b = selEnd else { return }
        if a != b { replaceSelection(with: ""); return }
        guard a < fullBytes.count else { return }
        selStart = a; selEnd = a + 1
        replaceSelection(with: "")
    }

    private func step(_ action: (Project, String) throws -> Bool) {
        guard let project, let id = selection else { return }
        run {
            if try action(project, id) {
                self.reloadCurrent()
                self.clearSelection()
            }
        }
    }

    /// Number of possible undo steps and whether redo is possible.
    var undoState: (undo: Int, redo: Bool) {
        guard let project, let id = selection, let s = try? project.editState(id: id), s.count == 2 else { return (0, false) }
        return (Int(s[0]), s[1] != 0)
    }

    /// Reload document data, keeping the selection.
    func reloadCurrent() {
        let keepA = selStart, keepB = selEnd
        loadSelection()
        selStart = keepA
        selEnd = keepB
    }

    // MARK: Selection in the sequence view

    var hasSelection: Bool {
        if let a = selStart, let b = selEnd { return a != b }
        return false
    }

    func clearSelection() {
        selStart = nil
        selEnd = nil
        selStats = nil
    }

    /// Sets the selection (positions between bases) and fetches the statistics.
    func select(from a: Int, to b: Int) {
        selStart = a
        selEnd = b
        updateStats()
    }

    private func updateStats() {
        guard let project, let id = selection, let a = selStart, let b = selEnd, a != b else {
            selStats = nil
            return
        }
        selStats = try? project.selectionStats(id: id, start: UInt64(min(a, b)), end: UInt64(max(a, b)))
    }

    /// Copies the selection to the clipboard (`bases`, `fasta` or `reverse_complement`).
    func copySelection(format: String) {
        guard let project, let id = selection, let a = selStart, let b = selEnd, a != b else { return }
        run {
            let text = try project.selectionText(id: id, start: UInt64(min(a, b)), end: UInt64(max(a, b)), format: format)
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }
    }

    // MARK: Sequence tools (back-translation, joining, comparison, alignment columns)

    /// Back-translates the selected protein into a new DNA document. `true` on success.
    @discardableResult
    func backTranslateSelected(mode: String, name: String) -> Bool {
        guard let project, let id = selection else { return false }
        var ok = false
        run {
            let info = try project.backTranslateDocument(id: id, mode: mode, name: name)
            try self.refresh()
            self.multiSelection = [info.id]
            self.selection = info.id
            ok = true
        }
        return ok
    }

    /// Joins the sequences selected in the list (order as in the list) into a new one.
    @discardableResult
    func concatenateSelected(name: String, circular: Bool) -> Bool {
        guard let project, multiSelection.count >= 2 else { return false }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        var ok = false
        run {
            let info = try project.concatenateDocuments(ids: ids, name: name, circular: circular)
            try self.refresh()
            self.multiSelection = [info.id]
            self.selection = info.id
            ok = true
        }
        return ok
    }

    /// Compares the features of the first two selected sequences.
    func compareAnnotationsSelected(mode: String) -> AnnotationComparisonDto? {
        guard let project else { return nil }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        guard ids.count == 2 else {
            errorMessage = tr("Select exactly two sequences to compare.")
            return nil
        }
        var result: AnnotationComparisonDto?
        run { result = try project.compareAnnotations(idA: ids[0], idB: ids[1], mode: mode) }
        return result
    }

    /// Features of a sequence (for displaying comparison results).
    func featuresOf(_ id: String) -> [FeatureInfo] {
        (try? project?.features(id: id)) ?? []
    }

    /// Removes columns from the selected alignment (`gaps`, `constant`, `codon`) and selects the result.
    @discardableResult
    func stripColumns(kind: String, param: Double, start: Int, name: String) -> Bool {
        guard let project, let id = selection, selectedType == "alignment" else { return false }
        var ok = false
        run {
            let info = try project.stripAlignmentColumns(id: id, kind: kind, param: param, start: UInt64(max(0, start)), name: name)
            try self.refresh()
            self.selection = info.id
            ok = true
        }
        return ok
    }

    /// New sequence: reference row of the selected alignment with the variants of another row.
    @discardableResult
    func sequenceWithVariants(reference: Int, row: Int, name: String) -> Bool {
        guard let project, let id = selection, selectedType == "alignment" else { return false }
        var ok = false
        run {
            let info = try project.sequenceWithVariants(id: id, reference: UInt64(reference), row: UInt64(row), name: name)
            try self.refresh()
            self.multiSelection = [info.id]
            self.selection = info.id
            ok = true
        }
        return ok
    }

    /// Possible silent changes in the CDS feature `feature` of the selected sequence.
    func silentChanges(feature: Int, site: String, mode: String) -> [SilentChangeDto] {
        guard let project, let id = selection else { return [] }
        var list: [SilentChangeDto] = []
        run { list = try project.silentChanges(id: id, featureIndex: UInt64(feature), site: site, mode: mode) }
        return list
    }

    /// Applies a silent change (replaces the single base).
    func applySilentChange(_ c: SilentChangeDto) {
        let p = Int(c.position)
        selStart = p
        selEnd = p + 1
        replaceSelection(with: c.to)
    }

    /// Creates new sequences from the features of the selected type (empty = all). Returns the number of new documents.
    @discardableResult
    func extractFeatures(type: String, flank: Int) -> Int {
        guard let project, let id = selection else { return 0 }
        var count = 0
        run {
            let infos = try project.extractFeatures(id: id, featureType: type, flank: UInt64(max(0, flank)))
            try self.refresh()
            count = infos.count
            self.announce(tr("%@ new sequence(s) created from features", "\(infos.count)"))
        }
        return count
    }

    /// Shows `text` at the bottom of the window for a few seconds.
    func announce(_ text: String) {
        notice = text
        noticeToken += 1
        let token = noticeToken
        DispatchQueue.main.asyncAfter(deadline: .now() + 3.5) { [weak self] in
            if self?.noticeToken == token { self?.notice = nil }
        }
    }

    private func run(_ work: () throws -> Void) {
        do { try work() } catch { errorMessage = message(for: error) }
    }

    private func message(for error: Error) -> String {
        if let e = error as? ApiError {
            switch e {
            case .InvalidInput(let m), .NotFound(let m), .Io(let m), .Internal(let m): return m
            }
        }
        return error.localizedDescription
    }
}
