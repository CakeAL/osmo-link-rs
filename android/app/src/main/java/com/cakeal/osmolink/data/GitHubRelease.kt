package com.cakeal.osmolink.data

import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

internal sealed interface UpdateCheckResult {
    data object NoRelease : UpdateCheckResult
    data object UpToDate : UpdateCheckResult
    data class Available(val tag: String, val notes: String, val pageUrl: String) : UpdateCheckResult
}

private const val LATEST_RELEASE = "https://api.github.com/repos/CakeAL/osmo-link-rs/releases/latest"
private const val RELEASE_PAGE = "https://github.com/CakeAL/osmo-link-rs/releases/tag/"
private val releaseVersion = Regex("[vV]?([0-9]+)\\.([0-9]+)(?:\\.([0-9]+))?")

internal suspend fun checkForUpdate(installedVersion: String): UpdateCheckResult =
    withContext(Dispatchers.IO) {
        val connection = URL(LATEST_RELEASE).openConnection() as HttpURLConnection
        try {
            connection.connectTimeout = 10_000
            connection.readTimeout = 10_000
            connection.setRequestProperty("Accept", "application/vnd.github+json")
            when (val status = connection.responseCode) {
                HttpURLConnection.HTTP_NOT_FOUND -> UpdateCheckResult.NoRelease
                HttpURLConnection.HTTP_OK -> {
                    val release = connection.inputStream.bufferedReader(Charsets.UTF_8).use {
                        JSONObject(it.readText())
                    }
                    val tag = release.getString("tag_name")
                    if (isNewerRelease(tag, installedVersion)) {
                        val pageUrl = release.getString("html_url")
                        if (!pageUrl.startsWith(RELEASE_PAGE, ignoreCase = true)) {
                            throw IOException("Invalid GitHub release URL")
                        }
                        val notes = if (release.isNull("body")) "" else release.getString("body").trim()
                        UpdateCheckResult.Available(tag, notes, pageUrl)
                    } else {
                        UpdateCheckResult.UpToDate
                    }
                }
                else -> throw IOException("GitHub release request returned HTTP $status")
            }
        } finally {
            connection.disconnect()
        }
    }

internal fun isNewerRelease(tag: String, installedVersion: String): Boolean {
    val latest = versionParts(tag)
    val installed = versionParts(installedVersion)
    for ((newPart, installedPart) in latest.zip(installed)) {
        if (newPart != installedPart) return newPart > installedPart
    }
    return false
}

private fun versionParts(version: String): List<Int> {
    val match = releaseVersion.matchEntire(version) ?: throw IOException("Invalid release version: $version")
    return match.groupValues.drop(1).map { part ->
        if (part.isEmpty()) 0 else part.toIntOrNull() ?: throw IOException("Invalid release version: $version")
    }
}
