package com.cakeal.osmolink.bridge

import android.bluetooth.le.ScanResult

/** JNI entry points for btleplug and Android's platform-owned scan results. */
object OsmoNative {
    init {
        System.loadLibrary("osmo_mobile")
    }

    external fun nativeBluetoothInit(): Boolean

    external fun nativeScanResult(result: ScanResult): Boolean
}
