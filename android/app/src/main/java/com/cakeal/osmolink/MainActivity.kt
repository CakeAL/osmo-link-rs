package com.cakeal.osmolink

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.lifecycle.viewmodel.compose.viewModel
import com.cakeal.osmolink.ui.OsmoLinkApp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setOsmoContent { preferences ->
            val viewModel: ControllerViewModel = viewModel()
            OsmoLinkApp(viewModel, preferences, onOpenSystemSettings = {
                startActivity(Intent(this, SettingsActivity::class.java))
            })
        }
    }
}
