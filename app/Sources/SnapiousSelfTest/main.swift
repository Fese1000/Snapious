import Foundation
import SnapiousFFI

// Selbsttest der Verbindung Swift ↔ Rust (ohne Fenster).
// Aufruf:  SnapiousSelfTest [Pfad/zu/mini.gb]
if CommandLine.arguments.count >= 4, CommandLine.arguments[1] == "--make-demo" {
    // SnapiousSelfTest --make-demo <Projektdatei> <Beispieldateien…>: legt ein Demo-Projekt an (zum Ansehen der Oberfläche)
    do {
        let project = try Project.open(path: CommandLine.arguments[2])
        var ids: [String] = []
        for file in CommandLine.arguments.dropFirst(3) {
            ids += try project.importFile(path: file).map { $0.id }
        }
        let seqs = Array(ids.suffix(3))
        if seqs.count == 3 {
            _ = try project.alignDocuments(ids: seqs, name: "Demo-Alignment", backend: "builtin")
        }
        print("Demo-Projekt angelegt")
        exit(0)
    } catch {
        print("Fehler: \(error)")
        exit(1)
    }
}
exit(SelfTest.run(arguments: ["--selftest"] + Array(CommandLine.arguments.dropFirst())))
