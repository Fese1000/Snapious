import SnapiousFFI
import SwiftUI

/// Rückübersetzung eines Proteins in DNA (Werkzeuge-Reiter, nur bei Proteinen).
struct BackTranslateSection: View {
    @ObservedObject var model: AppModel
    @State private var mode = "preferred"
    @State private var name = ""

    var body: some View {
        GroupBox(tr("Rückübersetzung (Protein → DNA)")) {
            VStack(alignment: .leading, spacing: 8) {
                Picker("", selection: $mode) {
                    Text(tr("Häufigste E.-coli-Codons")).tag("preferred")
                    Text(tr("Mehrdeutig (IUPAC, z. B. GCN für Ala)")).tag("degenerate")
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 460)
                HStack {
                    TextField(tr("Name der DNA (optional)"), text: $name).textFieldStyle(.roundedBorder).frame(width: 260)
                    Button(tr("Als neue Sequenz anlegen")) { model.backTranslateSelected(mode: mode, name: name) }
                }
                Text(tr("Für ein Protein mit Stopp (*) wird dieses als Stopp-Codon übernommen, wenn es in der Sequenz steht."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(6)
        }
    }
}

/// Stille Einzelbasenänderungen in einem CDS (z. B. um eine Schnittstelle zu erzeugen oder zu entfernen).
struct SilentMutationsSection: View {
    @ObservedObject var model: AppModel
    @State private var feature = -1
    @State private var site = ""
    @State private var mode = "all"
    @State private var changes: [SilentRow] = []

    private var cdsList: [(Int, FeatureInfo)] {
        model.features.enumerated().filter { $0.element.featureType.uppercased() == "CDS" && $0.element.strand != "REVERSE" }.map { ($0.offset, $0.element) }
    }

    var body: some View {
        GroupBox(tr("Stille Mutationen (gleiche Aminosäure, andere Base)")) {
            VStack(alignment: .leading, spacing: 8) {
                if cdsList.isEmpty {
                    Text(tr("Diese Sequenz hat kein CDS auf dem Vorwärtsstrang. Trage zuerst ein CDS-Feature ein (Sequenz → Auswahl → Feature…)."))
                        .foregroundStyle(.secondary)
                } else {
                    HStack {
                        Picker("CDS", selection: $feature) {
                            Text(tr("Bitte wählen")).tag(-1)
                            ForEach(cdsList, id: \.0) { Text("\($0.1.name) (\($0.1.start + 1)–\($0.1.end))").tag($0.0) }
                        }.frame(width: 260)
                        Picker(tr("Gesucht"), selection: $mode) {
                            Text(tr("Alle")).tag("all")
                            Text(tr("Erzeugt Stelle")).tag("create")
                            Text(tr("Zerstört Stelle")).tag("remove")
                        }.frame(width: 220)
                    }
                    HStack {
                        TextField(tr("Enzym oder DNA-Folge, z. B. EcoRI"), text: $site).textFieldStyle(.roundedBorder).frame(width: 260)
                        Button(tr("Suchen")) { search() }.disabled(feature < 0)
                    }
                    if !changes.isEmpty {
                        Table(changes) {
                            TableColumn(tr("Position")) { c in Text("\(c.dto.position + 1)") }.width(80)
                            TableColumn(tr("Codon")) { c in Text("\(c.dto.codon)") }.width(60)
                            TableColumn(tr("Änderung")) { c in Text("\(c.dto.from) → \(c.dto.to)") }.width(100)
                            TableColumn("") { c in
                                Button(tr("Anwenden")) { model.applySilentChange(c.dto); search() }.buttonStyle(.borderless)
                            }.width(90)
                        }
                        .frame(minHeight: 120, maxHeight: 220)
                    }
                }
            }
            .padding(6)
        }
        .onChange(of: model.selection) { _, _ in changes = []; feature = -1 }
    }

    private func search() {
        changes = model.silentChanges(feature: feature, site: site, mode: mode).enumerated().map { SilentRow(index: $0.offset, dto: $0.element) }
    }
}

struct SilentRow: Identifiable {
    let index: Int
    let dto: SilentChangeDto
    var id: Int { index }
}

/// Mehrere gewählte Sequenzen hintereinanderhängen.
struct ConcatSection: View {
    @ObservedObject var model: AppModel
    @State private var name = ""
    @State private var circular = false

    var body: some View {
        GroupBox(tr("Zusammenfügen (hintereinander, Reihenfolge wie in der Liste)")) {
            VStack(alignment: .leading, spacing: 8) {
                Toggle(tr("Ergebnis zirkulär"), isOn: $circular)
                HStack {
                    TextField(tr("Name (optional)"), text: $name).textFieldStyle(.roundedBorder).frame(width: 260)
                    Button(tr("Zusammenfügen")) { model.concatenateSelected(name: name, circular: circular) }
                }
                Text(tr("Die Basen werden direkt aneinandergehängt, Features wandern mit. Ohne Überlappung – für überlappende Enden Gibson verwenden."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(6)
        }
    }
}

/// Features von zwei gewählten Sequenzen vergleichen.
struct CompareAnnotationsSection: View {
    @ObservedObject var model: AppModel
    @State private var mode = "overlap"
    @State private var result: AnnotationComparisonDto?
    @State private var namesA: [FeatureInfo] = []
    @State private var namesB: [FeatureInfo] = []

    var body: some View {
        GroupBox(tr("Annotationen vergleichen (genau zwei Sequenzen)")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Picker(tr("Gleich, wenn"), selection: $mode) {
                        Text(tr("Typ gleich und überlappend")).tag("overlap")
                        Text(tr("Typ, Beginn und Ende gleich")).tag("exact")
                    }.frame(width: 340)
                    Button(tr("Vergleichen")) { compare() }.disabled(model.multiSelection.count != 2)
                }
                if let r = result {
                    list(tr("Nur in der ersten"), r.onlyA, namesA)
                    list(tr("Nur in der zweiten"), r.onlyB, namesB)
                    list(tr("In beiden (erste)"), r.commonA, namesA)
                }
            }
            .padding(6)
        }
        .onChange(of: model.multiSelection) { _, _ in result = nil }
    }

    private func list(_ title: String, _ idx: [UInt64], _ feats: [FeatureInfo]) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("\(title): \(idx.count)").font(.headline)
            ForEach(idx.prefix(30), id: \.self) { i in
                if Int(i) < feats.count {
                    let f = feats[Int(i)]
                    Text("\(f.featureType) \(f.name) \(f.start + 1)–\(f.end)").font(.callout).foregroundStyle(.secondary)
                }
            }
            if idx.count > 30 { Text(tr("… und %@ weitere", "\(idx.count - 30)")).font(.caption).foregroundStyle(.secondary) }
        }
    }

    private func compare() {
        let ids = model.documents.filter { model.multiSelection.contains($0.id) && $0.docType == "sequence" }.map { $0.id }
        guard ids.count == 2, let r = model.compareAnnotationsSelected(mode: mode) else { return }
        namesA = model.featuresOf(ids[0])
        namesB = model.featuresOf(ids[1])
        result = r
    }
}

/// Dialog „Spalten entfernen“ für ein Alignment.
struct ColumnStripSheet: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var kind = "gaps"
    @State private var gapPercent = 50.0
    @State private var codonPosition = 3
    @State private var start = 1
    @State private var name = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("Spalten entfernen")).font(.title2.bold())
            Picker(tr("Welche Spalten"), selection: $kind) {
                Text(tr("Mit vielen Lücken")).tag("gaps")
                Text(tr("Ohne Unterschied (konstant)")).tag("constant")
                Text(tr("Eine Codonposition")).tag("codon")
            }
            .pickerStyle(.segmented)
            switch kind {
            case "gaps":
                Stepper(tr("Lückenanteil mindestens %@ %", "\(Int(gapPercent))"), value: $gapPercent, in: 1...100, step: 5)
            case "codon":
                Picker(tr("Codonposition"), selection: $codonPosition) {
                    Text(tr("1. Position")).tag(1)
                    Text(tr("2. Position")).tag(2)
                    Text(tr("3. Position (wackelnd)")).tag(3)
                }
                Stepper(tr("Erstes Codon beginnt in Spalte %@", "\(start)"), value: $start, in: 1...100000)
            default:
                Text(tr("Entfernt alle Spalten, in denen alle Zeilen dieselbe Base haben und keine Lücke steht."))
                    .font(.callout).foregroundStyle(.secondary)
            }
            TextField(tr("Name des neuen Alignments (optional)"), text: $name).textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button(tr("Abbrechen"), role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
                Button(tr("Entfernen")) {
                    let param = kind == "gaps" ? gapPercent / 100 : Double(codonPosition)
                    if model.stripColumns(kind: kind, param: param, start: start - 1, name: name) { dismiss() }
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 460)
    }
}

