package com.cakeal.osmolink.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.DeviceItem
import com.cakeal.osmolink.data.UiState

@Composable
internal fun ConnectScreen(
    state: UiState,
    onScan: () -> Unit,
    onConnect: (Int) -> Unit,
    onOpenSettings: () -> Unit,
) {
    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) {
        RemoteTopBar("Osmo Link") {
            SettingsButton(onOpenSettings, enabled = !state.connecting)
        }
        Column(
            modifier = Modifier
                .widthIn(max = 560.dp)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            DiscoveryHero()
            Button(
                onClick = onScan,
                enabled = !state.scanning && !state.connecting,
                modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp),
            ) {
                UiIcon(R.drawable.ic_ui_search, Modifier.size(ButtonDefaults.IconSize))
                Text(
                    text = if (state.hasSearched) "重新搜索相机" else "搜索相机",
                    modifier = Modifier.padding(start = ButtonDefaults.IconSpacing),
                )
            }
            NearbyDevices(state, onConnect)
            ConnectionHints()
            RecentEventsCard(state)
        }
    }
}

@Composable
private fun DiscoveryHero() {
    val colors = MaterialTheme.colorScheme
    CameraHeroSurface {
        Column(
            modifier = Modifier.padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Surface(color = colors.primary, contentColor = colors.onPrimary, shape = MaterialTheme.shapes.large) {
                    Box(Modifier.size(48.dp), contentAlignment = Alignment.Center) {
                        UiIcon(R.drawable.ic_ui_camera)
                    }
                }
                StatusPill("蓝牙连接", icon = R.drawable.ic_ui_bluetooth)
            }
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("连接 Osmo 相机", style = MaterialTheme.typography.headlineMedium)
                Text("查看相机状态、切换拍摄模式和遥控拍摄。", style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}

@Composable
private fun NearbyDevices(state: UiState, onConnect: (Int) -> Unit) {
    val colors = MaterialTheme.colorScheme
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "附近相机",
                modifier = Modifier.weight(1f).semantics { heading() },
                style = MaterialTheme.typography.titleMedium,
            )
            if (state.devices.isNotEmpty() && !state.connecting) {
                StatusPill("${state.devices.size} 台", color = colors.secondaryContainer, contentColor = colors.onSecondaryContainer)
            }
        }
        when {
            state.scanning || state.connecting -> RemoteCard {
                val auto = state.connecting && state.pairing?.auto == true
                ListItem(
                    headlineContent = {
                        Text(
                            when {
                                auto -> "正在自动连接"
                                state.connecting -> "正在连接相机"
                                else -> "正在搜索相机"
                            },
                        )
                    },
                    supportingContent = {
                        Text(
                            when {
                                auto -> "已找到上次连接的相机，正在恢复连接。"
                                state.connecting -> "请留意相机屏幕上的配对提示。"
                                else -> "请保持相机唤醒。"
                            },
                        )
                    },
                    leadingContent = { CircularProgressIndicator(Modifier.size(24.dp), strokeWidth = 2.dp) },
                    colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                )
            }
            state.devices.isEmpty() -> RemoteCard {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Surface(shape = CircleShape, color = colors.secondaryContainer, contentColor = colors.onSecondaryContainer) {
                        Box(Modifier.size(48.dp), contentAlignment = Alignment.Center) {
                            UiIcon(if (state.hasSearched) R.drawable.ic_ui_search else R.drawable.ic_ui_camera)
                        }
                    }
                    Text(if (state.hasSearched) "没有找到相机" else "等待搜索", style = MaterialTheme.typography.titleMedium)
                    Text(
                        text = if (state.hasSearched) "唤醒相机并开启无线连接，然后重新搜索。" else "附近可连接的 Osmo 相机会显示在这里。",
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                    )
                }
            }
            else -> state.devices.forEachIndexed { index, device ->
                DeviceRow(device, onClick = { onConnect(index) })
            }
        }
    }
}

@Composable
private fun DeviceRow(device: DeviceItem, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    Surface(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth(),
        color = colors.surfaceContainer,
        shape = MaterialTheme.shapes.large,
    ) {
        ListItem(
            headlineContent = { Text(device.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
            supportingContent = {
                val signal = device.rssi?.let { "$it dBm" } ?: "信号未知"
                Text("$signal · ${device.id}", maxLines = 1, overflow = TextOverflow.Ellipsis)
            },
            leadingContent = {
                Surface(shape = MaterialTheme.shapes.medium, color = colors.secondaryContainer, contentColor = colors.onSecondaryContainer) {
                    Box(Modifier.size(40.dp), contentAlignment = Alignment.Center) {
                        UiIcon(R.drawable.ic_ui_camera, Modifier.size(20.dp))
                    }
                }
            },
            trailingContent = { SignalBars(device.rssi) },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        )
    }
}

@Composable
private fun SignalBars(rssi: Int?) {
    val colors = MaterialTheme.colorScheme
    val level = when {
        rssi == null -> 0
        rssi <= -80 -> 1
        rssi <= -65 -> 2
        rssi <= -50 -> 3
        else -> 4
    }
    Row(horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.Bottom) {
        repeat(4) { index ->
            val color = if (index < level) colors.primary else colors.outlineVariant
            Box(Modifier.width(3.dp).height((5 + 3 * index).dp).background(color, CircleShape))
        }
    }
}

@Composable
private fun ConnectionHints() {
    var expanded by rememberSaveable { mutableStateOf(false) }
    Column {
        DisclosureRow("连接提示", R.drawable.ic_ui_info, expanded, onToggle = { expanded = !expanded })
        if (expanded) {
            Column(
                modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (hint in listOf(
                    "唤醒相机并开启无线连接",
                    "断开其他遥控器或 Mimo App",
                    "首次连接需在相机屏幕上确认配对码",
                    "扫描依赖相机广播，找不到时可稍等后重试",
                )) {
                    Text(hint, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}
