package com.cakeal.osmolink

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.SideEffect
import androidx.core.view.WindowCompat
import androidx.lifecycle.viewmodel.compose.viewModel
import com.cakeal.osmolink.data.AppPreferences
import com.cakeal.osmolink.data.ThemeMode
import com.cakeal.osmolink.ui.OsmoLinkApp
import com.cakeal.osmolink.ui.theme.OsmoLinkTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val preferences = AppPreferences(this)
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
                val viewModel: ControllerViewModel = viewModel()
                OsmoLinkApp(viewModel, appearance, preferences::updateAppearance)
            }
        }
    }
}