/// Dialog „Sequenz mit Varianten“: Referenzzeile plus die Abweichungen einer anderen Zeile.
struct ApplyVariantsSheet: View {
    @ObservedObject var model: AppModel
    let rowNames: [String]
    @Environment(\.dismiss) private var dismiss
    @State private var reference = 0
    @State private var row = 1
    @State private var name = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("Sequenz mit Varianten")).font(.title2.bold())
            Text(tr("Erzeugt die Referenzsequenz (ohne Lücken) mit allen Substitutionen, Einfügungen und Löschungen der gewählten Zeile."))
                .font(.callout).foregroundStyle(.secondary)
            Picker(tr("Referenz"), selection: $reference) {
                ForEach(Array(rowNames.enumerated()), id: \.offset) { Text($0.element).tag($0.offset) }
            }
            Picker(tr("Varianten von"), selection: $row) {
                ForEach(Array(rowNames.enumerated()), id: \.offset) { Text($0.element).tag($0.offset) }
            }
            TextField(tr("Name (optional)"), text: $name).textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button(tr("Abbrechen"), role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
                Button(tr("Anlegen")) {
                    if model.sequenceWithVariants(reference: reference, row: row, name: name) { dismiss() }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(reference == row)
            }
        }
        .padding(20)
        .frame(width: 460)
        .onAppear { row = rowNames.count > 1 ? 1 : 0 }
    }
}

