import SnapiousFFI
import SwiftUI

/// Plasmidkarte (Kreis) für zirkuläre Dokumente, lineare Karte für lineare. Zeichnet Features nach Spuren
/// (aus dem Rust-Kern) und die Schnittstellen der Einzelschneider-Enzyme außen.
struct MapView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        if let s = model.summary {
            if s.topology == "CIRCULAR" {
                CircularMap(model: model, length: max(1, Int(s.length)), name: s.name)
            } else {
                LinearMap(model: model, length: max(1, Int(s.length)))
            }
        }
    }
}

private func niceStep(length: Int, maxTicks: Int) -> Int {
    var exp = 1
    while true {
        for m in [1, 2, 5] {
            let step = m * exp
            if length <= step * max(1, maxTicks) { return step }
        }
        exp *= 10
    }
}

private func featureColor(_ type: String) -> Color { Theme.feature(type) }

struct CircularMap: View {
    @ObservedObject var model: AppModel
    let length: Int
    let name: String

    var body: some View {
        Canvas { ctx, size in
            let c = CGPoint(x: size.width / 2, y: size.height / 2)
            let r = max(40, min(size.width, size.height) / 2 - 95)
            func angle(_ p: Double) -> Double { 2 * .pi * p / Double(length) }
            func point(_ p: Double, _ rad: CGFloat) -> CGPoint {
                CGPoint(x: c.x + rad * CGFloat(sin(angle(p))), y: c.y - rad * CGFloat(cos(angle(p))))
            }
            // Rückgrat
            ctx.stroke(Path(ellipseIn: CGRect(x: c.x - r, y: c.y - r, width: 2 * r, height: 2 * r)),
                       with: .color(.secondary), lineWidth: 2)
            // Skala
            let step = niceStep(length: length, maxTicks: 12)
            var t = 0
            while t < length {
                var p = Path()
                p.move(to: point(Double(t), r - 4))
                p.addLine(to: point(Double(t), r + 4))
                ctx.stroke(p, with: .color(.secondary), lineWidth: 1)
                ctx.draw(Text(verbatim: "\(t)").font(.system(size: 9)).foregroundStyle(Theme.inkSoft),
                         at: point(Double(t), r - 16), anchor: .center)
                t += step
            }
            // Features (Spuren nach innen)
            for f in model.lanes where f.featureType.lowercased() != "source" {
                let rad = r - 26 - CGFloat(f.lane) * 14
                guard rad > 20 else { continue }
                let a0 = angle(Double(f.start)) - .pi / 2
                let a1 = angle(Double(f.end)) - .pi / 2
                var arc = Path()
                arc.addArc(center: c, radius: rad, startAngle: .radians(a0), endAngle: .radians(a1), clockwise: false)
                ctx.stroke(arc, with: .color(featureColor(f.featureType)), lineWidth: 10)
                let mid = Double(f.start + f.end) / 2
                if Double(f.end - f.start) / Double(length) > 0.04 {
                    ctx.draw(Text(f.name).font(.system(size: 9)).foregroundStyle(.white), at: point(mid, rad), anchor: .center)
                }
            }
            // Enzymmarken außen, Beschriftungen auseinandergeschoben (nur Vorwärtsdurchgang)
            let sites = model.uniqueSites
            var lastAngle = -10.0
            for s in sites {
                var a = angle(Double(s.position))
                if a < lastAngle + 0.09 { a = lastAngle + 0.09 }
                lastAngle = a
                var tick = Path()
                tick.move(to: point(Double(s.position), r))
                tick.addLine(to: point(Double(s.position), r + 10))
                ctx.stroke(tick, with: .color(.primary), lineWidth: 1)
                let lp = CGPoint(x: c.x + (r + 20) * CGFloat(sin(a)), y: c.y - (r + 20) * CGFloat(cos(a)))
                let anchor: UnitPoint = sin(a) >= 0 ? .leading : .trailing
                ctx.draw(Text(verbatim: "\(s.enzyme) (\(s.position + 1))").font(.system(size: 10)), at: lp, anchor: anchor)
            }
            // Mitte
            ctx.draw(Text(name).font(Theme.heading()), at: CGPoint(x: c.x, y: c.y - 8), anchor: .center)
            ctx.draw(Text(verbatim: "\(length) bp").font(.subheadline).foregroundStyle(Theme.inkSoft),
                     at: CGPoint(x: c.x, y: c.y + 12), anchor: .center)
        }
        .padding(8)
    }
}

struct LinearMap: View {
    @ObservedObject var model: AppModel
    let length: Int

    var body: some View {
        Canvas { ctx, size in
            let left: CGFloat = 30, right = size.width - 30
            let y0: CGFloat = 70
            func x(_ p: Double) -> CGFloat { left + (right - left) * CGFloat(p / Double(length)) }
            ctx.stroke(Path { $0.move(to: CGPoint(x: left, y: y0)); $0.addLine(to: CGPoint(x: right, y: y0)) },
                       with: .color(.secondary), lineWidth: 2)
            let step = niceStep(length: length, maxTicks: 10)
            var t = 0
            while t <= length {
                ctx.stroke(Path { $0.move(to: CGPoint(x: x(Double(t)), y: y0 - 4)); $0.addLine(to: CGPoint(x: x(Double(t)), y: y0 + 4)) },
                           with: .color(.secondary), lineWidth: 1)
                ctx.draw(Text(verbatim: "\(t)").font(.system(size: 9)).foregroundStyle(Theme.inkSoft),
                         at: CGPoint(x: x(Double(t)), y: y0 + 14), anchor: .center)
                t += step
            }
            for f in model.lanes where f.featureType.lowercased() != "source" {
                let y = y0 + 30 + CGFloat(f.lane) * 18
                let rect = CGRect(x: x(Double(f.start)), y: y, width: max(2, x(Double(f.end)) - x(Double(f.start))), height: 14)
                ctx.fill(Path(roundedRect: rect, cornerRadius: 3), with: .color(featureColor(f.featureType)))
                if rect.width > 40 {
                    ctx.draw(Text(f.name).font(.system(size: 9)).foregroundStyle(.white), at: CGPoint(x: rect.midX, y: rect.midY), anchor: .center)
                }
            }
            var lastX: CGFloat = -100
            var row = 0
            for s in model.uniqueSites {
                let px = x(Double(s.position))
                row = px < lastX + 70 ? (row + 1) % 4 : 0
                lastX = px
                ctx.stroke(Path { $0.move(to: CGPoint(x: px, y: y0)); $0.addLine(to: CGPoint(x: px, y: y0 - 8 - CGFloat(row) * 12)) },
                           with: .color(.primary), lineWidth: 1)
                ctx.draw(Text(s.enzyme).font(.system(size: 10)), at: CGPoint(x: px + 2, y: y0 - 12 - CGFloat(row) * 12), anchor: .bottomLeading)
            }
        }
        .padding(8)
    }
}
