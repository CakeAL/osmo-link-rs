package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.UiState

@Composable
internal fun ControlScreen(
    state: UiState,
    onShutter: () -> Unit,
    onRecord: () -> Unit,
    onMode: (Int) -> Unit,
    onDisconnect: () -> Unit,
    onGpsToggle: (Boolean) -> Unit,
    onOpenSettings: () -> Unit,
) {
    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) {
        RemoteTopBar("相机遥控") {
            TextButton(onClick = onDisconnect, enabled = state.connected) { Text("断开") }
            SettingsButton(onOpenSettings)
        }
        Column(
            modifier = Modifier
                .weight(1f)
                .widthIn(max = 560.dp)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            StatusHero(state)
            ModePicker(state, onMode)
            StatusDetails(state)
            GpsCard(state, onGpsToggle)
            RecentEventsCard(state)
        }
        ControlBar(state, onShutter, onRecord)
    }
}
