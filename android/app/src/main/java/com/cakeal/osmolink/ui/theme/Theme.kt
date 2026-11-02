package com.cakeal.osmolink.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val OsmoColorScheme = darkColorScheme(
    primary = OsmoAmber,
    onPrimary = OsmoOnAmber,
    secondary = OsmoGreen,
    onSecondary = OsmoOnGreen,
    error = OsmoRed,
    onError = OsmoText,
    background = OsmoBackground,
    onBackground = OsmoText,
    surface = OsmoSurface,
    onSurface = OsmoText,
    surfaceVariant = OsmoSurfaceHigh,
    onSurfaceVariant = OsmoMuted,
    outline = OsmoOutline,
    outlineVariant = OsmoOutline,
)

@Composable
fun OsmoLinkTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = OsmoColorScheme,
        typography = OsmoTypography,
        content = content,
    )
}
