import ActivityKit
import OSLog

@MainActor
protocol RecordingActivityUpdating {
    func reconcile(cameraName: String, state: RecordingState?, canStart: Bool)
}

@available(iOS 16.1, *)
@MainActor
final class RecordingActivity: RecordingActivityUpdating {
    private struct Update: Equatable {
        var cameraName: String
        var state: RecordingState?
        var canStart: Bool
    }

    private let logger = Logger(subsystem: "com.cakeal.osmolink", category: "RecordingActivity")
    private var previousIDs = Activity<RecordingAttributes>.activities.map(\.id)
    private var published: (id: String, cameraName: String, state: RecordingState)?
    private var desired: Update?
    private var task: Task<Void, Never>?

    func reconcile(cameraName: String, state: RecordingState?, canStart: Bool) {
        let update = Update(cameraName: cameraName, state: state, canStart: canStart)
        guard desired != update else { return }
        desired = update
        guard task == nil else { return }
        task = Task {
            // After a process restart there is no live BLE connection to these recordings.
            for id in previousIDs {
                await Self.publish(nil, to: id)
            }
            previousIDs.removeAll()
            var applied: Update?
            while let update = desired, update != applied {
                await apply(update)
                applied = update
            }
            task = nil
        }
    }

    private func apply(_ update: Update) async {
        if let published, update.state == nil || published.cameraName != update.cameraName {
            self.published = nil
            await Self.publish(nil, to: published.id)
        }
        guard let state = update.state else { return }
        if let published {
            guard published.state != state else { return }
            await Self.publish(state, to: published.id)
            self.published?.state = state
        } else if update.canStart, !state.reconnecting, ActivityAuthorizationInfo().areActivitiesEnabled {
            let attributes = RecordingAttributes(cameraName: update.cameraName)
            do {
                let activity: Activity<RecordingAttributes>
                if #available(iOS 16.2, *) {
                    activity = try Activity.request(
                        attributes: attributes,
                        content: ActivityContent(state: state, staleDate: nil)
                    )
                } else {
                    activity = try Activity.request(attributes: attributes, contentState: state)
                }
                published = (activity.id, update.cameraName, state)
            } catch {
                logger.error("Could not start recording Live Activity: \(error.localizedDescription)")
            }
        }
    }

    // Fetch the handle in the same isolation domain as the asynchronous ActivityKit calls.
    private nonisolated static func publish(_ state: RecordingState?, to id: String) async {
        guard let activity = Activity<RecordingAttributes>.activities.first(where: { $0.id == id }) else {
            return
        }
        if #available(iOS 16.2, *) {
            if let state {
                await activity.update(ActivityContent(state: state, staleDate: nil))
            } else {
                await activity.end(nil, dismissalPolicy: .immediate)
            }
        } else if let state {
            await activity.update(using: state)
        } else {
            await activity.end(dismissalPolicy: .immediate)
        }
    }
}
