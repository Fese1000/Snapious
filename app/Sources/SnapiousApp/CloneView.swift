import SnapiousFFI
import SwiftUI

/// Restriction cloning: the selected document is the vector, the insert comes from the document list.
struct RestrictionCloneView: View {
    @ObservedObject var model: AppModel
    @State private var insertID: String = ""
    @State private var vectorEnzymes = ""
    @State private var insertEnzymes = ""
    @State private var candidates: [CandidateRow] = []
    @State private var chosen: Int?
    @State private var newName = ""
    @State private var message: String?

    private var others: [DocumentInfoDto] {
        model.documents.filter { $0.id != model.selection && $0.docType == "sequence" }
    }

    private func names(_ text: String) -> [String] {
        text.split(whereSeparator: { ", ;".contains($0) }).map { String($0).trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(tr("Vector: %@", "\(model.summary?.name ?? "–")")).font(Theme.heading())
            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 8) {
                GridRow {
                    Text(tr("Insert"))
                    Picker("", selection: $insertID) {
                        Text(tr("Please choose")).tag("")
                        ForEach(others, id: \.id) { Text($0.name).tag($0.id) }
                    }.labelsHidden().frame(width: 260)
                }
                GridRow {
                    Text(tr("Vector enzymes"))
                    HStack {
                        TextField(tr("e.g. EcoRI, BamHI"), text: $vectorEnzymes).textFieldStyle(.roundedBorder).frame(width: 260)
                        enzymeMenu($vectorEnzymes)
                        EnzymeSetMenu(current: names(vectorEnzymes)) { vectorEnzymes = $0.joined(separator: ", ") }
                    }
                }
                GridRow {
                    Text(tr("Insert enzymes"))
                    HStack {
                        TextField(tr("e.g. EcoRI, BamHI"), text: $insertEnzymes).textFieldStyle(.roundedBorder).frame(width: 260)
                        enzymeMenu($insertEnzymes)
                        EnzymeSetMenu(current: names(insertEnzymes)) { insertEnzymes = $0.joined(separator: ", ") }
                    }
                }
            }
            HStack {
                Button(tr("Calculate products")) { compute() }
                    .disabled(insertID.isEmpty || model.selection == nil)
                if let message { Text(message).foregroundStyle(Theme.danger) }
            }
            if candidates.isEmpty {
                Text(tr("No products calculated.")).foregroundStyle(Theme.inkSoft)
            } else {
                Table(candidates, selection: $chosen) {
                    TableColumn(tr("No.")) { c in Text("\(c.index + 1)") }.width(40)
                    TableColumn(tr("Length")) { c in Text("\(c.dto.constructLength) bp") }.width(90)
                    TableColumn(tr("Directional")) { c in Text(c.dto.directional ? "ja" : "nein") }.width(80)
                    TableColumn(tr("Insert")) { c in Text(c.dto.insertReversed ? "umgekehrt" : tr("forward")) }.width(90)
                    TableColumn(tr("Notes")) { c in Text(c.dto.warnings.joined(separator: "; ")).foregroundStyle(Theme.inkSoft) }
                }
                HStack {
                    TextField(tr("Construct name (optional)"), text: $newName).textFieldStyle(.roundedBorder).frame(width: 300)
                    Button(tr("Save as document")) { save() }.disabled(chosen == nil)
                }
            }
            Spacer()
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: model.selection) { _, _ in
            candidates = []
            chosen = nil
            message = nil
        }
    }

    private static let commonEnzymes = ["EcoRI", "BamHI", "HindIII", "XbaI", "SpeI", "PstI", "NotI", "XhoI", "NcoI", "NdeI", "SalI", "KpnI", "SacI", "SmaI", "EcoRV", "BglII", "NheI", "AgeI", "ApaI", "SphI"]

    /// Menu for appending a common enzyme to the text field.
    private func enzymeMenu(_ text: Binding<String>) -> some View {
        Menu(tr("+ Enzyme")) {
            ForEach(Self.commonEnzymes, id: \.self) { e in
                Button(e) { text.wrappedValue = names(text.wrappedValue).contains(e) ? text.wrappedValue : (names(text.wrappedValue) + [e]).joined(separator: ", ") }
            }
        }
        .menuStyle(.borderlessButton).fixedSize()
    }

    private func compute() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        do {
            let list = try project.restrictionCandidates(
                vectorId: id, vectorEnzymes: names(vectorEnzymes), insertId: insertID, insertEnzymes: names(insertEnzymes))
            candidates = list.enumerated().map { CandidateRow(index: $0.offset, dto: $0.element) }
            chosen = nil
            if list.isEmpty { message = tr("No matching ends found.") }
        } catch {
            candidates = []
            message = error.localizedDescription
        }
    }

    private func save() {
        guard let project = model.project, let id = model.selection, let index = chosen else { return }
        do {
            let info = try project.saveCandidate(
                vectorId: id, vectorEnzymes: names(vectorEnzymes), insertId: insertID, insertEnzymes: names(insertEnzymes),
                index: UInt64(index), name: newName)
            model.reload(selecting: info.id)
        } catch {
            message = error.localizedDescription
        }
    }
}

