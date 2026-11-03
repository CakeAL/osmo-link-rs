package com.cakeal.osmolink.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.UiState

@Composable
internal fun StatusDetails(state: UiState) {
    val status = state.status
    // Photo-like modes show remaining shots; everything else records video.
    val photoMode = status.modeName in PHOTO_MODES
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
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Row {
                StatBlock(
                    label = "剩余容量",
                    value = if (status.hasData && status.remainingMb > 0) {
                        "%.1f GB".format(status.remainingMb / 1024.0)
                    } else {
                        "—"
                    },
                    modifier = Modifier.weight(1f),
                )
                StatBlock(
                    label = if (photoMode) "剩余照片" else "剩余时长",
                    value = when {
                        !status.hasData -> "—"
                        photoMode -> "${status.remainingPhotos}"
                        status.remainingSeconds > 0 -> formatRemainingDuration(status.remainingSeconds)
                        else -> "—"
                    },
                    modifier = Modifier.weight(1f),
                )
                StatBlock(
                    label = "电池",
                    value = if (status.hasData) "${status.battery}%" else "—",
                    modifier = Modifier.weight(1f),
                )
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            DetailRow("分辨率", status.resolution.ifBlank { "—" })
            DetailRow("帧率", status.fps.ifBlank { "—" })
            DetailRow("防抖", status.eis.ifBlank { "—" })
            if (status.temperatureCode >= 2) {
                DetailRow("温度", temperatureLabel(status), warning = true)
            }
            state.version?.let { DetailRow("版本", it) }
        }
    }
}

@Composable
private fun StatBlock(label: String, value: String, modifier: Modifier = Modifier) {
    Column(modifier) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(2.dp))
        Text(
            text = value,
            style = MaterialTheme.typography.titleMedium.mono(),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
private fun DetailRow(label: String, value: String, warning: Boolean = false) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.width(12.dp))
        Text(
            text = value,
            style = MaterialTheme.typography.bodySmall.mono(),
            color = if (warning) {
                MaterialTheme.colorScheme.error
            } else {
                MaterialTheme.colorScheme.onSurface
            },
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f),
        )
    }
}
