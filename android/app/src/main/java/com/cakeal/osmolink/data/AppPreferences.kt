package com.cakeal.osmolink.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

internal enum class ThemeMode(val label: String) {
    System("跟随系统"),
    Dark("暗色"),
    Light("浅色"),
}

internal data class Appearance(
    val mode: ThemeMode = ThemeMode.System,
    val accent: Int? = null,
)

internal class AppPreferences(context: Context) {
    private val storage = context.getSharedPreferences(BatteryAlertSettings.FILE, Context.MODE_PRIVATE)
    private val batteryAlerts = BatteryAlertSettings.load(storage)

    var appearance by mutableStateOf(
        Appearance(
            mode = ThemeMode.entries.firstOrNull { it.name == storage.getString("mode", null) }
                ?: ThemeMode.System,
            accent = if (storage.contains("accent")) storage.getInt("accent", 0) else null,
        ),
    )
        private set

    var compatibleBack by mutableStateOf(storage.getBoolean("compatible_back", true))
        private set

    var autoConnect by mutableStateOf(storage.getBoolean("auto_connect", true))
        private set

    var batteryAlertEnabled by mutableStateOf(batteryAlerts.enabled)
        private set

    var batteryAlertThreshold by mutableStateOf(batteryAlerts.threshold)
        private set

    fun updateCompatibleBack(enabled: Boolean) {
        storage.edit().putBoolean("compatible_back", enabled).apply()
        compatibleBack = enabled
    }

    fun updateAutoConnect(enabled: Boolean) {
        storage.edit().putBoolean("auto_connect", enabled).apply()
        autoConnect = enabled
    }

    fun updateBatteryAlertEnabled(enabled: Boolean) {
        BatteryAlertSettings.updateEnabled(storage, enabled)
        batteryAlertEnabled = enabled
    }

    fun updateBatteryAlertThreshold(threshold: Int) {
        if (threshold !in BatteryAlertSettings.THRESHOLDS) return
        BatteryAlertSettings.updateThreshold(storage, threshold)
        batteryAlertThreshold = threshold
    }

    fun updateAppearance(value: Appearance) {
        val editor = storage.edit().putString("mode", value.mode.name)
        val accent = value.accent
        if (accent == null) editor.remove("accent") else editor.putInt("accent", accent)
        editor.apply()
        appearance = value
    }
}
