// Draws the Snapious app icon (a DNA double helix) as assets/AppIcon.png (1024 x 1024).
// Usage: swift scripts/make-icon.swift assets/AppIcon.png
import AppKit

let size: CGFloat = 1024
let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "assets/AppIcon.png"

let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size), bitsPerSample: 8,
                           samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let ctx = NSGraphicsContext.current!.cgContext

// Background: macOS-style rounded square with a deep-blue gradient
let inset: CGFloat = 52
let rect = CGRect(x: inset, y: inset, width: size - 2 * inset, height: size - 2 * inset)
let bg = CGPath(roundedRect: rect, cornerWidth: 190, cornerHeight: 190, transform: nil)
ctx.saveGState()
ctx.addPath(bg); ctx.clip()
let space = CGColorSpaceCreateDeviceRGB()
let grad = CGGradient(colorsSpace: space, colors: [
    CGColor(red: 0.05, green: 0.11, blue: 0.22, alpha: 1),
    CGColor(red: 0.09, green: 0.27, blue: 0.40, alpha: 1)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: size), end: CGPoint(x: size, y: 0), options: [])
ctx.restoreGState()

// Helix: two sine strands drawn along a diagonal, with rungs
let teal = CGColor(red: 0.22, green: 0.80, blue: 0.83, alpha: 1)
let amber = CGColor(red: 0.98, green: 0.72, blue: 0.27, alpha: 1)
let steps = 400
let turns = 2.25
func point(_ t: Double, phase: Double) -> (CGPoint, Double) {
    // t in 0...1 along the axis, axis from lower-left to upper-right; strands swing sideways
    let a = 2 * Double.pi * turns * t + phase
    let ax = 250.0 + (size - 500) * t, ay = 230.0 + (size - 460) * t
    let amp = 150.0
    let dx = size - 500, dy = size - 460, len = (dx * dx + dy * dy).squareRoot()
    let nx = -dy / len, ny = dx / len // perpendicular to the axis direction
    let s = sin(a)
    return (CGPoint(x: ax + nx * amp * s, y: ay + ny * amp * s), cos(a))
}
ctx.setLineCap(.round)
// Rungs first (behind strands)
for i in 0...18 {
    let t = 0.03 + 0.94 * Double(i) / 18
    let (p1, d1) = point(t, phase: 0), (p2, _) = point(t, phase: .pi)
    if abs(d1) < 0.999 {
        ctx.setStrokeColor(CGColor(red: 1, green: 1, blue: 1, alpha: 0.55))
        ctx.setLineWidth(14)
        ctx.move(to: p1); ctx.addLine(to: p2); ctx.strokePath()
    }
}
// Strands: draw segments with the back part first so crossings look woven
func strand(phase: Double, color: CGColor, front: Bool) {
    for i in 0..<steps {
        let t0 = 0.02 + 0.96 * Double(i) / Double(steps), t1 = 0.02 + 0.96 * Double(i + 1) / Double(steps)
        let (p0, d0) = point(t0, phase: phase), (p1, _) = point(t1, phase: phase)
        if (d0 > 0) != front { continue }
        ctx.setStrokeColor(color); ctx.setLineWidth(front ? 46 : 38)
        ctx.move(to: p0); ctx.addLine(to: p1); ctx.strokePath()
    }
}
strand(phase: 0, color: teal.copy(alpha: 0.7)!, front: false)
strand(phase: .pi, color: amber.copy(alpha: 0.7)!, front: false)
strand(phase: 0, color: teal, front: true)
strand(phase: .pi, color: amber, front: true)

NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
print("Wrote \(out)")
