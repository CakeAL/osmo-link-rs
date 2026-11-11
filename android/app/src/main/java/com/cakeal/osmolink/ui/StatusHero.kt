package com.cakeal.osmolink.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun StatusHero(state: UiState) {
    val status = state.status
    val photoMode = status.modeName in PHOTO_MODES
    val colors = MaterialTheme.colorScheme
    CameraHeroSurface {
        Column(
            modifier = Modifier.fillMaxWidth().padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        state.modelName.ifBlank { state.deviceName.ifBlank { "相机" } },
                        style = MaterialTheme.typography.titleLarge,
                    )
                    Text(
                        text = when {
                            state.reconnectAttempt != null -> "正在重连 · 第 ${state.reconnectAttempt} 次"
                            status.hasData -> stateLabel(status.stateName)
                            else -> "蓝牙已连接"
                        },
                        style = MaterialTheme.typography.labelLarge,
                    )
                }
                if (status.hasData) {
                    val lowBattery = status.battery <= 15
                    StatusPill(
                        "${status.battery}%",
                        icon = R.drawable.ic_ui_battery,
                        color = if (lowBattery) colors.errorContainer else colors.onPrimaryContainer.copy(alpha = 0.10f),
                        contentColor = if (lowBattery) colors.onErrorContainer else colors.onPrimaryContainer,
                    )
                }
            }
            if (status.hasData) {
                if (photoMode) {
                    Text(modeLabel(status.modeName), style = MaterialTheme.typography.headlineLarge)
                } else {
                    Text(formatDuration(status.recordSeconds), style = MaterialTheme.typography.displayMedium)
                }
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    if (status.resolution.isNotBlank()) StatusPill(status.resolution)
                    if (status.fps.isNotBlank()) StatusPill(status.fps)
                    val remaining = if (photoMode) {
                        "剩余 ${status.remainingPhotos} 张"
                    } else {
                        "可录 ${formatRemainingDuration(status.remainingSeconds)}"
                    }
                    StatusPill(remaining)
                }
                if (status.isRecording && state.reconnectAttempt == null) {
                    Surface(shape = CircleShape, color = colors.errorContainer, contentColor = colors.onErrorContainer) {
                        Row(
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Box(Modifier.size(7.dp).background(colors.onErrorContainer, CircleShape))
                            Text("录制中", style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
            } else {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    CircularProgressIndicator(Modifier.size(20.dp), color = colors.onPrimaryContainer, strokeWidth = 2.dp)
                    Text("正在读取相机状态", style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
    }
}
