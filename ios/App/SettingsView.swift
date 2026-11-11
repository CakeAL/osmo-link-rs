import SwiftUI

struct SettingsView<Route: Hashable>: View {
    @AppStorage("appearance") private var appearance = Appearance.system
    @Environment(\.openURL) private var openURL
    @ObservedObject var remote: RemoteStore
    let about: Route

    var body: some View {
        Form {
            Section {
                Picker("显示模式", selection: $appearance) {
                    ForEach(Appearance.allCases) { appearance in
                        Text(appearance.title).tag(appearance)
                    }
                }
            } header: {
                Text("外观")
            } footer: {
                Text("默认跟随系统，也可以固定使用浅色或深色。")
            }

            Section {
                if #available(iOS 16.1, *) {
                    Toggle("录制实时活动", isOn: $remote.showsLiveActivity)
                } else {
                    LabeledContent("录制实时活动", value: "需要 iOS 16.1")
                }
            } header: {
                Text("录制")
            } footer: {
                Text("录制时在锁定屏幕和支持灵动岛的设备上显示时长与电量。也可在系统设置中管理实时活动。")
            }

            Section {
                Toggle("低电量提醒", isOn: Binding(
                    get: { remote.lowBatteryAlertEnabled },
                    set: { enabled in
                        remote.lowBatteryAlertEnabled = enabled
                        if enabled { remote.requestBatteryNotificationPermission() }
                    }
                ))
                if remote.lowBatteryAlertEnabled {
                    Picker("提醒阈值", selection: $remote.lowBatteryThreshold) {
                        ForEach(RemoteStore.batteryThresholds, id: \.self) { threshold in
                            Text("低于 \(threshold)%").tag(threshold)
                        }
                    }
                }
            } header: {
                Text("电量提醒")
            } footer: {
                Text("开启后，连接相机时电量低于所选阈值会震动并发送通知。通知权限可在系统设置中管理。")
            }

            Section {
                Toggle("自动连接已配对相机", isOn: $remote.autoConnectEnabled)
            } header: {
                Text("连接")
            } footer: {
                Text("搜索时如果发现之前连接过的相机，会直接连接上次使用的那台。关闭后可从附近相机列表手动选择。")
            }

            Section {
                Button {
                    if let url = URL(string: UIApplication.openSettingsURLString) { openURL(url) }
                } label: {
                    Label("打开系统设置", systemImage: "gear")
                }
            } header: {
                Text("权限")
            } footer: {
                Text("搜索和控制相机需要蓝牙权限；定位注入需要「使用 App 期间」位置权限。连接相机后会自动请求定位权限并开始注入。")
            }

            Section {
                NavigationLink(value: about) {
                    Label("关于 Osmo Link", systemImage: "info.circle.fill")
                }
            } footer: {
                Text("版本 \(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—")")
            }
        }
        .scrollContentBackground(.hidden)
        .background(AppBackground())
        .navigationTitle("设置")
    }
}

struct AboutView: View {
    private let repository = URL(string: "https://github.com/CakeAL/osmo-link-rs")!

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                appHeader
                repositoryCard
                detailsCard
                Text("用蓝牙把 iPhone 变成 DJI Osmo 相机的遥控器和状态监视器。")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)
            }
            .frame(maxWidth: AppTheme.contentWidth)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 20)
            .padding(.top, 12)
            .padding(.bottom, 36)
        }
        .scrollIndicators(.hidden)
        .background(AppBackground())
        .navigationTitle("关于")
        .navigationBarTitleDisplayMode(.inline)
    }

    private var appHeader: some View {
        VStack(spacing: 5) {
            Text("Osmo Link")
                .font(.system(size: 26, weight: .bold, design: .rounded))
            Text("版本 \(version) · \(build)")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .monospacedDigit()
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 8)
    }

    private var repositoryCard: some View {
        GlassCard(padding: 16) {
            Link(destination: repository) {
                HStack(spacing: 14) {
                    Image(systemName: "chevron.left.forwardslash.chevron.right")
                        .font(.system(size: 17, weight: .semibold))
                        .foregroundStyle(Color.accentColor)
                        .frame(width: 44, height: 44)
                        .background(Color.accentColor.opacity(0.12), in: RoundedRectangle(cornerRadius: 13, style: .continuous))

                    VStack(alignment: .leading, spacing: 3) {
                        Text("GitHub 仓库")
                            .font(.headline)
                            .foregroundStyle(.primary)
                        Text("CakeAL/osmo-link-rs")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Spacer(minLength: 8)
                    Image(systemName: "arrow.up.right")
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.tertiary)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityHint("在浏览器中打开 GitHub 仓库")
        }
    }

    private var detailsCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 14) {
                CardHeader("应用信息", systemImage: "info.circle.fill")
                VStack(spacing: 11) {
                    InfoRow(label: "应用", value: "Osmo Link", systemImage: "app")
                    InfoRow(label: "版本", value: "\(version) (\(build))", systemImage: "number")
                    InfoRow(label: "开源许可", value: "AGPL-3.0 及第三方许可", systemImage: "doc.text")
                }
                Divider().opacity(0.5)
                Text("支持通过公开的 DJI R SDK BLE 协议连接 Osmo Action 4 / 5 Pro / 6 与 Osmo 360 等相机。")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—"
    }

    private var build: String {
        Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "—"
    }
}
