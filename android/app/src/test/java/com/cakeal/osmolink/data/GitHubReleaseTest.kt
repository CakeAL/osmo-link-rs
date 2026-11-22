package com.cakeal.osmolink.data

import java.io.IOException
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GitHubReleaseTest {
    @Test
    fun comparesEachComponentNumerically() {
        assertTrue(isNewerRelease("v0.10", "v0.9"))
        assertTrue(isNewerRelease("v0.1.1", "v0.1"))
        assertTrue(isNewerRelease("V1.0", "0.99.99"))
        assertFalse(isNewerRelease("v0.1.10", "v0.2"))
    }

    @Test
    fun treatsAnOmittedPatchAsZero() {
        assertFalse(isNewerRelease("v0.1", "0.1.0"))
        assertFalse(isNewerRelease("v0.1.0", "v0.1"))
    }

    @Test(expected = IOException::class)
    fun rejectsAnUnrecognizedReleaseTag() {
        isNewerRelease("v0.2-beta.1", "v0.1")
    }
}
