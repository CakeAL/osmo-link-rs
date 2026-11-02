package com.cakeal.osmolink.ui

/** Recording timer: `MM:SS`, or `H:MM:SS` past one hour. */
internal fun formatDuration(seconds: Int): String {
    val hours = seconds / 3600
    val minutes = (seconds % 3600) / 60
    val secs = seconds % 60
    return if (hours > 0) {
        "%d:%02d:%02d".format(hours, minutes, secs)
    } else {
        "%02d:%02d".format(minutes, secs)
    }
}

/** Remaining recordable time: `1h 02m`, or `42m` below an hour. */
internal fun formatRemainingDuration(seconds: Int): String {
    if (seconds <= 0) return "0m"
    if (seconds < 60) return "<1m"
    val hours = seconds / 3600
    val minutes = (seconds % 3600) / 60
    return if (hours > 0) {
        "%dh %02dm".format(hours, minutes)
    } else {
        "${minutes}m"
    }
}
