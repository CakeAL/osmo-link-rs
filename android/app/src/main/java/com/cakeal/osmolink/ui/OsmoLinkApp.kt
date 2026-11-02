package com.cakeal.osmolink.ui

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import com.cakeal.osmolink.ControllerViewModel
import com.cakeal.osmolink.data.CameraStatus
import com.cakeal.osmolink.data.DeviceItem
import com.cakeal.osmolink.data.PairingInfo
import com.cakeal.osmolink.data.UiState
import com.cakeal.osmolink.ui.theme.OsmoAmber
import com.cakeal.osmolink.ui.theme.OsmoBackground
import com.cakeal.osmolink.ui.theme.OsmoGreen
import com.cakeal.osmolink.ui.theme.OsmoMuted
import com.cakeal.osmolink.ui.theme.OsmoOnAmber
import com.cakeal.osmolink.ui.theme.OsmoOutline
import com.cakeal.osmolink.ui.theme.OsmoRed
import com.cakeal.osmolink.ui.theme.OsmoSurface
import com.cakeal.osmolink.ui.theme.OsmoSurfaceHigh
import com.cakeal.osmolink.ui.theme.OsmoText

/** Modes where the shutter, not start/stop recording, is the primary action. */
private val PHOTO_MODES = setOf("Photo", "PanoramicPhoto", "SubjectTracking")

private enum class PermissionTarget { Scan, Gps }

@Composable
fun OsmoLinkApp(viewModel: ControllerViewModel) {
    val state by viewModel.state.collectAsState()
    val context = LocalContext.current
    val snackbar = remember { SnackbarHostState() }
    var pendingTarget by remember { mutableStateOf<PermissionTarget?>(null) }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions(),
    ) { result ->
        val target = pendingTarget
        pendingTarget = null
        val granted = result.values.isNotEmpty() && result.values.all { it }
        if (!granted) {
            viewModel.notify("权限被拒绝，无法使用该功能")
        } else {
            when (target) {
                PermissionTarget.Scan -> viewModel.scan()
                PermissionTarget.Gps -> viewModel.setGps(true)
                null -> Unit
            }
        }
    }

    fun requestPermissions(permissions: Array<String>, target: PermissionTarget) {
        val missing = permissions.filter {
            ContextCompat.checkSelfPermission(context, it) != PackageManager.PERMISSION_GRANTED
        }
        if (missing.isEmpty()) {
            when (target) {
                PermissionTarget.Scan -> viewModel.scan()
                PermissionTarget.Gps -> viewModel.setGps(true)
            }
        } else {
            pendingTarget = target
            permissionLauncher.launch(missing.toTypedArray())
        }
    }

    LaunchedEffect(state.message) {
        val message = state.message ?: return@LaunchedEffect
        snackbar.showSnackbar(message)
        viewModel.clearMessage()
    }

    Scaffold(
        containerColor = OsmoBackground,
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding)) {
            if (state.connected) {
                ControlScreen(
                    state = state,
                    onShutter = viewModel::shutter,
                    onRecord = viewModel::toggleRecord,
                    onMode = viewModel::setMode,
                    onDisconnect = viewModel::disconnect,
                    onGpsToggle = { enabled ->
                        if (enabled) {
                            requestPermissions(locationPermissions(), PermissionTarget.Gps)
                        } else {
                            viewModel.setGps(false)
                        }
                    },
                )
            } else {
                ConnectScreen(
                    state = state,
                    onScan = { requestPermissions(blePermissions(), PermissionTarget.Scan) },
                    onConnect = viewModel::connect,
                )
            }
            if (state.connecting) {
                state.pairing?.let { pairing ->
                    PairingDialog(pairing, onCancel = viewModel::disconnect)
                }
            }
        }
    }
}

