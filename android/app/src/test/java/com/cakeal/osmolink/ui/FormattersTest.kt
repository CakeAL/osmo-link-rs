package com.cakeal.osmolink.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class FormattersTest {
    @Test
    fun remainingDurationUsesHoursAndMinutes() {
        assertEquals("0m", formatRemainingDuration(0))
        assertEquals("<1m", formatRemainingDuration(30))
        assertEquals("5m", formatRemainingDuration(300))
        assertEquals("1h 02m", formatRemainingDuration(3725))
        assertEquals("12h 05m", formatRemainingDuration(43_500))
    }

    @Test
    fun recordingTimerKeepsClockFormat() {
        assertEquals("00:05", formatDuration(5))
        assertEquals("1:02:05", formatDuration(3725))
    }
}
