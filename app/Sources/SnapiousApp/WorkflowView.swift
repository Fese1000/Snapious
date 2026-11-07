import SnapiousFFI
import SwiftUI

/// Workflows: saved sequences of operations, applied to the selected documents.
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
    @State private var wfName = tr("New workflow")
    @State private var wfDescription = ""
    @State private var steps: [BuilderStep] = []

    /// A step in the step editor (input: result of the previous step; for the first step the first selected input).
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
                        Text(tr("%@ steps, %@ input(s)", "\(w.stepCount)", "\(w.inputCount)")).font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
                    }
                }
                Button(tr("New workflow")) { chosen = nil; text = Self.template; stepsFromJSON() }
            }
            .frame(minWidth: 200, maxWidth: 280)
            .padding(8)

            VStack(alignment: .leading, spacing: 8) {
                Picker("", selection: $mode) {
                    Text(tr("Steps")).tag("steps")
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
                if let message { Text(message).foregroundStyle(message.hasPrefix(tr("OK")) ? Theme.good : Theme.danger) }
                HStack {
                    Button(tr("Save")) { if mode == "steps" { text = jsonFromSteps() }; save() }.disabled(mode == "steps" ? steps.isEmpty : text.isEmpty)
                    Button(tr("Delete"), role: .destructive) { delete() }.disabled(chosen == nil)
                    Button(tr("Operations…")) { showOperations.toggle() }
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
                    Text(tr("%@ document(s) selected", "\(model.multiSelection.count)")).foregroundStyle(Theme.inkSoft)
                    Spacer()
                    Button(tr("Apply to selection")) { run() }
                        .disabled(chosen == nil || model.multiSelection.isEmpty)
                    Button(tr("Close")) { dismiss() }
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

    /// Step editor: add operations, reorder, enter parameters.
    private var stepEditor: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                TextField(tr("Name"), text: $wfName).textFieldStyle(.roundedBorder).frame(width: 240)
                TextField(tr("Description"), text: $wfDescription).textFieldStyle(.roundedBorder)
            }
            Text(tr("Each step receives the result of the previous step (the first one the first selected input). Parameters as a JSON object, e.g. {\"position\": 1, \"bases\": \"GG\"}."))
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
                        TextField(tr("Parameters (JSON)"), text: $st.parameters).textFieldStyle(.roundedBorder).font(.system(.body, design: .monospaced))
                        Button { move(st, -1) } label: { Image(systemName: "chevron.up") }.buttonStyle(.borderless).accessibilityLabel(tr("Step up"))
                        Button { move(st, 1) } label: { Image(systemName: "chevron.down") }.buttonStyle(.borderless).accessibilityLabel(tr("Step down"))
                        Button { steps.removeAll { $0.id == st.id } } label: { Image(systemName: "minus.circle") }.buttonStyle(.borderless).accessibilityLabel(tr("Remove step"))
                    }
                }
            }
            .frame(minHeight: 190)
            Menu(tr("Add step")) {
                ForEach(operations, id: \.id) { o in Button("\(o.label) (\(o.id))") { steps.append(BuilderStep(operation: o.id)) } }
            }
            .menuStyle(.borderlessButton).fixedSize()
        }
    }

    private func move(_ st: BuilderStep, _ d: Int) {
        guard let i = steps.firstIndex(of: st), steps.indices.contains(i + d) else { return }
        steps.swapAt(i, i + d)
    }

    /// Definition (JSON) from the step editor.
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

    /// Steps from the JSON definition (simple, linear workflows).
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
            message = tr("OK: “%@” saved", "\(name)")
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
                ? tr("OK: %@ result(s) in the project", "\(r.documents.count)")
                : r.errors.joined(separator: "\n") + (r.documents.isEmpty ? "" : tr("\n(%@ result(s) saved anyway)", "\(r.documents.count)"))
        } catch {
            message = error.localizedDescription
        }
    }
}
