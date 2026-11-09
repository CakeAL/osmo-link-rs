package com.cakeal.osmolink.ui

import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun GpsRow(state: UiState, onToggle: (Boolean) -> Unit) {
    val enabled = state.connected || state.gpsEnabled
    ListItem(
        headlineContent = { Text("定位注入") },
        supportingContent = {
            Text(
                if (state.gpsEnabled) {
                    buildString {
                        append("%.1f Hz · %d 星".format(state.gpsRate, state.gpsSatellites))
                        state.gpsAccuracy?.let { append(" · ±%.1f m".format(it)) }
                    }
                } else {
                    "使用手机定位"
                },
            )
        },
        leadingContent = { UiIcon(R.drawable.ic_ui_pin) },
        trailingContent = {
            Switch(checked = state.gpsEnabled, enabled = enabled, onCheckedChange = null)
        },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        modifier = Modifier.toggleable(
            value = state.gpsEnabled,
            enabled = enabled,
            role = Role.Switch,
            onValueChange = onToggle,
        ),
    )
}
