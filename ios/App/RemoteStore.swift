import Foundation
import OSLog
import SwiftUI

private let bridgeCallback: @convention(c) (UnsafePointer<CChar>?) -> Void = { event in
    guard let event else { return }
    // The Rust string is valid only while the C callback is executing.
    let json = String(cString: event)
    DispatchQueue.main.async {
        RemoteStore.shared.receive(json)
    }
}

@MainActor
final class RemoteStore: ObservableObject {
    enum Phase: Equatable {
        case idle, scanning, connecting, connected, reconnecting(Int), disconnecting
    }

    static let shared = RemoteStore()

    @Published private(set) var phase = Phase.idle
    @Published private(set) var devices: [CameraDevice] = []
    @Published private(set) var modes: [CameraMode] = []
    @Published private(set) var pairing: Pairing?
    @Published private(set) var camera: ConnectedCamera?
    @Published private(set) var status: CameraStatus?
    @Published private(set) var version: String?
    @Published private(set) var modeInfo: String?
    @Published private(set) var hasSearched = false
    @Published private(set) var pending = false
    @Published var message: String?
    @Published var showsLiveActivity = UserDefaults.standard.object(forKey: "showsLiveActivity") as? Bool ?? true {
        didSet {
            UserDefaults.standard.set(showsLiveActivity, forKey: "showsLiveActivity")
            updateActivity()
        }
    }

    private let logger = Logger(subsystem: "com.cakeal.osmolink", category: "Bridge")
    private let activity: (any RecordingActivityUpdating)?
    private var recording: RecordingState?
    private var isActive = false
    private var started = false

    private init() {
        if #available(iOS 16.1, *) {
            activity = RecordingActivity()
        } else {
            activity = nil
        }
    }

    var canCapture: Bool {
        guard phase == .connected, !pending, let status, modes.contains(where: { $0.code == status.modeCode }) else {
            return false
        }
        return !status.photoMode || !status.capturing
    }

    var canChangeMode: Bool {
        phase == .connected && !pending && status?.capturing == false
    }

    func activate(_ active: Bool) {
        isActive = active
        updateActivity()
    }

    func search() {
        guard phase == .idle, startBridge() else { return }
        guard osmo_ios_scan() else {
            message = "暂时无法开始搜索，请重试。"
            return
        }
        devices = []
        message = nil
        hasSearched = true
        phase = .scanning
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
        queueCommand(osmo_ios_capture(status.modeCode, status.capturing))
    }

    func selectMode(_ mode: CameraMode) {
        guard canChangeMode, mode.code != status?.modeCode else { return }
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
        if !accepted { message = "暂时无法发送命令，请重试。" }
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
        case let .modes(modes):
            self.modes = modes
        case let .devices(devices):
            self.devices = devices
            phase = .idle
        case let .pairing(info):
            pairing = info
        case let .connected(camera):
            self.camera = camera
            pairing = nil
            phase = .connected
        case let .status(status):
            if phase == .connected {
                if self.status?.modeCode != status.modeCode { modeInfo = nil }
                self.status = status
                updateRecording(from: status)
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
                updateActivity()
            }
        case .reconnected:
            if phase != .disconnecting { phase = .connected }
        case let .disconnected(reason):
            phase = .idle
            pairing = nil
            camera = nil
            status = nil
            version = nil
            modeInfo = nil
            pending = false
            recording = nil
            message = reason
            updateActivity()
        case .commandDone:
            pending = false
        case let .commandFailed(error):
            pending = false
            message = error
        case let .scanFailed(error), let .connectFailed(error):
            phase = .idle
            pairing = nil
            message = error
        case let .warning(warning):
            logger.warning("\(warning, privacy: .public)")
            if phase == .connected || phase == .disconnecting { message = warning }
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
