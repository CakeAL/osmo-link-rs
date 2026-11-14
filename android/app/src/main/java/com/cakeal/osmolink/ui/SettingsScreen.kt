package com.cakeal.osmolink.ui

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import com.cakeal.osmolink.BuildConfig
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.AppPreferences
import com.cakeal.osmolink.data.BatteryAlertSettings
import com.cakeal.osmolink.data.ThemeMode

private val RGB_HEX = Regex("[0-9A-Fa-f]{6}")

private enum class AppearanceEditor { Mode, Accent }

@Composable
internal fun SettingsScreen(
    preferences: AppPreferences,
    onBack: () -> Unit,
    onAbout: () -> Unit,
    onError: (String) -> Unit,
) {
    val context = LocalContext.current
    val appearance = preferences.appearance
    var permissions by remember { mutableStateOf(context.permissionStatus()) }
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        permissions = context.permissionStatus()
    }
    var editor by rememberSaveable { mutableStateOf<AppearanceEditor?>(null) }
    var batteryDialog by rememberSaveable { mutableStateOf(false) }
    val appDetails = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))

    SettingsPage("设置", onBack) {
        SettingsGroup("权限管理") {
            SettingsRow(
                "蓝牙",
                when {
                    Build.VERSION.SDK_INT < Build.VERSION_CODES.S -> "由系统管理"
                    permissions.bluetooth -> "附近设备已授权"
                    else -> "附近设备未授权"
                },
                onClick = {
                    val intent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                        appDetails
                    } else {
                        Intent(Settings.ACTION_BLUETOOTH_SETTINGS)
                    }
                    context.openScreen(intent, onError)
                },
            )
            HorizontalDivider(Modifier.padding(horizontal = 16.dp))
            val location = when {
                permissions.backgroundLocation && permissions.preciseLocation -> "始终允许精确位置"
                permissions.backgroundLocation -> "始终允许大致位置"
                permissions.preciseLocation -> "已授权精确位置"
                permissions.approximateLocation -> "已授权大致位置"
                else -> "未授权"
            }
            SettingsRow(
                "位置权限",
                if (permissions.backgroundLocation) location else "$location · 可在系统设置中选择「始终允许」",
                onClick = { context.openScreen(appDetails, onError) },
            )
            HorizontalDivider(Modifier.padding(horizontal = 16.dp))
            SettingsRow(
                "通知",
                if (permissions.notifications) "已开启" else "已关闭",
                onClick = {
                    val intent = Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                        .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
                    if (!context.openScreen(intent)) context.openScreen(appDetails, onError)
                },
            )
        }
        SettingsGroup("外观") {
            SettingsRow("明暗模式", appearance.mode.label, onClick = { editor = AppearanceEditor.Mode })
            HorizontalDivider(Modifier.padding(horizontal = 16.dp))
            SettingsRow(
                "主题色",
                appearance.accent?.let { "#%06X".format(it and 0xFFFFFF) } ?: defaultAccentLabel(),
                onClick = { editor = AppearanceEditor.Accent },
            )
            HorizontalDivider(Modifier.padding(horizontal = 16.dp))
            ListItem(
                headlineContent = { Text("兼容返回动画") },
                supportingContent = {
                    val mode = if (preferences.compatibleBack) "应用内预见式返回" else "系统跨 Activity"
                    Text("$mode · 下次打开设置时生效")
                },
                trailingContent = {
                    Switch(checked = preferences.compatibleBack, onCheckedChange = null)
                },
                modifier = Modifier.toggleable(
                    value = preferences.compatibleBack,
                    role = Role.Switch,
                    onValueChange = preferences::updateCompatibleBack,
                ),
                colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            )
        }
        SettingsGroup("连接") {
            ListItem(
                headlineContent = { Text("自动连接已配对相机") },
                supportingContent = {
                    Text(
                        if (preferences.autoConnect) {
                            "搜索时发现之前连接过的相机后直接连接上次使用的那台"
                        } else {
                            "搜索结果只列出附近相机，由你手动选择"
                        },
                    )
                },
                trailingContent = {
                    Switch(checked = preferences.autoConnect, onCheckedChange = null)
                },
                modifier = Modifier.toggleable(
                    value = preferences.autoConnect,
                    role = Role.Switch,
                    onValueChange = preferences::updateAutoConnect,
                ),
                colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            )
        }
        SettingsGroup("电量提醒") {
            ListItem(
                headlineContent = { Text("低电量提醒") },
                supportingContent = {
                    Text(
                        if (preferences.batteryAlertEnabled) {
                            "电量低于 ${preferences.batteryAlertThreshold}% 时震动并发送通知"
                        } else {
                            "已关闭，不会震动或发送通知"
                        },
                    )
                },
                trailingContent = {
                    Switch(checked = preferences.batteryAlertEnabled, onCheckedChange = null)
                },
                modifier = Modifier.toggleable(
                    value = preferences.batteryAlertEnabled,
                    role = Role.Switch,
                    onValueChange = preferences::updateBatteryAlertEnabled,
                ),
                colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            )
            if (preferences.batteryAlertEnabled) {
                HorizontalDivider(Modifier.padding(horizontal = 16.dp))
                SettingsRow(
                    "提醒阈值",
                    "低于 ${preferences.batteryAlertThreshold}% 时提醒",
                    onClick = { batteryDialog = true },
                )
            }
        }
        SettingsGroup("温度提醒") {
            ListItem(
                headlineContent = { Text("高温通知") },
                supportingContent = {
                    Text(
                        if (preferences.temperatureAlertEnabled) {
                            "相机温度达到「无法录制」或「即将关机」时震动并发送通知"
                        } else {
                            "已关闭，不会震动或发送通知"
                        },
                    )
                },
                trailingContent = {
                    Switch(checked = preferences.temperatureAlertEnabled, onCheckedChange = null)
                },
                modifier = Modifier.toggleable(
                    value = preferences.temperatureAlertEnabled,
                    role = Role.Switch,
                    onValueChange = preferences::updateTemperatureAlertEnabled,
                ),
                colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            )
        }
        SettingsGroup {
            SettingsRow("关于 Osmo Link", "版本 ${BuildConfig.VERSION_NAME}", onClick = onAbout)
        }
    }

    when (editor) {
        AppearanceEditor.Mode -> ThemeModeDialog(
            current = appearance.mode,
            onSelect = { mode ->
                preferences.updateAppearance(appearance.copy(mode = mode))
                editor = null
            },
            onDismiss = { editor = null },
        )
        AppearanceEditor.Accent -> AccentDialog(
            current = appearance.accent,
            onSelect = { accent ->
                preferences.updateAppearance(appearance.copy(accent = accent))
                editor = null
            },
            onDismiss = { editor = null },
        )
        null -> Unit
    }

    if (batteryDialog) {
        BatteryThresholdDialog(
            current = preferences.batteryAlertThreshold,
            onSelect = { threshold ->
                preferences.updateBatteryAlertThreshold(threshold)
                batteryDialog = false
            },
            onDismiss = { batteryDialog = false },
        )
    }
}

