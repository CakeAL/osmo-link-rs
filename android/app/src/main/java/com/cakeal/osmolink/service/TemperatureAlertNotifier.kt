package com.cakeal.osmolink.service

import android.content.Context

/** Vibrates and posts one notification when the camera reports overheating. */
internal object TemperatureAlertNotifier {
    private const val CHANNEL_ID = "osmo_link_temperature"
    private const val NOTIFICATION_ID = 1003

    /**
     * Whether a status sample should produce a new alert.
     *
     * `temp_over` is a level, not an event: alert when it climbs into a
     * recording-stopping level so a camera that stays hot does not notify
     * on every 2 Hz status push.
     */
    fun shouldAlert(previousCode: Int, code: Int): Boolean = code >= 2 && code > previousCode

    fun title(code: Int): String = if (code >= 3) "相机即将关机" else "相机温度过高"

    fun message(cameraName: String, code: Int): String = if (code >= 3) {
        "$cameraName 温度过高，即将自动关机。请立即停止拍摄并降温。"
    } else {
        "$cameraName 温度高，已无法录制。请暂停使用并等待降温。"
    }

    fun notify(context: Context, cameraName: String, code: Int) {
        AlertNotifier.vibrate(context)
        AlertNotifier.post(
            context = context,
            channelId = CHANNEL_ID,
            channelName = "高温提醒",
            channelDescription = "相机温度过高时通知",
            notificationId = NOTIFICATION_ID,
            title = title(code),
            text = message(cameraName, code),
        )
    }
}
