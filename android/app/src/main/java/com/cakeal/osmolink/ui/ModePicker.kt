package com.cakeal.osmolink.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.ModeItem
import com.cakeal.osmolink.data.UiState
import kotlin.math.abs

private const val MODE_DRAG_STEP_DP = 32

@Composable
internal fun ModePicker(state: UiState, onSelect: (Int) -> Unit) {
    val status = state.status
    val modes = state.modes
    val canSwitch = state.canControlCamera && !status.capturing && modes.isNotEmpty()
    val selectedIndex = modes.indexOfFirst { it.code == status.modeCode }
    val startIndex = selectedIndex.coerceAtLeast(0)
    var expanded by remember(modes, status.modeCode, canSwitch) { mutableStateOf(false) }
    var previewIndex by remember(modes, status.modeCode, canSwitch) { mutableStateOf<Int?>(null) }
    val selectMode by rememberUpdatedState(onSelect)
    val colors = MaterialTheme.colorScheme

    RemoteCard {
        Column(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            SectionTitle("拍摄模式", R.drawable.ic_ui_camera)
            if (!status.hasData) {
                Text("等待相机状态", style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
            } else {
                val shape = MaterialTheme.shapes.large
                val drag = if (canSwitch) {
                    Modifier.pointerInput(modes, status.modeCode) {
                        var distance = 0f
                        detectVerticalDragGestures(
                            onDragStart = {
                                distance = 0f
                                previewIndex = startIndex
                                expanded = true
                            },
                            onDragEnd = {
                                val mode = previewIndex?.let(modes::get)
                                previewIndex = null
                                if (abs(distance) >= MODE_DRAG_STEP_DP && mode != null && mode.code != status.modeCode) {
                                    expanded = false
                                    selectMode(mode.code)
                                }
                            },
                            onDragCancel = { previewIndex = null },
                            onVerticalDrag = { _, delta ->
                                distance += delta.toDp().value
                                previewIndex = modeIndexAfterDrag(startIndex, distance, modes.lastIndex)
                            },
                        )
                    }
                } else {
                    Modifier
                }
                val preview = previewIndex?.let(modes::get)
                Surface(shape = shape, color = colors.surfaceContainerHigh) {
                    ListItem(
                        modifier = Modifier
                            .clip(shape)
                            .then(drag)
                            .clickable(enabled = canSwitch, role = Role.Button) { expanded = !expanded },
                        headlineContent = { Text(modeLabel(preview?.name ?: status.modeName)) },
                        supportingContent = {
                            Text(
                                when {
                                    status.capturing -> "拍摄中不可切换"
                                    modes.isEmpty() -> "此机型没有可切换的模式"
                                    canSwitch -> "上下拖动或点按"
                                    else -> "暂不可切换"
                                },
                            )
                        },
                        trailingContent = if (canSwitch) {
                            {
                                UiIcon(
                                    R.drawable.ic_ui_chevron,
                                    Modifier.rotate(if (expanded) 180f else 0f),
                                )
                            }
                        } else {
                            null
                        },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                    )
                }
                if (expanded && canSwitch) {
                    ModeList(modes, selectedIndex, previewIndex) { mode ->
                        expanded = false
                        if (mode.code != status.modeCode) selectMode(mode.code)
                    }
                }
            }
        }
    }
}

@Composable
private fun ModeList(
    modes: List<ModeItem>,
    selectedIndex: Int,
    previewIndex: Int?,
    onSelect: (ModeItem) -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val listState = rememberLazyListState(initialFirstVisibleItemIndex = (selectedIndex - 1).coerceAtLeast(0))
    LaunchedEffect(previewIndex) {
        if (previewIndex != null) listState.scrollToItem((previewIndex - 1).coerceAtLeast(0))
    }
    LazyColumn(
        modifier = Modifier.fillMaxWidth().heightIn(max = 192.dp).selectableGroup(),
        state = listState,
        userScrollEnabled = previewIndex == null,
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        itemsIndexed(modes, key = { _, mode -> mode.code }) { index, mode ->
            val selected = index == selectedIndex
            val highlighted = index == previewIndex
            ListItem(
                modifier = Modifier
                    .clip(MaterialTheme.shapes.medium)
                    .selectable(selected = selected, role = Role.RadioButton, onClick = { onSelect(mode) }),
                headlineContent = { Text(modeLabel(mode.name)) },
                trailingContent = if (selected) {
                    { UiIcon(R.drawable.ic_ui_check, Modifier.size(18.dp)) }
                } else {
                    null
                },
                colors = when {
                    highlighted -> ListItemDefaults.colors(
                        containerColor = colors.primaryContainer,
                        headlineColor = colors.onPrimaryContainer,
                        trailingIconColor = colors.onPrimaryContainer,
                    )
                    selected -> ListItemDefaults.colors(
                        containerColor = colors.secondaryContainer,
                        headlineColor = colors.onSecondaryContainer,
                        trailingIconColor = colors.onSecondaryContainer,
                    )
                    else -> ListItemDefaults.colors(containerColor = Color.Transparent)
                },
            )
        }
    }
}

internal fun modeIndexAfterDrag(startIndex: Int, distanceDp: Float, lastIndex: Int): Int =
    (startIndex - (distanceDp / MODE_DRAG_STEP_DP).toInt()).coerceIn(0, lastIndex)
