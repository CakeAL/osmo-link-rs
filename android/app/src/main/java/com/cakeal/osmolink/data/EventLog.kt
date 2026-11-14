package com.cakeal.osmolink.data

/** One line in the home screen activity feed. */
data class RecentEvent(
    val id: Long,
    val atMillis: Long,
    val text: String,
)

/**
 * Newest-first in-memory activity feed.
 *
 * The feed is intentionally session-scoped: it explains what the app just
 * did without writing a log file. Status pushes arrive at 2 Hz, so a repeat
 * of the same line inside [DUPLICATE_WINDOW_MILLIS] is dropped.
 */
internal class EventLog(private val capacity: Int = DEFAULT_CAPACITY) {
    private var nextId = 1L
    private val entries = ArrayDeque<RecentEvent>()

    @Synchronized
    fun add(text: String, now: Long = System.currentTimeMillis()): List<RecentEvent> {
        val latest = entries.firstOrNull()
        if (latest != null && latest.text == text && now - latest.atMillis < DUPLICATE_WINDOW_MILLIS) {
            return entries.toList()
        }
        entries.addFirst(RecentEvent(nextId++, now, text))
        while (entries.size > capacity) entries.removeLast()
        return entries.toList()
    }

    @Synchronized
    fun snapshot(): List<RecentEvent> = entries.toList()

    companion object {
        const val DEFAULT_CAPACITY = 20
        const val DUPLICATE_WINDOW_MILLIS = 2_000L
    }
}
