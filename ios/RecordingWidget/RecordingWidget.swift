import ActivityKit
import SwiftUI
import WidgetKit

@main
struct OsmoRecordingWidget: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: RecordingAttributes.self) { context in
            VStack(alignment: .leading, spacing: 8) {
                Text(context.attributes.cameraName)
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Label(context.state.mode, systemImage: "record.circle.fill")
                        .foregroundStyle(.red)
                        .lineLimit(1)
                    Spacer(minLength: 0)
                    RecordingTimer(state: context.state)
                        .font(.title3.weight(.semibold))
                }
                BatteryLevel(percent: context.state.battery)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .padding()
            .widgetURL(URL(string: "osmolink://remote"))
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    Label(context.state.mode, systemImage: "record.circle.fill")
                        .foregroundStyle(.red)
                        .lineLimit(1)
                }
                DynamicIslandExpandedRegion(.trailing) {
                    BatteryLevel(percent: context.state.battery)
                        .font(.subheadline.monospacedDigit())
                }
                DynamicIslandExpandedRegion(.bottom) {
                    HStack {
                        Text(context.attributes.cameraName)
                            .lineLimit(1)
                            .foregroundStyle(.secondary)
                        Spacer(minLength: 8)
                        RecordingTimer(state: context.state)
                            .font(.headline)
                    }
                    .padding(.top, 4)
                }
            } compactLeading: {
                recordingIcon
            } compactTrailing: {
                if context.state.reconnecting {
                    Image(systemName: "arrow.clockwise")
                        .accessibilityLabel("正在重连")
                } else {
                    RecordingTimer(state: context.state)
                        .font(.caption.weight(.medium))
                        .frame(width: 47, alignment: .trailing)
                }
            } minimal: {
                recordingIcon
            }
            .widgetURL(URL(string: "osmolink://remote"))
            .keylineTint(.red)
        }
    }

    private var recordingIcon: some View {
        Image(systemName: "record.circle.fill")
            .foregroundStyle(.red)
            .accessibilityLabel("相机录制")
    }
}

private struct RecordingTimer: View {
    let state: RecordingState

    var body: some View {
        if state.reconnecting {
            Text("重连中")
                .foregroundStyle(.secondary)
        } else {
            Text(state.startedAt, style: .timer)
                .monospacedDigit()
                .multilineTextAlignment(.trailing)
                .lineLimit(1)
                .minimumScaleFactor(0.65)
        }
    }
}

private struct BatteryLevel: View {
    let percent: Int

    var body: some View {
        Label("\(percent)%", systemImage: percent <= 15 ? "battery.0percent" : "battery.100percent")
            .monospacedDigit()
            .accessibilityLabel("相机电量 \(percent)%")
    }
}
