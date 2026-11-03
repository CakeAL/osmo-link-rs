package com.cakeal.osmolink.ui

import android.Manifest
import android.os.Build

internal fun blePermissions(): Array<String> {
    val permissions = mutableListOf<String>()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        permissions += Manifest.permission.BLUETOOTH_SCAN
        permissions += Manifest.permission.BLUETOOTH_CONNECT
    }
    // Scanning below API 31 needs location, and GPS injection needs it anyway.
    permissions += Manifest.permission.ACCESS_FINE_LOCATION
    return permissions.distinct().toTypedArray()
}

internal fun locationPermissions(): Array<String> =
    arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
