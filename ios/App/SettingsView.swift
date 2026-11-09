import SwiftUI

struct SettingsView<Route: Hashable>: View {
    @AppStorage("appearance") private var appearance = Appearance.system
    @Environment(\.openURL) private var openURL
    @ObservedObject var remote: RemoteStore
    let about: Route

    var body: some View {
        Form {
            Section("外观") {
                Picker("显示模式", selection: $appearance) {
                    ForEach(Appearance.allCases) { appearance in
                        Text(appearance.title).tag(appearance)
                    }
                }
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
                Button {
                    if let url = URL(string: UIApplication.openSettingsURLString) { openURL(url) }
                } label: {
                    Label("打开系统设置", systemImage: "gear")
                }
            } header: {
                Text("权限")
            } footer: {
                Text("搜索和控制相机需要蓝牙权限。iOS 定位接口缺少相机所需的卫星数，暂无法注入 GPS。")
            }

            Section {
                NavigationLink(value: about) {
                    Label("关于 Osmo Link", systemImage: "info.circle")
                }
            }
        }
        .navigationTitle("设置")
    }
}

struct AboutView: View {
    var body: some View {
        Form {
            Section {
                LabeledContent("应用", value: "Osmo Link")
                LabeledContent("版本", value: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—")
                Text("通过蓝牙连接 DJI Osmo 相机，查看相机状态并遥控拍摄。")
                    .foregroundStyle(.secondary)
            }
        }
        .navigationTitle("关于")
    }
}
