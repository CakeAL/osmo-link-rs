package com.cakeal.osmolink.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.cakeal.osmolink.MainActivity
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.OsmoRepository
import com.cakeal.osmolink.data.UiState
import com.cakeal.osmolink.ui.isRecording
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
 *
 * While recording it also asks Android 16+ to promote the notification to a
 * Live Update; older systems get the same content as a normal ongoing notice.
 */
class OsmoService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var foregroundTypes = 0
    private var lastContent: NotificationContent? = null
    private var lastPromotionRequested = false
    private var promotionFailed = false
    private var recordingStartedAt: Long? = null

    override fun onCreate() {
        super.onCreate()
        createChannel()
        scope.launch {
            OsmoRepository.state.collect { state ->
                if (foregroundTypes != 0) updateNotification(state)
            }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        updateNotification(OsmoRepository.state.value)
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

    private fun updateNotification(state: UiState) {
        val types = foregroundTypes(state)
        if (types == 0) {
            stopSelf()
            return
        }
        val content = notificationContent(state)
        val startedAt = recordingStart(state)
        val requestPromotion = startedAt != null && !promotionFailed && canPromoteLiveUpdate()
        if (types == foregroundTypes && content == lastContent && requestPromotion == lastPromotionRequested) return

        fun post(notification: Notification) {
            if (types == foregroundTypes) {
                NotificationManagerCompat.from(this).notify(NOTIFICATION_ID, notification)
            } else {
                startForeground(NOTIFICATION_ID, notification, types)
                foregroundTypes = types
            }
        }

        val promoted = if (requestPromotion) {
            runCatching { postLiveUpdate(content, state, startedAt, ::post) }.getOrElse {
                promotionFailed = true
                Log.w(TAG, "Live update failed; using standard notification", it)
                false
            }
        } else {
            false
        }
        if (!promoted) {
            try {
                post(buildCompatNotification(content, state, startedAt))
            } catch (error: RuntimeException) {
                Log.w(TAG, "Unable to publish foreground notification", error)
                if (foregroundTypes == 0) stopSelf()
                return
            }
        }
        lastContent = content
        lastPromotionRequested = requestPromotion && !promotionFailed
    }

    private fun foregroundTypes(state: UiState): Int {
        val camera = if (state.connected) ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE else 0
        val location = if (state.gpsEnabled) ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION else 0
        return camera or location
    }

    private fun buildCompatNotification(
        content: NotificationContent,
        state: UiState,
        startedAt: Long?,
    ): Notification {
        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_osmo)
            .setContentTitle(content.title)
            .setContentText(content.text)
            .setStyle(NotificationCompat.BigTextStyle().bigText(content.text))
            .setContentIntent(openApp())
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setShowWhen(startedAt != null)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)

        if (startedAt != null) {
            // The chronometer ticks natively even if status pushes stall.
            builder.setWhen(startedAt).setUsesChronometer(true)
        }
        if (state.connected) {
            builder.addAction(0, "断开", serviceAction(ACTION_DISCONNECT, 1))
        }
        if (state.gpsEnabled) {
            builder.addAction(0, "停止定位", serviceAction(ACTION_STOP_GPS, 2))
        }
        return builder.build()
    }

    private fun postLiveUpdate(
        content: NotificationContent,
        state: UiState,
        startedAt: Long,
        post: (Notification) -> Unit,
    ): Boolean {
        if (Build.VERSION.SDK_INT < 36) return false
        val notification = buildLiveUpdate(content, state, startedAt)
        if (!notification.hasPromotableCharacteristics()) return false
        post(notification)
        return true
    }

    @RequiresApi(36)
    private fun buildLiveUpdate(
        content: NotificationContent,
        state: UiState,
        startedAt: Long,
    ): Notification {
        val builder = Notification.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_osmo)
            .setContentTitle(content.title)
            .setContentText(content.text)
            .setContentIntent(openApp())
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setShowWhen(true)
            .setWhen(startedAt)
            .setUsesChronometer(true)
            .setCategory(Notification.CATEGORY_SERVICE)
            .setVisibility(Notification.VISIBILITY_PUBLIC)
            .setStyle(Notification.ProgressStyle().setProgressIndeterminate(true))
            .setRequestPromotedOngoing(true)
            .setShortCriticalText("REC")

        if (state.connected) {
            builder.addAction(platformAction("断开", ACTION_DISCONNECT, 1))
        }
        if (state.gpsEnabled) {
            builder.addAction(platformAction("停止定位", ACTION_STOP_GPS, 2))
        }
        return builder.build()
    }

    private fun canPromoteLiveUpdate(): Boolean {
        if (Build.VERSION.SDK_INT < 36) return false
        // Some OEM builds expose the API before the implementation is stable.
        // Keep promotion off on Xiaomi until it has been validated on device.
        if (Build.MANUFACTURER.equals("xiaomi", ignoreCase = true)) return false
        return try {
            getSystemService(NotificationManager::class.java)
                ?.canPostPromotedNotifications() == true
        } catch (_: Throwable) {
            false
        }
    }

    private fun recordingStart(state: UiState): Long? {
        if (!state.connected || !state.status.isRecording) {
            recordingStartedAt = null
            return null
        }
        val existing = recordingStartedAt
        if (existing != null) return existing
        val started = System.currentTimeMillis() - state.status.recordSeconds * 1_000L
        recordingStartedAt = started
        return started
    }

    private fun openApp(): PendingIntent = PendingIntent.getActivity(
        this,
        0,
        Intent(this, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    private fun platformAction(title: String, action: String, requestCode: Int): Notification.Action =
        Notification.Action.Builder(
            Icon.createWithResource(this, R.drawable.ic_stat_osmo),
            title,
            serviceAction(action, requestCode),
        ).build()

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
