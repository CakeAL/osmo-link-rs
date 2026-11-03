package com.cakeal.osmolink.ui

import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import com.cakeal.osmolink.ControllerViewModel

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
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding)) {
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
            }
            if (state.connecting) {
                state.pairing?.let { pairing ->
                    PairingDialog(pairing, onCancel = viewModel::disconnect)
                }
            }
        }
    }
}
