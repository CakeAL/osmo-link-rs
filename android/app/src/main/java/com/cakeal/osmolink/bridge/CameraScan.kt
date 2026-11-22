package com.cakeal.osmolink.bridge

import android.annotation.SuppressLint
import android.bluetooth.BluetoothManager
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context

internal class CameraScan(private val context: Context) {
    // Callbacks are delivered on the main thread while Rust starts and stops the scan.
    private var active: Pair<BluetoothLeScanner, ScanCallback>? = null

    @Synchronized
    @SuppressLint("MissingPermission")
    fun setScanning(enabled: Boolean): String? {
        return try {
            if (enabled) {
                if (active != null) return "已有蓝牙搜索正在进行"
                val scanner = context.getSystemService(BluetoothManager::class.java)
                    ?.adapter?.bluetoothLeScanner ?: return "蓝牙未开启或不可用"
                val callback = object : ScanCallback() {
                    override fun onScanResult(callbackType: Int, result: ScanResult) {
                        synchronized(this@CameraScan) {
                            val hasDjiData = result.scanRecord?.getManufacturerSpecificData(0x08aa) != null
                            if (active?.second === this && hasDjiData) {
                                OsmoNative.nativeScanResult(result)
                            }
                        }
                    }

                    override fun onScanFailed(errorCode: Int) {
                        synchronized(this@CameraScan) {
                            if (active?.second === this) OsmoNative.nativeScanFailed(errorCode)
                        }
                    }
                }
                val settings = ScanSettings.Builder()
                    .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                    .setLegacy(false)
                    .build()
                scanner.startScan(emptyList(), settings, callback)
                active = scanner to callback
            } else {
                val (scanner, callback) = active ?: return null
                active = null
                scanner.stopScan(callback)
            }
            null
        } catch (_: SecurityException) {
            "蓝牙权限不足，请允许访问附近设备"
        } catch (error: IllegalStateException) {
            error.message ?: "蓝牙扫描不可用"
        }
    }
}
