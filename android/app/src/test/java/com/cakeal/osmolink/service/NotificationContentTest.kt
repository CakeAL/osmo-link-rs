package com.cakeal.osmolink.service

import com.cakeal.osmolink.data.CameraStatus
import com.cakeal.osmolink.data.UiState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationContentTest {
    @Test
    fun recordingShowsDurationModeAndRemainingTime() {
        val content = notificationContent(
            UiState(
                connected = true,
                deviceName = "OsmoAction4-B15B",
                status = CameraStatus(
                    modeName = "Video",
                    capturing = true,
                    hasData = true,
                    recordSeconds = 42,
                    remainingSeconds = 3600,
                    battery = 80,
                    resolution = "4K 16:9",
                    fps = "30 fps",
                ),
            ),
        )
        assertEquals("OsmoAction4-B15B · 录制中 00:42", content.title)
        assertTrue(content.text.contains("4K 16:9"))
        assertTrue(content.text.contains("电量 80%"))
        assertTrue(content.text.contains("剩余 1h 00m"))
    }

    @Test
    fun photoModeShowsRemainingShots() {
        val content = notificationContent(
            UiState(
                connected = true,
                deviceName = "Cam",
                status = CameraStatus(
                    modeName = "Photo",
                    hasData = true,
                    remainingPhotos = 123,
                    battery = 66,
                ),
            ),
        )
        assertTrue(content.text.contains("剩余 123 张"))
    }

    @Test
    fun gpsOnlyShowsInjectionStatus() {
        val content = notificationContent(
            UiState(gpsEnabled = true, gpsRate = 9.8, gpsSatellites = 12),
        )
        assertEquals("Osmo Link · 定位注入", content.title)
        assertTrue(content.text.contains("定位 9.8Hz"))
        assertTrue(content.text.contains("12 星"))
    }
}
