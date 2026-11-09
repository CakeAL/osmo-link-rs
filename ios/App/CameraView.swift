import SwiftUI

struct CameraView: View {
    @ObservedObject var remote: RemoteStore
    let camera: ConnectedCamera

    var body: some View {
        Form {
            Section {
                VStack(spacing: 8) {
                    connectionLabel
                        .font(.subheadline.weight(.medium))
                    if let status = remote.status {
                        Text(status.photoMode ? status.modeTitle : duration(status.recordSeconds))
                            .font(status.photoMode ? .largeTitle.bold() : .system(.largeTitle, design: .monospaced).bold())
                            .multilineTextAlignment(.center)
                            .accessibilityLabel(status.photoMode ? status.modeTitle : "录制时长 \(status.recordSeconds) 秒")
                        Text("\(status.resolution) · \(status.fps)")
                            .font(.subheadline)
                        Text(remaining(status))
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 16)
            } header: {
                HStack {
                    Text(camera.title)
                    Spacer()
                    if let status = remote.status {
                        Label("\(status.battery)%", systemImage: batterySymbol(status.battery))
                            .foregroundStyle(status.battery <= 15 ? Color.red : Color.secondary)
                            .monospacedDigit()
                            .accessibilityLabel("电量 \(status.battery)%")
                    }
                }
                .textCase(nil)
            }

            Section("拍摄模式") {
                Picker("模式", selection: Binding<UInt8?>(
                    get: { remote.status?.modeCode },
                    set: { code in
                        if let mode = remote.modes.first(where: { $0.code == code }) { remote.selectMode(mode) }
                    }
                )) {
                    if remote.status == nil {
                        Text("等待相机状态").tag(nil as UInt8?)
                    } else if let status = remote.status, !remote.modes.contains(where: { $0.code == status.modeCode }) {
                        Text(status.modeTitle).tag(Optional(status.modeCode))
                    }
                    ForEach(remote.modes) { mode in
                        Text(mode.title).tag(Optional(mode.code))
                    }
                }
                .pickerStyle(.menu)
                .disabled(!remote.canChangeMode)
            }

            Section("相机详情") {
                if camera.name != camera.title {
                    LabeledContent("设备名称", value: camera.name)
                }
                if let status = remote.status {
                    LabeledContent("可用容量", value: "\((Double(status.remainingMb) / 1024).formatted(.number.precision(.fractionLength(1)))) GB")
                    LabeledContent("防抖", value: status.eis)
                    if let warning = status.temperatureWarning {
                        Label(warning, systemImage: "thermometer")
                            .foregroundStyle(.red)
                    }
                }
                if let version = remote.version {
                    LabeledContent("固件版本", value: version)
                }
                if let modeInfo = remote.modeInfo, !modeInfo.isEmpty {
                    LabeledContent("模式参数", value: modeInfo)
                }
            }

            Section("定位") {
                Label("相机定位注入不可用", systemImage: "location.slash")
                    .foregroundStyle(.secondary)
            }
        }
        .safeAreaInset(edge: .bottom, spacing: 0) {
            captureDock
        }
    }

    @ViewBuilder
    private var connectionLabel: some View {
        switch remote.phase {
        case let .reconnecting(attempt):
            ProgressView("正在重连（\(attempt)/2）")
        case .disconnecting:
            ProgressView("正在断开")
        default:
            if let status = remote.status {
                if status.isRecording {
                    Label("录制中", systemImage: "circle.fill")
                        .foregroundStyle(.red)
                } else {
                    Text(status.stateTitle).foregroundStyle(.secondary)
                }
            } else {
                ProgressView("正在读取相机状态")
            }
        }
    }

    private var captureDock: some View {
        let status = remote.status
        let recording = status?.isRecording == true
        let photo = status?.photoMode == true
        let title = status == nil ? "拍摄" : photo ? "拍照" : recording ? "停止录像" : "开始录像"
        let color: Color = status == nil ? .secondary : photo ? .primary : .red
        return VStack(spacing: 4) {
            Button(action: remote.capture) {
                ZStack {
                    Circle().strokeBorder(color, lineWidth: 2)
                        .frame(width: 54, height: 54)
                    if remote.pending {
                        ProgressView().tint(color)
                    } else if recording {
                        RoundedRectangle(cornerRadius: 5)
                            .fill(color)
                            .frame(width: 22, height: 22)
                    } else {
                        Circle().fill(color)
                            .frame(width: 42, height: 42)
                    }
                }
                .frame(width: 72, height: 72)
                .contentShape(Circle())
                .modifier(CaptureGlass())
            }
            .buttonStyle(.plain)
            .opacity(remote.canCapture || remote.pending ? 1 : 0.5)
            .disabled(!remote.canCapture)
            .accessibilityLabel(title)

            Text(title)
                .font(.footnote.weight(.medium))
                .accessibilityHidden(true)
        }
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity)
    }

    private func remaining(_ status: CameraStatus) -> String {
        if status.photoMode {
            return "剩余 \(status.remainingPhotos) 张照片"
        }
        return "可录制 \(duration(status.remainingSeconds))"
    }

    private func duration(_ seconds: Int) -> String {
        if seconds >= 3600 {
            return String(format: "%d:%02d:%02d", seconds / 3600, seconds / 60 % 60, seconds % 60)
        }
        return String(format: "%02d:%02d", seconds / 60, seconds % 60)
    }

    private func batterySymbol(_ percent: Int) -> String {
        switch percent {
        case ...15: "battery.0percent"
        case ...37: "battery.25percent"
        case ...62: "battery.50percent"
        case ...87: "battery.75percent"
        default: "battery.100percent"
        }
    }
}

private struct CaptureGlass: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 26.0, *) {
            content.glassEffect(.regular.interactive(), in: .circle)
        } else {
            content.background(.regularMaterial, in: Circle())
        }
    }
}
