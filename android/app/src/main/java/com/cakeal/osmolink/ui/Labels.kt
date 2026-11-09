package com.cakeal.osmolink.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import com.cakeal.osmolink.data.CameraStatus

/** Modes where the shutter, not start/stop recording, is the primary action. */
internal val PHOTO_MODES = setOf("Photo", "PanoramicPhoto", "SubjectTracking")

internal val CameraStatus.isRecording: Boolean
    get() = capturing && modeName !in PHOTO_MODES

/** Camera metrics use a monospace face inside the Material 3 type scale. */
internal fun TextStyle.mono(): TextStyle = copy(fontFamily = FontFamily.Monospace)

internal fun modeLabel(name: String): String = when (name) {
    "Video" -> "视频"
    "Photo" -> "拍照"
    "SlowMotion" -> "慢动作"
    "Timelapse" -> "延时"
    "Hyperlapse" -> "移动延时"
    "SuperNight" -> "超级夜景"
    "SubjectTracking" -> "人像跟随"
    "Selfie" -> "自拍"
    "PanoramicVideo" -> "全景视频"
    "PanoramicPhoto" -> "全景照片"
    "PanoramicHyperlapse" -> "全景延时"
    "PanoramicSuperNight" -> "全景夜景"
    "SingleLensSuperNight" -> "单镜夜景"
    "UltraWideVideo" -> "超广角"
    "TimeFreeze" -> "时间静止"
    "LiveStream" -> "直播"
    "UvcLiveStream" -> "UVC 直播"
    else -> name.ifBlank { "未知模式" }
}

internal fun stateLabel(name: String): String = when (name) {
    "Capturing" -> "拍摄中"
    "LiveView" -> "待机"
    "Playback" -> "回放"
    "PreRecording" -> "预录制"
    "ScreenOff" -> "息屏"
    else -> "未知状态"
}

internal fun temperatureLabel(status: CameraStatus): String = when (status.temperatureCode) {
    2 -> "过热，无法录制"
    3 -> "即将关机"
    else -> status.temperatureName
}

@Composable
internal fun batteryColor(percent: Int): Color = when {
    percent <= 15 -> MaterialTheme.colorScheme.error
    percent <= 40 -> MaterialTheme.colorScheme.primary
    else -> MaterialTheme.colorScheme.secondary
}

internal fun rssiLabel(rssi: Int?): String = rssi?.let { "$it dBm" } ?: "—"
