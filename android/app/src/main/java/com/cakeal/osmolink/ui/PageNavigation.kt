package com.cakeal.osmolink.ui

import androidx.activity.BackEventCompat
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.hideFromAccessibility
import androidx.compose.ui.unit.LayoutDirection
import com.arkivanov.decompose.ExperimentalDecomposeApi
import com.arkivanov.decompose.extensions.compose.stack.animation.predictiveback.PredictiveBackAnimatable
import com.arkivanov.decompose.extensions.compose.stack.animation.predictiveback.androidPredictiveBackAnimatableV1
import com.arkivanov.essenty.backhandler.BackEvent
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

internal enum class Page(val parent: Page? = null) {
    Remote,
    Settings(Remote),
    About(Settings),
}

private data class PageLayers(val behind: Page, val front: Page)

@OptIn(ExperimentalDecomposeApi::class)
@Composable
internal fun PageNavigation(
    contentInsets: PaddingValues,
    content: @Composable (Page, (Page) -> Unit) -> Unit,
) {
    var page by rememberSaveable { mutableStateOf(Page.Remote) }
    var layers by remember { mutableStateOf<PageLayers?>(null) }
    var offset by remember { mutableFloatStateOf(0f) }
    var preview by remember { mutableStateOf<PredictiveBackAnimatable?>(null) }
    var settleJob by remember { mutableStateOf<Job?>(null) }
    val pages = rememberSaveableStateHolder()
    val scope = rememberCoroutineScope()
    val trailingEdge = if (LocalLayoutDirection.current == LayoutDirection.Ltr) 1f else -1f

    fun settle(animation: suspend () -> Unit) {
        settleJob?.cancel()
        settleJob = scope.launch {
            animation()
            preview = null
            layers = null
        }
    }

    fun navigateTo(destination: Page) {
        if (destination == page) return
        val returning = destination == page.parent
        preview = null
        layers = if (returning) PageLayers(destination, page) else PageLayers(page, destination)
        offset = if (returning) 0f else trailingEdge
        page = destination
        settle {
            val target = if (returning) trailingEdge else 0f
            animate(offset, target, animationSpec = tween(240, easing = FastOutSlowInEasing)) { value, _ ->
                offset = value
            }
        }
    }

    PredictiveBackHandler(enabled = page.parent != null) { events ->
        val origin = page
        val destination = origin.parent ?: return@PredictiveBackHandler
        var gesture: PredictiveBackAnimatable? = null
        try {
            events.collect { event ->
                if (page == origin) {
                    val backEvent = event.toDecomposeEvent()
                    val animatable = gesture ?: androidPredictiveBackAnimatableV1(backEvent).also {
                        settleJob?.cancel()
                        layers = PageLayers(destination, origin)
                        preview = it
                        gesture = it
                    }
                    // Decompose replaces its running spring as new progress arrives.
                    scope.launch(start = CoroutineStart.UNDISPATCHED) {
                        animatable.animate(backEvent)
                    }
                }
            }
            if (page == origin) {
                val animatable = gesture
                if (animatable != null) {
                    page = destination
                    settle { animatable.finish() }
                } else {
                    navigateTo(destination)
                }
            }
        } catch (cancelled: CancellationException) {
            val animatable = gesture
            if (animatable != null && preview === animatable && page == origin) {
                settle { animatable.cancel() }
            }
            throw cancelled
        }
    }

    val animatable = preview
    val colors = MaterialTheme.colorScheme
    Box(
        Modifier.fillMaxSize().clipToBounds()
            .background(if (animatable == null) colors.surface else colors.surfaceDim),
    ) {
        val visible = layers
        for (visiblePage in listOfNotNull(visible?.behind, visible?.front ?: page)) {
            key(visiblePage) {
                val front = visiblePage == visible?.front
                val motion = when {
                    animatable != null -> if (front) animatable.exitModifier else animatable.enterModifier
                    front -> Modifier.graphicsLayer { translationX = offset * size.width }
                    else -> Modifier
                }
                val input = if (visiblePage != page) {
                    Modifier.clearAndSetSemantics { hideFromAccessibility() }.pointerInput(Unit) {
                        awaitPointerEventScope {
                            while (true) {
                                awaitPointerEvent(PointerEventPass.Initial).changes.forEach { it.consume() }
                            }
                        }
                    }
                } else {
                    Modifier
                }
                Box(
                    Modifier.fillMaxSize().then(motion).background(colors.surface).then(input)
                        .padding(contentInsets).consumeWindowInsets(contentInsets),
                ) {
                    pages.SaveableStateProvider(visiblePage.name) {
                        content(visiblePage) { destination ->
                            if (visiblePage == page) navigateTo(destination)
                        }
                    }
                }
            }
        }
    }
}

private fun BackEventCompat.toDecomposeEvent() = BackEvent(
    progress = progress,
    touchX = touchX,
    touchY = touchY,
    swipeEdge = when (swipeEdge) {
        BackEventCompat.EDGE_LEFT -> BackEvent.SwipeEdge.LEFT
        BackEventCompat.EDGE_RIGHT -> BackEvent.SwipeEdge.RIGHT
        else -> BackEvent.SwipeEdge.UNKNOWN
    },
)
