package com.cakeal.osmolink.data

import org.junit.Assert.assertEquals
import org.junit.Test

class EventLogTest {
    @Test
    fun newestFirst() {
        val log = EventLog()
        log.add("first", now = 1_000)
        val events = log.add("second", now = 5_000)
        assertEquals(listOf("second", "first"), events.map { it.text })
    }

    @Test
    fun dropsRepeatsInsideTheWindow() {
        val log = EventLog()
        log.add("same", now = 1_000)
        assertEquals(1, log.add("same", now = 1_500).size)
        assertEquals(2, log.add("same", now = 4_000).size)
    }

    @Test
    fun keepsTheNewestEntriesOnly() {
        val log = EventLog(capacity = 2)
        log.add("a", now = 1)
        log.add("b", now = 2)
        val events = log.add("c", now = 3)
        assertEquals(listOf("c", "b"), events.map { it.text })
    }
}
