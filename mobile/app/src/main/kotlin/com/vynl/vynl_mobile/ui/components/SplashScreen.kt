package com.vynl.vynl_mobile.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.vynl.vynl_mobile.R
import com.vynl.vynl_mobile.ui.theme.VynlColors
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Shown at launch: the logo fades and scales in, then the whole splash
 * fades up and away, revealing the app underneath.
 */
@Composable
fun SplashScreen(onExited: () -> Unit) {
    val enterEasing = CubicBezierEasing(0.16f, 1f, 0.3f, 1f)
    val exitEasing = CubicBezierEasing(0.4f, 0f, 0.2f, 1f)
    val alpha = remember { Animatable(0f) }
    val scale = remember { Animatable(0.5f) }
    val turn = remember { Animatable(-10f) }
    val exit = remember { Animatable(0f) }
    var exiting by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        launch { alpha.animateTo(1f, tween(900, delayMillis = 120, easing = enterEasing)) }
        launch { scale.animateTo(1f, tween(900, delayMillis = 120, easing = enterEasing)) }
        launch { turn.animateTo(0f, tween(900, delayMillis = 120, easing = enterEasing)) }
        delay(1_520)
        exiting = true
        exit.animateTo(1f, tween(400, easing = exitEasing))
        onExited()
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg),
        contentAlignment = Alignment.Center,
    ) {
        Image(
            painter = painterResource(R.mipmap.ic_launcher_foreground),
            contentDescription = "Vynl",
            contentScale = ContentScale.Fit,
            modifier = Modifier
                .size(160.dp)
                .graphicsLayer {
                    this.alpha = alpha.value * (1f - exit.value)
                    val grow = 1f + 0.05f * exit.value
                    scaleX = scale.value * grow
                    scaleY = scale.value * grow
                    rotationZ = turn.value
                },
        )
    }
}
