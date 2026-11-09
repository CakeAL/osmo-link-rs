package com.cakeal.osmolink.ui.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.core.graphics.ColorUtils

internal val OsmoLightColors = lightColorScheme(
    primary = Color(0xFF815511),
    onPrimary = Color.White,
    primaryContainer = Color(0xFFFFE2B1),
    onPrimaryContainer = Color(0xFF332000),
    secondary = Color(0xFF6E5B40),
    onSecondary = Color.White,
    secondaryContainer = Color(0xFFF4DFBF),
    onSecondaryContainer = Color(0xFF271A07),
    error = Color(0xFFA62926),
    onError = Color.White,
    errorContainer = Color(0xFFFFDED9),
    onErrorContainer = Color(0xFF75211D),
    surface = Color(0xFFFFF9EF),
    onSurface = Color(0xFF241E17),
    surfaceVariant = Color(0xFFF0E0CC),
    onSurfaceVariant = Color(0xFF55463A),
    surfaceContainerLow = Color(0xFFFBF1E2),
    surfaceContainer = Color(0xFFF5EADA),
    surfaceContainerHigh = Color(0xFFF0E3D2),
    surfaceContainerHighest = Color(0xFFEADDCB),
    outline = Color(0xFF807366),
    outlineVariant = Color(0xFFD5C6B5),
    inverseSurface = Color(0xFF393027),
    inverseOnSurface = Color(0xFFFDEFE0),
    inversePrimary = Color(0xFFF7C36E),
)

internal val OsmoDarkColors = darkColorScheme(
    primary = Color(0xFFF7C36E),
    onPrimary = Color(0xFF432C00),
    primaryContainer = Color(0xFF5D430C),
    onPrimaryContainer = Color(0xFFFFE2B1),
    secondary = Color(0xFFD7C3A4),
    onSecondary = Color(0xFF3B2E18),
    secondaryContainer = Color(0xFF514028),
    onSecondaryContainer = Color(0xFFF4DFBF),
    error = Color(0xFFFFB4A9),
    onError = Color(0xFF681311),
    errorContainer = Color(0xFF542924),
    onErrorContainer = Color(0xFFFFD9D2),
    surface = Color(0xFF19150F),
    onSurface = Color(0xFFF0E7DC),
    surfaceVariant = Color(0xFF4C3E2E),
    onSurfaceVariant = Color(0xFFD5C4B1),
    surfaceContainerLow = Color(0xFF231D14),
    surfaceContainer = Color(0xFF2B241A),
    surfaceContainerHigh = Color(0xFF372F23),
    surfaceContainerHighest = Color(0xFF40372B),
    outline = Color(0xFF9F9080),
    outlineVariant = Color(0xFF5A4E40),
    inverseSurface = Color(0xFFF0E7DC),
    inverseOnSurface = Color(0xFF393027),
    inversePrimary = Color(0xFF815511),
)

internal fun customColors(seed: Color, dark: Boolean): ColorScheme {
    val base = if (dark) OsmoDarkColors else OsmoLightColors
    val neutral = if (dark) Color.Black else Color.White
    val foreground = if (dark) Color.White else Color.Black
    val tintSeed = lerp(seed, foreground, 0.25f)
    fun tint(amount: Float) = Color(ColorUtils.blendARGB(neutral.toArgb(), tintSeed.toArgb(), amount))

    val surface = tint(if (dark) 0.15f else 0.035f)
    val primary = legibleAccent(lerp(seed, foreground, 0.36f), surface, foreground)
    val primaryContainer = tint(if (dark) 0.55f else 0.23f)
    val secondary = lerp(primary, foreground, 0.15f)
    val secondaryContainer = tint(if (dark) 0.39f else 0.14f)
    val onSurface = lerp(foreground, seed, 0.08f)
    val onSurfaceVariant = lerp(foreground, seed, if (dark) 0.24f else 0.36f)
    val container = tint(if (dark) 0.23f else 0.085f)
    val containerHigh = tint(if (dark) 0.27f else 0.11f)
    val containerHighest = tint(if (dark) 0.32f else 0.14f)

    return base.copy(
        primary = primary,
        onPrimary = onColor(primary),
        primaryContainer = primaryContainer,
        onPrimaryContainer = onColor(primaryContainer),
        secondary = secondary,
        onSecondary = onColor(secondary),
        secondaryContainer = secondaryContainer,
        onSecondaryContainer = onColor(secondaryContainer),
        background = surface,
        onBackground = onSurface,
        surface = surface,
        onSurface = onSurface,
        surfaceTint = primary,
        surfaceVariant = containerHighest,
        onSurfaceVariant = onSurfaceVariant,
        surfaceContainerLow = tint(if (dark) 0.19f else 0.055f),
        surfaceContainer = container,
        surfaceContainerHigh = containerHigh,
        surfaceContainerHighest = containerHighest,
        outline = lerp(foreground, surface, 0.42f),
        outlineVariant = lerp(foreground, surface, 0.70f),
        inverseSurface = onSurface,
        inverseOnSurface = surface,
        inversePrimary = legibleAccent(lerp(seed, neutral, 0.34f), onSurface, neutral),
    )
}

private fun onColor(color: Color): Color =
    if (color.luminance() > 0.179f) Color.Black else Color.White

private fun legibleAccent(color: Color, surface: Color, toward: Color): Color {
    val surfaceLuminance = surface.luminance()
    var accent = color
    repeat(8) {
        val luminance = accent.luminance()
        val contrast = (maxOf(luminance, surfaceLuminance) + 0.05f) /
            (minOf(luminance, surfaceLuminance) + 0.05f)
        if (contrast >= 4.5f) return accent
        accent = lerp(accent, toward, 0.25f)
    }
    return toward
}
