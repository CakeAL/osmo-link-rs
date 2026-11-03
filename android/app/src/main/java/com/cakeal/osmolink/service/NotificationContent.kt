package com.cakeal.osmolink.service

import com.cakeal.osmolink.data.UiState
import com.cakeal.osmolink.ui.PHOTO_MODES
import com.cakeal.osmolink.ui.formatDuration
import com.cakeal.osmolink.ui.formatRemainingDuration
import com.cakeal.osmolink.ui.modeLabel

/** Plain text for the persistent notification, kept free of Android types. */
internal data class NotificationContent(val title: String, val text: String)

internal fun notificationContent(state: UiState): NotificationContent {
    val name = state.deviceName.ifBlank { "相机" }
    val title = when {
        state.connected && state.status.capturing ->
            "$name · 录制中 ${formatDuration(state.status.recordSeconds)}"
        state.connected -> "$name · 已连接"
        state.gpsEnabled -> "Osmo Link · 定位注入"
        else -> "Osmo Link"
    }
    val parts = buildList {
        if (state.connected) {
            if (state.status.hasData) {
                val summary = listOf(
                    modeLabel(state.status.modeName),
                    state.status.resolution,
                    state.status.fps,
                ).filter { it.isNotBlank() }.joinToString(" ")
                if (summary.isNotBlank()) add(summary)
                add("电量 ${state.status.battery}%")
                if (state.status.modeName in PHOTO_MODES) {
                    add("剩余 ${state.status.remainingPhotos} 张")
                } else if (state.status.remainingSeconds > 0) {
                    add("剩余 ${formatRemainingDuration(state.status.remainingSeconds)}")
                }
            } else {
                add("等待相机状态")
            }
        }
        if (state.gpsEnabled) {
            add("定位 ${"%.1f".format(state.gpsRate)}Hz · ${state.gpsSatellites} 星")
        }
        if (!state.connected && !state.gpsEnabled) {
            add("未连接")
        }
    }
    return NotificationContent(title, parts.joinToString(" · "))
}
