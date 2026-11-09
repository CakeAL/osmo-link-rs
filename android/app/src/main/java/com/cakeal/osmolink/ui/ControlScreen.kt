package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
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
        ControlTopBar(state, onDisconnect, onOpenSettings)
        Column(
            modifier = Modifier
                .weight(1f)
                .widthIn(max = 560.dp)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(start = 16.dp, end = 16.dp, bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            StatusHero(state)
            if (state.modes.isNotEmpty()) {
                ModePicker(state, onMode)
            }
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = MaterialTheme.shapes.extraLarge,
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
            ) {
                GpsRow(state, onGpsToggle)
                HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
                StatusDetails(state)
            }
        }
        ControlBar(state, onShutter, onRecord)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ControlTopBar(state: UiState, onDisconnect: () -> Unit, onOpenSettings: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    TopAppBar(
        modifier = Modifier.widthIn(max = 560.dp),
        title = {
            Column {
                Text(
                    text = state.modelName.ifBlank { state.deviceName.ifBlank { "相机" } },
                    style = MaterialTheme.typography.titleLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = if (state.deviceName.isNotBlank() && state.modelName.isNotBlank()) {
                        "${state.deviceName} · 已连接"
                    } else {
                        "已连接"
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        },
        actions = {
            val batteryColor = if (state.status.hasData) batteryColor(state.status.battery) else colors.onSurfaceVariant
            CompositionLocalProvider(LocalContentColor provides batteryColor) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    UiIcon(R.drawable.ic_ui_battery, Modifier.size(16.dp))
                    Text(
                        text = if (state.status.hasData) "${state.status.battery}%" else "—",
                        style = MaterialTheme.typography.labelMedium,
                    )
                }
            }
            TextButton(onClick = onDisconnect) { Text("断开") }
            SettingsButton(onOpenSettings)
        },
        colors = TopAppBarDefaults.topAppBarColors(containerColor = colors.surface),
        windowInsets = WindowInsets(0, 0, 0, 0),
    )
}
