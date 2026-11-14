import Foundation
import OSLog
import SwiftUI
import UIKit
import UserNotifications

private let bridgeCallback: @convention(c) (UnsafePointer<CChar>?) -> Void = { event in
    guard let event else { return }
    // The Rust string is valid only while the C callback is executing.
    let json = String(cString: event)
    DispatchQueue.main.async {
        RemoteStore.shared.receive(json)
    }
}

/// One line in the home screen activity feed.
struct RecentEvent: Identifiable, Equatable {
    let id = UUID()
    let at: Date
    let text: String
}

private final class CameraNotificationDelegate: NSObject, UNUserNotificationCenterDelegate {
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }
}

@MainActor
final class RemoteStore: ObservableObject {
    enum Phase: Equatable {
        case idle, scanning, connecting, connected, reconnecting(Int), disconnecting
    }

    static let shared = RemoteStore()
    static let batteryThresholds = [5, 10, 15, 20, 30]
    static let recentEventLimit = 20

    @Published private(set) var phase = Phase.idle
    @Published private(set) var devices: [CameraDevice] = []
    @Published private(set) var pairing: Pairing?
    @Published private(set) var camera: ConnectedCamera?
    @Published private(set) var status: CameraStatus?
    @Published private(set) var version: String?
    @Published private(set) var modeInfo: String?
    @Published private(set) var hasSearched = false
    @Published private(set) var pending = false
    /// Session-scoped activity feed, newest first.
    @Published private(set) var recentEvents: [RecentEvent] = []
    @Published var message: String?
    @Published var locationInjectionEnabled: Bool {
        didSet {
            UserDefaults.standard.set(locationInjectionEnabled, forKey: "locationInjectionEnabled")
            guard phase == .connected else { return }
            if locationInjectionEnabled {
                location.start()
                record("定位注入已开启")
            } else {
                location.stop()
                record("定位注入已暂停")
            }
        }
    }
    @Published var showsLiveActivity = UserDefaults.standard.object(forKey: "showsLiveActivity") as? Bool ?? true {
        didSet {
            UserDefaults.standard.set(showsLiveActivity, forKey: "showsLiveActivity")
            updateActivity()
        }
    }
    @Published var autoConnectEnabled = UserDefaults.standard.object(forKey: "autoConnectPaired") as? Bool ?? true {
        didSet {
            UserDefaults.standard.set(autoConnectEnabled, forKey: "autoConnectPaired")
        }
    }
    @Published var lowBatteryAlertEnabled = UserDefaults.standard.object(forKey: "lowBatteryAlertEnabled") as? Bool ?? true {
        didSet {
            UserDefaults.standard.set(lowBatteryAlertEnabled, forKey: "lowBatteryAlertEnabled")
        }
    }
    @Published var lowBatteryThreshold = RemoteStore.storedBatteryThreshold {
        didSet {
            UserDefaults.standard.set(lowBatteryThreshold, forKey: "lowBatteryAlertThreshold")
        }
    }
    @Published var temperatureAlertEnabled = UserDefaults.standard.object(forKey: "temperatureAlertEnabled") as? Bool ?? true {
        didSet {
            UserDefaults.standard.set(temperatureAlertEnabled, forKey: "temperatureAlertEnabled")
        }
    }

    let location = LocationInjector()

    private let logger = Logger(subsystem: "com.cakeal.osmolink", category: "Bridge")
    private let notificationDelegate = CameraNotificationDelegate()
    private let activity: (any RecordingActivityUpdating)?
    private var recording: RecordingState?
    private var lowBatteryAlertSent = false
    private var lastTemperatureCode = 0
    /// Feed line to record when the in-flight command succeeds.
    private var pendingCommandNote: String?
    private var isActive = false
    private var started = false

