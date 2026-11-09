package com.cakeal.osmolink

import androidx.lifecycle.ViewModel
import com.cakeal.osmolink.data.OsmoRepository
import com.cakeal.osmolink.data.UiState
import kotlinx.coroutines.flow.StateFlow

/**
 * Thin activity-scoped facade over [OsmoRepository]. The bridge and GPS live in
 * the process-wide repository so they survive the UI.
 */
class ControllerViewModel : ViewModel() {
    val state: StateFlow<UiState> = OsmoRepository.state

    fun scan() = OsmoRepository.scan()

    fun connect(index: Int) = OsmoRepository.connect(index)

    fun disconnect() = OsmoRepository.disconnect()

    fun setMode(code: Int) = OsmoRepository.setMode(code)

    fun toggleRecord() = OsmoRepository.toggleRecord()

    fun shutter() = OsmoRepository.shutter()

    fun startDefaultGps() = OsmoRepository.startDefaultGps()

    fun setGps(enabled: Boolean) = OsmoRepository.setGps(enabled)

    fun notify(message: String) = OsmoRepository.notify(message)

    fun clearMessage() = OsmoRepository.clearMessage()
}
