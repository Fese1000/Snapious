import AppKit
import SwiftUI

/// Tastaturbearbeitung für Sequenz-, Ring- und Linearansicht: Basen tippen, Rücktaste/Entf löschen,
/// ⌘C kopieren, ⌘X ausschneiden, ⌘V einfügen, Pfeiltasten bewegen den Cursor.
private struct EditKeys: ViewModifier {
    @ObservedObject var model: AppModel
    var focus: FocusState<Bool>.Binding

    func body(content: Content) -> some View {
        content
            .focusable()
            .focused(focus)
            .focusEffectDisabled()
            .onCopyCommand {
                guard let t = model.selectionString() else { return [] }
                return [NSItemProvider(object: t as NSString)]
            }
            .onCutCommand {
                guard let t = model.selectionString() else { return [] }
                model.replaceSelection(with: "")
                return [NSItemProvider(object: t as NSString)]
            }
            .onPasteCommand(of: [.plainText]) { providers in
                _ = providers.first?.loadObject(ofClass: NSString.self) { obj, _ in
                    if let t = obj as? String { Task { @MainActor in model.replaceSelection(with: t) } }
                }
            }
            .onKeyPress(phases: .down) { press in
                if press.modifiers.contains(.command) { return .ignored }
                switch press.key {
                case .delete: model.backspace(); return .handled
                case .deleteForward: model.forwardDelete(); return .handled
                case .leftArrow: model.moveCaret(by: -1); return .handled
                case .rightArrow: model.moveCaret(by: 1); return .handled
                default: break
                }
                let ch = press.characters.uppercased()
                let allowed = model.summary?.seqType == "PROTEIN" ? "ACDEFGHIKLMNPQRSTVWYXBZUOJ*" : "ACGTUNRYSWKMBDHV"
                if ch.count == 1, allowed.contains(ch), model.selStart != nil {
                    model.replaceSelection(with: ch)
                    return .handled
                }
                return .ignored
            }
    }
}

extension View {
    func editKeys(_ model: AppModel, focus: FocusState<Bool>.Binding) -> some View {
        modifier(EditKeys(model: model, focus: focus))
    }
}