struct CandidateRow: Identifiable, Hashable {
    let index: Int
    let dto: CandidateDto
    var id: Int { index }
    static func == (a: CandidateRow, b: CandidateRow) -> Bool { a.index == b.index }
    func hash(into h: inout Hasher) { h.combine(index) }
}


/// Cloning: choose a method (restriction, Gibson, In-Fusion, SLIC, Golden Gate) and show the matching input.
struct CloneView: View {
    @ObservedObject var model: AppModel
    @State private var method = "restriction"

    private static let methods: [(String, String)] = [
        ("restriction", tr("Restriction + ligation")),
        ("gibson", "Gibson"),
        ("infusion", "In-Fusion"),
        ("slic", "SLIC"),
        ("goldengate", "Golden Gate"),
        ("topo", "TA / TOPO"),
        ("gateway", "Gateway"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(tr("Method")).font(.headline)
                Picker("", selection: $method) {
                    ForEach(Self.methods, id: \.0) { Text($0.1).tag($0.0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
            }
            .padding(12)
            Divider()
            if method == "restriction" {
                RestrictionCloneView(model: model)
            } else if method == "topo" {
                TopoCloneView(model: model)
            } else if method == "gateway" {
                GatewayCloneView(model: model)
            } else {
                AssemblyCloneView(model: model, method: method)
                    .id(method)
            }
        }
    }
}

/// Assembly of several fragments in a chosen order (Gibson, In-Fusion, SLIC, Golden Gate).
struct AssemblyCloneView: View {
    @ObservedObject var model: AppModel
    let method: String
    @State private var ids: [String] = []
    @State private var circular = true
    @State private var enzyme = "BsaI"
    @State private var name = ""
    @State private var warnings: [String] = []
    @State private var done = false
    @State private var info: [String: (length: Int, circular: Bool)] = [:]

    private var sequences: [DocumentInfoDto] { model.documents.filter { $0.docType == "sequence" } }

    private func name(of id: String) -> String { model.documents.first { $0.id == id }?.name ?? id }

    private var help: String {
        switch method {
        case "gibson": return tr("Gibson: adjacent fragments need an identical 15–40 bp overlap at their ends. The exonuclease exposes the ends, they anneal, and polymerase and ligase fill and seal.")
        case "infusion": return tr("In-Fusion: like Gibson, but with 15–20 bp overlaps; fragments are joined directly (enzyme mix with 3′→5′ exonuclease activity).")
        case "slic": return tr("SLIC: overlapping ends (25–80 bp) are exposed by T4 polymerase and anneal; the cell closes the gaps.")
        default: return tr("Golden Gate: a type IIS enzyme (e.g. BsaI) cuts outside its recognition site and creates 4-nt overhangs. The overhangs of adjacent fragments must match, and the enzyme sites disappear in the product. The enzyme must not cut inside the fragments.")
        }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text(help).font(.callout).foregroundStyle(.secondary)
                GroupBox(tr("Fragments in this order")) {
                    VStack(alignment: .leading, spacing: 6) {
                        if ids.isEmpty {
                            Text(tr("No fragments chosen yet.")).foregroundStyle(.secondary)
                        }
                        ForEach(Array(ids.enumerated()), id: \.offset) { i, id in
                            HStack {
                                Text("\(i + 1).").frame(width: 24, alignment: .trailing).foregroundStyle(.secondary)
                                Text(name(of: id))
                                if let d = info[id] {
                                    Text("\(d.length) bp\(d.circular ? " · zirkulär" : "")").foregroundStyle(.secondary)
                                }
                                Spacer()
                                Button { move(i, -1) } label: { Image(systemName: "chevron.up") }.accessibilityLabel(tr("Move fragment up")).disabled(i == 0)
                                Button { move(i, 1) } label: { Image(systemName: "chevron.down") }.accessibilityLabel(tr("Move fragment down")).disabled(i == ids.count - 1)
                                Button { ids.remove(at: i); done = false } label: { Image(systemName: "minus.circle") }.accessibilityLabel(tr("Remove fragment"))
                            }
                            .buttonStyle(.borderless)
                        }
                        Menu(tr("Add fragment")) {
                            ForEach(sequences.filter { !ids.contains($0.id) }, id: \.id) { d in
                                Button(d.name) { ids.append(d.id); loadInfo(d.id); done = false }
                            }
                        }
                        .menuStyle(.borderlessButton).fixedSize()
                        .disabled(sequences.count == ids.count)
                    }
                }
                GroupBox(tr("Settings")) {
                    VStack(alignment: .leading, spacing: 8) {
                        Toggle(tr("Circular result (plasmid)"), isOn: $circular)
                        if method == "goldengate" {
                            Picker(tr("Enzyme"), selection: $enzyme) {
                                ForEach(["BsaI", "BsmBI", "Esp3I", "BbsI", "SapI"], id: \.self) { Text($0).tag($0) }
                            }.frame(width: 220)
                        }
                        TextField(tr("Construct name (optional)"), text: $name).textFieldStyle(.roundedBorder).frame(width: 320)
                    }
                }
                HStack {
                    Button(tr("Assemble")) { run() }
                        .buttonStyle(.borderedProminent)
                        .disabled(ids.count < 2)
                    if ids.count == 1 { Text(tr("At least two fragments needed.")).foregroundStyle(.secondary) }
                    if done && warnings.isEmpty { Text(tr("Done – the construct was created as a new document.")).foregroundStyle(.green) }
                }
                if !warnings.isEmpty {
                    GroupBox(tr("Notes")) {
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(Array(warnings.enumerated()), id: \.offset) { _, w in
                                Label(w, systemImage: "exclamationmark.triangle").foregroundStyle(.orange)
                            }
                        }
                    }
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .onAppear {
            // Start: the sequences selected in the list, otherwise the displayed document
            let chosen = model.documents.filter { model.multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
            ids = chosen.count >= 2 ? chosen : (model.selection.map { [$0] } ?? [])
            ids.forEach(loadInfo)
        }
    }

    private func move(_ i: Int, _ d: Int) {
        ids.swapAt(i, i + d)
        done = false
    }

    private func loadInfo(_ id: String) {
        guard let project = model.project, let s = try? project.summary(id: id) else { return }
        info[id] = (Int(s.length), s.topology == "CIRCULAR")
    }

    private func run() {
        if let w = model.assembleFragments(ids: ids, kind: method, circular: circular, enzyme: enzyme, name: name) {
            warnings = w
            done = true
        }
    }
}
