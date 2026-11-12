package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import com.vynl.vynl_mobile.ui.theme.VynlColors

/** The colours the app offers, and a slider for one of your own. */
@Composable
fun AccentPicker(
    accent: Color,
    onPick: (Color) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        VynlColors.presets.forEach { preset ->
            val chosen = preset == accent
            Box(
                modifier = Modifier
                    .size(30.dp)
                    .clip(CircleShape)
                    .background(preset)
                    .then(
                        if (chosen) {
                            Modifier.border(2.dp, VynlColors.text, CircleShape)
                        } else {
                            Modifier
                        },
                    )
                    .clickable { onPick(preset) },
            )
        }
    }
}

/** A bar of hues, so the accent can be any colour at all. */
@Composable
fun HueSlider(
    hue: Float,
    onHue: (Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    var position by remember(hue) { mutableFloatStateOf(hue) }

    Canvas(
        modifier = modifier
            .fillMaxWidth()
            .height(28.dp)
            .pointerInput(Unit) {
                detectHorizontalDragGestures(
                    onDragStart = { offset ->
                        position = (offset.x / size.width).coerceIn(0f, 1f)
                        onHue(position)
                    },
                ) { change, _ ->
                    position = (change.position.x / size.width).coerceIn(0f, 1f)
                    onHue(position)
                }
            },
    ) {
        val height = size.height
        val thumb = 20.dp.toPx()
        drawRect(
            brush = Brush.horizontalGradient(
                listOf(
                    Color.Red,
                    Color.Yellow,
                    Color.Green,
                    Color.Cyan,
                    Color.Blue,
                    Color.Magenta,
                    Color.Red,
                ),
            ),
            topLeft = androidx.compose.ui.geometry.Offset(0f, (height - 8.dp.toPx()) / 2f),
            size = androidx.compose.ui.geometry.Size(size.width, 8.dp.toPx()),
        )
        val x = (position * size.width).coerceIn(thumb / 2f, size.width - thumb / 2f)
        drawCircle(
            color = Color.hsv(hue * 360f, 1f, 1f),
            radius = thumb / 2f,
            center = androidx.compose.ui.geometry.Offset(x, height / 2f),
        )
        drawCircle(
            color = VynlColors.text,
            radius = thumb / 2f,
            center = androidx.compose.ui.geometry.Offset(x, height / 2f),
            style = androidx.compose.ui.graphics.drawscope.Stroke(width = 2.dp.toPx()),
        )
    }
}
