package com.cakeal.osmolink.ui

import com.cakeal.osmolink.data.CameraStatus
import com.cakeal.osmolink.data.UiState
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraControlsTest {
    @Test
    fun cameraControlsRequireLiveStatusAndNoPendingCommand() {
        val ready = UiState(connected = true, status = CameraStatus(hasData = true))

        assertTrue(ready.canControlCamera)
        assertFalse(ready.copy(connected = false).canControlCamera)
        assertFalse(ready.copy(status = CameraStatus()).canControlCamera)
        assertFalse(ready.copy(reconnectAttempt = 1).canControlCamera)
        assertFalse(ready.copy(pendingAction = "mode").canControlCamera)
    }
}