    private init() {
        locationInjectionEnabled =
            UserDefaults.standard.object(forKey: "locationInjectionEnabled") as? Bool ?? true
        if #available(iOS 16.1, *) {
            activity = RecordingActivity()
        } else {
            activity = nil
        }
        UNUserNotificationCenter.current().delegate = notificationDelegate
    }

    private static var storedBatteryThreshold: Int {
        let stored = UserDefaults.standard.integer(forKey: "lowBatteryAlertThreshold")
        return batteryThresholds.contains(stored) ? stored : 10
    }

    var modes: [CameraMode] { camera?.modes ?? [] }

    var canCapture: Bool {
        guard phase == .connected, !pending, let status, !status.modeName.hasPrefix("Unknown(") else {
            return false
        }
        return !status.photoMode || !status.capturing
    }

    var canChangeMode: Bool {
        phase == .connected && !pending && status?.capturing == false && !modes.isEmpty
    }

    func activate(_ active: Bool) {
        isActive = active
        updateActivity()
    }

    func search() {
        guard phase == .idle, startBridge() else { return }
        guard osmo_ios_scan(autoConnectEnabled) else {
            message = "暂时无法开始搜索，请重试。"
            return
        }
        devices = []
        message = nil
        hasSearched = true
        phase = .scanning
    }

    func requestNotificationPermission() {
        guard lowBatteryAlertEnabled || temperatureAlertEnabled else { return }
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
    }

    func connect(_ device: CameraDevice) {
        guard phase == .idle, let index = devices.firstIndex(where: { $0.id == device.id }) else { return }
        if osmo_ios_connect(UInt32(index)) {
            message = nil
            phase = .connecting
        } else {
            message = "暂时无法连接相机，请重试。"
        }
    }

    func disconnect() {
        guard camera != nil, phase != .disconnecting else { return }
        if osmo_ios_disconnect() {
            phase = .disconnecting
        } else {
            message = "暂时无法断开连接，请重试。"
        }
    }

    func capture() {
        guard canCapture, let status else { return }
        pendingCommandNote = status.photoMode ? "已拍照" : (status.isRecording ? "停止录像" : "开始录像")
        queueCommand(osmo_ios_capture(status.modeCode, status.capturing))
    }

    func selectMode(_ mode: CameraMode) {
        guard canChangeMode, mode.code != status?.modeCode, modes.contains(where: { $0.code == mode.code }) else { return }
        queueCommand(osmo_ios_set_mode(mode.code))
    }

    private func startBridge() -> Bool {
        if started { return true }
        do {
            let support = try FileManager.default.url(
                for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true
            )
            let directory = support.appendingPathComponent("Osmo Link", isDirectory: true)
            started = directory.path.withCString { osmo_ios_start($0, bridgeCallback) }
            if !started { message = "无法启动蓝牙控制模块。" }
        } catch {
            message = error.localizedDescription
        }
        return started
    }

    private func queueCommand(_ accepted: Bool) {
        pending = accepted
        if !accepted {
            pendingCommandNote = nil
            message = "暂时无法发送命令，请重试。"
        }
    }

    fileprivate func receive(_ json: String) {
        let event: BridgeEvent
        do {
            event = try JSONDecoder().decode(BridgeEvent.self, from: Data(json.utf8))
        } catch {
            logger.error("Could not decode camera event: \(error.localizedDescription)")
            return
        }
        switch event {
        case let .devices(devices):
            self.devices = devices
            phase = .idle
        case let .pairing(info):
            pairing = info
            // The bridge connects automatically right after a scan finds a
            // saved camera, without a matching tap in the UI.
            if phase == .idle || phase == .scanning {
                phase = .connecting
            }
        case let .connected(camera):
            self.camera = camera
            pairing = nil
            phase = .connected
            lowBatteryAlertSent = false
            lastTemperatureCode = 0
            record("已连接 \(camera.title)")
            requestNotificationPermission()
            if locationInjectionEnabled {
                location.start()
            }
        case let .status(status):
            if phase == .connected {
                let previous = self.status
                if previous?.modeCode != status.modeCode {
                    modeInfo = nil
                    record("相机模式切换至\(status.modeTitle) (0x\(String(status.modeCode, radix: 16, uppercase: true)))")
                }
                if let previous,
                   status.temperatureCode >= 2,
                   status.temperatureCode > previous.temperatureCode {
                    record(temperatureEventText(status.temperatureCode))
                }
                self.status = status
                updateRecording(from: status)
                checkLowBattery(status.battery)
                checkTemperature(status.temperatureCode)
            }
        case let .version(details):
            version = details.isEmpty ? nil : details
        case let .modeInfo(name, parameters):
            modeInfo = [name, parameters].filter { !$0.isEmpty }.joined(separator: " · ")
        case let .reconnecting(attempt):
            if phase != .disconnecting {
                phase = .reconnecting(attempt)
                status = nil
                modeInfo = nil
                recording?.reconnecting = true
                record("连接中断，正在重连（第 \(attempt) 次）")
                updateActivity()
            }
        case .reconnected:
            if phase != .disconnecting {
                phase = .connected
                record("已重新连接")
                if locationInjectionEnabled { location.start() }
            }
        case let .disconnected(reason):
            phase = .idle
            pairing = nil
            camera = nil
            status = nil
            version = nil
            modeInfo = nil
            pending = false
            recording = nil
            lowBatteryAlertSent = false
            lastTemperatureCode = 0
            pendingCommandNote = nil
            location.stop()
            record("已断开连接")
            message = reason
            updateActivity()
        case .commandDone:
            pending = false
            if let note = pendingCommandNote {
                record(note)
                pendingCommandNote = nil
            }
        case let .commandFailed(error):
            pending = false
            pendingCommandNote = nil
            record("操作失败：\(error)")
            message = error
        case let .scanFailed(error):
            phase = .idle
            pairing = nil
            record("搜索失败：\(error)")
            message = error
        case let .connectFailed(error, auto):
            phase = .idle
            pairing = nil
            if !auto {
                // Auto-connect misses on every scan when no paired camera is
                // nearby; only user-initiated failures belong in the feed.
                record("连接失败：\(error)")
            }
            message = auto ? "自动连接失败：\(error)" : error
        case let .warning(warning):
            logger.warning("\(warning, privacy: .public)")
            if phase == .connected || phase == .disconnecting { message = warning }
        }
    }

    private func checkLowBattery(_ battery: Int) {
        guard lowBatteryAlertEnabled else { return }
        guard battery < lowBatteryThreshold else {
            lowBatteryAlertSent = false
            return
        }
        guard !lowBatteryAlertSent else { return }
        lowBatteryAlertSent = true
        record("相机电量低：\(battery)%")

        UINotificationFeedbackGenerator().notificationOccurred(.warning)
        let content = UNMutableNotificationContent()
        content.title = "相机电量低"
        content.body = "\(camera?.title ?? "相机") 电量仅剩 \(battery)%，请及时充电或更换电池。"
        content.sound = .default
        let request = UNNotificationRequest(
            identifier: "low-battery-\(UUID().uuidString)",
            content: content,
            trigger: nil
        )
        UNUserNotificationCenter.current().add(request, withCompletionHandler: nil)
    }

    private func checkTemperature(_ code: Int) {
        let previous = lastTemperatureCode
        lastTemperatureCode = code
        guard temperatureAlertEnabled, code >= 2, code > previous else { return }

        UINotificationFeedbackGenerator().notificationOccurred(.warning)
        let content = UNMutableNotificationContent()
        content.title = code >= 3 ? "相机即将关机" : "相机温度过高"
        content.body = temperatureMessage(code)
        content.sound = .default
        let request = UNNotificationRequest(
            identifier: "temperature-\(UUID().uuidString)",
            content: content,
            trigger: nil
        )
        UNUserNotificationCenter.current().add(request, withCompletionHandler: nil)
    }

    private func temperatureMessage(_ code: Int) -> String {
        let name = camera?.title ?? "相机"
        return code >= 3
            ? "\(name) 温度过高，即将自动关机。请立即停止拍摄并降温。"
            : "\(name) 温度高，已无法录制。请暂停使用并等待降温。"
    }

    private func temperatureEventText(_ code: Int) -> String {
        code >= 3 ? "相机温度过高，即将关机" : "相机温度过高，无法录制"
    }

    /// Newest first, with a short window to absorb duplicate pushes.
    private func record(_ text: String) {
        let now = Date()
        if let latest = recentEvents.first,
           latest.text == text,
           now.timeIntervalSince(latest.at) < 2 {
            return
        }
        recentEvents.insert(RecentEvent(at: now, text: text), at: 0)
        if recentEvents.count > Self.recentEventLimit {
            recentEvents.removeLast(recentEvents.count - Self.recentEventLimit)
        }
    }

    private func updateRecording(from status: CameraStatus) {
        if status.isRecording {
            let estimate = Date().addingTimeInterval(-Double(status.recordSeconds))
            var startedAt = estimate
            if let recording, !recording.reconnecting, abs(recording.startedAt.timeIntervalSince(estimate)) < 3 {
                startedAt = recording.startedAt
            }
            recording = RecordingState(startedAt: startedAt, mode: status.modeTitle, battery: status.battery)
        } else {
            recording = nil
        }
        updateActivity()
    }

    private func updateActivity() {
        activity?.reconcile(
            cameraName: camera?.name ?? "",
            state: showsLiveActivity ? recording : nil,
            canStart: isActive
        )
    }
}
