package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun ModePicker(state: UiState, onSelect: (Int) -> Unit) {
    val status = state.status
    val canSwitch = state.canControlCamera && !status.capturing
    RemoteCard {
        Column(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                SectionTitle("拍摄模式", R.drawable.ic_ui_camera, Modifier.weight(1f))
                if (status.capturing) {
                    Text(
                        "拍摄中不可切换",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (status.hasData) {
                LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (state.modes.none { it.code == status.modeCode }) {
                        item {
                            FilterChip(
                                selected = true,
                                enabled = false,
                                onClick = {},
                                label = { Text(modeLabel(status.modeName)) },
                            )
                        }
                    }
                    items(state.modes, key = { it.code }) { mode ->
                        val selected = mode.code == status.modeCode
                        FilterChip(
                            selected = selected,
                            onClick = { if (!selected) onSelect(mode.code) },
                            enabled = canSwitch,
                            label = { Text(modeLabel(mode.name)) },
                            leadingIcon = if (selected) {
                                { UiIcon(R.drawable.ic_ui_check, Modifier.size(FilterChipDefaults.IconSize)) }
                            } else {
                                null
                            },
                        )
                    }
                }
            } else {
                Text("等待相机状态", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
