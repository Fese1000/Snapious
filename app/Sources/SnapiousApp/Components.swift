import SwiftUI

/// Surface with border and rounded corners.
struct Card<Content: View>: View {
    var padding: CGFloat = Theme.pad
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(padding)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Theme.panel, in: RoundedRectangle(cornerRadius: Theme.radius))
            .overlay(RoundedRectangle(cornerRadius: Theme.radius).strokeBorder(Theme.line, lineWidth: 1))
    }
}

/// Section heading with a small color bar.
struct SectionHeader: View {
    let text: String
    var detail: String?

    var body: some View {
        HStack(spacing: 8) {
                        Text(text).font(Theme.heading()).foregroundStyle(Theme.ink)
            if let detail { Text(detail).font(Theme.label()).foregroundStyle(Theme.inkSoft) }
            Spacer(minLength: 0)
        }
    }
}

/// Metric with label.
struct StatTile: View {
    let title: String
    let value: String
    var tint: Color = Theme.accent

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(Theme.label(11)).foregroundStyle(Theme.inkSoft)
            Text(value).font(.system(size: 20, weight: .semibold)).foregroundStyle(tint)
                .lineLimit(1).minimumScaleFactor(0.6)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.panel, in: RoundedRectangle(cornerRadius: Theme.radiusSmall))
        .overlay(RoundedRectangle(cornerRadius: Theme.radiusSmall).strokeBorder(Theme.line, lineWidth: 1))
    }
}

/// Small label.
struct Chip: View {
    let text: String
    var tint: Color = Theme.accent

    var body: some View {
        Text(text)
            .font(Theme.label(11))
            .padding(.horizontal, 8).padding(.vertical, 3)
            .foregroundStyle(tint)
            .background(tint.opacity(0.12), in: RoundedRectangle(cornerRadius: 4))
    }
}

/// Capsule-shaped button; `prominent` fills with the accent color.
struct PillButtonStyle: ButtonStyle {
    var prominent = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(Theme.label(13))
            .padding(.horizontal, 14).padding(.vertical, 6)
            .foregroundStyle(prominent ? Color.white : Theme.accent)
            .background(prominent ? Theme.accent : Theme.accent.opacity(0.12), in: Capsule())
            .opacity(configuration.isPressed ? 0.7 : 1)
    }
}

/// Empty state with hint and optional button.
struct EmptyState: View {
    let icon: String
    let title: String
    let message: String
    var actions: [(String, () -> Void)] = []

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: icon).font(.system(size: 40, weight: .light)).foregroundStyle(Theme.accent)
            Text(title).font(Theme.title(20)).foregroundStyle(Theme.ink)
            Text(message).font(Theme.label(13)).foregroundStyle(Theme.inkSoft).multilineTextAlignment(.center)
            VStack(spacing: 8) {
                ForEach(Array(actions.enumerated()), id: \.offset) { i, a in
                    if i == 0 {
                        Button(a.0, action: a.1).buttonStyle(.borderedProminent)
                    } else {
                        Button(a.0, action: a.1).buttonStyle(.bordered)
                    }
                }
            }
        }
        .padding(20)
        .frame(maxWidth: 420)
    }
}

/// Uniform style for all `GroupBox` fields.
struct PanelGroupBoxStyle: GroupBoxStyle {
    func makeBody(configuration: Configuration) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            configuration.label.font(Theme.heading(13)).foregroundStyle(Theme.ink)
            configuration.content
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.panel, in: RoundedRectangle(cornerRadius: Theme.radius))
        .overlay(RoundedRectangle(cornerRadius: Theme.radius).strokeBorder(Theme.line, lineWidth: 1))
    }
}
