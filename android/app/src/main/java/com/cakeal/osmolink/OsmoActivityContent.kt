package com.cakeal.osmolink

import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.core.view.WindowCompat
import com.cakeal.osmolink.data.AppPreferences
import com.cakeal.osmolink.data.ThemeMode
import com.cakeal.osmolink.ui.theme.OsmoLinkTheme

internal fun ComponentActivity.setOsmoContent(content: @Composable (AppPreferences) -> Unit) {
    enableEdgeToEdge()
    val preferences = (application as OsmoApp).preferences
    val insets = WindowCompat.getInsetsController(window, window.decorView)
    setContent {
        val appearance = preferences.appearance
        val darkTheme = when (appearance.mode) {
            ThemeMode.System -> isSystemInDarkTheme()
            ThemeMode.Dark -> true
            ThemeMode.Light -> false
        }
        SideEffect {
            insets.isAppearanceLightStatusBars = !darkTheme
            insets.isAppearanceLightNavigationBars = !darkTheme
        }
        OsmoLinkTheme(darkTheme = darkTheme, accent = appearance.accent) {
            content(preferences)
        }
    }
}
