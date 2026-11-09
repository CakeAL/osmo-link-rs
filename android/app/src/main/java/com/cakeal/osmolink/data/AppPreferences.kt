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
    private val storage = context.getSharedPreferences("appearance", Context.MODE_PRIVATE)

    var appearance by mutableStateOf(
        Appearance(
            mode = ThemeMode.entries.firstOrNull { it.name == storage.getString("mode", null) }
                ?: ThemeMode.System,
            accent = if (storage.contains("accent")) storage.getInt("accent", 0) else null,
        ),
    )
        private set

    fun updateAppearance(value: Appearance) {
        val editor = storage.edit().putString("mode", value.mode.name)
        val accent = value.accent
        if (accent == null) editor.remove("accent") else editor.putInt("accent", accent)
        editor.apply()
        appearance = value
    }
}
