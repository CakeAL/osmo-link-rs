import Foundation

struct CameraDevice: Decodable, Identifiable {
    let id: String
    let name: String
    let rssi: Int?
}

struct CameraMode: Decodable, Identifiable {
    let code: UInt8
    let name: String

    var id: UInt8 { code }
    var title: String { Self.title(for: name) }

    static func title(for name: String) -> String {
        switch name {
        case "Video": "视频"
        case "Photo": "拍照"
        case "SlowMotion": "慢动作"
        case "Timelapse": "延时"
        case "Hyperlapse": "移动延时"
        case "SuperNight": "超级夜景"
        case "SubjectTracking": "人像跟随"
        case "Selfie": "自拍"
        case "PanoramicVideo": "全景视频"
        case "PanoramicPhoto": "全景照片"
        case "PanoramicHyperlapse": "全景延时"
        case "PanoramicSuperNight": "全景夜景"
        case "SingleLensSuperNight": "单镜夜景"
        case "UltraWideVideo": "超广角"
        case "TimeFreeze": "时间静止"
        case "LiveStream": "直播"
        case "UvcLiveStream": "UVC 直播"
        default: "未知模式"
        }
    }

    static func symbol(for name: String) -> String {
        switch name {
        case "Photo", "PanoramicPhoto": "camera.fill"
        case "SlowMotion", "Timelapse": "clock"
        case "Hyperlapse", "PanoramicHyperlapse", "TimeFreeze": "timer"
        case "SuperNight", "PanoramicSuperNight", "SingleLensSuperNight": "moon.stars.fill"
        case "SubjectTracking", "Selfie": "viewfinder"
        case "PanoramicVideo": "globe"
        case "LiveStream", "UvcLiveStream": "dot.radiowaves.left.and.right"
        default: "video.fill"
        }
    }
}

struct CameraStatus: Decodable {
    let modeCode: UInt8
    let modeName: String
    let stateName: String
    let photoMode: Bool
    let capturing: Bool
    let resolution: String
    let fps: String
    let eis: String
    let recordSeconds: Int
    let remainingMb: Int
    let remainingPhotos: Int
    let remainingSeconds: Int
    let battery: Int
    let temperatureCode: Int

    var modeTitle: String { CameraMode.title(for: modeName) }
    var isRecording: Bool { capturing && !photoMode && !modeName.hasPrefix("Unknown(") }

    var stateTitle: String {
        if isRecording { return "录制中" }
        switch stateName {
        case "Capturing": return "拍摄中"
        case "LiveView": return "待机"
        case "Playback": return "回放"
        case "PreRecording": return "预录制"
        case "ScreenOff": return "息屏"
        default: return "未知状态"
        }
    }

    var temperatureWarning: String? {
        switch temperatureCode {
        case 2: "相机过热，无法录制"
        case 3: "相机过热，即将关机"
        default: nil
        }
    }
}

struct Pairing: Decodable, Identifiable {
    let device: String
    let firstTime: Bool
    let code: Int

    var id: String { device }
}

struct ConnectedCamera: Decodable {
    let name: String
    let model: String
    let modes: [CameraMode]

    var title: String {
        switch model {
        case "Action4": "Osmo Action 4"
        case "Action5Pro": "Osmo Action 5 Pro"
        case "Action6": "Osmo Action 6"
        case "Osmo360": "Osmo 360"
        default: name
        }
    }
}

enum BridgeEvent: Decodable {
    case devices([CameraDevice])
    case pairing(Pairing)
    case connected(ConnectedCamera)
    case status(CameraStatus)
    case version(String)
    case modeInfo(String, String)
    case reconnecting(Int)
    case reconnected
    case disconnected(String?)
    case commandDone
    case commandFailed(String)
    case scanFailed(String)
    case connectFailed(String)
    case warning(String)

    private enum Keys: String, CodingKey {
        case kind, payload, devices, details, attempt, message, name, parameters
    }

    init(from decoder: Decoder) throws {
        let event = try decoder.container(keyedBy: Keys.self)
        let payload = try event.nestedContainer(keyedBy: Keys.self, forKey: .payload)
        let kind = try event.decode(String.self, forKey: .kind)
        switch kind {
        case "devices": self = .devices(try payload.decode([CameraDevice].self, forKey: .devices))
        case "pairing": self = .pairing(try event.decode(Pairing.self, forKey: .payload))
        case "connected": self = .connected(try event.decode(ConnectedCamera.self, forKey: .payload))
        case "status": self = .status(try event.decode(CameraStatus.self, forKey: .payload))
        case "version": self = .version(try payload.decode(String.self, forKey: .details))
        case "modeInfo":
            self = .modeInfo(
                try payload.decode(String.self, forKey: .name),
                try payload.decode(String.self, forKey: .parameters)
            )
        case "reconnecting": self = .reconnecting(try payload.decode(Int.self, forKey: .attempt))
        case "reconnected": self = .reconnected
        case "disconnected": self = .disconnected(try payload.decodeIfPresent(String.self, forKey: .message))
        case "commandDone": self = .commandDone
        case "commandFailed": self = .commandFailed(try payload.decode(String.self, forKey: .message))
        case "scanFailed": self = .scanFailed(try payload.decode(String.self, forKey: .message))
        case "connectFailed": self = .connectFailed(try payload.decode(String.self, forKey: .message))
        case "warning": self = .warning(try payload.decode(String.self, forKey: .message))
        default:
            throw DecodingError.dataCorruptedError(forKey: .kind, in: event, debugDescription: kind)
        }
    }
}
