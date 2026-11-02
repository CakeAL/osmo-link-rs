package com.cakeal.osmolink.location

import android.content.Context
import android.location.GnssStatus
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import kotlin.math.cos
import kotlin.math.sin

/**
 * Feeds `GpsSample` JSON to the rust bridge at the system's best-effort rate
 * (requested 100ms). Android does not guarantee 10Hz; the UI shows the rate
 * actually received instead of assuming it.
 *
 * Samples without satellites in fix are dropped: `osmo_core` rejects them and
 * a fix without a satellite count is not a real fix.
 */
class GpsTracker(
    context: Context,
    private val onFix: (sampleJson: String, accuracyMeters: Double, satellites: Int) -> Unit,
) {
    private val manager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    private var running = false
    private var satellites = 0

    private val gnssCallback = object : GnssStatus.Callback() {
        override fun onSatelliteStatusChanged(status: GnssStatus) {
            var used = 0
            for (index in 0 until status.satelliteCount) {
                if (status.usedInFix(index)) used++
            }
            satellites = used
        }
    }

    private val locationListener = object : LocationListener {
        override fun onLocationChanged(location: Location) {
            emit(location)
        }

        @Deprecated("Deprecated in Java")
        override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) = Unit

        override fun onProviderEnabled(provider: String) = Unit

        override fun onProviderDisabled(provider: String) = Unit
    }

    @Suppress("DEPRECATION")
    fun start(): Boolean {
        val manager = manager ?: return false
        if (running) return true
        val provider = LocationManager.GPS_PROVIDER
        if (!manager.isProviderEnabled(provider)) return false
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                manager.registerGnssStatusCallback(gnssCallback, Handler(Looper.getMainLooper()))
            } else {
                // The Handler overload is API 30+; start() always runs on the main thread.
                manager.registerGnssStatusCallback(gnssCallback)
            }
            manager.requestLocationUpdates(
                provider,
                100L,
                0f,
                locationListener,
                Looper.getMainLooper(),
            )
            running = true
            true
        } catch (_: SecurityException) {
            false
        } catch (_: IllegalArgumentException) {
            false
        }
    }

    fun stop() {
        val manager = manager
        if (running && manager != null) {
            try {
                manager.removeUpdates(locationListener)
            } catch (_: SecurityException) {
                // Permission can be revoked while tracking.
            }
            try {
                manager.unregisterGnssStatusCallback(gnssCallback)
            } catch (_: SecurityException) {
                // Same as above.
            }
        }
        running = false
        satellites = 0
    }

    private fun emit(location: Location) {
        if (satellites <= 0) return
        val speed = if (location.hasSpeed()) location.speed.toDouble() else 0.0
        val bearing = if (location.hasBearing()) Math.toRadians(location.bearing.toDouble()) else 0.0
        val sample = JSONObject().apply {
            put("timestamp_ms", location.time)
            put("longitude", location.longitude)
            put("latitude", location.latitude)
            put("altitude_m", if (location.hasAltitude()) location.altitude else 0.0)
            put("north_mps", speed * cos(bearing))
            put("east_mps", speed * sin(bearing))
            put("down_mps", 0.0)
            put(
                "vertical_accuracy_m",
                if (location.hasVerticalAccuracy()) location.verticalAccuracyMeters.toDouble() else 0.0,
            )
            put("horizontal_accuracy_m", location.accuracy.toDouble())
            put(
                "speed_accuracy_mps",
                if (location.hasSpeedAccuracy()) location.speedAccuracyMetersPerSecond.toDouble() else 0.0,
            )
            put("satellites", satellites)
        }
        onFix(sample.toString(), location.accuracy.toDouble(), satellites)
    }
}