@Composable
private fun ConnectScreen(
    state: UiState,
    onScan: () -> Unit,
    onConnect: (Int) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 24.dp, vertical = 28.dp),
        verticalArrangement = Arrangement.spacedBy(18.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(OsmoAmber))
            Spacer(Modifier.width(8.dp))
            Text(
                text = "OSMO LINK",
                color = OsmoAmber,
                fontSize = 12.sp,
                letterSpacing = 3.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Medium,
            )
        }
        Text(
            text = "相机遥控",
            color = OsmoText,
            fontSize = 34.sp,
            fontWeight = FontWeight.SemiBold,
            letterSpacing = (-0.5).sp,
        )
        Text(
            text = "通过蓝牙连接 DJI Osmo 相机，注入手机定位并远程控制拍录。",
            color = OsmoMuted,
            fontSize = 14.sp,
            lineHeight = 21.sp,
        )

        Surface(
            color = OsmoSurface,
            shape = RoundedCornerShape(18.dp),
            border = BorderStroke(1.dp, OsmoOutline),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "附近设备",
                        color = OsmoText,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Medium,
                        modifier = Modifier.weight(1f),
                    )
                    if (state.scanning) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(14.dp),
                            color = OsmoAmber,
                            strokeWidth = 2.dp,
                        )
                        Spacer(Modifier.width(8.dp))
                        Text("扫描中", color = OsmoMuted, fontSize = 11.sp)
                    }
                }
                if (state.devices.isEmpty()) {
                    Text(
                        text = if (state.scanning) "正在搜索附近的相机…" else "尚未发现相机",
                        color = OsmoMuted,
                        fontSize = 13.sp,
                    )
                } else {
                    state.devices.forEachIndexed { index, device ->
                        DeviceRow(
                            device = device,
                            enabled = !state.connecting,
                            onClick = { onConnect(index) },
                        )
                    }
                }
                Button(
                    onClick = onScan,
                    enabled = !state.scanning && !state.connecting,
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = OsmoAmber,
                        contentColor = OsmoOnAmber,
                        disabledContainerColor = OsmoSurfaceHigh,
                        disabledContentColor = OsmoMuted,
                    ),
                    shape = RoundedCornerShape(12.dp),
                ) {
                    Text(
                        text = when {
                            state.connecting -> "连接中…"
                            state.scanning -> "扫描中…"
                            else -> "搜索相机"
                        },
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 14.sp,
                    )
                }
            }
        }

        Surface(
            color = OsmoBackground,
            shape = RoundedCornerShape(14.dp),
            border = BorderStroke(1.dp, OsmoOutline),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("连接提示", color = OsmoText, fontSize = 13.sp, fontWeight = FontWeight.Medium)
                HintLine("唤醒相机并开启无线连接")
                HintLine("断开其它遥控器或 Mimo App")
                HintLine("首次连接需在相机屏幕上确认配对码")
                HintLine("扫描依赖相机广播，找不到时可稍等后重试")
            }
        }
        Spacer(Modifier.height(8.dp))
    }
}

@Composable
private fun HintLine(text: String) {
    Row(verticalAlignment = Alignment.Top) {
        Text("·", color = OsmoAmber, fontSize = 13.sp)
        Spacer(Modifier.width(8.dp))
        Text(text, color = OsmoMuted, fontSize = 12.sp, lineHeight = 18.sp)
    }
}

@Composable
private fun DeviceRow(device: DeviceItem, enabled: Boolean, onClick: () -> Unit) {
    Surface(
        color = OsmoSurfaceHigh,
        shape = RoundedCornerShape(12.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(device.name, color = OsmoText, fontSize = 14.sp, fontWeight = FontWeight.Medium)
                Text(
                    text = device.id,
                    color = OsmoMuted,
                    fontSize = 10.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Text(
                text = rssiLabel(device.rssi),
                color = OsmoMuted,
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
            )
            Spacer(Modifier.width(10.dp))
            TextButton(onClick = onClick, enabled = enabled) {
                Text("连接", color = if (enabled) OsmoAmber else OsmoMuted, fontSize = 13.sp)
            }
        }
    }
}

@Composable
private fun ControlScreen(
    state: UiState,
    onShutter: () -> Unit,
    onRecord: () -> Unit,
    onMode: (Int) -> Unit,
    onDisconnect: () -> Unit,
    onGpsToggle: (Boolean) -> Unit,
) {
    Column(Modifier.fillMaxSize()) {
        ControlTopBar(state, onDisconnect)
        Column(
            modifier = Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            StatusHero(state)
            if (state.modes.isNotEmpty()) {
                ModePicker(state, onMode)
            }
            StatusDetails(state)
            GpsCard(state, onGpsToggle)
            Spacer(Modifier.height(4.dp))
        }
        ControlBar(
            state = state,
            onShutter = onShutter,
            onRecord = onRecord,
            onGpsToggle = onGpsToggle,
        )
    }
}

@Composable
private fun ControlTopBar(state: UiState, onDisconnect: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(OsmoGreen))
        Spacer(Modifier.width(8.dp))
        Column(Modifier.weight(1f)) {
            Text(
                text = state.deviceName.ifBlank { "相机" },
                color = OsmoText,
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = state.modelName.ifBlank { "已连接" },
                color = OsmoMuted,
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
            )
        }
        Text(
            text = "${state.status.battery}%",
            color = batteryColor(state.status.battery),
            fontSize = 12.sp,
            fontFamily = FontFamily.Monospace,
        )
        Spacer(Modifier.width(8.dp))
        TextButton(onClick = onDisconnect) {
            Text("断开", color = OsmoMuted, fontSize = 12.sp)
        }
    }
}

