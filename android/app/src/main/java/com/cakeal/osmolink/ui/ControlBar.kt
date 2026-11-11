package com.cakeal.osmolink.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.UiState

@Composable
internal fun ControlBar(
    state: UiState,
    onShutter: () -> Unit,
    onRecord: () -> Unit,
) {
    val photoMode = state.status.modeName in PHOTO_MODES
    val pending = state.pendingAction != null
    val enabled = state.canControlCamera && !(photoMode && state.status.capturing)
    Surface(
        color = MaterialTheme.colorScheme.surfaceContainer,
        shape = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Box(
            modifier = Modifier.windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Bottom)),
            contentAlignment = Alignment.Center,
        ) {
            CaptureAction(
                modifier = Modifier.widthIn(min = 112.dp).padding(vertical = 8.dp),
                photoMode = photoMode,
                recording = state.status.isRecording,
                hasStatus = state.status.hasData,
                pending = pending,
                enabled = enabled,
                onClick = if (photoMode) onShutter else onRecord,
            )
        }
    }
}

@Composable
private fun CaptureAction(
    modifier: Modifier,
    photoMode: Boolean,
    recording: Boolean,
    hasStatus: Boolean,
    pending: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val accent = when {
        !enabled && !pending -> colors.outline
        photoMode -> colors.primary
        else -> colors.error
    }
    Surface(
        onClick = onClick,
        enabled = enabled,
        shape = MaterialTheme.shapes.extraLarge,
        color = Color.Transparent,
        contentColor = accent,
        modifier = modifier.semantics { role = Role.Button },
    ) {
        Column(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Surface(
                shape = CircleShape,
                border = BorderStroke(2.dp, accent),
                color = Color.Transparent,
                modifier = Modifier.size(64.dp),
            ) {
                Box(contentAlignment = Alignment.Center) {
                    when {
                        pending -> CircularProgressIndicator(
                            modifier = Modifier.size(26.dp),
                            color = accent,
                            strokeWidth = 3.dp,
                        )
                        recording -> Box(Modifier.size(22.dp).background(accent, MaterialTheme.shapes.extraSmall))
                        else -> Box(Modifier.size(46.dp).background(accent, CircleShape))
                    }
                }
            }
            Text(
                text = when {
                    !hasStatus -> "拍摄"
                    photoMode -> "拍照"
                    recording -> "停止录像"
                    else -> "开始录像"
                },
                style = MaterialTheme.typography.labelMedium,
                textAlign = TextAlign.Center,
            )
        }
    }
}
