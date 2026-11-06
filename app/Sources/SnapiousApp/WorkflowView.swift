import SnapiousFFI
import SwiftUI

/// Workflows: gespeicherte Abfolgen von Operationen, auf die gewählten Dokumente angewendet.
struct WorkflowView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var workflows: [WorkflowDto] = []
    @State private var chosen: String?
    @State private var text = ""
    @State private var operations: [OperationInfoDto] = []
    @State private var message: String?
    @State private var showOperations = false
    @State private var mode = "steps"
    @State private var wfName = "Neuer Workflow"
    @State private var wfDescription = ""
    @State private var steps: [BuilderStep] = []

    /// Ein Schritt im Schritt-Editor (Eingabe: Ergebnis des vorigen Schritts, beim ersten Schritt die erste gewählte Eingabe).
    struct BuilderStep: Identifiable, Equatable {
        let id = UUID()
        var operation: String
        var parameters: String = ""
    }

    private static let template = """
    {
      "name": "Neuer Workflow",
      "description": "Reverse-Komplement, danach Basen einfügen",
      "inputCount": 1,
      "steps": [
        {"id": "rc", "operation": "edit.reverseComplement", "inputs": ["$in:0"]},
        {"id": "ins", "operation": "edit.insert", "parameters": {"position": 1, "bases": "GG"}, "inputs": ["$step:0:0"]}
      ],
      "outputs": ["$step:1:0"]
    }
    """

    var body: some View {
        HSplitView {
            VStack(alignment: .leading) {
                List(workflows, id: \.name, selection: $chosen) { w in
                    VStack(alignment: .leading) {
                        Text(w.name)
                        Text("\(w.stepCount) Schritte, \(w.inputCount) Eingabe(n)").font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
                    }
                }
                Button("Neuer Workflow") { chosen = nil; text = Self.template; stepsFromJSON() }
            }
            .frame(minWidth: 200, maxWidth: 280)
            .padding(8)

            VStack(alignment: .leading, spacing: 8) {
                Picker("", selection: $mode) {
                    Text("Schritte").tag("steps")
                    Text("JSON").tag("json")
                }
                .pickerStyle(.segmented).labelsHidden().frame(width: 220)
                .onChange(of: mode) { _, m in if m == "steps" { stepsFromJSON() } else { text = jsonFromSteps() } }
                if mode == "steps" {
                    stepEditor
                } else {
                    TextEditor(text: $text)
                        .font(Theme.mono(13))
                        .frame(minHeight: 260)
                }
                if let message { Text(message).foregroundStyle(message.hasPrefix("OK") ? Theme.good : Theme.danger) }
                HStack {
                    Button("Speichern") { if mode == "steps" { text = jsonFromSteps() }; save() }.disabled(mode == "steps" ? steps.isEmpty : text.isEmpty)
                    Button("Löschen", role: .destructive) { delete() }.disabled(chosen == nil)
                    Button("Operationen…") { showOperations.toggle() }
                        .popover(isPresented: $showOperations) {
                            ScrollView {
                                VStack(alignment: .leading, spacing: 4) {
                                    ForEach(operations, id: \.id) { o in
                                        HStack { Text(o.id).font(Theme.mono(13)); Text(o.label).foregroundStyle(Theme.inkSoft) }
                                    }
                                }
                                .padding(12)
                            }
                            .frame(width: 440, height: 320)
                        }
                    Spacer()
                }
                HStack {
                    Text("\(model.multiSelection.count) Dokument(e) gewählt").foregroundStyle(Theme.inkSoft)
                    Spacer()
                    Button("Auf Auswahl anwenden") { run() }
                        .disabled(chosen == nil || model.multiSelection.isEmpty)
                    Button("Schließen") { dismiss() }
                }
            }
            .padding(8)
        }
        .frame(minWidth: 1000, idealWidth: 1040, minHeight: 560, idealHeight: 600)
        .onAppear { reload(); stepsFromJSON() }
        .onChange(of: chosen) { _, _ in
            if let c = chosen, let w = workflows.first(where: { $0.name == c }) { text = w.json; stepsFromJSON() }
        }
    }

    /// Schritt-Editor: Operationen hinzufügen, ordnen, Parameter eintragen.
    private var stepEditor: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                TextField("Name", text: $wfName).textFieldStyle(.roundedBorder).frame(width: 240)
                TextField("Beschreibung", text: $wfDescription).textFieldStyle(.roundedBorder)
            }
            Text("Jeder Schritt bekommt das Ergebnis des vorigen Schritts (der erste die erste gewählte Eingabe). Parameter als JSON-Objekt, z. B. {\"position\": 1, \"bases\": \"GG\"}.")
                .font(.caption).foregroundStyle(.secondary)
            List {
                ForEach($steps) { $st in
                    HStack {
                        Text("\((steps.firstIndex(of: st) ?? 0) + 1).").foregroundStyle(.secondary).frame(width: 26, alignment: .trailing)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(operations.first { $0.id == st.operation }?.label ?? st.operation)
                            Text(st.operation).font(.caption.monospaced()).foregroundStyle(.secondary)
                        }
                        .frame(width: 210, alignment: .leading)
                        TextField("Parameter (JSON)", text: $st.parameters).textFieldStyle(.roundedBorder).font(.system(.body, design: .monospaced))
                        Button { move(st, -1) } label: { Image(systemName: "chevron.up") }.buttonStyle(.borderless).accessibilityLabel("Schritt nach oben")
                        Button { move(st, 1) } label: { Image(systemName: "chevron.down") }.buttonStyle(.borderless).accessibilityLabel("Schritt nach unten")
                        Button { steps.removeAll { $0.id == st.id } } label: { Image(systemName: "minus.circle") }.buttonStyle(.borderless).accessibilityLabel("Schritt entfernen")
                    }
                }
            }
            .frame(minHeight: 190)
            Menu("Schritt hinzufügen") {
                ForEach(operations, id: \.id) { o in Button("\(o.label) (\(o.id))") { steps.append(BuilderStep(operation: o.id)) } }
            }
            .menuStyle(.borderlessButton).fixedSize()
        }
    }

    private func move(_ st: BuilderStep, _ d: Int) {
        guard let i = steps.firstIndex(of: st), steps.indices.contains(i + d) else { return }
        steps.swapAt(i, i + d)
    }

    /// Definition (JSON) aus dem Schritt-Editor.
    private func jsonFromSteps() -> String {
        var list: [[String: Any]] = []
        for (i, st) in steps.enumerated() {
            var d: [String: Any] = ["id": "s\(i + 1)", "operation": st.operation, "inputs": [i == 0 ? "$in:0" : "$step:\(i - 1):0"]]
            if let data = st.parameters.data(using: .utf8), !st.parameters.trimmingCharacters(in: .whitespaces).isEmpty,
               let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                d["parameters"] = obj
            }
            list.append(d)
        }
        let root: [String: Any] = ["name": wfName, "description": wfDescription, "inputCount": 1, "steps": list, "outputs": steps.isEmpty ? [] : ["$step:\(steps.count - 1):0"]]
        guard let data = try? JSONSerialization.data(withJSONObject: root, options: [.prettyPrinted, .sortedKeys]) else { return "" }
        return String(data: data, encoding: .utf8) ?? ""
    }

    /// Schritte aus der JSON-Definition (einfache, geradlinige Workflows).
    private func stepsFromJSON() {
        guard let data = text.data(using: .utf8), let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
        wfName = root["name"] as? String ?? wfName
        wfDescription = root["description"] as? String ?? ""
        steps = (root["steps"] as? [[String: Any]] ?? []).map { st in
            var p = ""
            if let obj = st["parameters"], let d = try? JSONSerialization.data(withJSONObject: obj), let t = String(data: d, encoding: .utf8) { p = t }
            return BuilderStep(operation: st["operation"] as? String ?? "", parameters: p)
        }
    }

    private func reload() {
        guard let project = model.project else { return }
        operations = project.operationList()
        workflows = (try? project.workflowList()) ?? []
    }

    private func save() {
        guard let project = model.project else { return }
        do {
            let name = try project.workflowSave(json: text)
            message = "OK: „\(name)“ gespeichert"
            reload()
            chosen = name
        } catch {
            message = error.localizedDescription
        }
    }

    private func delete() {
        guard let project = model.project, let c = chosen else { return }
        try? project.workflowDelete(name: c)
        chosen = nil
        text = ""
        reload()
    }

    private func run() {
        guard let project = model.project, let c = chosen else { return }
        let ids = model.documents.filter { model.multiSelection.contains($0.id) }.map { $0.id }
        do {
            let r = try project.workflowRun(name: c, inputIds: ids)
            model.reload(selecting: r.documents.last?.id)
            message = r.errors.isEmpty
                ? "OK: \(r.documents.count) Ergebnis(se) im Projekt"
                : r.errors.joined(separator: "\n") + (r.documents.isEmpty ? "" : "\n(\(r.documents.count) Ergebnis(se) trotzdem gespeichert)")
        } catch {
            message = error.localizedDescription
        }
    }
}
