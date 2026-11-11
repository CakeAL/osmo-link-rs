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
                    CameraView(remote: remote, location: remote.location, camera: camera)
                } else {
                    discovery
                }
            }
            .navigationTitle(remote.camera == nil ? "Osmo Link" : "相机遥控")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItemGroup(placement: .navigationBarTrailing) {
                    if remote.camera != nil {
                        Button("断开", action: remote.disconnect)
                            .disabled(remote.phase == .disconnecting)
                    }
                    NavigationLink(value: Route.settings) {
                        Label("设置", systemImage: "gearshape.fill")
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
            pairingSheetContent(pairing)
                .presentationDetents([.medium])
                .presentationDragIndicator(.visible)
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

    private func pairingSheetContent(_ pairing: Pairing) -> some View {
        VStack(spacing: 18) {
            ZStack {
                Circle()
                    .fill(
                        LinearGradient(
                            colors: [Color.accentColor, Color.accentColor.opacity(0.65)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                    .frame(width: 76, height: 76)
                Image(systemName: "camera.fill")
                    .font(.system(size: 30, weight: .semibold))
                    .foregroundStyle(.white)
            }
            .accessibilityHidden(true)

            VStack(spacing: 6) {
                Text("确认配对")
                    .font(.title2.bold())
                Text("请在 \(pairing.device) 上确认配对码")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            Text(pairing.code.formatted(.number.grouping(.never)))
                .font(.system(size: 42, weight: .bold, design: .rounded))
                .monospacedDigit()
                .tracking(6)
                .padding(.horizontal, 26)
                .padding(.vertical, 14)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
                .overlay {
                    RoundedRectangle(cornerRadius: 20, style: .continuous)
                        .strokeBorder(Color.accentColor.opacity(0.25), lineWidth: 1)
                }
                .accessibilityLabel("配对码 \(pairing.code.formatted(.number.grouping(.never)))")

            ProgressView("正在等待相机")
                .font(.footnote)
        }
        .multilineTextAlignment(.center)
        .padding(28)
    }

    private var discovery: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                hero
                searchButton
                nearbyCameras
                if let message = remote.message {
                    GlassCard(padding: 14) {
                        Label(message, systemImage: "exclamationmark.triangle.fill")
                            .font(.subheadline)
                            .foregroundStyle(.orange)
                    }
                }
                Label(
                    "支持 Osmo Action 4 / 5 Pro / 6 与 Osmo 360",
                    systemImage: "checkmark.seal"
                )
                .font(.caption2)
                .foregroundStyle(.tertiary)
                .frame(maxWidth: .infinity)
                .padding(.top, 2)
            }
            .frame(maxWidth: AppTheme.contentWidth)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 20)
            .padding(.top, 8)
            .padding(.bottom, 40)
        }
        .scrollIndicators(.hidden)
        .background(AppBackground())
    }

    private var hero: some View {
        ZStack(alignment: .topTrailing) {
            LinearGradient(
                colors: [
                    Color.accentColor.opacity(0.98),
                    Color.accentColor.opacity(0.72),
                    Color.black.opacity(0.84),
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            Circle()
                .fill(.white.opacity(0.10))
                .frame(width: 190, height: 190)
                .offset(x: 66, y: -74)
            Circle()
                .stroke(.white.opacity(0.12), lineWidth: 1)
                .frame(width: 130, height: 130)
                .offset(x: 92, y: 48)

            VStack(alignment: .leading, spacing: 18) {
                HStack(alignment: .top) {
                    Image(systemName: "camera.aperture")
                        .font(.system(size: 28, weight: .medium))
                        .frame(width: 54, height: 54)
                        .background(.white.opacity(0.16), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
                    Spacer()
                    Label("蓝牙连接", systemImage: "antenna.radiowaves.left.and.right")
                        .font(.caption.weight(.semibold))
                        .padding(.horizontal, 11)
                        .padding(.vertical, 7)
                        .background(.white.opacity(0.14), in: Capsule())
                }

                VStack(alignment: .leading, spacing: 8) {
                    Text("连接 Osmo 相机")
                        .font(.system(size: 30, weight: .bold, design: .rounded))
                    Text("查看电量与拍摄状态，切换模式并遥控录制。")
                        .font(.subheadline)
                        .foregroundStyle(.white.opacity(0.82))
                        .fixedSize(horizontal: false, vertical: true)
                }

                HStack(spacing: 16) {
                    heroFeature("记录状态", systemImage: "gauge.with.dots.needle.50percent")
                    heroFeature("遥控拍摄", systemImage: "record.circle")
                }
            }
            .padding(24)
        }
        .foregroundStyle(.white)
        .clipShape(RoundedRectangle(cornerRadius: 30, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 30, style: .continuous)
                .strokeBorder(.white.opacity(0.12), lineWidth: 1)
        }
        .shadow(color: Color.accentColor.opacity(0.28), radius: 24, y: 12)
    }

    private func heroFeature(_ title: String, systemImage: String) -> some View {
        Label(title, systemImage: systemImage)
            .font(.caption.weight(.medium))
            .foregroundStyle(.white.opacity(0.78))
    }

    private var searchButton: some View {
        Button(action: remote.search) {
            HStack(spacing: 11) {
                Image(systemName: "magnifyingglass")
                    .font(.headline.weight(.semibold))
                Text(remote.hasSearched ? "重新搜索相机" : "搜索相机")
                Spacer()
                Image(systemName: "arrow.right")
                    .font(.subheadline.weight(.semibold))
                    .opacity(0.85)
            }
        }
        .buttonStyle(ProminentActionStyle())
        .disabled(remote.phase != .idle)
        .accessibilityHint("搜索附近已开启无线连接的 DJI Osmo 相机")
    }

    private var nearbyCameras: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text("附近相机")
                    .font(.title3.bold())
                Spacer()
                if !remote.devices.isEmpty {
                    Text("\(remote.devices.count) 台")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 9)
                        .padding(.vertical, 4)
                        .background(Color.secondary.opacity(0.10), in: Capsule())
                }
            }
            .padding(.horizontal, 4)

            if remote.phase == .scanning {
                statusCard(title: "正在搜索相机", detail: "通常需要几秒钟，请保持相机唤醒。", showsProgress: true)
            } else if remote.phase == .connecting {
                statusCard(
                    title: remote.pairing?.auto == true ? "正在自动连接" : "正在连接相机",
                    detail: remote.pairing?.auto == true
                        ? "已找到上次连接的相机，正在恢复连接。"
                        : "请在相机屏幕上留意配对提示。",
                    showsProgress: true
                )
            } else if remote.devices.isEmpty {
                GlassCard {
                    EmptyDevicesView(hasSearched: remote.hasSearched)
                }
            } else {
                VStack(spacing: 10) {
                    ForEach(remote.devices) { device in
                        deviceRow(device)
                    }
                }
            }
        }
    }

    private func statusCard(title: String, detail: String, showsProgress: Bool) -> some View {
        GlassCard {
            HStack(spacing: 14) {
                if showsProgress {
                    ProgressView()
                        .controlSize(.regular)
                        .frame(width: 38, height: 38)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                        .font(.headline)
                    Text(detail)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
            }
        }
    }

    private func deviceRow(_ device: CameraDevice) -> some View {
        Button {
            remote.connect(device)
        } label: {
            HStack(spacing: 14) {
                Image(systemName: "video.fill")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
                    .frame(width: 46, height: 46)
                    .background(Color.accentColor.opacity(0.12), in: RoundedRectangle(cornerRadius: 14, style: .continuous))

                VStack(alignment: .leading, spacing: 3) {
                    Text(device.name)
                        .font(.headline)
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                    Text(device.rssi.map { "\($0) dBm" } ?? "可连接")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }

                Spacer(minLength: 8)
                SignalBadge(rssi: device.rssi)
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.bold))
                    .foregroundStyle(.tertiary)
            }
            .padding(14)
            .background(
                .regularMaterial,
                in: RoundedRectangle(cornerRadius: 20, style: .continuous)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 20, style: .continuous)
                    .strokeBorder(Color.primary.opacity(0.06), lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
        .accessibilityHint("连接 \(device.name)")
    }
}
