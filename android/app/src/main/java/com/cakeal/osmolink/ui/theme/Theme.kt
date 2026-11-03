package com.cakeal.osmolink.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

private val OsmoColorScheme = darkColorScheme(
    primary = OsmoPrimary,
    onPrimary = OsmoOnPrimary,
    primaryContainer = OsmoPrimaryContainer,
    onPrimaryContainer = OsmoOnPrimaryContainer,
    secondary = OsmoSecondary,
    onSecondary = OsmoOnSecondary,
    secondaryContainer = OsmoSecondaryContainer,
    onSecondaryContainer = OsmoOnSecondaryContainer,
    tertiary = OsmoSecondary,
    onTertiary = OsmoOnSecondary,
    error = OsmoError,
    onError = OsmoOnError,
    errorContainer = OsmoErrorContainer,
    onErrorContainer = OsmoOnErrorContainer,
    background = OsmoBackground,
    onBackground = OsmoOnSurface,
    surface = OsmoBackground,
    onSurface = OsmoOnSurface,
    surfaceVariant = OsmoSurfaceContainerHigh,
    onSurfaceVariant = OsmoOnSurfaceVariant,
    surfaceTint = OsmoPrimary,
    surfaceDim = OsmoBackground,
    surfaceBright = OsmoSurfaceContainerHighest,
    surfaceContainerLowest = OsmoSurfaceContainerLowest,
    surfaceContainerLow = OsmoSurfaceContainerLow,
    surfaceContainer = OsmoSurfaceContainer,
    surfaceContainerHigh = OsmoSurfaceContainerHigh,
    surfaceContainerHighest = OsmoSurfaceContainerHighest,
    inverseSurface = OsmoOnSurface,
    inverseOnSurface = OsmoBackground,
    inversePrimary = Color(0xFF7A5200),
    outline = OsmoOutline,
    outlineVariant = OsmoOutlineVariant,
    scrim = Color.Black,
)

private val OsmoShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(28.dp),
)

@Composable
fun OsmoLinkTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = OsmoColorScheme,
        typography = OsmoTypography,
        shapes = OsmoShapes,
        content = content,
    )
}
