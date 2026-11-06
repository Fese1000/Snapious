import SnapiousFFI
import SwiftUI

@main
struct SnapiousApp: App {
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
        }
        .defaultSize(width: 1280, height: 820)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Neues Projekt…") { model.newProject() }
                    .keyboardShortcut("n")
                Button("Projekt öffnen…") { model.openProject() }
                    .keyboardShortcut("o")
                Divider()
                Button("Sequenzen importieren…") { model.chooseFilesToImport() }
                    .keyboardShortcut("i")
                    .disabled(model.project == nil)
            }
        }
    }
}
