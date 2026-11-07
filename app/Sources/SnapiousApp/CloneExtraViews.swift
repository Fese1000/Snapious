import SnapiousFFI
import SwiftUI

/// TA and TOPO cloning: place the insert directly after the linear vector.
struct TopoCloneView: View {
    @ObservedObject var model: AppModel
    @State private var vectorID = ""
    @State private var insertID = ""
    @State private var message: String?

    private var sequences: [DocumentInfoDto] { model.documents.filter { $0.docType == "sequence" } }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text(tr("TA: the PCR product has 3′-A overhangs from Taq polymerase and fits the T-vector in both orientations. Directional TOPO: the insert must start with CACC and goes in one orientation only. Vector and insert must be linear."))
                    .font(.callout).foregroundStyle(.secondary)
                GroupBox(tr("Documents")) {
                    VStack(alignment: .leading, spacing: 8) {
                        Picker(tr("Linear (T or TOPO) vector"), selection: $vectorID) {
                            Text(tr("Please choose")).tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 440)
                        Picker(tr("Insert (PCR product)"), selection: $insertID) {
                            Text(tr("Please choose")).tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 440)
                    }
                }
                HStack {
                    Button(tr("TA cloning (both orientations)")) { runTA() }
                        .buttonStyle(.borderedProminent)
                        .disabled(vectorID.isEmpty || insertID.isEmpty)
                    Button(tr("Directional TOPO (CACC)")) { runTopo() }
                        .disabled(vectorID.isEmpty || insertID.isEmpty)
                }
                if let message { Text(message).foregroundStyle(message.hasPrefix(tr("Done")) ? Color.green : Color.red) }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func runTA() {
        guard let project = model.project else { return }
        do {
            let r = try project.cloneTa(tVectorId: vectorID, pcrProductId: insertID)
            model.reload(selecting: r.first?.id)
            message = tr("Done: %@ constructs created (orientation + and −).", "\(r.count)")
        } catch { message = error.localizedDescription }
    }

    private func runTopo() {
        guard let project = model.project else { return }
        do {
            let r = try project.cloneTopo(vectorId: vectorID, insertId: insertID)
            model.reload(selecting: r.id)
            message = tr("Done: construct created.")
        } catch { message = error.localizedDescription }
    }
}

/// Gateway cloning (BP and LR reaction) with att sites of the kit entered manually.
struct GatewayCloneView: View {
    @ObservedObject var model: AppModel
    @State private var kind = "bp"
    @State private var firstID = ""
    @State private var secondID = ""
    @State private var att1 = GatewayFields(name: "1")
    @State private var att2 = GatewayFields(name: "2")
    @State private var message: String?

    struct GatewayFields: Codable, Equatable {
        var name: String
        var leftB = "", core = "", rightB = "", leftP = "", rightP = ""
    }

    private var sequences: [DocumentInfoDto] { model.documents.filter { $0.docType == "sequence" } }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text(tr("The att sites differ between kits. Enter them from the manual (capital letters, no spaces); Snapious remembers them. attB = left B + core + right B, attP = left P + core + right P; attL and attR arise from these in the reaction."))
                    .font(.callout).foregroundStyle(.secondary)
                Picker(tr("Reaction"), selection: $kind) {
                    Text(tr("BP (attB product + donor → entry clone)")).tag("bp")
                    Text(tr("LR (entry clone + destination vector → expression clone)")).tag("lr")
                }
                .pickerStyle(.segmented)
                GroupBox(tr("Documents")) {
                    VStack(alignment: .leading, spacing: 8) {
                        Picker(kind == "bp" ? tr("Linear product with attB1 … attB2") : tr("Entry clone (circular, attL1 … attL2)"), selection: $firstID) {
                            Text(tr("Please choose")).tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 520)
                        Picker(kind == "bp" ? tr("Donor vector (circular, attP1 … attP2)") : tr("Destination vector (circular, attR1 … attR2)"), selection: $secondID) {
                            Text(tr("Please choose")).tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 520)
                    }
                }
                siteBox(tr("att site 1"), $att1)
                siteBox(tr("att site 2"), $att2)
                HStack {
                    Button(kind == "bp" ? tr("Run BP reaction") : tr("Run LR reaction")) { run() }
                        .buttonStyle(.borderedProminent)
                        .disabled(firstID.isEmpty || secondID.isEmpty)
                }
                if let message { Text(message).foregroundStyle(message.hasPrefix(tr("Done")) ? Color.green : Color.red) }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .onAppear { load() }
    }

    private func siteBox(_ title: String, _ f: Binding<GatewayFields>) -> some View {
        GroupBox(title) {
            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
                GridRow { Text(tr("Name")); TextField("", text: f.name).frame(width: 120) }
                GridRow { Text(tr("Left B")); field(f.leftB) }
                GridRow { Text(tr("Core")); field(f.core) }
                GridRow { Text(tr("Right B")); field(f.rightB) }
                GridRow { Text(tr("Left P")); field(f.leftP) }
                GridRow { Text(tr("Right P")); field(f.rightP) }
            }
            .textFieldStyle(.roundedBorder)
        }
    }

    private func field(_ b: Binding<String>) -> some View {
        TextField("", text: b).font(.system(.body, design: .monospaced)).frame(width: 420)
    }

    private func run() {
        guard let project = model.project else { return }
        save()
        func dto(_ f: GatewayFields) -> GatewaySiteDto {
            GatewaySiteDto(name: f.name, leftArmB: f.leftB, core: f.core, rightArmB: f.rightB, leftArmP: f.leftP, rightArmP: f.rightP)
        }
        do {
            let r = try project.cloneGateway(kind: kind, firstId: firstID, secondId: secondID, att1: dto(att1), att2: dto(att2))
            model.reload(selecting: r.id)
            message = tr("Done: construct created.")
        } catch { message = error.localizedDescription }
    }

    private func save() {
        if let data = try? JSONEncoder().encode([att1, att2]) { UserDefaults.standard.set(data, forKey: "gatewaySites") }
    }

    private func load() {
        if let data = UserDefaults.standard.data(forKey: "gatewaySites"),
           let list = try? JSONDecoder().decode([GatewayFields].self, from: data), list.count == 2 {
            att1 = list[0]
            att2 = list[1]
        }
    }
}
