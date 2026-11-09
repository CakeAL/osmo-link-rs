import SwiftUI

@main
struct OsmoLinkApp: App {
    @AppStorage("appearance") private var appearance = Appearance.system
    @Environment(\.scenePhase) private var scenePhase
    @StateObject private var remote = RemoteStore.shared

    var body: some Scene {
        WindowGroup {
            RemoteView(remote: remote)
                .preferredColorScheme(appearance.colorScheme)
                .task(id: scenePhase) {
                    remote.activate(scenePhase == .active)
                }
        }
    }
}

enum Appearance: String, CaseIterable, Identifiable {
    case system, light, dark

    var id: Self { self }

    var title: String {
        switch self {
        case .system: "跟随系统"
        case .light: "浅色"
        case .dark: "深色"
        }
    }

    var colorScheme: ColorScheme? {
        switch self {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }
}
