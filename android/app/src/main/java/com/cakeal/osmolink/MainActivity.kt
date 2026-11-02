package com.cakeal.osmolink

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.viewmodel.compose.viewModel
import com.cakeal.osmolink.ui.OsmoLinkApp
import com.cakeal.osmolink.ui.theme.OsmoLinkTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // The app is always dark; keep system bars transparent with light icons.
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT),
        )
        setContent {
            OsmoLinkTheme {
                val viewModel: ControllerViewModel = viewModel()
                OsmoLinkApp(viewModel)
            }
        }
    }
}
