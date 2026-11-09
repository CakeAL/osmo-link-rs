package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun ModePicker(state: UiState, onSelect: (Int) -> Unit) {
    Column {
        Text("拍摄模式", style = MaterialTheme.typography.titleSmall)
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.modes, key = { it.code }) { mode ->
                val selected = mode.code == state.status.modeCode
                FilterChip(
                    selected = selected,
                    onClick = { onSelect(mode.code) },
                    enabled = state.pendingAction == null,
                    label = { Text(modeLabel(mode.name)) },
                    leadingIcon = if (selected) {
                        { UiIcon(R.drawable.ic_ui_check, Modifier.size(FilterChipDefaults.IconSize)) }
                    } else {
                        null
                    },
                )
            }
        }
    }
}
