package com.cakeal.osmolink.ui

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat

internal fun Context.hasPermission(permission: String): Boolean =
    ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED

internal fun bluetoothPermissions(): List<String> =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        listOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT)
    } else {
        emptyList()
    }

internal fun blePermissions(): List<String> = bluetoothPermissions() + Manifest.permission.ACCESS_FINE_LOCATION

internal data class PermissionStatus(
    val bluetooth: Boolean,
    val preciseLocation: Boolean,
    val approximateLocation: Boolean,
    val backgroundLocation: Boolean,
    val notifications: Boolean,
)

internal fun Context.permissionStatus() = PermissionStatus(
    bluetooth = bluetoothPermissions().all { hasPermission(it) },
    preciseLocation = hasPermission(Manifest.permission.ACCESS_FINE_LOCATION),
    approximateLocation = hasPermission(Manifest.permission.ACCESS_COARSE_LOCATION),
    backgroundLocation = hasPermission(Manifest.permission.ACCESS_BACKGROUND_LOCATION),
    notifications = NotificationManagerCompat.from(this).areNotificationsEnabled(),
)
