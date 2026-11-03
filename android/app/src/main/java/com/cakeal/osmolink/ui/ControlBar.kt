package com.cakeal.osmolink.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.UiState

@Composable
internal fun ControlBar(
    state: UiState,
    onShutter: () -> Unit,
    onRecord: () -> Unit,
    onGpsToggle: (Boolean) -> Unit,
) {
    val photoMode = state.status.modeName in PHOTO_MODES
    Surface(
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        shape = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 28.dp, vertical = 16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (photoMode) {
                Spacer(Modifier.size(56.dp))
            } else {
                CircleAction(
                    label = "快门",
                    enabled = state.connected && state.pendingAction == null,
                    active = false,
                    onClick = onShutter,
                )
            }
            BigAction(
                photoMode = photoMode,
                capturing = state.status.capturing,
                pending = state.pendingAction != null,
                enabled = state.connected,
                onClick = if (photoMode) onShutter else onRecord,
            )
            CircleAction(
                label = if (state.gpsEnabled) "定位中" else "定位",
                enabled = state.connected || state.gpsEnabled,
                active = state.gpsEnabled,
                onClick = { onGpsToggle(!state.gpsEnabled) },
            )
        }
    }
}

@Composable
private fun BigAction(
    photoMode: Boolean,
    capturing: Boolean,
    pending: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val accent = if (photoMode) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.error
    }
    Surface(
        onClick = onClick,
        enabled = enabled && !pending,
        shape = CircleShape,
        color = Color.Transparent,
        border = BorderStroke(3.dp, if (enabled) accent else MaterialTheme.colorScheme.outline),
        modifier = Modifier.size(88.dp),
    ) {
        Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxSize()) {
            when {
                pending -> CircularProgressIndicator(
                    modifier = Modifier.size(30.dp),
                    color = accent,
                    strokeWidth = 3.dp,
                )
                photoMode -> Box(Modifier.size(62.dp).background(accent, CircleShape))
                capturing -> Box(
                    Modifier
                        .size(28.dp)
                        .background(accent, RoundedCornerShape(6.dp)),
                )
                else -> Box(Modifier.size(62.dp).background(accent, CircleShape))
            }
        }
    }
}

@Composable
private fun CircleAction(
    label: String,
    enabled: Boolean,
    active: Boolean,
    onClick: () -> Unit,
) {
    Surface(
        onClick = onClick,
        enabled = enabled,
        shape = CircleShape,
        color = if (active) {
            MaterialTheme.colorScheme.secondaryContainer
        } else {
            MaterialTheme.colorScheme.surfaceContainerHigh
        },
        contentColor = if (active) {
            MaterialTheme.colorScheme.onSecondaryContainer
        } else {
            MaterialTheme.colorScheme.onSurface
        },
        border = BorderStroke(
            1.dp,
            if (active) {
                MaterialTheme.colorScheme.secondary
            } else {
                MaterialTheme.colorScheme.outlineVariant
            },
        ),
        modifier = Modifier.size(56.dp),
    ) {
        Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxSize()) {
            Text(label, style = MaterialTheme.typography.labelMedium)
        }
    }
}
