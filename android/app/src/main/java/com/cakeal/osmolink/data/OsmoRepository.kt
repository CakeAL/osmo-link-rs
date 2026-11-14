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
import com.cakeal.osmolink.service.BatteryAlertNotifier
import com.cakeal.osmolink.service.OsmoService
import com.cakeal.osmolink.service.TemperatureAlertNotifier
import com.cakeal.osmolink.ui.modeLabel
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
    private var defaultGpsPending = false
    private val fixTimes = ArrayDeque<Long>()
    private val events = EventLog()
    @Volatile
    private var lowBatteryAlertSent = false
    @Volatile
    private var lastTemperatureCode = 0

    /** Called once from [com.cakeal.osmolink.OsmoApp]. */
    fun attach(application: Context) {
        if (attached) return
        attached = true
        context = application.applicationContext
        if (!OsmoNative.nativeInit(context.filesDir.absolutePath, this)) {
            update { it.copy(message = "原生蓝牙初始化失败") }
        }
    }

    fun scan(autoConnect: Boolean) {
        val current = _state.value
        if (current.scanning || current.connected || current.connecting) return
        update { it.copy(scanning = true, hasSearched = true, devices = emptyList(), message = null) }
        if (!OsmoNative.nativeScan(12, autoConnect)) {
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
                reconnectAttempt = null,
                status = CameraStatus(),
                modes = emptyList(),
                version = null,
                modeInfo = null,
                deviceName = "",
                modelName = "",
                devices = emptyList(),
            )
        }
    }

    fun setMode(code: Int) {
        val current = _state.value
        if (!current.connected || current.pendingAction != null || current.reconnectAttempt != null ||
            !current.status.hasData || current.status.capturing || current.status.modeCode == code ||
            current.modes.none { it.code == code }
        ) return
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

    @Synchronized
    fun startDefaultGps() {
        if (defaultGpsPending && hasPreciseLocation()) {
            setGps(true)
        }
    }

    @Synchronized
    fun setGps(enabled: Boolean) {
        defaultGpsPending = false
        if (!enabled) {
            stopGps()
            return
        }
        if (!_state.value.connected || gps != null) return
        if (!hasPreciseLocation()) {
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
            val recent = record("定位注入已开启")
            update { it.copy(gpsEnabled = true, recentEvents = recent) }
        } else {
            tracker.stop()
            update { it.copy(message = "无法启动定位，请确认 GPS 已开启") }
        }
    }

    private fun hasPreciseLocation(): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED

    @Synchronized
    fun stopGps() {
        defaultGpsPending = false
        gps?.stop()
        gps = null
        OsmoNative.nativeGpsStop()
        fixTimes.clear()
        val recent = if (_state.value.gpsEnabled) record("定位注入已暂停") else null
        update {
            it.copy(
                gpsEnabled = false,
                gpsRate = 0.0,
                gpsAccuracy = null,
                gpsSatellites = 0,
                recentEvents = recent ?: it.recentEvents,
            )
        }
    }

    fun notify(message: String) {
        update { it.copy(message = message) }
    }

    fun clearMessage() {
        update { it.copy(message = null) }
    }

    @Synchronized
    private fun stopGpsInternal() {
        defaultGpsPending = false
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
            "devices" -> update { it.copy(scanning = false, devices = parseDevices(json)) }
            "pairing" -> update {
                it.copy(
                    connecting = true,
                    pairing = PairingInfo(
                        code = json.optInt("code"),
                        device = json.optString("device"),
                        firstTime = json.optBoolean("firstTime"),
                        auto = json.optBoolean("auto"),
                    ),
                )
            }
            "connected" -> {
                lowBatteryAlertSent = false
                lastTemperatureCode = 0
                val name = json.optString("name").ifBlank { "Osmo 相机" }
                val recent = record("已连接 $name")
                update {
                    defaultGpsPending = true
                    it.copy(
                        connected = true,
                        connecting = false,
                        pairing = null,
                        devices = emptyList(),
                        deviceName = json.optString("name"),
                        modelName = json.optString("modelName"),
                        modes = parseModes(json),
                        recentEvents = recent,
                        message = null,
                    )
                }
            }
            "connectFailed" -> {
                val detail = json.optString("message")
                val auto = json.optBoolean("auto")
                // Auto-connect misses on every scan when no paired camera is
                // nearby; only user-initiated failures belong in the feed.
                val recent = if (auto) null else record("连接失败：${detail.ifBlank { "未知原因" }}")
                update {
                    it.copy(
                        connecting = false,
                        pairing = null,
                        message = if (auto) {
                            "自动连接失败：$detail"
                        } else {
                            "连接失败：$detail"
                        },
                        recentEvents = recent ?: it.recentEvents,
                    )
                }
            }
            "status" -> {
                val status = parseStatus(json)
                val previous = _state.value.status
                val lines = buildList {
                    if (previous.hasData && previous.modeCode != status.modeCode) {
                        add("相机模式切换至${modeLabel(status.modeName)} (0x${status.modeCode.toString(16).uppercase()})")
                    }
                    if (previous.hasData && status.temperatureCode >= 2 &&
                        status.temperatureCode > previous.temperatureCode
                    ) {
                        add(temperatureEventText(status.temperatureCode))
                    }
                }
                val recent = if (lines.isEmpty()) null else record(*lines.toTypedArray())
                update { current ->
                    current.copy(
                        status = status,
                        modeInfo = current.modeInfo.takeIf { current.status.modeCode == status.modeCode },
                        recentEvents = recent ?: current.recentEvents,
                    )
                }
                checkLowBattery(status.battery)
                checkTemperature(status.temperatureCode)
            }
            "modeInfo" -> update {
                val info = listOf(json.optString("name"), json.optString("parameters"))
                    .filter { value -> value.isNotBlank() }
                    .joinToString(" · ")
                it.copy(modeInfo = info.ifBlank { null })
            }
            "version" -> {
                val product = json.optString("product")
                val details = json.optString("details")
                val text = listOf(product, details).filter { value -> value.isNotBlank() }
                update { it.copy(version = text.joinToString(" · ").ifBlank { null }) }
            }
            "command" -> {
                val action = json.optString("action")
                val ok = json.optBoolean("ok")
                val label = commandLabel(action)
                val event = when {
                    ok && action == "record.start" -> "开始录像"
                    ok && action == "record.stop" -> "停止录像"
                    ok && action == "shutter" -> "已拍照"
                    // Mode changes are reported by the following status push.
                    ok -> null
                    else -> "操作失败：$label"
                }
                val recent = event?.let { record(it) }
                update { current ->
                    val status = when {
                        action == "record.start" && ok -> current.status.copy(capturing = true)
                        action == "record.stop" && ok -> current.status.copy(capturing = false)
                        else -> current.status
                    }
                    current.copy(
                        pendingAction = null,
                        status = status,
                        recentEvents = recent ?: current.recentEvents,
                        message = if (ok) {
                            null
                        } else {
                            json.optString("error").ifBlank { "操作失败：$label" }
                        },
                    )
                }
            }
            "warning", "error", "gpsError" ->
                update { it.copy(message = json.optString("message")) }
            "reconnecting" -> {
                val attempt = json.optInt("attempt")
                val text = "连接中断，正在重连（第 $attempt 次）"
                val recent = record(text)
                update { it.copy(reconnectAttempt = attempt, message = text, recentEvents = recent) }
            }
            "reconnected" -> {
                val recent = record("已重新连接")
                update { it.copy(reconnectAttempt = null, message = "已重新连接", recentEvents = recent) }
            }
            "disconnected" -> {
                lowBatteryAlertSent = false
                lastTemperatureCode = 0
                stopGpsInternal()
                val recent = record("已断开连接")
                update {
                    it.copy(
                        connected = false,
                        connecting = false,
                        pairing = null,
                        pendingAction = null,
                        reconnectAttempt = null,
                        status = CameraStatus(),
                        modes = emptyList(),
                        version = null,
                        modeInfo = null,
                        deviceName = "",
                        modelName = "",
                        recentEvents = recent,
                    )
                }
            }
            "gps" -> Unit
        }
    }

    /** Alerts once when the battery crosses below the configured threshold. */
    @Synchronized
    private fun checkLowBattery(battery: Int) {
        if (!attached) return
        val settings = BatteryAlertSettings.load(context)
        if (!settings.enabled) return
        if (battery >= settings.threshold) {
            lowBatteryAlertSent = false
            return
        }
        if (!BatteryAlertNotifier.shouldAlert(battery, settings.threshold, lowBatteryAlertSent)) return
        lowBatteryAlertSent = true
        val name = _state.value.deviceName.ifBlank { "Osmo 相机" }
        BatteryAlertNotifier.notify(context, name, battery, settings.threshold)
        val recent = record("相机电量低：$battery%")
        update { it.copy(recentEvents = recent) }
    }

    /**
     * Alerts once per escalation into a recording-stopping temperature level.
     *
     * The level is remembered even while alerts are off so turning the
     * setting on does not replay a warning the camera already reported.
     */
    @Synchronized
    private fun checkTemperature(code: Int) {
        if (!attached) return
        val previous = lastTemperatureCode
        lastTemperatureCode = code
        if (!TemperatureAlertSettings.load(context).enabled) return
        if (!TemperatureAlertNotifier.shouldAlert(previous, code)) return
        val name = _state.value.deviceName.ifBlank { "Osmo 相机" }
        TemperatureAlertNotifier.notify(context, name, code)
    }

    /** Append lines to the in-memory activity feed and return the new snapshot. */
    private fun record(vararg lines: String): List<RecentEvent> {
        for (line in lines) events.add(line)
        return events.snapshot()
    }

    private fun temperatureEventText(code: Int): String =
        if (code >= 3) "相机温度过高，即将关机" else "相机温度过高，无法录制"

    private fun commandLabel(action: String): String = when (action) {
        "record.start" -> "开始录像"
        "record.stop" -> "停止录像"
        "shutter" -> "拍照"
        "mode" -> "切换模式"
        else -> action
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
