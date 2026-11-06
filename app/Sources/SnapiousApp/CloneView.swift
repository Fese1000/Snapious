import SnapiousFFI
import SwiftUI

/// Restriktionsklonierung: das gewählte Dokument ist der Vektor, das Insert kommt aus der Dokumentliste.
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
            Text("Vektor: \(model.summary?.name ?? "–")").font(Theme.heading())
            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 8) {
                GridRow {
                    Text("Insert")
                    Picker("", selection: $insertID) {
                        Text("Bitte wählen").tag("")
                        ForEach(others, id: \.id) { Text($0.name).tag($0.id) }
                    }.labelsHidden().frame(width: 260)
                }
                GridRow {
                    Text("Enzyme Vektor")
                    HStack {
                        TextField("z. B. EcoRI, BamHI", text: $vectorEnzymes).textFieldStyle(.roundedBorder).frame(width: 260)
                        enzymeMenu($vectorEnzymes)
                        EnzymeSetMenu(current: names(vectorEnzymes)) { vectorEnzymes = $0.joined(separator: ", ") }
                    }
                }
                GridRow {
                    Text("Enzyme Insert")
                    HStack {
                        TextField("z. B. EcoRI, BamHI", text: $insertEnzymes).textFieldStyle(.roundedBorder).frame(width: 260)
                        enzymeMenu($insertEnzymes)
                        EnzymeSetMenu(current: names(insertEnzymes)) { insertEnzymes = $0.joined(separator: ", ") }
                    }
                }
            }
            HStack {
                Button("Produkte berechnen") { compute() }
                    .disabled(insertID.isEmpty || model.selection == nil)
                if let message { Text(message).foregroundStyle(Theme.danger) }
            }
            if candidates.isEmpty {
                Text("Keine Produkte berechnet.").foregroundStyle(Theme.inkSoft)
            } else {
                Table(candidates, selection: $chosen) {
                    TableColumn("Nr.") { c in Text("\(c.index + 1)") }.width(40)
                    TableColumn("Länge") { c in Text("\(c.dto.constructLength) bp") }.width(90)
                    TableColumn("Gerichtet") { c in Text(c.dto.directional ? "ja" : "nein") }.width(80)
                    TableColumn("Insert") { c in Text(c.dto.insertReversed ? "umgekehrt" : "vorwärts") }.width(90)
                    TableColumn("Hinweise") { c in Text(c.dto.warnings.joined(separator: "; ")).foregroundStyle(Theme.inkSoft) }
                }
                HStack {
                    TextField("Name des Konstrukts (optional)", text: $newName).textFieldStyle(.roundedBorder).frame(width: 300)
                    Button("Als Dokument speichern") { save() }.disabled(chosen == nil)
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

    /// Menü zum Anhängen eines häufigen Enzyms an das Textfeld.
    private func enzymeMenu(_ text: Binding<String>) -> some View {
        Menu("+ Enzym") {
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
            if list.isEmpty { message = "Keine passenden Enden gefunden." }
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


/// Klonierung: Methode wählen (Restriktion, Gibson, In-Fusion, SLIC, Golden Gate) und die passende Eingabe zeigen.
struct CloneView: View {
    @ObservedObject var model: AppModel
    @State private var method = "restriction"

    private static let methods: [(String, String)] = [
        ("restriction", "Restriktion + Ligation"),
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
                Text("Methode").font(.headline)
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

/// Zusammenbau mehrerer Fragmente in gewählter Reihenfolge (Gibson, In-Fusion, SLIC, Golden Gate).
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
        case "gibson": return "Gibson: Benachbarte Fragmente brauchen am Ende/Anfang eine gleiche Überlappung von 15–40 bp. Die Exonuklease legt die Enden frei, sie lagern sich an, Polymerase und Ligase füllen und verschließen."
        case "infusion": return "In-Fusion: Wie Gibson, aber mit 15–20 bp Überlappung an den Enden; die Fragmente werden direkt zusammengefügt (Enzym-Mix mit 3′→5′-Exonuklease-Aktivität)."
        case "slic": return "SLIC: Überlappende Enden (25–80 bp) werden durch T4-Polymerase freigelegt und lagern sich an; die Lücken schließt die Zelle."
        default: return "Golden Gate: Typ-IIS-Enzym (z. B. BsaI) schneidet außerhalb der Erkennungsstelle und erzeugt 4-nt-Überhänge. Die Überhänge benachbarter Fragmente müssen passen, die Enzymstellen verschwinden im Produkt. Das Enzym darf nicht zusätzlich in den Fragmenten schneiden."
        }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text(help).font(.callout).foregroundStyle(.secondary)
                GroupBox("Fragmente in dieser Reihenfolge") {
                    VStack(alignment: .leading, spacing: 6) {
                        if ids.isEmpty {
                            Text("Noch keine Fragmente gewählt.").foregroundStyle(.secondary)
                        }
                        ForEach(Array(ids.enumerated()), id: \.offset) { i, id in
                            HStack {
                                Text("\(i + 1).").frame(width: 24, alignment: .trailing).foregroundStyle(.secondary)
                                Text(name(of: id))
                                if let d = info[id] {
                                    Text("\(d.length) bp\(d.circular ? " · zirkulär" : "")").foregroundStyle(.secondary)
                                }
                                Spacer()
                                Button { move(i, -1) } label: { Image(systemName: "chevron.up") }.accessibilityLabel("Fragment nach oben").disabled(i == 0)
                                Button { move(i, 1) } label: { Image(systemName: "chevron.down") }.accessibilityLabel("Fragment nach unten").disabled(i == ids.count - 1)
                                Button { ids.remove(at: i); done = false } label: { Image(systemName: "minus.circle") }.accessibilityLabel("Fragment entfernen")
                            }
                            .buttonStyle(.borderless)
                        }
                        Menu("Fragment hinzufügen") {
                            ForEach(sequences.filter { !ids.contains($0.id) }, id: \.id) { d in
                                Button(d.name) { ids.append(d.id); loadInfo(d.id); done = false }
                            }
                        }
                        .menuStyle(.borderlessButton).fixedSize()
                        .disabled(sequences.count == ids.count)
                    }
                }
                GroupBox("Einstellungen") {
                    VStack(alignment: .leading, spacing: 8) {
                        Toggle("Ergebnis zirkulär (Plasmid)", isOn: $circular)
                        if method == "goldengate" {
                            Picker("Enzym", selection: $enzyme) {
                                ForEach(["BsaI", "BsmBI", "Esp3I", "BbsI", "SapI"], id: \.self) { Text($0).tag($0) }
                            }.frame(width: 220)
                        }
                        TextField("Name des Konstrukts (optional)", text: $name).textFieldStyle(.roundedBorder).frame(width: 320)
                    }
                }
                HStack {
                    Button("Zusammenbauen") { run() }
                        .buttonStyle(.borderedProminent)
                        .disabled(ids.count < 2)
                    if ids.count == 1 { Text("Mindestens zwei Fragmente nötig.").foregroundStyle(.secondary) }
                    if done && warnings.isEmpty { Text("Fertig – das Konstrukt ist als neues Dokument angelegt.").foregroundStyle(.green) }
                }
                if !warnings.isEmpty {
                    GroupBox("Hinweise") {
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
            // Start: die in der Liste gewählten Sequenzen, sonst das angezeigte Dokument
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
