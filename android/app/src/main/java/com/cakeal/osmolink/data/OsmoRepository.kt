package com.cakeal.osmolink.data

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationManager
import android.util.Log
import androidx.core.content.ContextCompat
import com.cakeal.osmolink.bridge.BridgeListener
import com.cakeal.osmolink.bridge.OsmoNative
import com.cakeal.osmolink.location.GpsTracker
import com.cakeal.osmolink.service.OsmoService
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import org.json.JSONException
import org.json.JSONObject

/**
 * Process-wide owner of the bridge, GPS tracker and UI state.
 *
 * It lives outside the activity so a foreground service can keep the camera
 * connection and location stream alive after the UI is gone. The activity only
 * observes [state] and forwards intents.
 */
object OsmoRepository : BridgeListener {
    private val _state = MutableStateFlow(UiState())
    val state: StateFlow<UiState> = _state.asStateFlow()

    private lateinit var context: Context
    private var attached = false
    private var serviceRunning = false
    private var gps: GpsTracker? = null
    private val fixTimes = ArrayDeque<Long>()

    /** Called once from [com.cakeal.osmolink.OsmoApp]. */
    fun attach(application: Context) {
        if (attached) return
        attached = true
        context = application.applicationContext
        if (!OsmoNative.nativeInit(context.filesDir.absolutePath, this)) {
            update { it.copy(message = "原生蓝牙初始化失败") }
        }
    }

    fun scan() {
        val current = _state.value
        if (current.scanning || current.connected || current.connecting) return
        update { it.copy(scanning = true, devices = emptyList(), message = null) }
        if (!OsmoNative.nativeScan(12)) {
            update { it.copy(scanning = false, message = "无法开始扫描，请检查蓝牙与权限") }
        }
    }

    fun connect(index: Int) {
        if (_state.value.connecting || _state.value.connected) return
        update { it.copy(connecting = true, message = null) }
        if (!OsmoNative.nativeConnect(index)) {
            update { it.copy(connecting = false, message = "连接请求失败") }
        }
    }

    fun disconnect() {
        OsmoNative.nativeDisconnect()
        stopGpsInternal()
        update {
            it.copy(
                connected = false,
                connecting = false,
                pairing = null,
                pendingAction = null,
                status = CameraStatus(),
                version = null,
                deviceName = "",
                modelName = "",
                devices = emptyList(),
            )
        }
    }

    fun setMode(code: Int) {
        if (!_state.value.connected || _state.value.pendingAction != null) return
        update { it.copy(pendingAction = "mode") }
        if (!OsmoNative.nativeSetMode(code)) {
            update { it.copy(pendingAction = null) }
        }
    }

    fun toggleRecord() {
        val current = _state.value
        if (!current.connected || current.pendingAction != null) return
        val starting = !current.status.capturing
        val action = if (starting) "record.start" else "record.stop"
        update { it.copy(pendingAction = action) }
        val sent = if (starting) OsmoNative.nativeRecordStart() else OsmoNative.nativeRecordStop()
        if (!sent) update { it.copy(pendingAction = null) }
    }

    fun shutter() {
        if (!_state.value.connected) return
        update { it.copy(pendingAction = "shutter") }
        if (!OsmoNative.nativeShutter()) {
            update { it.copy(pendingAction = null) }
        }
    }