@Composable
private fun StatusHero(state: UiState) {
    val status = state.status
    Surface(
        color = OsmoSurface,
        shape = RoundedCornerShape(20.dp),
        border = BorderStroke(1.dp, OsmoOutline),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 22.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = if (status.hasData) formatDuration(status.recordSeconds) else "--:--",
                color = OsmoText,
                fontSize = 46.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Bold,
                letterSpacing = 1.sp,
            )
            Spacer(Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (status.capturing) {
                    Box(Modifier.size(8.dp).clip(CircleShape).background(OsmoRed))
                    Spacer(Modifier.width(6.dp))
                    Text(
                        text = "REC",
                        color = OsmoRed,
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.width(10.dp))
                }
                Text(stateLabel(status.stateName), color = OsmoMuted, fontSize = 12.sp)
            }
            Spacer(Modifier.height(6.dp))
            Text(
                text = listOf(
                    modeLabel(status.modeName),
                    status.resolution,
                    status.fps,
                ).filter { it.isNotBlank() }.joinToString(" · ").ifBlank { "等待相机状态" },
                color = OsmoMuted,
                fontSize = 12.sp,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
private fun ModePicker(state: UiState, onSelect: (Int) -> Unit) {
    LazyRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        contentPadding = PaddingValues(vertical = 2.dp),
    ) {
        items(state.modes, key = { it.code }) { mode ->
            FilterChip(
                selected = mode.code == state.status.modeCode,
                onClick = { onSelect(mode.code) },
                enabled = state.pendingAction == null,
                label = { Text(modeLabel(mode.name), fontSize = 12.sp) },
                colors = FilterChipDefaults.filterChipColors(
                    containerColor = OsmoSurfaceHigh,
                    labelColor = OsmoMuted,
                    selectedContainerColor = OsmoAmber,
                    selectedLabelColor = OsmoOnAmber,
                ),
            )
        }
    }
}

@Composable
private fun StatusDetails(state: UiState) {
    val status = state.status
    Surface(
        color = OsmoSurface,
        shape = RoundedCornerShape(16.dp),
        border = BorderStroke(1.dp, OsmoOutline),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(11.dp),
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
                    label = "剩余照片",
                    value = if (status.hasData) "${status.remainingPhotos}" else "—",
                    modifier = Modifier.weight(1f),
                )
                StatBlock(
                    label = "电池",
                    value = if (status.hasData) "${status.battery}%" else "—",
                    modifier = Modifier.weight(1f),
                )
            }
            Box(Modifier.fillMaxWidth().height(1.dp).background(OsmoOutline))
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
        Text(label, color = OsmoMuted, fontSize = 10.sp)
        Spacer(Modifier.height(2.dp))
        Text(
            text = value,
            color = OsmoText,
            fontSize = 15.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Medium,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
private fun DetailRow(label: String, value: String, warning: Boolean = false) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(label, color = OsmoMuted, fontSize = 12.sp)
        Spacer(Modifier.width(12.dp))
        Text(
            text = value,
            color = if (warning) OsmoRed else OsmoText,
            fontSize = 12.sp,
            fontFamily = FontFamily.Monospace,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f),
        )
    }
}

@Composable
private fun GpsCard(state: UiState, onToggle: (Boolean) -> Unit) {
    val accuracy = state.gpsAccuracy
    Surface(
        color = OsmoSurface,
        shape = RoundedCornerShape(16.dp),
        border = BorderStroke(1.dp, OsmoOutline),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("定位注入", color = OsmoText, fontSize = 14.sp, fontWeight = FontWeight.Medium)
                    Text(
                        text = if (state.gpsEnabled) "向相机持续发送手机定位" else "关闭",
                        color = OsmoMuted,
                        fontSize = 11.sp,
                    )
                }
                Switch(
                    checked = state.gpsEnabled,
                    onCheckedChange = onToggle,
                    enabled = state.connected || state.gpsEnabled,
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = OsmoOnAmber,
                        checkedTrackColor = OsmoAmber,
                        uncheckedThumbColor = OsmoMuted,
                        uncheckedTrackColor = OsmoSurfaceHigh,
                    ),
                )
            }
            if (state.gpsEnabled) {
                Text(
                    text = buildString {
                        append("%.1f Hz".format(state.gpsRate))
                        append(" · ")
                        append("${state.gpsSatellites} 星")
                        if (accuracy != null) {
                            append(" · ")
                            append("±%.1f m".format(accuracy))
                        }
                    },
                    color = OsmoGreen,
                    fontSize = 12.sp,
                    fontFamily = FontFamily.Monospace,
                )
            }
            Text(
                text = "按 100ms 目标间隔发送最近定位；实际频率由系统定位决定，录像中开启只影响之后的画面。",
                color = OsmoMuted,
                fontSize = 10.sp,
                lineHeight = 15.sp,
            )
        }
    }
}

