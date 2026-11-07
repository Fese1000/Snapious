import AppKit
import SnapiousFFI
import SwiftUI

/// 3D-Ansicht einer PDB-Datei als C-alpha-Spur (ohne Netz und ohne WebView): ziehen dreht, Scrollen zoomt.
struct StructureView: View {
    let pdb: PdbDto
    let name: String
    @Environment(\.dismiss) private var dismiss
    @State private var yaw = 0.4
    @State private var pitch = 0.3
    @State private var zoom = 1.0
    @State private var dragStart: (Double, Double)?

    private let palette: [Color] = [.blue, .orange, .green, .red, .purple, .teal, .pink, .brown]

    /// Mittelpunkt und größter Abstand aller C-alpha-Atome, damit die Spur in die Zeichenfläche passt.
    private var bounds: (center: [Double], radius: Double) {
        var sum = [0.0, 0.0, 0.0]
        var n = 0.0
        for c in pdb.chains {
            for i in stride(from: 0, to: c.ca.count - 2, by: 3) {
                for k in 0..<3 { sum[k] += c.ca[i + k] }
                n += 1
            }
        }
        for i in stride(from: 0, to: pdb.atoms.count - 2, by: 3) {
            for k in 0..<3 { sum[k] += pdb.atoms[i + k] }
            n += 1
        }
        guard n > 0 else { return ([0, 0, 0], 1) }
        let center = sum.map { $0 / n }
        var r = 1.0
        for i in stride(from: 0, to: pdb.atoms.count - 2, by: 3) {
            let d = (0..<3).map { pdb.atoms[i + $0] - center[$0] }
            r = max(r, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).squareRoot())
        }
        for c in pdb.chains {
            for i in stride(from: 0, to: c.ca.count - 2, by: 3) {
                let d = (0..<3).map { c.ca[i + $0] - center[$0] }
                r = max(r, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).squareRoot())
            }
        }
        return (center, r)
    }

    var chart: some View {
        let b = bounds
        return Canvas { ctx, size in
            let scale = min(size.width, size.height) / 2 / b.radius * 0.9 * zoom
            let cy = cos(yaw), sy = sin(yaw), cp = cos(pitch), sp = sin(pitch)
            func project(_ p: [Double]) -> (CGPoint, Double) {
                let x = p[0] - b.center[0], y = p[1] - b.center[1], z = p[2] - b.center[2]
                let x1 = x * cy + z * sy
                let z1 = -x * sy + z * cy
                let y1 = y * cp - z1 * sp
                let z2 = y * sp + z1 * cp
                return (CGPoint(x: size.width / 2 + x1 * scale, y: size.height / 2 - y1 * scale), z2)
            }
            if !pdb.atoms.isEmpty {
                let pts = stride(from: 0, to: pdb.atoms.count - 2, by: 3).map { project([pdb.atoms[$0], pdb.atoms[$0 + 1], pdb.atoms[$0 + 2]]) }
                for i in stride(from: 0, to: pdb.bonds.count - 1, by: 2) {
                    let a = Int(pdb.bonds[i]), b = Int(pdb.bonds[i + 1])
                    guard a < pts.count, b < pts.count else { continue }
                    var path = Path()
                    path.move(to: pts[a].0)
                    path.addLine(to: pts[b].0)
                    ctx.stroke(path, with: .color(.gray.opacity(0.8)), style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                }
                // hintere Atome zuerst zeichnen
                for i in pts.indices.sorted(by: { pts[$0].1 > pts[$1].1 }) {
                    let r: CGFloat = pdb.elements[i] == "H" ? 5 : 9
                    let rect = CGRect(x: pts[i].0.x - r, y: pts[i].0.y - r, width: 2 * r, height: 2 * r)
                    ctx.fill(Path(ellipseIn: rect), with: .color(elementColor(pdb.elements[i])))
                    ctx.stroke(Path(ellipseIn: rect), with: .color(.black.opacity(0.35)), lineWidth: 0.8)
                    if pdb.atoms.count / 3 <= 60 && pdb.elements[i] != "H" {
                        ctx.draw(Text(pdb.elements[i]).font(.system(size: 9, weight: .bold)).foregroundStyle(.white), at: pts[i].0)
                    }
                }
            }
            for (ci, c) in pdb.chains.enumerated() {
                let pts = stride(from: 0, to: c.ca.count - 2, by: 3).map { project([c.ca[$0], c.ca[$0 + 1], c.ca[$0 + 2]]) }
                guard pts.count > 1 else { continue }
                for i in 1..<pts.count {
                    var path = Path()
                    path.move(to: pts[i - 1].0)
                    path.addLine(to: pts[i].0)
                    // nähere Abschnitte kräftiger
                    let depth = (pts[i].1 + pts[i - 1].1) / 2 / b.radius
                    let alpha = 0.45 + 0.5 * (1 - (depth + 1) / 2)
                    ctx.stroke(path, with: .color(palette[ci % palette.count].opacity(alpha)),
                               style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                }
            }
        }
    }

    /// Farbe nach Element (CPK-ähnlich).
    private func elementColor(_ e: String) -> Color {
        switch e {
        case "H": return Color(white: 0.85)
        case "C": return Color(white: 0.35)
        case "N": return .blue
        case "O": return .red
        case "S": return .yellow
        case "P": return .orange
        case "F", "Cl": return .green
        case "Br": return .brown
        default: return .purple
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(name).font(Theme.heading())
            Text(pdb.atoms.isEmpty
                 ? tr("%@ Atome, %@ Kette(n): ", "\(pdb.atomCount)", "\(pdb.chains.count)") + pdb.chains.map { tr("%@ (%@ aa)", "\($0.id)", "\($0.sequence.count)") }.joined(separator: ", ")
                 : tr("Molekül: %@ Atome, %@ Bindungen", "\(pdb.atomCount)", "\(pdb.bonds.count / 2)"))
                .font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            chart
                .frame(minWidth: 480, minHeight: 360)
                .background(Theme.panel, in: RoundedRectangle(cornerRadius: 6))
                .gesture(DragGesture()
                    .onChanged { v in
                        if dragStart == nil { dragStart = (yaw, pitch) }
                        yaw = (dragStart?.0 ?? 0) + Double(v.translation.width) / 120
                        pitch = (dragStart?.1 ?? 0) + Double(v.translation.height) / 120
                    }
                    .onEnded { _ in dragStart = nil })
            HStack {
                Slider(value: $zoom, in: 0.5...3).frame(width: 160)
                Text("Zoom").font(Theme.label(11))
                Spacer()
                Button(tr("Schließen")) { dismiss() }
            }
        }
        .padding(16)
    }
}
