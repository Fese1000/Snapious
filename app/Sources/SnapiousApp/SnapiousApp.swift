import SnapiousFFI
import AppKit
import SwiftUI

/// Ask for confirmation on quit when there are unsaved changes to the sequence.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        MainActor.assumeIsolated {
            guard let m = AppModel.current, m.currentDirty else { return .terminateNow }
            let alert = NSAlert()
            alert.messageText = tr("Save changes to “%@”?", "\(m.selectedName)")
            alert.informativeText = tr("Discard reverts all changes since it was opened.")
            alert.addButton(withTitle: tr("Save"))
            alert.addButton(withTitle: tr("Discard"))
            alert.addButton(withTitle: tr("Cancel"))
            switch alert.runModal() {
            case .alertFirstButtonReturn: m.commitEdits(); return .terminateNow
            case .alertSecondButtonReturn: m.discardEdits(); return .terminateNow
            default: return .terminateCancel
            }
        }
    }
}

@main
struct SnapiousApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var model = AppModel()

    init() {
        // Without a window, output views as PNG:  Snapious --snapshot <project> <folder>
        let args = CommandLine.arguments
        if let i = args.firstIndex(of: "--snapshot"), args.count > i + 2 {
            MainActor.assumeIsolated { Snapshot.run(project: args[i + 1], outDir: args[i + 2]) }
            exit(0)
        }
    }

    var body: some Scene {
        WindowGroup("Snapious") {
            ContentView(model: model)
                .id(model.language)
        }
        .defaultSize(width: 1280, height: 820)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button(tr("New project…")) { model.newProject() }
                    .keyboardShortcut("n")
                Button(tr("Open project…")) { model.openProject() }
                    .keyboardShortcut("o")
                Divider()
                Button(tr("Import sequences…")) { model.chooseFilesToImport() }
                    .keyboardShortcut("i")
                    .disabled(model.project == nil)
            }
        }
        Settings {
            Form {
                Picker("Language / Sprache", selection: $model.language) {
                    Text("English").tag("en")
                    Text("Deutsch").tag("de")
                }
                .pickerStyle(.radioGroup)
            }
            .padding(20)
            .frame(width: 320)
        }
    }
}
