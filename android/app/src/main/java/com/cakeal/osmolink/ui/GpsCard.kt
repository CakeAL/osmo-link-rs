package com.cakeal.osmolink.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.UiState

@Composable
internal fun GpsCard(state: UiState, onToggle: (Boolean) -> Unit) {
    val accuracy = state.gpsAccuracy
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        shape = MaterialTheme.shapes.large,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("定位注入", style = MaterialTheme.typography.titleMedium)
                    Text(
                        text = if (state.gpsEnabled) "向相机持续发送手机定位" else "关闭",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = state.gpsEnabled,
                    onCheckedChange = onToggle,
                    enabled = state.connected || state.gpsEnabled,
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = MaterialTheme.colorScheme.onPrimary,
                        checkedTrackColor = MaterialTheme.colorScheme.primary,
                        uncheckedThumbColor = MaterialTheme.colorScheme.onSurfaceVariant,
                        uncheckedTrackColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                        uncheckedBorderColor = MaterialTheme.colorScheme.outline,
                        checkedBorderColor = MaterialTheme.colorScheme.primary,
                    ),
                )
            }
            if (state.gpsEnabled) {
                Text(
                    text = buildString {
                        append("%.1f Hz".format(state.gpsRate))
                        append(" · ")
                        append("${state.gpsSatellites} 星")
                        if (accuracy != null) {
                            append(" · ")
                            append("±%.1f m".format(accuracy))
                        }
                    },
                    style = MaterialTheme.typography.labelLarge.mono(),
                    color = MaterialTheme.colorScheme.secondary,
                )
            }
            Text(
                text = "按 100ms 目标间隔发送最近定位；实际频率由系统定位决定，录像中开启只影响之后的画面。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
