package com.cakeal.osmolink.ui

import kotlin.math.roundToInt
import org.junit.Assert.assertEquals
import org.junit.Test

class ModePickerTest {
    @Test
    fun wheelFollowsTheFingerAndSnapsToTheNearestMode() {
        val first = moveModeWheel(2f, -0.4f, 5)
        assertEquals(2.4f, first, 0.0001f)
        assertEquals(2, first.roundToInt())
        assertEquals(3, moveModeWheel(first, -0.2f, 5).roundToInt())
        assertEquals(1, moveModeWheel(2f, 0.7f, 5).roundToInt())
    }

    @Test
    fun wheelCanReverseImmediatelyAtEitherEndOfTheCatalog() {
        val top = moveModeWheel(1f, 20f, 5)
        val bottom = moveModeWheel(3f, -20f, 5)
        assertEquals(0f, top, 0f)
        assertEquals(5f, bottom, 0f)
        assertEquals(1f, moveModeWheel(top, -1f, 5), 0f)
        assertEquals(4f, moveModeWheel(bottom, 1f, 5), 0f)
    }
}
