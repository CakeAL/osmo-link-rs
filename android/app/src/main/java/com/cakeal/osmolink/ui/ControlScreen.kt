package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
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
) {
    Column(Modifier.fillMaxSize()) {
        ControlTopBar(state, onDisconnect)
        Column(
            modifier = Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            StatusHero(state)
            if (state.modes.isNotEmpty()) {
                ModePicker(state, onMode)
            }
            StatusDetails(state)
            GpsCard(state, onGpsToggle)
            Spacer(Modifier.height(4.dp))
        }
        ControlBar(
            state = state,
            onShutter = onShutter,
            onRecord = onRecord,
            onGpsToggle = onGpsToggle,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ControlTopBar(state: UiState, onDisconnect: () -> Unit) {
    TopAppBar(
        title = {
            Column {
                Text(
                    text = state.deviceName.ifBlank { "相机" },
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = state.modelName.ifBlank { "已连接" },
                    style = MaterialTheme.typography.labelSmall.mono(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        actions = {
            Text(
                text = if (state.status.hasData) "${state.status.battery}%" else "—",
                style = MaterialTheme.typography.labelLarge.mono(),
                color = batteryColor(state.status.battery),
                modifier = Modifier.padding(end = 4.dp),
            )
            TextButton(onClick = onDisconnect) {
                Text("断开", style = MaterialTheme.typography.labelLarge)
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = Color.Transparent,
            titleContentColor = MaterialTheme.colorScheme.onBackground,
        ),
        windowInsets = WindowInsets(0, 0, 0, 0),
    )
}
