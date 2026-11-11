package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp

@Composable
internal fun CameraHeroSurface(content: @Composable () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val base = colors.primaryContainer
    val foreground = colors.onPrimaryContainer
    // Shade away from the foreground luminance to preserve the theme's text contrast.
    val neutral = if (foreground.luminance() > base.luminance()) Color.Black else Color.White
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.extraLarge,
        color = base,
        contentColor = foreground,
    ) {
        Box(
            modifier = Modifier.fillMaxWidth().drawWithCache {
                val fromLeft = layoutDirection == LayoutDirection.Ltr
                val start = if (fromLeft) 0f else size.width
                val end = if (fromLeft) size.width else 0f
                val inward = if (fromLeft) -1 else 1
                val gradient = Brush.linearGradient(
                    colors = listOf(base, lerp(base, neutral, 0.42f)),
                    start = Offset(start, 0f),
                    end = Offset(end, size.height),
                )
                val circleCenter = Offset(end + inward * 18.dp.toPx(), 8.dp.toPx())
                val ringCenter = Offset(end + inward * 6.dp.toPx(), 94.dp.toPx())
                val circleRadius = 80.dp.toPx()
                val ringRadius = 57.dp.toPx()
                val ringStroke = Stroke(1.dp.toPx())

                onDrawBehind {
                    drawRect(gradient)
                    drawCircle(neutral.copy(alpha = 0.10f), circleRadius, circleCenter)
                    drawCircle(neutral.copy(alpha = 0.24f), ringRadius, ringCenter, style = ringStroke)
                }
            },
        ) {
            content()
        }
    }
}
