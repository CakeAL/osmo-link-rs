package com.cakeal.osmolink.bridge

/**
 * Receives every asynchronous result from the rust bridge. Callbacks arrive on
 * a rust worker thread, so implementations must be thread-safe.
 */
interface BridgeListener {
    fun onEvent(type: String, payload: String)
}
