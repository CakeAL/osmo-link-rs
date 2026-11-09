package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
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
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
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
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
        Column(
            modifier = Modifier
                .widthIn(max = 560.dp)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 24.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            Row(verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("相机遥控", style = MaterialTheme.typography.headlineLarge)
                    Text(
                        text = "通过蓝牙连接 DJI Osmo 相机",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                SettingsButton(onOpenSettings)
            }
            ScanCard(state, onScan)
            NearbyDevices(state, onConnect)
            ConnectionHints()
        }
    }
}

@Composable
private fun ScanCard(state: UiState, onScan: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.extraLarge,
        colors = CardDefaults.cardColors(
            containerColor = colors.primaryContainer,
            contentColor = colors.onPrimaryContainer,
        ),
    ) {
        Column(
            modifier = Modifier.padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Surface(
                color = colors.primary,
                contentColor = colors.onPrimary,
                shape = MaterialTheme.shapes.medium,
            ) {
                Box(Modifier.size(44.dp), contentAlignment = Alignment.Center) {
                    UiIcon(R.drawable.ic_ui_search)
                }
            }
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("查找附近的相机", style = MaterialTheme.typography.titleLarge)
                Text("唤醒相机并开启无线连接", style = MaterialTheme.typography.bodyMedium)
            }
            Button(onClick = onScan, enabled = !state.scanning && !state.connecting) {
                UiIcon(R.drawable.ic_ui_search, Modifier.size(ButtonDefaults.IconSize))
                Text(
                    text = when {
                        state.connecting -> "连接中…"
                        state.scanning -> "扫描中…"
                        else -> "搜索相机"
                    },
                    modifier = Modifier.padding(start = ButtonDefaults.IconSpacing),
                )
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
            Text("附近设备", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            val status = when {
                state.connecting -> "正在连接"
                state.scanning -> "正在搜索"
                state.devices.isNotEmpty() -> "找到 ${state.devices.size} 台相机"
                else -> null
            }
            if (status != null) {
                Text(status, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant)
            }
        }
        if (state.scanning || state.connecting) {
            LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
        }
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = MaterialTheme.shapes.extraLarge,
            colors = CardDefaults.cardColors(containerColor = colors.surfaceContainer),
        ) {
            if (state.devices.isEmpty()) {
                Text(
                    text = if (state.scanning) "正在搜索附近的相机…" else "尚未发现相机",
                    modifier = Modifier.padding(16.dp),
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.onSurfaceVariant,
                )
            } else {
                state.devices.forEachIndexed { index, device ->
                    if (index > 0) HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
                    DeviceRow(device, enabled = !state.connecting, onClick = { onConnect(index) })
                }
            }
        }
    }
}

@Composable
private fun DeviceRow(device: DeviceItem, enabled: Boolean, onClick: () -> Unit) {
    ListItem(
        headlineContent = {
            Text(device.name, maxLines = 1, overflow = TextOverflow.Ellipsis)
        },
        supportingContent = {
            Text("${rssiLabel(device.rssi)} · ${device.id}", maxLines = 1, overflow = TextOverflow.Ellipsis)
        },
        leadingContent = {
            Surface(
                shape = MaterialTheme.shapes.medium,
                color = MaterialTheme.colorScheme.secondaryContainer,
                contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
            ) {
                Box(Modifier.size(40.dp), contentAlignment = Alignment.Center) {
                    UiIcon(R.drawable.ic_ui_camera, Modifier.size(20.dp))
                }
            }
        },
        trailingContent = {
            FilledTonalButton(
                onClick = onClick,
                enabled = enabled,
                modifier = Modifier.semantics { contentDescription = "连接 ${device.name}" },
            ) {
                Text("连接")
            }
        },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
    )
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
