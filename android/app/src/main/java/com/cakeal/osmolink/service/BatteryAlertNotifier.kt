package com.cakeal.osmolink.service

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.VibrationEffect
import android.os.Vibrator
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import com.cakeal.osmolink.MainActivity
import com.cakeal.osmolink.R

/** Vibrates and posts one notification when a connected camera runs low. */
internal object BatteryAlertNotifier {
    private const val CHANNEL_ID = "osmo_link_battery"
    private const val NOTIFICATION_ID = 1002

    /** Whether a status sample should produce a new alert. */
    fun shouldAlert(battery: Int, threshold: Int, alreadySent: Boolean): Boolean =
        !alreadySent && battery < threshold

    fun notify(context: Context, cameraName: String, battery: Int, threshold: Int) {
        vibrate(context)
        post(context, cameraName, battery, threshold)
    }

    private fun vibrate(context: Context) {
        val vibrator = context.getSystemService(Vibrator::class.java) ?: return
        if (!vibrator.hasVibrator()) return
        vibrator.vibrate(VibrationEffect.createWaveform(longArrayOf(0, 250, 150, 250), -1))
    }

    private fun post(context: Context, cameraName: String, battery: Int, threshold: Int) {
        val manager = NotificationManagerCompat.from(context)
        if (!manager.areNotificationsEnabled()) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            "低电量提醒",
            NotificationManager.IMPORTANCE_HIGH,
        ).apply {
            description = "相机电量低于提醒阈值时通知"
        }
        context.getSystemService(NotificationManager::class.java)?.createNotificationChannel(channel)

        val open = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val text = "$cameraName 电量仅剩 $battery%，低于 $threshold% 提醒阈值。请及时充电。"
        val notification = NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_osmo)
            .setContentTitle("相机电量低")
            .setContentText(text)
            .setStyle(NotificationCompat.BigTextStyle().bigText(text))
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setAutoCancel(true)
            .setContentIntent(open)
            .build()
        try {
            manager.notify(NOTIFICATION_ID, notification)
        } catch (_: SecurityException) {
            // Notification permission was revoked between the check and post.
        }
    }
}
