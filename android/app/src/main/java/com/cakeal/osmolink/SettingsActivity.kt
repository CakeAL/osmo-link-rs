package com.cakeal.osmolink

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import com.cakeal.osmolink.ui.AboutScreen
import com.cakeal.osmolink.ui.SettingsScreen
import kotlinx.coroutines.launch

class SettingsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setOsmoContent { preferences ->
            Subpage { showError ->
                SettingsScreen(
                    preferences = preferences,
                    onBack = ::finish,
                    onAbout = { startActivity(Intent(this, AboutActivity::class.java)) },
                    onError = showError,
                )
            }
        }
    }
}

class AboutActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setOsmoContent {
            Subpage { showError ->
                AboutScreen(onBack = ::finish, onError = showError)
            }
        }
    }
}

@Composable
private fun Subpage(content: @Composable ((String) -> Unit) -> Unit) {
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surface,
        contentWindowInsets = WindowInsets.safeDrawing,
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding)) {
            content { message -> scope.launch { snackbar.showSnackbar(message) } }
        }
    }
}
