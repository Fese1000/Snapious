import SnapiousFFI
import SwiftUI

private let annotationTypes = ["misc_feature", "gene", "CDS", "promoter", "terminator", "rep_origin", "primer_bind", "protein_bind", "misc_binding", "regulatory", "sig_peptide", "mat_peptide"]

/// Annotationsliste neben Sequenz und Karte: ansehen, anspringen, anlegen, ändern, löschen, nach Typ ein-/ausblenden.
struct AnnotationPanel: View {
    @ObservedObject var model: AppModel
    @State private var editing: Int?
    @State private var showNew = false

    private var types: [String] { Array(Set(model.features.map { $0.featureType })).sorted() }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(tr("Annotationen")).font(Theme.heading())
                Text("\(model.features.count)").foregroundStyle(Theme.inkSoft)
                Spacer()
                Menu {
                    ForEach(types, id: \.self) { t in
                        Toggle(t, isOn: Binding(get: { !model.hiddenTypes.contains(t) },
                                                set: { if $0 { model.hiddenTypes.remove(t) } else { model.hiddenTypes.insert(t) } }))
                    }
                    Divider()
                    Button(tr("Alle zeigen")) { model.hiddenTypes = [] }
                } label: { Image(systemName: model.hiddenTypes.isEmpty ? "eye" : "eye.slash") }
                    .menuStyle(.borderlessButton).fixedSize().help(tr("Typen ein- und ausblenden"))
            }
            .padding(10)
            Divider()
            if model.features.isEmpty {
                EmptyState(icon: "tag", title: tr("Keine Annotationen"), message: tr("Bereich wählen und „Neu aus Auswahl“ drücken."))
                    .frame(maxHeight: .infinity)
            } else {
                List {
                    ForEach(Array(model.features.enumerated()), id: \.offset) { i, f in
                        row(i, f)
                            .listRowBackground(model.selectedFeature == i ? Theme.accent.opacity(0.2) : Color.clear)
                    }
                }
                .listStyle(.plain)
            }
            Divider()
            HStack {
                Button { showNew = true } label: { Label(tr("Neu aus Auswahl"), systemImage: "plus") }
                    .disabled(model.selStart == nil)
                    .popover(isPresented: $showNew) {
                        let a = min(model.selStart ?? 0, model.selEnd ?? 0), b = max(model.selStart ?? 0, model.selEnd ?? 0)
                        AnnotationForm(title: tr("Neue Annotation"), initial: FeatureInfo(featureType: "misc_feature", name: "", start: UInt64(a), end: UInt64(b == a ? min(a + 1, model.docLength) : b), strand: "FORWARD"),
                                       length: model.docLength) { info in
                            model.addAnnotation(info); showNew = false
                        }
                    }
                Spacer()
            }
            .padding(8)
        }
        .frame(width: 270)
        .background(.bar)
    }

    private func row(_ i: Int, _ f: FeatureInfo) -> some View {
        HStack(spacing: 8) {
            Circle().fill(Theme.feature(f.featureType)).frame(width: 9, height: 9)
                .opacity(model.hiddenTypes.contains(f.featureType) ? 0.25 : 1)
            VStack(alignment: .leading, spacing: 1) {
                Text(f.name.isEmpty ? f.featureType : f.name).lineLimit(1)
                Text("\(f.featureType) · \(f.start + 1)–\(f.end) · \(f.strand == "FORWARD" ? "+" : f.strand == "REVERSE" ? "−" : "·")")
                    .font(Theme.label(11)).foregroundStyle(Theme.inkSoft).lineLimit(1)
            }
            Spacer()
            Button { editing = i } label: { Image(systemName: "pencil") }.buttonStyle(.plain).help(tr("Ändern"))
                .popover(isPresented: Binding(get: { editing == i }, set: { if !$0 { editing = nil } })) {
                    AnnotationForm(title: tr("Annotation ändern"), initial: f, length: model.docLength) { info in
                        model.updateAnnotation(index: i, to: info); editing = nil
                    }
                }
            Button { model.removeFeature(index: i) } label: { Image(systemName: "trash") }.buttonStyle(.plain).help(tr("Löschen"))
        }
        .contentShape(Rectangle())
        .onTapGesture { model.focusFeature(i) }
    }
}

/// Eingabemaske für Name, Typ, Strang und Bereich (angezeigt ab 1, einschließlich Ende).
struct AnnotationForm: View {
    let title: String
    let initial: FeatureInfo
    let length: Int
    let onSave: (FeatureInfo) -> Void
    @State private var name = ""
    @State private var type = "misc_feature"
    @State private var strand = "FORWARD"
    @State private var from = ""
    @State private var to = ""

    private var valid: Bool {
        guard let a = Int(from), let b = Int(to) else { return false }
        return a >= 1 && b >= a && b <= length
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(Theme.heading())
            TextField(tr("Name"), text: $name).textFieldStyle(.roundedBorder).frame(width: 240)
            Picker(tr("Typ"), selection: $type) {
                ForEach(Array(Set(annotationTypes + [initial.featureType])).sorted(), id: \.self) { Text($0).tag($0) }
            }
            Picker(tr("Strang"), selection: $strand) {
                Text("+").tag("FORWARD"); Text("−").tag("REVERSE"); Text("ohne").tag("NONE")
            }.pickerStyle(.segmented)
            HStack {
                TextField("von", text: $from).textFieldStyle(.roundedBorder).frame(width: 80)
                Text("–")
                TextField("bis", text: $to).textFieldStyle(.roundedBorder).frame(width: 80)
                Text(tr("von %@", "\(length)")).foregroundStyle(Theme.inkSoft)
            }
            HStack {
                Spacer()
                Button(tr("Speichern")) {
                    onSave(FeatureInfo(featureType: type, name: name.isEmpty ? type : name, start: UInt64((Int(from) ?? 1) - 1), end: UInt64(Int(to) ?? 1), strand: strand))
                }
                .keyboardShortcut(.defaultAction).disabled(!valid)
            }
        }
        .padding(14)
        .onAppear {
            name = initial.name; type = initial.featureType; strand = initial.strand
            from = "\(initial.start + 1)"; to = "\(initial.end)"
        }
    }
}
