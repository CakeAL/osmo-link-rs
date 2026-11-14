package com.cakeal.osmolink.service

import android.content.Context

/** Vibrates and posts one notification when a connected camera runs low. */
internal object BatteryAlertNotifier {
    private const val CHANNEL_ID = "osmo_link_battery"
    private const val NOTIFICATION_ID = 1002

    /** Whether a status sample should produce a new alert. */
    fun shouldAlert(battery: Int, threshold: Int, alreadySent: Boolean): Boolean =
        !alreadySent && battery < threshold

    fun notify(context: Context, cameraName: String, battery: Int, threshold: Int) {
        AlertNotifier.vibrate(context)
        val text = "$cameraName 电量仅剩 $battery%，低于 $threshold% 提醒阈值。请及时充电。"
        AlertNotifier.post(
            context = context,
            channelId = CHANNEL_ID,
            channelName = "低电量提醒",
            channelDescription = "相机电量低于提醒阈值时通知",
            notificationId = NOTIFICATION_ID,
            title = "相机电量低",
            text = text,
        )
    }
}
