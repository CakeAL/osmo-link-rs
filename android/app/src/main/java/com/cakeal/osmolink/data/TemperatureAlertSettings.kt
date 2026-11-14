package com.cakeal.osmolink.data

import android.content.Context
import android.content.SharedPreferences

internal data class TemperatureAlertSettings(val enabled: Boolean) {
    companion object {
        const val FILE = "appearance"

        private const val ENABLED = "temperature_alert_enabled"

        fun load(context: Context): TemperatureAlertSettings =
            load(context.getSharedPreferences(FILE, Context.MODE_PRIVATE))

        fun load(storage: SharedPreferences): TemperatureAlertSettings =
            TemperatureAlertSettings(enabled = storage.getBoolean(ENABLED, true))

        fun updateEnabled(storage: SharedPreferences, enabled: Boolean) {
            storage.edit().putBoolean(ENABLED, enabled).apply()
        }
    }
}
