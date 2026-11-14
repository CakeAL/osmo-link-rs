package com.cakeal.osmolink.service

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TemperatureAlertNotifierTest {
    @Test
    fun alertsOnlyWhenEscalatingIntoAlertLevels() {
        assertFalse(TemperatureAlertNotifier.shouldAlert(0, 0))
        assertFalse(TemperatureAlertNotifier.shouldAlert(0, 1))
        assertTrue(TemperatureAlertNotifier.shouldAlert(0, 2))
        assertTrue(TemperatureAlertNotifier.shouldAlert(1, 2))
        assertTrue(TemperatureAlertNotifier.shouldAlert(2, 3))
        assertFalse(TemperatureAlertNotifier.shouldAlert(2, 2))
        assertFalse(TemperatureAlertNotifier.shouldAlert(3, 2))
    }

    @Test
    fun namesTheLevel() {
        assertEquals("相机温度过高", TemperatureAlertNotifier.title(2))
        assertEquals("相机即将关机", TemperatureAlertNotifier.title(3))
        assertTrue(TemperatureAlertNotifier.message("Osmo", 3).contains("即将自动关机"))
    }
}
