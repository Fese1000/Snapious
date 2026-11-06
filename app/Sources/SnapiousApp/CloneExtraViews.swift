import SnapiousFFI
import SwiftUI

/// TA- und TOPO-Klonierung: Insert direkt hinter den linearen Vektor setzen.
struct TopoCloneView: View {
    @ObservedObject var model: AppModel
    @State private var vectorID = ""
    @State private var insertID = ""
    @State private var message: String?

    private var sequences: [DocumentInfoDto] { model.documents.filter { $0.docType == "sequence" } }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text("TA: Das PCR-Produkt hat durch die Taq-Polymerase 3′-A-Überhänge und passt in beiden Orientierungen in den T-Vektor. Gerichtetes TOPO: Das Insert muss mit CACC beginnen und wird in genau einer Orientierung eingesetzt. Vektor und Insert müssen linear sein.")
                    .font(.callout).foregroundStyle(.secondary)
                GroupBox("Dokumente") {
                    VStack(alignment: .leading, spacing: 8) {
                        Picker("Linearer (T- oder TOPO-)Vektor", selection: $vectorID) {
                            Text("Bitte wählen").tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 440)
                        Picker("Insert (PCR-Produkt)", selection: $insertID) {
                            Text("Bitte wählen").tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 440)
                    }
                }
                HStack {
                    Button("TA-Klonierung (beide Orientierungen)") { runTA() }
                        .buttonStyle(.borderedProminent)
                        .disabled(vectorID.isEmpty || insertID.isEmpty)
                    Button("Gerichtetes TOPO (CACC)") { runTopo() }
                        .disabled(vectorID.isEmpty || insertID.isEmpty)
                }
                if let message { Text(message).foregroundStyle(message.hasPrefix("Fertig") ? Color.green : Color.red) }
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
            message = "Fertig: \(r.count) Konstrukte angelegt (Orientierung + und −)."
        } catch { message = error.localizedDescription }
    }

    private func runTopo() {
        guard let project = model.project else { return }
        do {
            let r = try project.cloneTopo(vectorId: vectorID, insertId: insertID)
            model.reload(selecting: r.id)
            message = "Fertig: Konstrukt angelegt."
        } catch { message = error.localizedDescription }
    }
}

/// Gateway-Klonierung (BP- und LR-Reaktion) mit selbst eingegebenen att-Stellen des Kits.
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
                Text("Die att-Stellen unterscheiden sich je nach Kit. Trage sie aus dem Handbuch ein (Großbuchstaben, ohne Leerzeichen); Snapious merkt sie sich. Es gilt attB = links B + Kern + rechts B, attP = links P + Kern + rechts P; attL und attR entstehen daraus bei der Reaktion.")
                    .font(.callout).foregroundStyle(.secondary)
                Picker("Reaktion", selection: $kind) {
                    Text("BP (attB-Produkt + Spender → Entry-Klon)").tag("bp")
                    Text("LR (Entry-Klon + Zielvektor → Expressionsklon)").tag("lr")
                }
                .pickerStyle(.segmented)
                GroupBox("Dokumente") {
                    VStack(alignment: .leading, spacing: 8) {
                        Picker(kind == "bp" ? "Lineares Produkt mit attB1 … attB2" : "Entry-Klon (zirkulär, attL1 … attL2)", selection: $firstID) {
                            Text("Bitte wählen").tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 520)
                        Picker(kind == "bp" ? "Spendervektor (zirkulär, attP1 … attP2)" : "Zielvektor (zirkulär, attR1 … attR2)", selection: $secondID) {
                            Text("Bitte wählen").tag("")
                            ForEach(sequences, id: \.id) { Text($0.name).tag($0.id) }
                        }
                        .frame(width: 520)
                    }
                }
                siteBox("att-Stelle 1", $att1)
                siteBox("att-Stelle 2", $att2)
                HStack {
                    Button(kind == "bp" ? "BP-Reaktion ausführen" : "LR-Reaktion ausführen") { run() }
                        .buttonStyle(.borderedProminent)
                        .disabled(firstID.isEmpty || secondID.isEmpty)
                }
                if let message { Text(message).foregroundStyle(message.hasPrefix("Fertig") ? Color.green : Color.red) }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .onAppear { load() }
    }

    private func siteBox(_ title: String, _ f: Binding<GatewayFields>) -> some View {
        GroupBox(title) {
            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
                GridRow { Text("Name"); TextField("", text: f.name).frame(width: 120) }
                GridRow { Text("Links B"); field(f.leftB) }
                GridRow { Text("Kern"); field(f.core) }
                GridRow { Text("Rechts B"); field(f.rightB) }
                GridRow { Text("Links P"); field(f.leftP) }
                GridRow { Text("Rechts P"); field(f.rightP) }
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
            message = "Fertig: Konstrukt angelegt."
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