@Composable
private fun BatteryThresholdDialog(current: Int, onSelect: (Int) -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("低电量提醒阈值") },
        text = {
            Column(Modifier.selectableGroup()) {
                for (threshold in BatteryAlertSettings.THRESHOLDS) {
                    ListItem(
                        headlineContent = { Text("低于 $threshold%") },
                        leadingContent = {
                            RadioButton(selected = current == threshold, onClick = null)
                        },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                        modifier = Modifier.selectable(
                            selected = current == threshold,
                            role = Role.RadioButton,
                            onClick = { onSelect(threshold) },
                        ),
                    )
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}

@Composable
internal fun AboutScreen(onBack: () -> Unit, onError: (String) -> Unit) {
    val context = LocalContext.current
    SettingsPage("关于 Osmo Link", onBack) {
        val colors = MaterialTheme.colorScheme
        Column(
            modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text("Osmo Link", style = MaterialTheme.typography.headlineMedium)
            Text(
                "版本 ${BuildConfig.VERSION_NAME} · ${BuildConfig.VERSION_CODE}",
                style = MaterialTheme.typography.bodyMedium.mono(),
                color = colors.onSurfaceVariant,
            )
        }
        Surface(
            onClick = {
                context.openScreen(
                    Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/CakeAL/osmo-link-rs")),
                    onError,
                )
            },
            modifier = Modifier.fillMaxWidth(),
            shape = MaterialTheme.shapes.extraLarge,
            color = colors.secondaryContainer,
            contentColor = colors.onSecondaryContainer,
        ) {
            ListItem(
                headlineContent = { Text("GitHub 仓库") },
                supportingContent = { Text("CakeAL/osmo-link-rs") },
                trailingContent = { UiIcon(R.drawable.ic_ui_chevron, Modifier.rotate(-90f)) },
                colors = ListItemDefaults.colors(
                    containerColor = Color.Transparent,
                    headlineColor = colors.onSecondaryContainer,
                    supportingColor = colors.onSecondaryContainer,
                    trailingIconColor = colors.onSecondaryContainer,
                ),
            )
        }
        RemoteCard {
            Column(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                SectionTitle("应用信息", R.drawable.ic_ui_info)
                DetailRow("作者", "CakeAL")
                DetailRow("开源许可", "AGPL-3.0 及第三方许可")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SettingsPage(title: String, onBack: () -> Unit, content: @Composable () -> Unit) {
    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) {
        TopAppBar(
            modifier = Modifier.widthIn(max = 560.dp),
            title = { Text(title) },
            navigationIcon = {
                IconButton(onClick = onBack) {
                    Icon(painterResource(R.drawable.ic_ui_back), contentDescription = "返回")
                }
            },
            windowInsets = WindowInsets(0, 0, 0, 0),
        )
        Column(
            modifier = Modifier
                .widthIn(max = 560.dp)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            content()
        }
    }
}

@Composable
private fun SettingsGroup(title: String? = null, content: @Composable () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (title != null) {
            Text(title, Modifier.padding(start = 4.dp), style = MaterialTheme.typography.titleSmall)
        }
        RemoteCard { content() }
    }
}

@Composable
private fun SettingsRow(title: String, subtitle: String, onClick: (() -> Unit)? = null) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = { Text(subtitle) },
        modifier = if (onClick == null) Modifier else Modifier.clickable(role = Role.Button, onClick = onClick),
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
    )
}

@Composable
private fun ThemeModeDialog(current: ThemeMode, onSelect: (ThemeMode) -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("明暗模式") },
        text = {
            Column(Modifier.selectableGroup()) {
                for (mode in ThemeMode.entries) {
                    ListItem(
                        headlineContent = { Text(mode.label) },
                        leadingContent = { RadioButton(selected = current == mode, onClick = null) },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                        modifier = Modifier.selectable(
                            selected = current == mode,
                            role = Role.RadioButton,
                            onClick = { onSelect(mode) },
                        ),
                    )
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}

@Composable
private fun AccentDialog(current: Int?, onSelect: (Int?) -> Unit, onDismiss: () -> Unit) {
    var hex by rememberSaveable { mutableStateOf(current?.let { "%06X".format(it and 0xFFFFFF) }.orEmpty()) }
    val accent = hex.trim().removePrefix("#").takeIf(RGB_HEX::matches)
        ?.let { 0xFF000000.toInt() or it.toInt(16) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("主题色") },
        text = {
            OutlinedTextField(
                value = hex,
                onValueChange = { hex = it },
                label = { Text("十六进制色值") },
                placeholder = { Text("#815511") },
                supportingText = { Text("输入 6 位 RGB 色值，例如 #815511") },
                leadingIcon = if (accent != null) {
                    { Box(Modifier.size(22.dp).background(Color(accent), CircleShape)) }
                } else {
                    null
                },
                isError = hex.isNotBlank() && accent == null,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
                singleLine = true,
            )
        },
        confirmButton = {
            TextButton(onClick = { accent?.let(onSelect) }, enabled = accent != null) {
                Text("应用")
            }
        },
        dismissButton = { TextButton(onClick = { onSelect(null) }) { Text("恢复默认") } },
    )
}

private fun defaultAccentLabel(): String = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) "壁纸配色" else "默认琥珀色"

private fun Context.openScreen(intent: Intent): Boolean = try {
    startActivity(intent)
    true
} catch (_: ActivityNotFoundException) {
    false
} catch (_: SecurityException) {
    false
}

private fun Context.openScreen(intent: Intent, onError: (String) -> Unit) {
    if (!openScreen(intent)) onError("无法打开此页面")
}
