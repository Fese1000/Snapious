import AppKit
import Foundation
import SnapiousFFI
import SwiftUI

/// Eine Aminosäure über ihren drei Codonbasen `[lo, hi]` (0-basiert).
struct CodonMark {
    let lo: Int
    let hi: Int
    let letter: String
    let isStart: Bool
    let isStop: Bool
    /// Zeile unter den Basen (0 = erste Aminosäurezeile).
    var row: Int = 0
}

/// Eine Schnittstelle eines Enzyms (Beginn der Erkennungssequenz, 0-basiert).
struct EnzymeSite: Hashable {
    let enzyme: String
    let position: Int
}

/// Zustand der App: geöffnetes Projekt, Dokumentliste, Auswahl und Fehlermeldungen.
/// Alle Fachlogik steckt im Rust-Kern; hier wird nur aufgerufen und angezeigt.
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
    /// Mehrfachauswahl in der Dokumentliste (z. B. für ein Alignment); bei genau einem Eintrag ist das auch `selection`.
    @Published var multiSelection: Set<String> = [] {
        didSet {
            if multiSelection.count == 1, let only = multiSelection.first, selection != only {
                selection = only
            } else if multiSelection.isEmpty, selection != nil {
                selection = nil
            }
        }
    }
    // MARK: Ungespeicherte Änderungen
    enum Leave { case tab(Int); case selection(String?) }
    /// Ziel, das der Nutzer ansteuern wollte, während Änderungen an der Sequenz offen sind (löst die Rückfrage aus).
    @Published var pendingLeave: Leave?
    var guardBypass = false
    /// Zahl der Rückgängig-Schritte beim Öffnen bzw. beim letzten „Speichern“ je Dokument.
    var baselines: [String: Int] = [:]
    /// Schritte je Nutzeraktion (Einfügen + Annotation = 2), damit Rückgängig die Aktion als Ganzes zurücknimmt.
    /// Mehrschrittige Aktionen: (Zahl der Schritte danach, Zahl der Schritte der Aktion).
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
    var selectedName: String { summary?.name ?? "Sequenz" }

    /// Änderungen sind schon im Projekt gespeichert; „Speichern“ übernimmt sie als neuen Ausgangspunkt.
    func commitEdits() {
        if let id = selection { baselines[id] = undoCount(of: id); undoGroups[id] = []; redoGroups[id] = [] }
    }

    /// Verwirft alle Änderungen seit dem Öffnen bzw. dem letzten Speichern.
    func discardEdits() {
        guard let project, let id = selection else { return }
        let base = baselines[id] ?? 0
        while undoCount(of: id) > base, (try? project.editUndo(id: id)) == true {}
        undoGroups[id] = []; redoGroups[id] = []
        reloadCurrent()
    }

    /// Antwort auf die Rückfrage: `save`, `discard` oder `cancel`.
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

    /// Das gewählte Alignment (wenn das gewählte Dokument eines ist).
    @Published var alignment: AlignmentDto?
    @Published var summary: DocumentSummary?
    @Published var features: [FeatureInfo] = []
    /// Lesetiefe der gewählten Sequenz (auf höchstens 1000 Werte gemittelt) aus einer verknüpften SAM/BAM-Datei.
    @Published var coverage: [Double] = []
    @Published var coverageLabel = ""
    @Published var preview: String = ""
    @Published var lanes: [PlacedFeatureDto] = []
    /// Übersetzungen der CDS-Features für die Sequenzansicht: Aminosäuren je Codon, mit der Spanne `[lo, hi]` der Codonbasen.
    @Published var codons: [CodonMark] = []
    /// Angezeigte Leserahmen: 1, 2, 3 vorwärts, −1, −2, −3 Gegenstrang.
    @Published var aaFrames: [Int] = []
    /// Aminosäuren der CDS-Features zeigen.
    @Published var showCdsTranslation = true
    /// Enzyme der Standarddatenbank (für die Verdauansicht).
    @Published var enzymes: [EnzymeInfoDto] = []
    /// Einzelschneider-Enzyme mit ihrer Position (eine Zeile je Schnittstelle), nach Position sortiert.
    @Published var uniqueSites: [EnzymeSite] = []
    /// Alle Schnittstellen aller Enzyme der Standarddatenbank (Grundlage für den Filter der Kartenansicht).
    @Published var allSites: [EnzymeSite] = []
    /// Enzymfilter der Karte: `unique`, `double`, `all`, `custom` oder `set:<Name>`.
    @Published var siteFilter: String = UserDefaults.standard.string(forKey: "siteFilter") ?? "set:Häufige Klonierung" {
        didSet { UserDefaults.standard.set(siteFilter, forKey: "siteFilter") }
    }
    @Published var customEnzymes: Set<String> = Set(UserDefaults.standard.stringArray(forKey: "customEnzymes") ?? []) {
        didSet { UserDefaults.standard.set(Array(customEnzymes), forKey: "customEnzymes") }
    }
    /// Darstellung der Karte: `auto` (nach Topologie), `ring` oder `linear`.
    @Published var mapStyle = "auto"
    /// Vergrößerung (1 = ganze Sequenz) und linker Rand des sichtbaren Ausschnitts in Basen.
    @Published var zoom = 1.0
    @Published var viewStart = 0.0
    /// Ganze Sequenz für die Sequenzansicht (bis `fullLimit` Basen).
    @Published var fullBases: String = "" {
        didSet { fullBytes = Array(fullBases.utf8) }
    }
    /// Dieselben Basen als Bytes (schneller Zugriff beim Zeichnen).
    private(set) var fullBytes: [UInt8] = []
    /// Auswahl in der Sequenzansicht: Positionen zwischen den Basen, `nil` = nichts ausgewählt.
    @Published var selStart: Int?
    @Published var selEnd: Int?
    @Published var selStats: SelectionStatsDto?
    @Published var errorMessage: String?
    /// Gewählter Reiter der Detailansicht (0 = Übersicht … 12 = NGS).
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
    /// Kurze Rückmeldung unten im Fenster (verschwindet nach wenigen Sekunden).
    @Published var notice: String?
    private var noticeToken = 0
    /// Zuletzt geöffnete Projekte (neueste zuerst), gespeichert in den Benutzereinstellungen.
    @Published var recentProjects: [String] = (UserDefaults.standard.stringArray(forKey: "recentProjects") ?? []).filter { FileManager.default.fileExists(atPath: $0) }
    /// Hinweise des letzten Zusammenbaus (Gibson/Golden Gate), `nil` wenn keine.
    @Published var assemblyNotice: String?

    private static let lastProjectKey = "lastProjectPath"
    /// Zahl der Basen, die in der Vorschau erscheinen.
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
        guard let p = project else { return "Kein Projekt geöffnet" }
        return URL(fileURLWithPath: p.path()).deletingPathExtension().lastPathComponent
    }

    // MARK: Projekt

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
        panel.title = "Neues Snapious-Projekt"
        panel.nameFieldStringValue = "Projekt.snapious"
        panel.canCreateDirectories = true
        // Immer im Dokumente-Ordner beginnen (das Panel merkt sich sonst den zuletzt benutzten Ordner)
        panel.directoryURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        guard panel.runModal() == .OK, let url = panel.url else { return }
        open(path: url.path)
    }

    func openProject() {
        let panel = NSOpenPanel()
        panel.title = "Snapious-Projekt öffnen"
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        open(path: url.path)
    }

    // MARK: Dokumente

    func importFiles(_ urls: [URL]) {
        guard let project else {
            errorMessage = "Bitte zuerst ein Projekt anlegen oder öffnen."
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
            self.announce(urls.count == 1 ? "„\(urls[0].lastPathComponent)“ importiert" : "\(urls.count) Dateien importiert")
        }
    }

    func chooseFilesToImport() {
        let panel = NSOpenPanel()
        panel.title = "Importieren (FASTA, FASTQ, GenBank, EMBL, SnapGene, PIR, GCG, MEGA, NEXUS, Clustal, PHYLIP)"
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK else { return }
        importFiles(panel.urls)
    }

    /// Speichert ein Dokument über einen Dialog in einer Datei (`format` wie bei `exportDocument`, `ext` ist die Dateiendung).
    func export(id: String, format: String, ext: String) {
        guard let project, let info = documents.first(where: { $0.id == id }) else { return }
        let panel = NSSavePanel()
        panel.title = "Exportieren als \(format)"
        panel.nameFieldStringValue = "\(info.name).\(ext)"
        panel.directoryURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        guard panel.runModal() == .OK, let url = panel.url else { return }
        run { try project.exportDocument(id: id, path: url.path, format: format) }
    }

    /// Liest Annotationen (GFF3 oder BED) aus einer Datei und trägt sie als Features ins Dokument ein.
    func importAnnotations(id: String) {
        guard let project else { return }
        let panel = NSOpenPanel()
        panel.title = "Annotationen importieren (GFF3 oder BED)"
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        run {
            let n = try project.importAnnotations(id: id, path: url.path)
            if self.selection == id { self.reloadCurrent() }
            self.assemblyNotice = "\(n) Annotation(en) importiert"
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

    /// Liste neu laden und ein Dokument auswählen (z. B. nach dem Speichern eines Konstrukts).
    func reload(selecting id: String?) {
        run {
            try self.refresh()
            if let id { self.selection = id }
        }
    }

    func refresh() throws {
        documents = try project?.documents() ?? []
    }

    /// Art des gewählten Dokuments (`sequence`, `alignment`) oder `nil`.
    var selectedType: String? {
        documents.first { $0.id == selection }?.docType
    }

    /// Aminosäuren aller CDS-Features (nur bei kurzen Dokumenten, damit die Ansicht schnell bleibt).
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

    /// Wählt die angezeigten Leserahmen und lädt die Aminosäuren neu.
    func setFrames(_ frames: [Int], cds: Bool) {
        aaFrames = frames
        showCdsTranslation = cds
        guard let project, let id = selection, selectedType == "sequence" else { return }
        codons = loadCodons(project: project, id: id)
    }

    /// Legt Sequenzen aus eingefügtem Text an und wählt die erste. `true` bei Erfolg.
    func createSequence(name: String, text: String, kind: String, circular: Bool) -> Bool {
        guard let project else { return false }
        var ok = false
        run {
            let infos = try project.createSequence(name: name, text: text, kind: kind, circular: circular)
            try self.refresh()
            self.multiSelection = Set(infos.prefix(1).map { $0.id })
            self.selection = infos.first?.id
            self.announce(infos.count == 1 ? "Sequenz „\(infos[0].name)“ angelegt" : "\(infos.count) Sequenzen angelegt")
            ok = true
        }
        return ok
    }

    /// Setzt die Fragmente in der gegebenen Reihenfolge zusammen (`gibson`, `infusion`, `slic`, `goldengate`);
    /// gibt die Hinweise des Zusammenbaus zurück (`nil` bei Fehler, der Fehler steht dann in `errorMessage`).
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
            self.announce("Konstrukt „\(result.document.name)“ angelegt")
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

    // MARK: Alignment und Baum

    /// Richtet die in der Liste gewählten Sequenzen aus und wählt das neue Alignment.
    func alignSelected(backend: String = "builtin") {
        guard let project, multiSelection.count >= 2 else { return }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        run {
            let info = try project.alignDocuments(ids: ids, name: "Alignment", backend: backend)
            try self.refresh()
            self.selection = info.id
        }
    }

    /// Setzt die in der Liste gewählten Sequenzen zusammen: `kind` ist `gibson`, `infusion`, `slic` oder `goldengate` (dann mit `enzyme`).
    /// Meldet Hinweise des Zusammenbaus über `errorMessage`-ähnlichen Text in `assemblyNotice`.
    func assembleSelected(kind: String, circular: Bool, enzyme: String = "BsaI") {
        guard let project, multiSelection.count >= 2 else { return }
        // Reihenfolge wie in der Liste
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

    /// Lädt die Lesetiefe aus einer SAM/BAM-Datei für die gewählte Sequenz und zeigt sie unter der Sequenzansicht.
    func showCoverage(path: String, label: String) {
        guard let project, let id = selection else { return }
        // Große SAM/BAM-Dateien brauchen Zeit: im Hintergrund lesen, damit die Oberfläche nicht einfriert
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

    /// Setzt die gewählten Sequenzen zu Contigs zusammen und wählt den ersten.
    func assembleReadsSelected(minOverlap: Int) {
        guard let project, multiSelection.count >= 2 else { return }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        run {
            let infos = try project.assembleReads(ids: ids, minOverlap: UInt64(minOverlap))
            try self.refresh()
            if let first = infos.first { self.selection = first.id }
            self.assemblyNotice = "\(infos.count) Contig(s) aus \(ids.count) Reads"
        }
    }

    /// Bildet die übrigen gewählten Sequenzen auf die erste gewählte (Referenz) ab.
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

    /// Neues Alignment ohne Spalten, in denen mindestens der Anteil `threshold` der Zeilen eine Lücke hat.
    func stripSelectedAlignment(threshold: Double) {
        guard let project, let id = selection, selectedType == "alignment" else { return }
        run {
            let info = try project.alignmentStrip(id: id, threshold: threshold)
            try self.refresh()
            self.selection = info.id
        }
    }

    /// Konsenssequenz des gewählten Alignments als neues Dokument.
    func consensusOfSelectedAlignment() {
        guard let project, let id = selection, selectedType == "alignment" else { return }
        run {
            let info = try project.alignmentConsensus(id: id)
            try self.refresh()
            self.selection = info.id
        }
    }

    /// Konsens aus den in der Liste gewählten Sequenzierungen (Alignment und Konsenssequenz werden angelegt).
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
            self.announce("Konsens aus \(ids.count) Reads angelegt (Alignment der Reads ebenfalls)")
            ok = true
        }
        return ok
    }

    // MARK: Features bearbeiten

    /// Trägt die Auswahl als Feature ein.
    func addFeatureFromSelection(name: String, type: String, strand: String) {
        guard let project, let id = selection, let a = selStart, let b = selEnd, a != b else { return }
        let info = FeatureInfo(featureType: type, name: name, start: UInt64(min(a, b)), end: UInt64(max(a, b)), strand: strand)
        run {
            try project.addFeatures(id: id, features: [info])
            self.reloadCurrent()
        }
    }

    /// Entfernt das Feature mit diesem Index aus dem Dokument.
    func removeFeature(index: Int) {
        guard let project, let id = selection else { return }
        run {
            try project.removeFeature(id: id, index: UInt64(index))
            self.reloadCurrent()
        }
    }

    // MARK: Suchen

    @Published var motifHits: [MotifHitDto] = []
    @Published var motifIndex = 0
    @Published var motifMessage: String?

    /// Sucht ein Motiv (IUPAC) im gewählten Dokument und wählt den ersten Treffer.
    func findMotif(_ motif: String) {
        motifMessage = nil
        guard let project, let id = selection, !motif.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        do {
            motifHits = try project.findMotifHits(id: id, motif: motif)
            motifIndex = -1
            if motifHits.isEmpty { motifMessage = "Nicht gefunden" } else { nextMotif(1) }
        } catch {
            motifHits = []
            motifMessage = error.localizedDescription
        }
    }

    /// Springt zum nächsten (+1) oder vorigen (−1) Treffer und wählt ihn.
    func nextMotif(_ step: Int) {
        guard !motifHits.isEmpty else { return }
        motifIndex = ((motifIndex + step) % motifHits.count + motifHits.count) % motifHits.count
        let h = motifHits[motifIndex]
        select(from: Int(h.start), to: Int(min(h.end, UInt64(fullBytes.count))))
        motifMessage = "Treffer \(motifIndex + 1) von \(motifHits.count)" + (h.reverse ? " (Gegenstrang)" : "")
    }

    // MARK: Karte: Enzymfilter und Zoom

    /// Zahl der Schnittstellen je Enzym in der aktuellen Sequenz.
    var siteCounts: [String: Int] {
        Dictionary(allSites.map { ($0.enzyme, 1) }, uniquingKeysWith: +)
    }

    /// Die nach `siteFilter` ausgewählten Schnittstellen.
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

    /// Mitglieder der Enzymsets (Name → Enzyme), damit das Zeichnen nicht jedes Mal die Datei liest.
    var setMembers: [String: Set<String>] = [:]

    func refreshEnzymeSets() {
        setMembers = Dictionary(enzymeSets().map { ($0.name, Set($0.enzymes)) }, uniquingKeysWith: { a, _ in a })
        objectWillChange.send()
    }

    /// Länge des Dokuments in Basen.
    var docLength: Int { Int(summary?.length ?? 0) }

    /// Sichtbare Spanne in Basen bei der aktuellen Vergrößerung.
    var viewSpan: Double { max(1, Double(docLength) / zoom) }

    /// Breite der Kartenansicht in Punkten (von der Ansicht gesetzt).
    var viewWidth: Double = 1000
    /// Schriftgröße der linearen Sequenzansicht als Faktor (1 = normal).
    @Published var textScale = 1.0

    /// Größte Vergrößerung: im linearen Modus die Stufe, ab der die Basen als Text erscheinen (neun Punkte je Base); im Ring bis acht Basen.
    var maxZoom: Double {
        let len = Double(max(1, docLength))
        let ringMode = mapStyle == "ring" || (mapStyle == "auto" && summary?.topology == "CIRCULAR")
        if ringMode || summary?.seqType == "PROTEIN" { return max(1, len / 8) }
        return max(1, len / 8)
    }

    func setZoom(_ z: Double) {
        let len = Double(max(1, docLength))
        var centre = viewStart + viewSpan / 2
        if let a = selStart, let b = selEnd { centre = Double(a + b) / 2 }
        zoom = min(max(1, z), maxZoom)
        viewStart = min(max(0, centre - viewSpan / 2), max(0, len - viewSpan))
    }

    func panView(to start: Double) {
        viewStart = min(max(0, start), max(0, Double(docLength) - viewSpan))
    }

    // MARK: Bearbeiten

    /// Ersetzt die Auswahl (oder fügt am Cursor ein) durch `bases`; leerer Text löscht die Auswahl.
    func replaceSelection(with bases: String) {
        guard let project, let id = selection else { return }
        let allowed = summary?.seqType == "PROTEIN" ? "ACDEFGHIKLMNPQRSTVWYXBZUOJ*" : "ACGTUNRYSWKMBDHV"
        let cleaned = bases.uppercased().filter { allowed.contains($0) }
        guard let a = selStart, let b = selEnd else { return }
        let lo = min(a, b), hi = max(a, b)
        if cleaned.isEmpty && lo == hi { return }
        // Weitertippen direkt hinter einer automatisch angelegten „Neu“-Annotation verlängert sie
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
                // Neues als Annotation markieren (ein Rückgängig-Schritt zusammen mit dem Einfügen)
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

    /// Löscht das Zeichen vor dem Cursor (Rücktaste) oder die Auswahl.
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

    /// Verschiebt den Cursor um `delta` Basen (hebt die Auswahl auf).
    func moveCaret(by delta: Int) {
        let total = fullBytes.count
        let base = (delta < 0 ? min(selStart ?? 0, selEnd ?? 0) : max(selStart ?? 0, selEnd ?? 0))
        let target = max(0, min(total, base + (hasSelection ? 0 : delta)))
        selStart = target
        selEnd = target
        selStats = nil
    }

    static let newName = "Neu"

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

    /// Text der Auswahl (für Kopieren/Ausschneiden per Tastatur).
    func selectionString() -> String? {
        guard let a = selStart, let b = selEnd, a != b else { return nil }
        let lo = max(0, min(a, b)), hi = min(fullBytes.count, max(a, b))
        guard lo < hi else { return nil }
        return String(decoding: fullBytes[lo..<hi], as: UTF8.self)
    }

    /// Entf-Taste: löscht die Auswahl oder die Base hinter dem Cursor.
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

    /// Zahl der möglichen Rückgängig-Schritte und ob Wiederholen möglich ist.
    var undoState: (undo: Int, redo: Bool) {
        guard let project, let id = selection, let s = try? project.editState(id: id), s.count == 2 else { return (0, false) }
        return (Int(s[0]), s[1] != 0)
    }

    /// Dokumentdaten neu laden, Auswahl bleibt erhalten.
    func reloadCurrent() {
        let keepA = selStart, keepB = selEnd
        loadSelection()
        selStart = keepA
        selEnd = keepB
    }

    // MARK: Auswahl in der Sequenzansicht

    var hasSelection: Bool {
        if let a = selStart, let b = selEnd { return a != b }
        return false
    }

    func clearSelection() {
        selStart = nil
        selEnd = nil
        selStats = nil
    }

    /// Setzt die Auswahl (Positionen zwischen den Basen) und holt die Kennzahlen.
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

    /// Kopiert die Auswahl in die Zwischenablage (`bases`, `fasta` oder `reverse_complement`).
    func copySelection(format: String) {
        guard let project, let id = selection, let a = selStart, let b = selEnd, a != b else { return }
        run {
            let text = try project.selectionText(id: id, start: UInt64(min(a, b)), end: UInt64(max(a, b)), format: format)
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }
    }

    // MARK: Sequenzwerkzeuge (Rückübersetzung, Zusammenfügen, Vergleich, Alignment-Spalten)

    /// Rückübersetzt das gewählte Protein in ein neues DNA-Dokument. `true` bei Erfolg.
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

    /// Fügt die in der Liste gewählten Sequenzen (Reihenfolge wie in der Liste) zu einer neuen zusammen.
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

    /// Vergleicht die Features der ersten beiden gewählten Sequenzen.
    func compareAnnotationsSelected(mode: String) -> AnnotationComparisonDto? {
        guard let project else { return nil }
        let ids = documents.filter { multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        guard ids.count == 2 else {
            errorMessage = "Zum Vergleichen genau zwei Sequenzen wählen."
            return nil
        }
        var result: AnnotationComparisonDto?
        run { result = try project.compareAnnotations(idA: ids[0], idB: ids[1], mode: mode) }
        return result
    }

    /// Features einer Sequenz (für die Anzeige von Vergleichsergebnissen).
    func featuresOf(_ id: String) -> [FeatureInfo] {
        (try? project?.features(id: id)) ?? []
    }

    /// Entfernt Spalten aus dem gewählten Alignment (`gaps`, `constant`, `codon`) und wählt das Ergebnis.
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

    /// Neue Sequenz: Referenzzeile des gewählten Alignments mit den Varianten einer anderen Zeile.
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

    /// Mögliche stille Änderungen im CDS-Feature `feature` der gewählten Sequenz.
    func silentChanges(feature: Int, site: String, mode: String) -> [SilentChangeDto] {
        guard let project, let id = selection else { return [] }
        var list: [SilentChangeDto] = []
        run { list = try project.silentChanges(id: id, featureIndex: UInt64(feature), site: site, mode: mode) }
        return list
    }

    /// Wendet eine stille Änderung an (ersetzt die eine Base).
    func applySilentChange(_ c: SilentChangeDto) {
        let p = Int(c.position)
        selStart = p
        selEnd = p + 1
        replaceSelection(with: c.to)
    }

    /// Legt aus den Features des gewählten Typs (leer = alle) neue Sequenzen an. Gibt die Zahl der neuen Dokumente zurück.
    @discardableResult
    func extractFeatures(type: String, flank: Int) -> Int {
        guard let project, let id = selection else { return 0 }
        var count = 0
        run {
            let infos = try project.extractFeatures(id: id, featureType: type, flank: UInt64(max(0, flank)))
            try self.refresh()
            count = infos.count
            self.announce("\(infos.count) neue Sequenz(en) aus Features angelegt")
        }
        return count
    }

    /// Zeigt `text` einige Sekunden unten im Fenster.
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
