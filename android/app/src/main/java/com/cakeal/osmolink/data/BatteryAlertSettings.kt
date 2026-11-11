package com.cakeal.osmolink.data

import android.content.Context
import android.content.SharedPreferences

internal data class BatteryAlertSettings(
    val enabled: Boolean,
    val threshold: Int,
) {
    companion object {
        const val FILE = "appearance"
        const val DEFAULT_THRESHOLD = 10
        val THRESHOLDS = listOf(5, 10, 15, 20, 30)

        private const val ENABLED = "battery_alert_enabled"
        private const val THRESHOLD = "battery_alert_threshold"

        fun load(context: Context): BatteryAlertSettings =
            load(context.getSharedPreferences(FILE, Context.MODE_PRIVATE))

        fun load(storage: SharedPreferences): BatteryAlertSettings {
            val threshold = storage.getInt(THRESHOLD, DEFAULT_THRESHOLD)
            return BatteryAlertSettings(
                enabled = storage.getBoolean(ENABLED, true),
                threshold = threshold.takeIf { it in THRESHOLDS } ?: DEFAULT_THRESHOLD,
            )
        }

        fun updateEnabled(storage: SharedPreferences, enabled: Boolean) {
            storage.edit().putBoolean(ENABLED, enabled).apply()
        }

        fun updateThreshold(storage: SharedPreferences, threshold: Int) {
            if (threshold in THRESHOLDS) storage.edit().putInt(THRESHOLD, threshold).apply()
        }
    }
}