@Composable
private fun ControlBar(
    state: UiState,
    onShutter: () -> Unit,
    onRecord: () -> Unit,
    onGpsToggle: (Boolean) -> Unit,
) {
    val photoMode = state.status.modeName in PHOTO_MODES
    Surface(color = OsmoBackground, modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 30.dp, vertical = 14.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (photoMode) {
                Spacer(Modifier.size(62.dp))
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
    val accent = if (photoMode) OsmoAmber else OsmoRed
    Box(
        modifier = Modifier
            .size(88.dp)
            .clip(CircleShape)
            .border(3.dp, if (enabled) accent else OsmoOutline, CircleShape)
            .clickable(enabled = enabled && !pending, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        when {
            pending -> CircularProgressIndicator(
                modifier = Modifier.size(30.dp),
                color = accent,
                strokeWidth = 3.dp,
            )
            photoMode -> Box(Modifier.size(62.dp).clip(CircleShape).background(accent))
            capturing -> Box(Modifier.size(28.dp).clip(RoundedCornerShape(6.dp)).background(accent))
            else -> Box(Modifier.size(62.dp).clip(CircleShape).background(accent))
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
    Box(
        modifier = Modifier
            .size(62.dp)
            .clip(CircleShape)
            .border(1.dp, if (active) OsmoAmber else OsmoOutline, CircleShape)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            color = when {
                active -> OsmoAmber
                enabled -> OsmoText
                else -> OsmoOutline
            },
            fontSize = 12.sp,
        )
    }
}

@Composable
private fun PairingDialog(pairing: PairingInfo, onCancel: () -> Unit) {
    AlertDialog(
        onDismissRequest = {},
        containerColor = OsmoSurface,
        titleContentColor = OsmoText,
        textContentColor = OsmoMuted,
        title = { Text(if (pairing.firstTime) "首次配对" else "重新连接") },
        text = {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("请在相机屏幕上确认配对码", fontSize = 13.sp)
                Text(
                    text = "%04d".format(pairing.code),
                    color = OsmoAmber,
                    fontSize = 40.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                )
                Text(pairing.device, fontSize = 12.sp)
            }
        },
        confirmButton = {},
        dismissButton = {
            TextButton(onClick = onCancel) { Text("取消", color = OsmoMuted) }
        },
    )
}

private fun modeLabel(name: String): String = when (name) {
    "Video" -> "视频"
    "Photo" -> "拍照"
    "SlowMotion" -> "慢动作"
    "Timelapse" -> "延时"
    "Hyperlapse" -> "移动延时"
    "SuperNight" -> "超级夜景"
    "SubjectTracking" -> "人像跟随"
    "Selfie" -> "自拍"
    "PanoramicVideo" -> "全景视频"
    "PanoramicPhoto" -> "全景照片"
    "PanoramicHyperlapse" -> "全景延时"
    "PanoramicSuperNight" -> "全景夜景"
    "SingleLensSuperNight" -> "单镜夜景"
    "UltraWideVideo" -> "超广角"
    "TimeFreeze" -> "时间静止"
    "LiveStream" -> "直播"
    "UvcLiveStream" -> "UVC 直播"
    else -> name.ifBlank { "未知模式" }
}

private fun stateLabel(name: String): String = when (name) {
    "Capturing" -> "拍摄中"
    "LiveView" -> "待机"
    "Playback" -> "回放"
    "PreRecording" -> "预录制"
    "ScreenOff" -> "息屏"
    else -> "未知状态"
}

private fun temperatureLabel(status: CameraStatus): String = when (status.temperatureCode) {
    2 -> "过热，无法录制"
    3 -> "即将关机"
    else -> status.temperatureName
}

private fun batteryColor(percent: Int): Color = when {
    percent <= 15 -> OsmoRed
    percent <= 40 -> OsmoAmber
    else -> OsmoGreen
}

private fun formatDuration(seconds: Int): String {
    val hours = seconds / 3600
    val minutes = (seconds % 3600) / 60
    val secs = seconds % 60
    return if (hours > 0) {
        "%d:%02d:%02d".format(hours, minutes, secs)
    } else {
        "%02d:%02d".format(minutes, secs)
    }
}

private fun rssiLabel(rssi: Int?): String = rssi?.let { "$it dBm" } ?: "—"

private fun blePermissions(): Array<String> {
    val permissions = mutableListOf<String>()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        permissions += Manifest.permission.BLUETOOTH_SCAN
        permissions += Manifest.permission.BLUETOOTH_CONNECT
    }
    // Scanning below API 31 needs location, and GPS injection needs it anyway.
    permissions += Manifest.permission.ACCESS_FINE_LOCATION
    return permissions.distinct().toTypedArray()
}

private fun locationPermissions(): Array<String> =
    arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
