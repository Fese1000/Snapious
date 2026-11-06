import AppKit
import SwiftUI

/// Gemeinsame Gestaltung: alle Farben, Schriften und Maße der App kommen von hier.
enum Theme {
    // MARK: Farben (hell / dunkel)
    static let ink = Color.primary
    static let inkSoft = Color.secondary
    static let paper = Color(nsColor: .windowBackgroundColor)
    static let panel = Color(nsColor: .controlBackgroundColor)
    static let sidebar = Color.clear
    static let line = Color(nsColor: .separatorColor)
    static let accent = Color.accentColor
    static let amber = Color.orange
    static let danger = Color.red
    static let good = Color.green

    /// Basenfarben (A grün, C blau, G bernstein, T rot).
    static func base(_ c: Character) -> Color {
        switch c {
        case "A", "a": return dynamic(light: 0x2E8B57, dark: 0x58C486)
        case "C", "c": return dynamic(light: 0x2F6DB5, dark: 0x6AA7EE)
        case "G", "g": return dynamic(light: 0xC77A12, dark: 0xF0A93B)
        case "T", "t", "U", "u": return dynamic(light: 0xC0392B, dark: 0xF07468)
        default: return inkSoft
        }
    }

    /// Farbe je Dokumenttyp (Seitenleiste).
    static func docColor(_ type: String) -> Color { accent }

    /// Farbe je Feature-Typ (Karte und Sequenzansicht).
    static func feature(_ type: String) -> Color {
        switch type.lowercased() {
        case "cds", "gene": return dynamic(light: 0x3B5BA9, dark: 0x7C9CF0)
        case "promoter": return dynamic(light: 0x0B7A8A, dark: 0x38C3D4)
        case "terminator": return dynamic(light: 0x8A5A2B, dark: 0xD19A66)
        case "rep_origin", "origin": return dynamic(light: 0x7B4EA3, dark: 0xB48AE0)
        case "primer_bind": return dynamic(light: 0xC2456E, dark: 0xF07FA3)
        case "misc_feature", "protein_bind": return inkSoft
        default: return dynamic(light: 0x4A7FB0, dark: 0x7FB2E0)
        }
    }

    // MARK: Maße
    static let radius: CGFloat = 8
    static let radiusSmall: CGFloat = 6
    static let gap: CGFloat = 12
    static let pad: CGFloat = 16

    // MARK: Schriften
    static func title(_ size: CGFloat = 26) -> Font { .system(size: size, weight: .bold) }
    static func heading(_ size: CGFloat = 15) -> Font { .system(size: size, weight: .semibold) }
    static func label(_ size: CGFloat = 12) -> Font { .system(size: size) }
    static func mono(_ size: CGFloat = 13) -> Font { .system(size: size, weight: .regular, design: .monospaced) }

    private static func dynamic(light: UInt32, dark: UInt32) -> Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
            return NSColor(hex: isDark ? dark : light)
        })
    }
}

extension NSColor {
    convenience init(hex: UInt32) {
        self.init(
            srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
            green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255,
            alpha: 1
        )
    }
}
