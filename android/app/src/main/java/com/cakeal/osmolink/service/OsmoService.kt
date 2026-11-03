package com.cakeal.osmolink.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.cakeal.osmolink.MainActivity
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.OsmoRepository
import com.cakeal.osmolink.data.UiState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch

/**
 * Keeps the BLE connection and GPS stream alive while the activity is away.
 *
 * The service owns no state of its own: it starts while the repository reports
 * an active camera or location stream, and renders the current state into an
 * ongoing notification (recording duration, mode, battery, GPS rate).
 */
class OsmoService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var foregroundStarted = false
    private var lastContent: NotificationContent? = null

    override fun onCreate() {
        super.onCreate()
        createChannel()
        scope.launch {
            OsmoRepository.state.collect { state ->
                if (foregroundStarted) updateNotification(state)
            }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        ensureForeground()
        when (intent?.action) {
            ACTION_DISCONNECT -> OsmoRepository.disconnect()
            ACTION_STOP_GPS -> OsmoRepository.stopGps()
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        scope.cancel()
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }

    private fun ensureForeground() {
        val state = OsmoRepository.state.value
        try {
            startForeground(NOTIFICATION_ID, buildNotification(state), TYPES)
            foregroundStarted = true
            lastContent = notificationContent(state)
        } catch (error: RuntimeException) {
            // Missing permission or a background-start restriction: give up
            // quietly rather than crashing the app.
            Log.w(TAG, "Unable to enter foreground", error)
            stopSelf()
        }
    }

    private fun updateNotification(state: UiState) {
        val content = notificationContent(state)
        if (content == lastContent) return
        lastContent = content
        try {
            NotificationManagerCompat.from(this).notify(NOTIFICATION_ID, buildNotification(state))
        } catch (_: SecurityException) {
            // The notification permission can be revoked at runtime; keep running.
        }
    }

    private fun buildNotification(state: UiState): Notification {
        val content = notificationContent(state)
        val openApp = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_osmo)
            .setContentTitle(content.title)
            .setContentText(content.text)
            .setStyle(NotificationCompat.BigTextStyle().bigText(content.text))
            .setContentIntent(openApp)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setShowWhen(false)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)

        if (state.connected) {
            builder.addAction(0, "断开", serviceAction(ACTION_DISCONNECT, 1))
        }
        if (state.gpsEnabled) {
            builder.addAction(0, "停止定位", serviceAction(ACTION_STOP_GPS, 2))
        }
        return builder.build()
    }

    private fun serviceAction(action: String, requestCode: Int): PendingIntent =
        PendingIntent.getService(
            this,
            requestCode,
            Intent(this, OsmoService::class.java).setAction(action),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

    private fun createChannel() {
        val manager = getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            CHANNEL_ID,
            "相机连接",
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "相机连接、录制时长与定位状态"
            setShowBadge(false)
        }
        manager.createNotificationChannel(channel)
    }

    companion object {
        private const val TAG = "OsmoService"
        private const val CHANNEL_ID = "osmo_link_camera"
        private const val NOTIFICATION_ID = 1001
        private const val ACTION_DISCONNECT = "com.cakeal.osmolink.action.DISCONNECT"
        private const val ACTION_STOP_GPS = "com.cakeal.osmolink.action.STOP_GPS"

        /** Android 14+ requires declared types; minSdk 29 supports the typed call. */
        private const val TYPES =
            ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE or
                ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION

        fun start(context: Context): Boolean = try {
            ContextCompat.startForegroundService(context, Intent(context, OsmoService::class.java))
            true
        } catch (error: RuntimeException) {
            Log.w(TAG, "Unable to start foreground service", error)
            false
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, OsmoService::class.java))
        }
    }
}
