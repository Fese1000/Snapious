import SnapiousFFI
import SwiftUI

/// Virtual restriction digest: choose enzymes, see fragments with ends and sizes.
struct DigestView: View {
    @ObservedObject var model: AppModel
    @State private var search = ""
    @State private var chosen: Set<String> = []
    @State private var fragments: [FragmentRow] = []
    @State private var message: String?
    @State private var agarose = 1.0
    @State private var showAnnotation = false
    @State private var annotType = ""
    @State private var atStart = true
    @State private var atEnd = true

    private var filtered: [EnzymeInfoDto] {
        let q = search.trimmingCharacters(in: .whitespaces).lowercased()
        return q.isEmpty ? model.enzymes : model.enzymes.filter { $0.name.lowercased().contains(q) || $0.recognition.lowercased().contains(q) }
    }

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            VStack(alignment: .leading, spacing: 8) {
                TextField(tr("Find enzyme"), text: $search)
                    .textFieldStyle(.roundedBorder)
                List(filtered, id: \.name) { e in
                    Toggle(isOn: Binding(
                        get: { chosen.contains(e.name) },
                        set: { on in
                            if on { chosen.insert(e.name) } else { chosen.remove(e.name) }
                            run()
                        }
                    )) {
                        HStack {
                            Text(e.name)
                            Spacer()
                            Text(e.recognition).font(.system(.caption, design: .monospaced)).foregroundStyle(Theme.inkSoft)
                        }
                    }
                }
                Button(tr("Cut by annotation …")) { showAnnotation = true }
                    .popover(isPresented: $showAnnotation) {
                        VStack(alignment: .leading, spacing: 10) {
                            Text(tr("Cut at annotations")).font(.headline)
                            Picker(tr("Feature type"), selection: $annotType) {
                                Text(tr("All types")).tag("")
                                ForEach(Array(Set(model.features.map { $0.featureType })).sorted(), id: \.self) { Text($0).tag($0) }
                            }
                            Toggle(tr("At the start of each feature"), isOn: $atStart)
                            Toggle(tr("At the end of each feature"), isOn: $atEnd)
                            HStack {
                                Spacer()
                                Button(tr("Cut")) { cutByAnnotation(); showAnnotation = false }
                                    .keyboardShortcut(.defaultAction)
                                    .disabled(!atStart && !atEnd)
                            }
                        }
                        .padding(14)
                        .frame(width: 280)
                    }
                HStack {
                    Text(tr("%@ enzymes selected", "\(chosen.count)")).font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
                    Spacer()
                    EnzymeSetMenu(current: chosen.sorted()) { names in
                        chosen = Set(names.filter { n in model.enzymes.contains { $0.name == n } })
                        run()
                    }
                }
            }
            .padding(8)
            .frame(width: 230)
            Divider()
            VStack(alignment: .leading, spacing: 8) {
                if let message {
                    Text(message).foregroundStyle(Theme.danger)
                }
                if fragments.isEmpty {
                    ContentUnavailableView(tr("No digest"), systemImage: "scissors",
                                           description: Text(tr("Select one or more enzymes on the left.")))
                } else {
                    gel
                        .frame(height: 232)
                    Table(fragments) {
                        TableColumn(tr("No.")) { f in Text("\(f.index)") }.width(40)
                        TableColumn(tr("Length")) { f in Text("\(f.dto.length) bp") }.width(90)
                        TableColumn(tr("Range")) { f in Text("\(f.dto.start + 1)–\(f.dto.end)") }
                        TableColumn(tr("Left end"), value: \.dto.leftEnd)
                        TableColumn(tr("Right end"), value: \.dto.rightEnd)
                    }
                }
            }
            .padding(8)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onChange(of: model.selection) { _, _ in
            chosen = []
            fragments = []
            message = nil
        }
    }

    /// Gel with size standard (left) and the digest lane (right).
    private var gel: some View {
        let ladder = (try? gelLadder(name: "generic_1kb", agarosePercent: agarose)) ?? []
        let lane = gelLane(sizes: fragments.map { $0.dto.length }, agarosePercent: agarose, massNg: 100)
        return VStack(alignment: .leading, spacing: 6) {
            Canvas { ctx, size in
                ctx.fill(Path(roundedRect: CGRect(origin: .zero, size: size), cornerRadius: 6), with: .color(.black.opacity(0.85)))
                let top: CGFloat = 12, height = size.height - 24
                func draw(_ bands: [GelBandDto], x: CGFloat, width: CGFloat, labels: Bool) {
                    for b in bands {
                        let y = top + CGFloat(b.mobility) * height
                        let rect = CGRect(x: x, y: y - 2, width: width, height: 4)
                        ctx.fill(Path(roundedRect: rect, cornerRadius: 2), with: .color(.white.opacity(0.25 + 0.7 * b.intensity)))
                        if labels {
                            ctx.draw(Text("\(b.sizeBp)").font(.system(size: 9)).foregroundStyle(.white.opacity(0.8)),
                                     at: CGPoint(x: x - 4, y: y), anchor: .trailing)
                        }
                    }
                }
                draw(ladder, x: 60, width: 60, labels: true)
                draw(lane, x: 150, width: 60, labels: false)
                for b in lane {
                    ctx.draw(Text("\(b.sizeBp)").font(.system(size: 9)).foregroundStyle(.white.opacity(0.8)),
                             at: CGPoint(x: 216, y: top + CGFloat(b.mobility) * height), anchor: .leading)
                }
            }
            .frame(width: 280, height: 200)
            HStack {
                Text(tr("Agarose %@ %", "\(String(format: "%.2f", agarose))")).font(Theme.label(11))
                Slider(value: $agarose, in: 0.5...3.0, step: 0.25).frame(width: 140)
            }
        }
    }

    /// Fragments when cutting at the starts/ends of features (replaces the enzyme selection).
    private func cutByAnnotation() {
        message = nil
        guard let project = model.project, let id = model.selection else { return }
        chosen = []
        do {
            let result = try project.fragmentsByAnnotation(id: id, featureType: annotType, atStart: atStart, atEnd: atEnd)
            fragments = result.enumerated().map { FragmentRow(index: $0.offset + 1, dto: $0.element) }
            if fragments.isEmpty { message = tr("No features of this type.") }
        } catch {
            fragments = []
            message = error.localizedDescription
        }
    }

    private func run() {
        message = nil
        guard let project = model.project, let id = model.selection, !chosen.isEmpty else {
            fragments = []
            return
        }
        do {
            let result = try project.digestDocument(id: id, enzymes: chosen.sorted())
            fragments = result.enumerated().map { FragmentRow(index: $0.offset + 1, dto: $0.element) }
        } catch {
            fragments = []
            message = error.localizedDescription
        }
    }
}

struct FragmentRow: Identifiable {
    let index: Int
    let dto: FragmentDto
    var id: Int { index }
}
