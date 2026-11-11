package com.cakeal.osmolink.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.ModeItem
import com.cakeal.osmolink.data.UiState
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

@Composable
internal fun ModePicker(state: UiState, onSelect: (Int) -> Unit) {
    val status = state.status
    val modes = state.modes
    val canSwitch = state.canControlCamera && !status.capturing && modes.isNotEmpty()
    val startIndex = modes.indexOfFirst { it.code == status.modeCode }.coerceAtLeast(0)
    val rowHeight = maxOf(48.dp, with(LocalDensity.current) { 22.sp.toDp() } + 16.dp)
    var expanded by remember(modes, status.modeCode, canSwitch) { mutableStateOf(false) }
    var position by remember(modes, status.modeCode, canSwitch) { mutableFloatStateOf(startIndex.toFloat()) }
    val selectMode by rememberUpdatedState(onSelect)
    val colors = MaterialTheme.colorScheme

    fun choose(index: Int) {
        expanded = false
        position = startIndex.toFloat()
        val mode = modes[index]
        if (mode.code != status.modeCode) selectMode(mode.code)
    }

    RemoteCard {
        Column(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            SectionTitle("拍摄模式", R.drawable.ic_ui_camera)
            if (!status.hasData) {
                Text("等待相机状态", style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
            } else {
                val gesture = if (canSwitch) {
                    Modifier.pointerInput(modes, status.modeCode, rowHeight) {
                        val rowPx = rowHeight.toPx()
                        detectVerticalDragGestures(
                            onDragStart = { expanded = true },
                            onDragCancel = { position = startIndex.toFloat() },
                            onDragEnd = {
                                val index = position.roundToInt()
                                position = index.toFloat()
                                if (index != startIndex) choose(index)
                            },
                            onVerticalDrag = { _, delta ->
                                position = moveModeWheel(position, delta / rowPx, modes.lastIndex)
                            },
                        )
                    }
                } else {
                    Modifier
                }
                Surface(
                    modifier = Modifier.fillMaxWidth().then(gesture),
                    shape = MaterialTheme.shapes.large,
                    color = colors.surfaceContainerHigh,
                ) {
                    if (expanded && canSwitch) {
                        ModeWheel(modes, position, rowHeight, ::choose)
                    } else {
                        Row(
                            modifier = Modifier
                                .semantics { if (canSwitch) stateDescription = "已折叠" }
                                .clickable(enabled = canSwitch, role = Role.Button) { expanded = true }
                                .padding(horizontal = 16.dp, vertical = 12.dp),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            UiIcon(modeIcon(status.modeName))
                            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(modeLabel(status.modeName), style = MaterialTheme.typography.titleMedium)
                                Text(
                                    text = when {
                                        status.capturing -> "拍摄中不可切换"
                                        modes.isEmpty() -> "此机型没有可切换的模式"
                                        canSwitch -> "上下滑动或点按"
                                        else -> "暂不可切换"
                                    },
                                    style = MaterialTheme.typography.bodySmall,
                                    color = colors.onSurfaceVariant,
                                )
                            }
                            if (canSwitch) UiIcon(R.drawable.ic_ui_chevron)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ModeWheel(
    modes: List<ModeItem>,
    position: Float,
    rowHeight: Dp,
    onSelect: (Int) -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val selected = position.roundToInt()
    Box(
        modifier = Modifier.fillMaxWidth().height(rowHeight * 4.4f).selectableGroup(),
        contentAlignment = Alignment.Center,
    ) {
        Surface(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp).height(rowHeight),
            shape = MaterialTheme.shapes.medium,
            color = colors.primaryContainer,
        ) {}
        modes.forEachIndexed { index, mode ->
            val distance = index - position
            if (abs(distance) < 3f) {
                val angle = distance * 0.4f
                val scale = cos(angle)
                val centered = index == selected
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(rowHeight)
                        .offset { IntOffset(0, (rowHeight.toPx() * sin(angle) / sin(0.4f)).roundToInt()) }
                        .graphicsLayer {
                            scaleX = 0.86f + 0.14f * scale
                            scaleY = scale
                            alpha = 1f - abs(distance) / 3f
                        }
                        .selectable(selected = centered, role = Role.RadioButton, onClick = { onSelect(index) })
                        .padding(horizontal = 20.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterHorizontally),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    val color = if (centered) colors.onPrimaryContainer else colors.onSurface
                    CompositionLocalProvider(LocalContentColor provides color) {
                        UiIcon(modeIcon(mode.name), Modifier.size(22.dp))
                        Text(
                            text = modeLabel(mode.name),
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = if (centered) FontWeight.Bold else FontWeight.Medium,
                            modifier = Modifier.weight(1f, fill = false),
                            maxLines = 1,
                        )
                    }
                }
            }
        }
    }
}

private fun modeIcon(name: String): Int = when (name) {
    "Photo", "PanoramicPhoto" -> R.drawable.ic_ui_camera
    "SlowMotion", "Timelapse" -> R.drawable.ic_ui_clock
    "Hyperlapse", "PanoramicHyperlapse", "TimeFreeze" -> R.drawable.ic_ui_motion
    "SuperNight", "PanoramicSuperNight", "SingleLensSuperNight" -> R.drawable.ic_ui_night
    "SubjectTracking", "Selfie" -> R.drawable.ic_ui_tracking
    "PanoramicVideo" -> R.drawable.ic_ui_panorama
    "LiveStream", "UvcLiveStream" -> R.drawable.ic_ui_live
    else -> R.drawable.ic_ui_video
}

internal fun moveModeWheel(position: Float, deltaRows: Float, lastIndex: Int): Float =
    (position - deltaRows).coerceIn(0f, lastIndex.toFloat())
