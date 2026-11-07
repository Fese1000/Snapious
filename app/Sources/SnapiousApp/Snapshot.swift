import AppKit
import SnapiousFFI
import SwiftUI

/// Renders views without a window into PNG files (to check the UI without screen access).
/// Usage:  Snapious --snapshot <project file> <output folder>
@MainActor
enum Snapshot {
    static func run(project: String, outDir: String) {
        try? FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)
        let model = AppModel()
        model.open(path: project)
        func render<V: View>(_ name: String, width: CGFloat, height: CGFloat, @ViewBuilder _ view: () -> V) {
            let renderer = ImageRenderer(content: view().frame(width: width, height: height).background(Color(nsColor: .windowBackgroundColor)))
            renderer.scale = 2
            guard let image = renderer.nsImage, let tiff = image.tiffRepresentation,
                  let rep = NSBitmapImageRep(data: tiff), let png = rep.representation(using: .png, properties: [:]) else {
                print(tr("No graphic for %@", "\(name)"))
                return
            }
            try? png.write(to: URL(fileURLWithPath: outDir).appendingPathComponent("\(name).png"))
            print(tr("written: %@.png", "\(name)"))
        }
        if let pdb = try? pdbOpen(path: "/tmp/gen/test.pdb") {
            render("struktur", width: 560, height: 420) { StructureView(pdb: pdb, name: "test.pdb").chart }
        }
        if let q = try? qcFastq(path: "/tmp/gen/test.fastq", maxReads: 0) {
            render("qc", width: 700, height: 330) { QcSummary(qc: q, name: "test.fastq").padding(12) }
        }
        for doc in model.documents {
            model.selection = doc.id
            switch doc.docType {
            case "sequence":
                if doc.name.contains("PUC19") {
                    render("sequenz", width: 1100, height: 520) { SequenceView(model: model).snapshot(width: 1100, lines: 6) }
                    render("karte", width: 700, height: 700) { MapView(model: model) }
                    render("analyse", width: 1000, height: 520) { AnalysisView(model: model) }
                    render("protein", width: 900, height: 520) { ProteinView(model: model) }
                    render("verdau", width: 1000, height: 600) { DigestView(model: model) }
                }
            case "alignment":
                render("alignment", width: 900, height: 300) { AlignmentView(model: model).snapshotCanvas() }
                if let m = try? model.project?.alignmentIdentityMatrix(id: doc.id) {
                    render("heatmap", width: 520, height: 260) { IdentityHeatmapView(matrix: m).grid }
                }
                if let l = try? model.project?.alignmentLogo(id: doc.id) {
                    render("logo", width: 900, height: 220) { SequenceLogoView(columns: l, protein: false).chart }
                }
            default: break
            }
        }
    }
}
