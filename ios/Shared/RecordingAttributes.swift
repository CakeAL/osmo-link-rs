import ActivityKit
import Foundation

struct RecordingState: Codable, Hashable {
    var startedAt: Date
    var mode: String
    var battery: Int
    var reconnecting = false
}

@available(iOS 16.1, *)
struct RecordingAttributes: ActivityAttributes {
    typealias ContentState = RecordingState

    let cameraName: String
}
