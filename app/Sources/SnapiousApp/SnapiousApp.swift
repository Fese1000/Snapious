import SnapiousFFI
import AppKit
import SwiftUI

/// Beim Beenden mit offenen Änderungen an der Sequenz nachfragen.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        MainActor.assumeIsolated {
            guard let m = AppModel.current, m.currentDirty else { return .terminateNow }
            let alert = NSAlert()
            alert.messageText = tr("Änderungen an „%@“ speichern?", "\(m.selectedName)")
            alert.informativeText = tr("Verwerfen nimmt alle Änderungen seit dem Öffnen zurück.")
            alert.addButton(withTitle: tr("Speichern"))
            alert.addButton(withTitle: tr("Verwerfen"))
            alert.addButton(withTitle: tr("Abbrechen"))
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
        // Ohne Fenster Ansichten als PNG ausgeben:  Snapious --snapshot <Projekt> <Ordner>
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
                Button(tr("Neues Projekt…")) { model.newProject() }
                    .keyboardShortcut("n")
                Button(tr("Projekt öffnen…")) { model.openProject() }
                    .keyboardShortcut("o")
                Divider()
                Button(tr("Sequenzen importieren…")) { model.chooseFilesToImport() }
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
