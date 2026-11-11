package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun GpsCard(state: UiState, onToggle: (Boolean) -> Unit) {
    val colors = MaterialTheme.colorScheme
    val enabled = state.connected || state.gpsEnabled
    RemoteCard {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .toggleable(
                    value = state.gpsEnabled,
                    enabled = enabled,
                    role = Role.Switch,
                    onValueChange = onToggle,
                )
                .padding(horizontal = 16.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SectionTitle("定位注入", R.drawable.ic_ui_pin, Modifier.weight(1f))
            Switch(checked = state.gpsEnabled, enabled = enabled, onCheckedChange = null)
        }
        Column(Modifier.padding(start = 16.dp, end = 16.dp, bottom = 16.dp)) {
            val accuracy = state.gpsAccuracy
            when {
                !state.gpsEnabled -> Text(
                    "已暂停。开启后使用手机定位。",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.onSurfaceVariant,
                )
                accuracy == null -> Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                    Text("等待首个定位…", style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
                }
                else -> FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    GpsMetric("频率", "%.1f Hz".format(state.gpsRate))
                    GpsMetric("精度", "±%.1f m".format(accuracy))
                    GpsMetric("卫星", "${state.gpsSatellites} 颗")
                }
            }
        }
    }
}

@Composable
private fun GpsMetric(label: String, value: String) {
    val colors = MaterialTheme.colorScheme
    Surface(
        modifier = Modifier.widthIn(min = 88.dp),
        shape = MaterialTheme.shapes.medium,
        color = colors.surfaceContainerHighest,
        contentColor = colors.onSurface,
    ) {
        Column(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(label, style = MaterialTheme.typography.labelSmall, color = colors.onSurfaceVariant)
            Text(value, style = MaterialTheme.typography.labelLarge.mono())
        }
    }
}
