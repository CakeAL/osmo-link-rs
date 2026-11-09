package com.cakeal.osmolink.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.UiState

@Composable
internal fun StatusHero(state: UiState) {
    val status = state.status
    val photoMode = status.modeName in PHOTO_MODES
    val recording = status.isRecording
    val colors = MaterialTheme.colorScheme
    val contentColor = if (recording) colors.onErrorContainer else colors.onPrimaryContainer
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.extraLarge,
        colors = CardDefaults.cardColors(
            containerColor = if (recording) colors.errorContainer else colors.primaryContainer,
            contentColor = contentColor,
        ),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                if (recording) {
                    Box(Modifier.size(8.dp).background(contentColor, CircleShape))
                }
                Text(
                    text = when {
                        !status.hasData -> "等待相机状态"
                        recording -> "录制中"
                        else -> stateLabel(status.stateName)
                    },
                    style = MaterialTheme.typography.labelLarge,
                )
            }
            Text(
                text = when {
                    photoMode -> modeLabel(status.modeName)
                    status.hasData -> formatDuration(status.recordSeconds)
                    else -> "--:--"
                },
                style = if (photoMode) MaterialTheme.typography.headlineLarge else MaterialTheme.typography.displayMedium,
                textAlign = TextAlign.Center,
            )
            if (status.hasData) {
                val format = listOf(status.resolution, status.fps).filter { it.isNotBlank() }.joinToString(" · ")
                if (format.isNotEmpty()) {
                    Text(format, style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.Center)
                }
                val remaining = when {
                    photoMode -> "剩余 ${status.remainingPhotos} 张照片"
                    status.remainingSeconds > 0 -> "剩余 ${formatRemainingDuration(status.remainingSeconds)}"
                    else -> "剩余时长 —"
                }
                Text(remaining, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
