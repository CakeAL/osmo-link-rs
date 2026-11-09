package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun StatusDetails(state: UiState) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    val status = state.status
    val capacity = if (status.hasData && status.remainingMb > 0) {
        "%.1f GB 可用".format(status.remainingMb / 1024.0)
    } else {
        "容量 —"
    }
    DisclosureRow(
        title = "相机详情",
        icon = R.drawable.ic_ui_settings,
        expanded = expanded,
        onToggle = { expanded = !expanded },
        supportingContent = {
            Column {
                Text(if (status.eis.isBlank()) capacity else "$capacity · ${status.eis}")
                if (status.temperatureCode >= 2) {
                    Text(temperatureLabel(status), color = MaterialTheme.colorScheme.error)
                }
            }
        },
    )
    if (expanded) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            DetailRow("分辨率", status.resolution.ifBlank { "—" })
            DetailRow("帧率", status.fps.ifBlank { "—" })
            DetailRow("防抖", status.eis.ifBlank { "—" })
            state.version?.let { DetailRow("固件版本", it) }
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f),
        )
    }
}
