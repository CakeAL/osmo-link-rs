package com.cakeal.osmolink.bridge

/**
 * JNI surface of `crates/android-bridge`.
 *
 * Every method only starts work and returns immediately; the outcome is
 * delivered through [BridgeListener.onEvent] as `(type, json)`. This keeps the
 * main thread free while BLE pairing, recording commands and scans run.
 */
object OsmoNative {
    init {
        System.loadLibrary("osmo_bridge")
    }

    external fun nativeInit(stateDir: String, listener: BridgeListener): Boolean

    external fun nativeScan(seconds: Int, autoConnect: Boolean): Boolean

    external fun nativeConnect(index: Int): Boolean

    external fun nativeDisconnect(): Boolean

    external fun nativeVersion(): Boolean

    external fun nativeRecordStart(): Boolean

    external fun nativeRecordStop(): Boolean

    external fun nativeShutter(): Boolean

    external fun nativeSetMode(code: Int): Boolean

    external fun nativeGpsUpdate(sampleJson: String): Boolean

    external fun nativeGpsStop(): Boolean
}
