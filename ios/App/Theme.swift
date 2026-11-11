import SwiftUI

enum AppTheme {
    static let cardRadius: CGFloat = 24
    static let contentWidth: CGFloat = 680
}

struct AppBackground: View {
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        ZStack {
            Color(uiColor: .systemGroupedBackground)
            LinearGradient(
                colors: [
                    Color.accentColor.opacity(colorScheme == .dark ? 0.20 : 0.14),
                    Color.accentColor.opacity(colorScheme == .dark ? 0.05 : 0.03),
                    .clear,
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            RadialGradient(
                colors: [Color.accentColor.opacity(colorScheme == .dark ? 0.10 : 0.06), .clear],
                center: .topTrailing,
                startRadius: 10,
                endRadius: 380
            )
        }
        .ignoresSafeArea()
    }
}

struct GlassCard<Content: View>: View {
    var padding: CGFloat = 18
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(padding)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                .regularMaterial,
                in: RoundedRectangle(cornerRadius: AppTheme.cardRadius, style: .continuous)
            )
            .overlay {
                RoundedRectangle(cornerRadius: AppTheme.cardRadius, style: .continuous)
                    .strokeBorder(Color.primary.opacity(0.06), lineWidth: 1)
            }
            .shadow(color: .black.opacity(0.05), radius: 14, y: 7)
    }
}

struct CardHeader: View {
    let title: String
    let systemImage: String
    var trailing: AnyView?

    init(_ title: String, systemImage: String) {
        self.title = title
        self.systemImage = systemImage
        trailing = nil
    }

    init(_ title: String, systemImage: String, @ViewBuilder trailing: () -> some View) {
        self.title = title
        self.systemImage = systemImage
        self.trailing = AnyView(trailing())
    }

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: systemImage)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(Color.accentColor)
                .frame(width: 28, height: 28)
                .background(Color.accentColor.opacity(0.12), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            Text(title)
                .font(.headline)
            Spacer(minLength: 8)
            trailing
        }
    }
}

struct InfoRow: View {
    let label: String
    let value: String
    var systemImage: String?
    var valueColor: Color = .primary

    var body: some View {
        HStack(spacing: 12) {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .frame(width: 20)
            }
            Text(label)
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Spacer(minLength: 12)
            Text(value)
                .font(.subheadline.weight(.medium))
                .foregroundStyle(valueColor)
                .multilineTextAlignment(.trailing)
        }
        .padding(.vertical, 2)
    }
}

struct ProminentActionStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .foregroundStyle(.white)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 20)
            .padding(.vertical, 16)
            .background(
                LinearGradient(
                    colors: [Color.accentColor, Color.accentColor.opacity(0.76)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                ),
                in: RoundedRectangle(cornerRadius: 18, style: .continuous)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .strokeBorder(.white.opacity(0.18), lineWidth: 1)
            }
            .shadow(color: Color.accentColor.opacity(isEnabled ? 0.28 : 0), radius: 12, y: 6)
            .scaleEffect(configuration.isPressed ? 0.985 : 1)
            .opacity(isEnabled ? 1 : 0.55)
            .animation(.easeOut(duration: 0.16), value: configuration.isPressed)
    }
}

struct SignalBadge: View {
    let rssi: Int?

    private var level: Int {
        guard let rssi else { return 0 }
        return switch rssi {
        case ...(-80): 1
        case ...(-65): 2
        case ...(-50): 3
        default: 4
        }
    }

    var body: some View {
        HStack(spacing: 3) {
            ForEach(0..<4, id: \.self) { index in
                Capsule()
                    .fill(index < level ? Color.accentColor : Color.secondary.opacity(0.22))
                    .frame(width: 3, height: CGFloat(5 + index * 3))
            }
        }
        .frame(height: 15, alignment: .bottom)
        .accessibilityLabel(rssi.map { "信号 \($0) dBm" } ?? "未知信号")
    }
}

struct EmptyDevicesView: View {
    let hasSearched: Bool

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: hasSearched ? "dot.radiowaves.left.and.right" : "camera.fill")
                .font(.system(size: 30, weight: .light))
                .foregroundStyle(Color.accentColor)
                .frame(width: 64, height: 64)
                .background(Color.accentColor.opacity(0.10), in: Circle())
            Text(hasSearched ? "没有找到相机" : "等待搜索")
                .font(.headline)
            Text(hasSearched ? "唤醒相机并打开无线连接，然后重新搜索。" : "附近可连接的 Osmo 相机会显示在这里。")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 28)
    }
}
