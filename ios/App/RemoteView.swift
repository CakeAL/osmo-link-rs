import SwiftUI

struct RemoteView: View {
    private enum Route: Hashable {
        case settings, about
    }

    @ObservedObject var remote: RemoteStore
    @State private var path: [Route] = []

    var body: some View {
        NavigationStack(path: $path) {
            Group {
                if let camera = remote.camera {
                    CameraView(remote: remote, camera: camera)
                } else {
                    discovery
                }
            }
            .navigationTitle(remote.camera == nil ? "Osmo Link" : "相机遥控")
            .toolbar {
                ToolbarItemGroup(placement: .navigationBarTrailing) {
                    if remote.camera != nil {
                        Button("断开", action: remote.disconnect)
                            .disabled(remote.phase == .disconnecting)
                    }
                    NavigationLink(value: Route.settings) {
                        Label("设置", systemImage: "gearshape")
                    }
                }
            }
            .navigationDestination(for: Route.self) { route in
                switch route {
                case .settings: SettingsView(remote: remote, about: Route.about)
                case .about: AboutView()
                }
            }
        }
        .sheet(item: pairingSheet) { pairing in
            VStack(spacing: 16) {
                Image(systemName: "camera.fill")
                    .font(.largeTitle)
                    .foregroundStyle(Color.accentColor)
                    .accessibilityHidden(true)
                Text("确认配对")
                    .font(.title2.bold())
                Text("请在 \(pairing.device) 上确认以下配对码。")
                    .foregroundStyle(.secondary)
                Text(pairing.code.formatted(.number.grouping(.never)))
                    .font(.largeTitle.monospacedDigit().bold())
                    .accessibilityLabel("配对码 \(pairing.code.formatted(.number.grouping(.never)))")
                ProgressView("正在等待相机")
                    .font(.footnote)
            }
            .multilineTextAlignment(.center)
            .padding(24)
            .presentationDetents([.medium, .large])
            .interactiveDismissDisabled()
        }
        .alert("相机连接与控制", isPresented: Binding(
            get: { remote.camera != nil && remote.message != nil },
            set: { if !$0 { remote.message = nil } }
        )) {
            Button("好", role: .cancel) {}
        } message: {
            Text(remote.message ?? "")
        }
        .onOpenURL { url in
            if url.scheme == "osmolink", url.host == "remote" { path.removeAll() }
        }
    }

    private var pairingSheet: Binding<Pairing?> {
        Binding(
            get: { remote.pairing?.firstTime == true ? remote.pairing : nil },
            set: { _ in }
        )
    }

    private var discovery: some View {
        List {
            Section {
                VStack(alignment: .leading, spacing: 12) {
                    Image(systemName: "camera.fill")
                        .font(.title)
                        .foregroundStyle(Color.accentColor)
                        .accessibilityHidden(true)
                    Text("连接 Osmo 相机")
                        .font(.title2.bold())
                    Text("唤醒相机并开启无线连接，然后搜索附近的设备。")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                    Button(action: remote.search) {
                        Label(remote.hasSearched ? "重新搜索" : "搜索相机", systemImage: "magnifyingglass")
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    .disabled(remote.phase != .idle)
                }
                .padding(.vertical, 8)
            }
            Section("附近相机") {
                if remote.phase == .scanning {
                    ProgressView("正在搜索相机")
                } else if remote.phase == .connecting {
                    ProgressView("正在连接相机")
                } else if remote.devices.isEmpty {
                    Text(remote.hasSearched ? "没有找到相机。请检查相机的无线连接后重试。" : "搜索后会在这里显示设备。")
                        .foregroundStyle(.secondary)
                } else {
                    ForEach(remote.devices) { device in
                        Button {
                            remote.connect(device)
                        } label: {
                            LabeledContent {
                                if let rssi = device.rssi {
                                    Text("\(rssi) dBm").monospacedDigit()
                                }
                            } label: {
                                Label(device.name, systemImage: "camera")
                            }
                        }
                    }
                }
            }
            if let message = remote.message {
                Section {
                    Label(message, systemImage: "exclamationmark.circle")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .listStyle(.insetGrouped)
    }
}
