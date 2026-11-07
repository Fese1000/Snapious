import SnapiousFFI
import SwiftUI

/// Reverse translation of a protein into DNA (Tools tab, proteins only).
struct BackTranslateSection: View {
    @ObservedObject var model: AppModel
    @State private var mode = "preferred"
    @State private var name = ""

    var body: some View {
        GroupBox(tr("Back-translation (protein → DNA)")) {
            VStack(alignment: .leading, spacing: 8) {
                Picker("", selection: $mode) {
                    Text(tr("Most frequent E. coli codons")).tag("preferred")
                    Text(tr("Ambiguous (IUPAC, e.g. GCN for Ala)")).tag("degenerate")
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 460)
                HStack {
                    TextField(tr("DNA name (optional)"), text: $name).textFieldStyle(.roundedBorder).frame(width: 260)
                    Button(tr("Create as new sequence")) { model.backTranslateSelected(mode: mode, name: name) }
                }
                Text(tr("For a protein with a stop (*), it is kept as a stop codon if it is in the sequence."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(6)
        }
    }
}

/// Silent single-base changes in a CDS (e.g. to create or remove a cut site).
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
        GroupBox(tr("Silent mutations (same amino acid, different base)")) {
            VStack(alignment: .leading, spacing: 8) {
                if cdsList.isEmpty {
                    Text(tr("This sequence has no CDS on the forward strand. Add a CDS feature first (sequence → selection → feature…)."))
                        .foregroundStyle(.secondary)
                } else {
                    HStack {
                        Picker("CDS", selection: $feature) {
                            Text(tr("Please choose")).tag(-1)
                            ForEach(cdsList, id: \.0) { Text("\($0.1.name) (\($0.1.start + 1)–\($0.1.end))").tag($0.0) }
                        }.frame(width: 260)
                        Picker(tr("Wanted"), selection: $mode) {
                            Text(tr("All")).tag("all")
                            Text(tr("Creates site")).tag("create")
                            Text(tr("Destroys site")).tag("remove")
                        }.frame(width: 220)
                    }
                    HStack {
                        TextField(tr("Enzyme or DNA sequence, e.g. EcoRI"), text: $site).textFieldStyle(.roundedBorder).frame(width: 260)
                        Button(tr("Search")) { search() }.disabled(feature < 0)
                    }
                    if !changes.isEmpty {
                        Table(changes) {
                            TableColumn(tr("Position")) { c in Text("\(c.dto.position + 1)") }.width(80)
                            TableColumn(tr("Codon")) { c in Text("\(c.dto.codon)") }.width(60)
                            TableColumn(tr("Change")) { c in Text("\(c.dto.from) → \(c.dto.to)") }.width(100)
                            TableColumn("") { c in
                                Button(tr("Apply")) { model.applySilentChange(c.dto); search() }.buttonStyle(.borderless)
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

/// Concatenate several selected sequences.
struct ConcatSection: View {
    @ObservedObject var model: AppModel
    @State private var name = ""
    @State private var circular = false

    var body: some View {
        GroupBox(tr("Concatenate (one after another, order as in the list)")) {
            VStack(alignment: .leading, spacing: 8) {
                Toggle(tr("Circular result"), isOn: $circular)
                HStack {
                    TextField(tr("Name (optional)"), text: $name).textFieldStyle(.roundedBorder).frame(width: 260)
                    Button(tr("Concatenate")) { model.concatenateSelected(name: name, circular: circular) }
                }
                Text(tr("The bases are joined directly, features move along. No overlap – use Gibson for overlapping ends."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(6)
        }
    }
}

/// Compare features of two selected sequences.
struct CompareAnnotationsSection: View {
    @ObservedObject var model: AppModel
    @State private var mode = "overlap"
    @State private var result: AnnotationComparisonDto?
    @State private var namesA: [FeatureInfo] = []
    @State private var namesB: [FeatureInfo] = []

    var body: some View {
        GroupBox(tr("Compare annotations (exactly two sequences)")) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Picker(tr("Equal if"), selection: $mode) {
                        Text(tr("Same type and overlapping")).tag("overlap")
                        Text(tr("Same type, start and end")).tag("exact")
                    }.frame(width: 340)
                    Button(tr("Compare")) { compare() }.disabled(model.multiSelection.count != 2)
                }
                if let r = result {
                    list(tr("Only in the first"), r.onlyA, namesA)
                    list(tr("Only in the second"), r.onlyB, namesB)
                    list(tr("In both (first)"), r.commonA, namesA)
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
            if idx.count > 30 { Text(tr("… and %@ more", "\(idx.count - 30)")).font(.caption).foregroundStyle(.secondary) }
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

/// "Remove columns" dialog for an alignment.
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
            Text(tr("Remove columns")).font(.title2.bold())
            Picker(tr("Which columns"), selection: $kind) {
                Text(tr("With many gaps")).tag("gaps")
                Text(tr("Without difference (constant)")).tag("constant")
                Text(tr("One codon position")).tag("codon")
            }
            .pickerStyle(.segmented)
            switch kind {
            case "gaps":
                Stepper(tr("Gap fraction at least %@ %", "\(Int(gapPercent))"), value: $gapPercent, in: 1...100, step: 5)
            case "codon":
                Picker(tr("Codon position"), selection: $codonPosition) {
                    Text(tr("1st position")).tag(1)
                    Text(tr("2nd position")).tag(2)
                    Text(tr("3rd position (wobble)")).tag(3)
                }
                Stepper(tr("First codon starts at column %@", "\(start)"), value: $start, in: 1...100000)
            default:
                Text(tr("Removes all columns in which all rows have the same base and no gap."))
                    .font(.callout).foregroundStyle(.secondary)
            }
            TextField(tr("Name of the new alignment (optional)"), text: $name).textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button(tr("Cancel"), role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
                Button(tr("Remove")) {
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

/// "Sequence with variants" dialog: reference row plus the differences of another row.
struct ApplyVariantsSheet: View {
    @ObservedObject var model: AppModel
    let rowNames: [String]
    @Environment(\.dismiss) private var dismiss
    @State private var reference = 0
    @State private var row = 1
    @State private var name = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("Sequence with variants")).font(.title2.bold())
            Text(tr("Creates the reference sequence (without gaps) with all substitutions, insertions and deletions of the selected row."))
                .font(.callout).foregroundStyle(.secondary)
            Picker(tr("Reference"), selection: $reference) {
                ForEach(Array(rowNames.enumerated()), id: \.offset) { Text($0.element).tag($0.offset) }
            }
            Picker(tr("Variants of"), selection: $row) {
                ForEach(Array(rowNames.enumerated()), id: \.offset) { Text($0.element).tag($0.offset) }
            }
            TextField(tr("Name (optional)"), text: $name).textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button(tr("Cancel"), role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
                Button(tr("Create")) {
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

/// Menu with enzyme sets: selecting applies the enzymes, custom sets can be saved and deleted.
struct EnzymeSetMenu: View {
    /// Currently selected enzymes (for "Save selection as set").
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
            Button(tr("Save selection as set …")) { showSave = true }.disabled(current.isEmpty)
            let own = sets.filter { !$0.builtin }
            if !own.isEmpty {
                Menu(tr("Delete custom set")) {
                    ForEach(own, id: \.name) { s in Button(s.name) { try? enzymeSetDelete(name: s.name); sets = enzymeSets() } }
                }
            }
        }
        .menuStyle(.borderlessButton).fixedSize()
        .onAppear { sets = enzymeSets() }
        .alert(tr("Save enzyme set"), isPresented: $showSave) {
            TextField(tr("Name"), text: $saveName)
            Button(tr("Save")) {
                do {
                    try enzymeSetSave(name: saveName, enzymes: current.joined(separator: ", "))
                    sets = enzymeSets()
                    saveName = ""
                } catch { self.error = error.localizedDescription }
            }
            Button(tr("Cancel"), role: .cancel) { saveName = "" }
        } message: {
            Text(error ?? tr("%@ enzymes: %@", "\(current.count)", "\(current.joined(separator: ", "))"))
        }
    }
}

/// Consensus from several Sanger reads: align (reverse strand is detected) and decide per column by quality.
struct ReadConsensusSection: View {
    @ObservedObject var model: AppModel
    @State private var minQuality = 20

    var body: some View {
        GroupBox(tr("Consensus from Sanger reads (qualities from the .ab1/.scf files)")) {
            VStack(alignment: .leading, spacing: 8) {
                Stepper(tr("Bases below quality %@ do not count", "\(minQuality)"), value: $minQuality, in: 0...60)
                Button(tr("Build consensus")) { model.consensusOfSelectedReads(minQuality: minQuality) }
                Text(tr("Reads that fit better as the reverse strand are flipped. Without a stored trace every base counts as quality 20. The alignment of the reads is created too."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(6)
        }
    }
}
