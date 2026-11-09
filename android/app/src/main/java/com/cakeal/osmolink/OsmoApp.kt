package com.cakeal.osmolink

import android.app.Application
import com.cakeal.osmolink.data.AppPreferences
import com.cakeal.osmolink.data.OsmoRepository

class OsmoApp : Application() {
    internal val preferences by lazy { AppPreferences(this) }

    override fun onCreate() {
        super.onCreate()
        // One bridge for the whole process; the activity and service share it.
        OsmoRepository.attach(this)
    }
}
