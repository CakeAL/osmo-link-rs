import SwiftUI

struct CameraView: View {
    @ObservedObject var remote: RemoteStore
    @ObservedObject var location: LocationInjector
    let camera: ConnectedCamera

    @Environment(\.openURL) private var openURL

    var body: some View {
        ScrollView {
            VStack(spacing: 18) {
                statusHero
                modeCard
                detailsCard
                gpsCard
            }
            .frame(maxWidth: AppTheme.contentWidth)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 20)
            .padding(.top, 8)
            .padding(.bottom, 24)
        }
        .scrollIndicators(.hidden)
        .background(AppBackground())
        .safeAreaInset(edge: .bottom, spacing: 0) {
            captureDock
        }
    }

    private var statusHero: some View {
        ZStack(alignment: .topTrailing) {
            LinearGradient(
                colors: [
                    Color.accentColor.opacity(0.96),
                    Color.accentColor.opacity(0.66),
                    Color.black.opacity(0.86),
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            Circle()
                .fill(.white.opacity(0.09))
                .frame(width: 180, height: 180)
                .offset(x: 68, y: -88)
            Circle()
                .stroke(.white.opacity(0.10), lineWidth: 1)
                .frame(width: 120, height: 120)
                .offset(x: 88, y: 58)

            VStack(alignment: .leading, spacing: 16) {
                HStack(alignment: .top, spacing: 12) {
                    VStack(alignment: .leading, spacing: 3) {
                        Text(camera.title)
                            .font(.headline)
                        connectionLabel
                    }
                    Spacer(minLength: 8)
                    if let status = remote.status {
                        Label("\(status.battery)%", systemImage: batterySymbol(status.battery))
                            .font(.subheadline.weight(.semibold))
                            .monospacedDigit()
                            .padding(.horizontal, 11)
                            .padding(.vertical, 7)
                            .background(.white.opacity(0.14), in: Capsule())
                            .foregroundStyle(status.battery <= 15 ? Color.red : Color.white)
                            .accessibilityLabel("电量 \(status.battery)%")
                    }
                }

                if let status = remote.status {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(status.photoMode ? status.modeTitle : duration(status.recordSeconds))
                            .font(
                                status.photoMode
                                    ? .system(size: 38, weight: .bold, design: .rounded)
                                    : .system(size: 42, weight: .bold, design: .monospaced)
                            )
                            .lineLimit(1)
                            .minimumScaleFactor(0.55)
                            .contentTransition(.numericText())
                            .accessibilityLabel(status.photoMode ? status.modeTitle : "录制时长 \(status.recordSeconds) 秒")

                        HStack(spacing: 7) {
                            if !status.resolution.isEmpty { heroChip(status.resolution) }
                            heroChip(status.fps)
                            heroChip(remaining(status))
                        }

                        if status.isRecording {
                            Label("录制中", systemImage: "circle.fill")
                                .font(.caption.weight(.bold))
                                .foregroundStyle(.white)
                                .padding(.horizontal, 11)
                                .padding(.vertical, 6)
                                .background(.red.opacity(0.85), in: Capsule())
                        }
                    }
                } else {
                    HStack(spacing: 10) {
                        ProgressView()
                            .tint(.white)
                        Text("正在读取相机状态")
                            .font(.subheadline.weight(.medium))
                    }
                    .padding(.vertical, 10)
                }
            }
            .padding(22)
        }
        .foregroundStyle(.white)
        .clipShape(RoundedRectangle(cornerRadius: 30, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 30, style: .continuous)
                .strokeBorder(.white.opacity(0.12), lineWidth: 1)
        }
        .shadow(color: Color.accentColor.opacity(0.28), radius: 24, y: 12)
    }

    @ViewBuilder
    private var connectionLabel: some View {
        switch remote.phase {
        case let .reconnecting(attempt):
            Text("正在重连 \(attempt)/2")
                .font(.caption)
                .foregroundStyle(.white.opacity(0.76))
        case .disconnecting:
            Text("正在断开")
                .font(.caption)
                .foregroundStyle(.white.opacity(0.76))
        default:
            if let status = remote.status {
                Text(status.stateTitle)
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.76))
            } else {
                Text("蓝牙已连接")
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.76))
            }
        }
    }

    private func heroChip(_ text: String) -> some View {
        Text(text)
            .font(.caption.weight(.medium))
            .foregroundStyle(.white.opacity(0.86))
            .lineLimit(1)
            .padding(.horizontal, 9)
            .padding(.vertical, 5)
            .background(.white.opacity(0.12), in: Capsule())
    }

    private var modeCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 12) {
                CardHeader("拍摄模式", systemImage: "camera.filters")
                if let status = remote.status {
                    ModeSelector(
                        status: status,
                        modes: remote.modes,
                        canSwitch: remote.canChangeMode,
                        onSelect: remote.selectMode
                    )
                    .id(status.modeCode)
                } else {
                    Text("等待相机状态")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var detailsCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 14) {
                CardHeader("相机详情", systemImage: "info.circle.fill")

                VStack(spacing: 11) {
                    if camera.name != camera.title {
                        InfoRow(label: "设备名称", value: camera.name, systemImage: "camera")
                    }
                    if let status = remote.status {
                        InfoRow(
                            label: "可用容量",
                            value: "\((Double(status.remainingMb) / 1024).formatted(.number.precision(.fractionLength(1)))) GB",
                            systemImage: "externaldrive"
                        )
                        InfoRow(label: "防抖", value: status.eis, systemImage: "hand.raised")
                        if let warning = status.temperatureWarning {
                            InfoRow(
                                label: "温度",
                                value: warning,
                                systemImage: "thermometer",
                                valueColor: .red
                            )
                        }
                    }
                    if let version = remote.version {
                        InfoRow(label: "固件版本", value: version, systemImage: "cpu")
                    }
                    if let modeInfo = remote.modeInfo, !modeInfo.isEmpty {
                        InfoRow(label: "模式参数", value: modeInfo, systemImage: "slider.horizontal.3")
                    }
                }

                if remote.status == nil && remote.version == nil {
                    Text("连接稳定后会自动读取。")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var gpsCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 14) {
                CardHeader("定位注入", systemImage: "location.fill") {
                    Toggle("定位注入", isOn: $remote.locationInjectionEnabled)
                        .labelsHidden()
                }

                if remote.locationInjectionEnabled {
                    if location.needsSettings {
                        VStack(alignment: .leading, spacing: 10) {
                            Label(location.message ?? "定位权限已关闭", systemImage: "location.slash.fill")
                                .font(.subheadline)
                                .foregroundStyle(.red)
                            Button {
                                if let url = URL(string: UIApplication.openSettingsURLString) {
                                    openURL(url)
                                }
                            } label: {
                                Label("打开系统设置", systemImage: "gear")
                            }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        }
                    } else if location.hasFix {
                        HStack(spacing: 8) {
                            gpsStat(
                                "频率",
                                value: location.rate > 0
                                    ? "\(location.rate.formatted(.number.precision(.fractionLength(1)))) Hz"
                                    : "—",
                                systemImage: "waveform.path.ecg"
                            )
                            gpsStat(
                                "精度",
                                value: location.accuracy.map { "±\($0.formatted(.number.precision(.fractionLength(1)))) m" } ?? "—",
                                systemImage: "scope"
                            )
                            gpsStat("卫星", value: "0", systemImage: "antenna.radiowaves.left.and.right")
                        }
                    } else {
                        HStack(spacing: 10) {
                            ProgressView()
                            Text(location.isRunning ? "等待首个定位…" : (location.message ?? "正在请求定位权限…"))
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                        }
                        .frame(minHeight: 38)
                    }
                } else {
                    Text("定位注入已暂停。开启后会使用手机定位为相机提供位置与时间。")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                Divider().opacity(0.5)

                Label(
                    "iOS 不提供参与定位的卫星数，当前只能将卫星数字段注入为 0，坐标、时间与精度仍会正常写入。",
                    systemImage: "info.circle"
                )
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private func gpsStat(_ title: String, value: String, systemImage: String) -> some View {
        VStack(spacing: 5) {
            Label(title, systemImage: systemImage)
                .font(.caption2.weight(.medium))
                .foregroundStyle(.secondary)
            Text(value)
                .font(.subheadline.weight(.semibold))
                .monospacedDigit()
                .lineLimit(1)
                .minimumScaleFactor(0.75)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 10)
        .background(Color.secondary.opacity(0.08), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }

    private var captureDock: some View {
        let status = remote.status
        let recording = status?.isRecording == true
        let photo = status?.photoMode == true
        let title = status == nil ? "拍摄" : photo ? "拍照" : recording ? "停止录像" : "开始录像"
        let color: Color = status == nil ? .secondary : photo ? .primary : .red
        return VStack(spacing: 5) {
            Button(action: remote.capture) {
                ZStack {
                    Circle().strokeBorder(color.opacity(0.9), lineWidth: 2)
                        .frame(width: 56, height: 56)
                    if remote.pending {
                        ProgressView().tint(color)
                    } else if recording {
                        RoundedRectangle(cornerRadius: 6)
                            .fill(color)
                            .frame(width: 23, height: 23)
                    } else {
                        Circle().fill(color)
                            .frame(width: 44, height: 44)
                    }
                }
                .frame(width: 74, height: 74)
                .contentShape(Circle())
                .modifier(CaptureGlass())
            }
            .buttonStyle(.plain)
            .opacity(remote.canCapture || remote.pending ? 1 : 0.5)
            .disabled(!remote.canCapture)
            .accessibilityLabel(title)

            Text(title)
                .font(.footnote.weight(.semibold))
                .accessibilityHidden(true)
        }
        .padding(.top, 10)
        .padding(.bottom, 4)
        .frame(maxWidth: .infinity)
        .background(.ultraThinMaterial)
        .overlay(alignment: .top) {
            Rectangle()
                .fill(Color.primary.opacity(0.08))
                .frame(height: 0.5)
        }
    }

    private func remaining(_ status: CameraStatus) -> String {
        if status.photoMode {
            return "剩余 \(status.remainingPhotos) 张"
        }
        return "可录 \(duration(status.remainingSeconds))"
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

private struct ModeSelector: View {
    let status: CameraStatus
    let modes: [CameraMode]
    let canSwitch: Bool
    let onSelect: (CameraMode) -> Void

    @State private var expanded = false
    @State private var previewIndex: Int?
    @GestureState private var dragging = false

    private let dragThreshold: CGFloat = 8
    private let distancePerMode: CGFloat = 32

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button {
                expanded.toggle()
            } label: {
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(previewIndex.map { modes[$0].title } ?? status.modeTitle)
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.primary)
                        Text(hint)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Spacer(minLength: 8)
                    if canSwitch {
                        Image(systemName: expanded ? "chevron.up" : "chevron.down")
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                .frame(minHeight: 48)
                .background(Color.secondary.opacity(0.10), in: RoundedRectangle(cornerRadius: 14))
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(!canSwitch)
            .highPriorityGesture(modeDrag, including: canSwitch ? .all : .none)

            if expanded && canSwitch {
                options
            }
        }
        .onChange(of: dragging) { active in
            if !active { previewIndex = nil }
        }
        .onChange(of: canSwitch) { enabled in
            if !enabled {
                expanded = false
                previewIndex = nil
            }
        }
    }

    private var hint: String {
        if status.capturing { return "拍摄中不可切换" }
        if modes.isEmpty { return "此机型没有可切换的模式" }
        return canSwitch ? "上下拖动或点按" : "暂不可切换"
    }

    private var modeDrag: some Gesture {
        DragGesture(minimumDistance: dragThreshold)
            .updating($dragging) { _, active, _ in active = true }
            .onChanged { value in
                guard canSwitch else { return }
                expanded = true
                previewIndex = index(at: steps(for: value.translation.height))
            }
            .onEnded { value in
                previewIndex = nil
                guard canSwitch else { return }
                let offset = steps(for: value.translation.height)
                guard offset != 0 else { return }
                let mode = modes[index(at: offset)]
                guard mode.code != status.modeCode else { return }
                choose(mode)
            }
    }

    private func steps(for distance: CGFloat) -> Int {
        let travelled = max(0, abs(distance) - dragThreshold)
        return Int(travelled / distancePerMode) * (distance < 0 ? 1 : -1)
    }

    private func index(at offset: Int) -> Int {
        let start = modes.firstIndex(where: { $0.code == status.modeCode }) ?? 0
        return min(max(start + offset, 0), modes.count - 1)
    }

    private var options: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(modes) { mode in
                        option(mode)
                            .id(mode.code)
                    }
                }
            }
            .frame(height: min(CGFloat(modes.count) * 50, 200))
            .scrollDisabled(dragging)
            .onAppear {
                proxy.scrollTo(previewIndex.map { modes[$0].code } ?? status.modeCode, anchor: .center)
            }
            .onChange(of: previewIndex) { index in
                if let index {
                    proxy.scrollTo(modes[index].code, anchor: .center)
                }
            }
        }
    }

    private func option(_ mode: CameraMode) -> some View {
        let selected = mode.code == status.modeCode
        let highlighted = previewIndex.map { modes[$0].code == mode.code } ?? false
        return Button {
            choose(mode)
        } label: {
            HStack(spacing: 8) {
                Text(mode.title)
                Spacer(minLength: 8)
                if selected {
                    Image(systemName: "checkmark")
                        .accessibilityHidden(true)
                }
            }
            .font(.subheadline.weight(selected || highlighted ? .semibold : .regular))
            .foregroundStyle(highlighted || selected ? Color.accentColor : .primary)
            .padding(.horizontal, 14)
            .padding(.vertical, 8)
            .frame(minHeight: 48)
            .background(
                Color.accentColor.opacity(highlighted ? 0.18 : (selected ? 0.10 : 0)),
                in: RoundedRectangle(cornerRadius: 12)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }

    private func choose(_ mode: CameraMode) {
        expanded = false
        if mode.code != status.modeCode { onSelect(mode) }
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
