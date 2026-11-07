import SnapiousFFI
import SwiftUI

extension AppModel {
    static let table: [String: String] = {
        let bases = Array("TCAG")
        let aa = Array("FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG")
        var t: [String: String] = [:]
        var i = 0
        for a in bases { for b in bases { for c in bases { t[String([a, b, c])] = String(aa[i]); i += 1 } } }
        return t
    }()

    /// Übersetzung der mit der Maus markierten Basen (eigene Zeile unter den übrigen).
    var selectionCodons: [CodonMark] {
        guard translateSelection != "off", hasSelection, summary?.seqType != "PROTEIN",
              let a = selStart, let b = selEnd else { return [] }
        let bytes = fullBytes
        let lo = max(0, min(a, b)), hi = min(bytes.count, max(a, b))
        guard hi - lo >= 3, hi - lo <= 30_000 else { return [] }
        let row = (codons.map { $0.row }.max().map { $0 + 1 }) ?? 0
        let comp: [UInt8: UInt8] = [65: 84, 84: 65, 67: 71, 71: 67]
        var out: [CodonMark] = []
        let reverse = translateSelection == "reverse"
        var k = 0
        while (k + 1) * 3 <= hi - lo {
            let from = reverse ? hi - (k + 1) * 3 : lo + k * 3
            var triplet = Array(bytes[from..<from + 3].map { $0 & 0xDF })
            if reverse { triplet = triplet.reversed().map { comp[$0] ?? 78 } }
            let letter = AppModel.table[String(decoding: triplet, as: UTF8.self)] ?? "X"
            out.append(CodonMark(lo: from, hi: from + 2, letter: letter, isStart: letter == "M" && k == 0, isStop: letter == "*", row: row))
            k += 1
        }
        return out
    }


    /// Alle angezeigten Codons: CDS, Leseraster und die Übersetzung der Markierung.
    var allCodons: [CodonMark] { codons + selectionCodons }
}

/// Farbe des Codon-Blocks nach Eigenschaft der Aminosäure.
func aminoColor(_ m: CodonMark) -> Color {
    if m.isStop { return .red }
    if m.isStart { return .green }
    switch m.letter {
    case "A", "V", "L", "I", "M", "F", "W": return .orange      // wasserabweisend
    case "S", "T", "N", "Q", "Y", "C": return .teal              // polar
    case "K", "R", "H": return .blue                             // positiv geladen
    case "D", "E": return .pink                                  // negativ geladen
    case "G", "P": return .gray                                  // Sonderfälle
    default: return .secondary
    }
}


/// Menü für die Aminosäure-Anzeige (Markierung, CDS, Leseraster).
struct AminoMenu: View {
    @ObservedObject var model: AppModel

    /// Auswahl der angezeigten Aminosäure-Leserahmen.
    var body: some View {
        Menu {
            Toggle(tr("Aminosäuren der CDS-Features"), isOn: Binding(
                get: { model.showCdsTranslation },
                set: { model.setFrames(model.aaFrames, cds: $0) }))
            Picker(tr("Markierte Basen"), selection: $model.translateSelection) {
                Text(tr("Markierung: nicht übersetzen")).tag("off")
                Text(tr("Markierung übersetzen (Vorwärts)")).tag("forward")
                Text(tr("Markierung übersetzen (Gegenstrang)")).tag("reverse")
            }
            .pickerStyle(.inline)
            Divider()
            ForEach([1, 2, 3, -1, -2, -3], id: \.self) { f in
                Toggle(f > 0 ? tr("Vorwärts, Start bei Base %@", "\(f)") : tr("Rückwärts (Gegenstrang), Start bei Base %@ vom Ende", "\(-f)"), isOn: Binding(
                    get: { model.aaFrames.contains(f) },
                    set: { on in
                        var list = model.aaFrames.filter { $0 != f }
                        if on { list.append(f) }
                        let order = [1, 2, 3, -1, -2, -3]
                        model.setFrames(list.sorted { order.firstIndex(of: $0)! < order.firstIndex(of: $1)! }, cds: model.showCdsTranslation)
                    }))
            }
            Divider()
            Button(tr("Alle 6 Leseraster")) { model.setFrames([1, 2, 3, -1, -2, -3], cds: model.showCdsTranslation) }
            Button(tr("Aus")) { model.setFrames([], cds: false) }
        } label: {
            Label(tr("Aminosäuren"), systemImage: "textformat.abc")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .help(tr("Aminosäuren unter der Sequenz anzeigen"))
    }
}
