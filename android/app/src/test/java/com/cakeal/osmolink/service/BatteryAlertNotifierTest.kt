package com.cakeal.osmolink.service

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BatteryAlertNotifierTest {
    @Test
    fun alertsOnlyBelowThresholdAndOnce() {
        assertTrue(BatteryAlertNotifier.shouldAlert(9, 10, false))
        assertFalse(BatteryAlertNotifier.shouldAlert(10, 10, false))
        assertFalse(BatteryAlertNotifier.shouldAlert(9, 10, true))
        assertFalse(BatteryAlertNotifier.shouldAlert(80, 10, false))
    }
}