    fun setGps(enabled: Boolean) {
        if (!enabled) {
            stopGps()
            return
        }
        if (gps != null) return
        if (ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.ACCESS_FINE_LOCATION,
            ) != PackageManager.PERMISSION_GRANTED
        ) {
            update { it.copy(message = "定位注入需要位置权限") }
            return
        }
        val manager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
        if (manager?.isProviderEnabled(LocationManager.GPS_PROVIDER) != true) {
            update { it.copy(message = "请先在系统设置中开启定位") }
            return
        }
        val tracker = GpsTracker(context) { sampleJson, accuracy, satellites ->
            val now = System.currentTimeMillis()
            fixTimes.addLast(now)
            while (fixTimes.isNotEmpty() && now - fixTimes.first() > 5_000) {
                fixTimes.removeFirst()
            }
            OsmoNative.nativeGpsUpdate(sampleJson)
            val rate = fixTimes.size / 5.0
            update {
                it.copy(
                    gpsEnabled = true,
                    gpsRate = rate,
                    gpsAccuracy = accuracy,
                    gpsSatellites = satellites,
                )
            }
        }
        if (tracker.start()) {
            gps = tracker
            update { it.copy(gpsEnabled = true) }
        } else {
            tracker.stop()
            update { it.copy(message = "无法启动定位，请确认 GPS 已开启") }
        }
    }

    fun stopGps() {
        gps?.stop()
        gps = null
        OsmoNative.nativeGpsStop()
        fixTimes.clear()
        update {
            it.copy(gpsEnabled = false, gpsRate = 0.0, gpsAccuracy = null, gpsSatellites = 0)
        }
    }

    fun notify(message: String) {
        update { it.copy(message = message) }
    }

    fun clearMessage() {
        update { it.copy(message = null) }
    }

    private fun stopGpsInternal() {
        gps?.stop()
        gps = null
        fixTimes.clear()
        update {
            it.copy(gpsEnabled = false, gpsRate = 0.0, gpsAccuracy = null, gpsSatellites = 0)
        }
    }

    @Synchronized
    private fun update(transform: (UiState) -> UiState) {
        _state.update(transform)
        syncService()
    }

    /** Keep the foreground service alive exactly while it has something to do. */
    private fun syncService() {
        if (!attached) return
        val desired = _state.value.connected || _state.value.gpsEnabled
        if (desired == serviceRunning) return
        serviceRunning = desired
        if (desired) {
            if (!OsmoService.start(context)) {
                serviceRunning = false
            }
        } else {
            OsmoService.stop(context)
        }
    }

    override fun onEvent(type: String, payload: String) {
        if (type != "status") {
            Log.d("OsmoBridge", "$type $payload")
        }
        val json = try {
            JSONObject(payload)
        } catch (_: JSONException) {
            JSONObject()
        }
        when (type) {
            "modes" -> update { it.copy(modes = parseModes(json)) }
            "devices" -> update { it.copy(scanning = false, devices = parseDevices(json)) }
            "pairing" -> update {
                it.copy(
                    connecting = true,
                    pairing = PairingInfo(
                        code = json.optInt("code"),
                        device = json.optString("device"),
                        firstTime = json.optBoolean("firstTime"),
                    ),
                )
            }
            "connected" -> update {
                it.copy(
                    connected = true,
                    connecting = false,
                    pairing = null,
                    devices = emptyList(),
                    deviceName = json.optString("name"),
                    modelName = json.optString("modelName"),
                    message = null,
                )
            }
            "connectFailed" -> update {
                it.copy(
                    connecting = false,
                    pairing = null,
                    message = "连接失败：${json.optString("message")}",
                )
            }
            "status" -> update { it.copy(status = parseStatus(json)) }
            "version" -> {
                val product = json.optString("product")
                val details = json.optString("details")
                val text = listOf(product, details).filter { value -> value.isNotBlank() }
                update { it.copy(version = text.joinToString(" · ").ifBlank { null }) }
            }
            "command" -> {
                val action = json.optString("action")
                val ok = json.optBoolean("ok")
                update { current ->
                    val status = when {
                        action == "record.start" && ok -> current.status.copy(capturing = true)
                        action == "record.stop" && ok -> current.status.copy(capturing = false)
                        else -> current.status
                    }
                    current.copy(
                        pendingAction = null,
                        status = status,
                        message = if (ok) {
                            null
                        } else {
                            json.optString("error").ifBlank { "操作失败：$action" }
                        },
                    )
                }
            }
            "warning", "error", "gpsError" ->
                update { it.copy(message = json.optString("message")) }
            "reconnecting" -> update {
                it.copy(message = "连接中断，正在重连（第 ${json.optInt("attempt")} 次）")
            }
            "reconnected" -> update { it.copy(message = "已重新连接") }
            "disconnected" -> {
                stopGpsInternal()
                update {
                    it.copy(
                        connected = false,
                        connecting = false,
                        pairing = null,
                        pendingAction = null,
                        status = CameraStatus(),
                        version = null,
                        deviceName = "",
                        modelName = "",
                    )
                }
            }
            "gps" -> Unit
        }
    }

    private fun parseModes(json: JSONObject): List<ModeItem> {
        val array = json.optJSONArray("modes") ?: return emptyList()
        return buildList {
            for (index in 0 until array.length()) {
                val item = array.optJSONObject(index) ?: continue
                add(ModeItem(code = item.optInt("code"), name = item.optString("name")))
            }
        }
    }

    private fun parseDevices(json: JSONObject): List<DeviceItem> {
        val array = json.optJSONArray("devices") ?: return emptyList()
        return buildList {
            for (index in 0 until array.length()) {
                val item = array.optJSONObject(index) ?: continue
                add(
                    DeviceItem(
                        id = item.optString("id"),
                        name = item.optString("name"),
                        rssi = if (item.isNull("rssi")) null else item.optInt("rssi"),
                    ),
                )
            }
        }
    }

    private fun parseStatus(json: JSONObject) = CameraStatus(
        modeCode = json.optInt("modeCode", -1),
        modeName = json.optString("modeName"),
        stateName = json.optString("stateName"),
        capturing = json.optBoolean("capturing"),
        resolution = json.optString("resolution"),
        fps = json.optString("fps"),
        eis = json.optString("eis"),
        recordSeconds = json.optInt("recordSeconds"),
        remainingMb = json.optInt("remainingMb"),
        remainingPhotos = json.optInt("remainingPhotos"),
        remainingSeconds = json.optInt("remainingSeconds"),
        battery = json.optInt("battery"),
        temperatureCode = json.optInt("temperatureCode"),
        temperatureName = json.optString("temperatureName"),
        hasData = true,
    )
}
