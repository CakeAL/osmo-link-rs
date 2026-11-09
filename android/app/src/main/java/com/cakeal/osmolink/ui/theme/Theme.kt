package com.cakeal.osmolink.ui.theme

import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

@Composable
internal fun OsmoLinkTheme(darkTheme: Boolean, accent: Int?, content: @Composable () -> Unit) {
    val context = LocalContext.current
    val colors = when {
        accent != null -> remember(accent, darkTheme) { customColors(Color(accent), darkTheme) }
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && darkTheme -> dynamicDarkColorScheme(context)
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> dynamicLightColorScheme(context)
        darkTheme -> OsmoDarkColors
        else -> OsmoLightColors
    }
    MaterialTheme(colorScheme = colors, typography = OsmoTypography, content = content)
}
