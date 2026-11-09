package com.cakeal.osmolink.ui

import android.Manifest
import android.os.Build
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.currentStateAsState
import com.cakeal.osmolink.ControllerViewModel
import com.cakeal.osmolink.data.Appearance

private enum class PermissionTarget { Scan, Gps }
private enum class Page { Remote, Settings, About }

@Composable
internal fun OsmoLinkApp(
    viewModel: ControllerViewModel,
    appearance: Appearance,
    onAppearanceChange: (Appearance) -> Unit,
) {
    val state by viewModel.state.collectAsState()
    val context = LocalContext.current
    val lifecycleState by LocalLifecycleOwner.current.lifecycle.currentStateAsState()
    val snackbar = remember { SnackbarHostState() }
    var page by rememberSaveable { mutableStateOf(Page.Remote) }
    var permissions by remember { mutableStateOf(context.permissionStatus()) }
    var pendingTarget by rememberSaveable { mutableStateOf<PermissionTarget?>(null) }
    var askedForNotifications by rememberSaveable { mutableStateOf(false) }

    fun requiredPermissions(target: PermissionTarget): List<String> = when (target) {
        PermissionTarget.Scan -> blePermissions()
        PermissionTarget.Gps -> listOf(Manifest.permission.ACCESS_FINE_LOCATION)
    }

    fun onGranted(target: PermissionTarget) {
        when (target) {
            PermissionTarget.Scan -> viewModel.scan()
            PermissionTarget.Gps -> viewModel.setGps(true)
        }
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions(),
    ) {
        val target = pendingTarget
        pendingTarget = null
        permissions = context.permissionStatus()
        if (target != null) {
            if (requiredPermissions(target).all { context.hasPermission(it) }) {
                onGranted(target)
            } else {
                viewModel.notify("权限被拒绝，无法使用该功能")
            }
        }
    }

    fun requestPermissions(target: PermissionTarget) {
        if (pendingTarget != null) return
        val missing = requiredPermissions(target).filterNot { context.hasPermission(it) }
        if (missing.isEmpty()) {
            onGranted(target)
        } else {
            pendingTarget = target
            // Android 12+ requires coarse and fine to be requested together.
            val request = if (Manifest.permission.ACCESS_FINE_LOCATION in missing) {
                missing + Manifest.permission.ACCESS_COARSE_LOCATION
            } else {
                missing
            }
            permissionLauncher.launch(request.toTypedArray())
        }
    }

    val notificationLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission(),
    ) { permissions = context.permissionStatus() }

    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        permissions = context.permissionStatus()
    }

    val resumed = lifecycleState.isAtLeast(Lifecycle.State.RESUMED)
    LaunchedEffect(state.connected, permissions.preciseLocation, resumed) {
        if (state.connected && permissions.preciseLocation && resumed) viewModel.startDefaultGps()
    }

    LaunchedEffect(state.connected, pendingTarget, resumed) {
        if (state.connected && resumed && pendingTarget == null && !askedForNotifications &&
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            !context.hasPermission(Manifest.permission.POST_NOTIFICATIONS)
        ) {
            askedForNotifications = true
            notificationLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    LaunchedEffect(state.message) {
        val message = state.message ?: return@LaunchedEffect
        snackbar.showSnackbar(message)
        viewModel.clearMessage()
    }

    BackHandler(enabled = page != Page.Remote) {
        page = if (page == Page.About) Page.Settings else Page.Remote
    }

    val bottomInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Bottom)
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surface,
        contentWindowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal),
        snackbarHost = { SnackbarHost(snackbar, Modifier.windowInsetsPadding(bottomInsets)) },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding)) {
            if (page == Page.Remote) {
                AnimatedContent(
                    targetState = state.connected,
                    transitionSpec = {
                        fadeIn(tween(220)) togetherWith fadeOut(tween(160))
                    },
                ) { connected ->
                    if (connected) {
                        ControlScreen(
                            state = state,
                            onShutter = viewModel::shutter,
                            onRecord = viewModel::toggleRecord,
                            onMode = viewModel::setMode,
                            onDisconnect = viewModel::disconnect,
                            onGpsToggle = { enabled ->
                                if (enabled) {
                                    requestPermissions(PermissionTarget.Gps)
                                } else {
                                    viewModel.setGps(false)
                                }
                            },
                            onOpenSettings = { page = Page.Settings },
                        )
                    } else {
                        Box(Modifier.windowInsetsPadding(bottomInsets)) {
                            ConnectScreen(
                                state = state,
                                onScan = { requestPermissions(PermissionTarget.Scan) },
                                onConnect = viewModel::connect,
                                onOpenSettings = { page = Page.Settings },
                            )
                        }
                    }
                }
            } else {
                Box(Modifier.windowInsetsPadding(bottomInsets)) {
                    if (page == Page.About) {
                        AboutScreen(onBack = { page = Page.Settings }, onError = viewModel::notify)
                    } else {
                        SettingsScreen(
                            appearance = appearance,
                            permissions = permissions,
                            onAppearanceChange = onAppearanceChange,
                            onBack = { page = Page.Remote },
                            onAbout = { page = Page.About },
                            onError = viewModel::notify,
                        )
                    }
                }
            }
            if (state.connecting) {
                state.pairing?.let { pairing ->
                    PairingDialog(pairing, onCancel = viewModel::disconnect)
                }
            }
        }
    }
}
