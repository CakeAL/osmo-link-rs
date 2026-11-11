package com.cakeal.osmolink.data

data class DeviceItem(
    val id: String,
    val name: String,
    val rssi: Int?,
)

data class ModeItem(
    val code: Int,
    val name: String,
)

data class PairingInfo(
    val code: Int,
    val device: String,
    val firstTime: Boolean,
)

/** Subset of `osmo_core::CameraStatus` the UI renders. */
data class CameraStatus(
    val modeCode: Int = -1,
    val modeName: String = "",
    val stateName: String = "",
    val capturing: Boolean = false,
    val resolution: String = "",
    val fps: String = "",
    val eis: String = "",
    val recordSeconds: Int = 0,
    val remainingMb: Int = 0,
    val remainingPhotos: Int = 0,
    val remainingSeconds: Int = 0,
    val battery: Int = 0,
    val temperatureCode: Int = 0,
    val temperatureName: String = "Normal",
    val hasData: Boolean = false,
)

data class UiState(
    val connected: Boolean = false,
    val connecting: Boolean = false,
    val scanning: Boolean = false,
    val hasSearched: Boolean = false,
    val reconnectAttempt: Int? = null,
    val devices: List<DeviceItem> = emptyList(),
    val modes: List<ModeItem> = emptyList(),
    val deviceName: String = "",
    val modelName: String = "",
    val version: String? = null,
    val modeInfo: String? = null,
    val status: CameraStatus = CameraStatus(),
    val pairing: PairingInfo? = null,
    val gpsEnabled: Boolean = false,
    val gpsRate: Double = 0.0,
    val gpsAccuracy: Double? = null,
    val gpsSatellites: Int = 0,
    /** Action name while a command is in flight; blocks duplicate taps. */
    val pendingAction: String? = null,
    val message: String? = null,
)
