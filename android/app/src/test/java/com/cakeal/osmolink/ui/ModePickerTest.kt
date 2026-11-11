package com.cakeal.osmolink.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ModePickerTest {
    @Test
    fun verticalDragPreviewsModesFromTheCurrentIndex() {
        assertEquals(2, modeIndexAfterDrag(2, -31f, 5))
        assertEquals(3, modeIndexAfterDrag(2, -32f, 5))
        assertEquals(1, modeIndexAfterDrag(2, 32f, 5))
        assertEquals(4, modeIndexAfterDrag(2, -80f, 5))
    }

    @Test
    fun verticalDragStopsAtEitherEndOfTheCatalog() {
        assertEquals(0, modeIndexAfterDrag(1, 320f, 5))
        assertEquals(5, modeIndexAfterDrag(3, -320f, 5))
    }
}