/// Menü mit Enzymsets: Auswahl übernimmt die Enzyme, eigene Sets lassen sich speichern und löschen.
struct EnzymeSetMenu: View {
    /// Aktuell gewählte Enzyme (für „Auswahl als Set speichern“).
    var current: [String] = []
    let onPick: ([String]) -> Void
    @State private var sets: [EnzymeSetDto] = []
    @State private var saveName = ""
    @State private var showSave = false
    @State private var error: String?

    var body: some View {
        Menu(tr("Sets")) {
            ForEach(sets, id: \.name) { s in
                Button("\(s.name) (\(s.enzymes.count))") { onPick(s.enzymes) }
            }
            Divider()
            Button(tr("Auswahl als Set speichern …")) { showSave = true }.disabled(current.isEmpty)
            let own = sets.filter { !$0.builtin }
            if !own.isEmpty {
                Menu(tr("Eigenes Set löschen")) {
                    ForEach(own, id: \.name) { s in Button(s.name) { try? enzymeSetDelete(name: s.name); sets = enzymeSets() } }
                }
            }
        }
        .menuStyle(.borderlessButton).fixedSize()
        .onAppear { sets = enzymeSets() }
        .alert(tr("Enzymset speichern"), isPresented: $showSave) {
            TextField(tr("Name"), text: $saveName)
            Button(tr("Speichern")) {
                do {
                    try enzymeSetSave(name: saveName, enzymes: current.joined(separator: ", "))
                    sets = enzymeSets()
                    saveName = ""
                } catch { self.error = error.localizedDescription }
            }
            Button(tr("Abbrechen"), role: .cancel) { saveName = "" }
        } message: {
            Text(error ?? tr("%@ Enzyme: %@", "\(current.count)", "\(current.joined(separator: ", "))"))
        }
    }
}

/// Konsens aus mehreren Sanger-Reads: ausrichten (Gegenstrang wird erkannt) und je Spalte nach Qualität entscheiden.
struct ReadConsensusSection: View {
    @ObservedObject var model: AppModel
    @State private var minQuality = 20

    var body: some View {
        GroupBox(tr("Konsens aus Sanger-Reads (Qualitäten der .ab1/.scf-Dateien)")) {
            VStack(alignment: .leading, spacing: 8) {
                Stepper(tr("Basen unter Qualität %@ zählen nicht", "\(minQuality)"), value: $minQuality, in: 0...60)
                Button(tr("Konsens bilden")) { model.consensusOfSelectedReads(minQuality: minQuality) }
                Text(tr("Reads, die als Gegenstrang besser passen, werden umgedreht. Ohne gespeicherte Spur gilt für jede Base die Qualität 20. Das Alignment der Reads wird mit angelegt."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(6)
        }
    }
}
