package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

/**
 * The seek bar of the player: a thin line with a small square that sits on
 * the position, and the time on either side.
 */
@Composable
fun VynlSeekBar(
    positionMs: Long,
    durationMs: Long,
    bufferedMs: Long,
    accent: Color,
    onSeek: (Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    var preview by remember { mutableFloatStateOf(-1f) }
    var dragging by remember { mutableStateOf(false) }
    val total = durationMs.coerceAtLeast(1L)
    val live = (positionMs.toFloat() / total.toFloat()).coerceIn(0f, 1f)
    val progress = if (preview >= 0f) preview else live
    val shown = if (preview >= 0f) (preview * total).toLong() else positionMs
    val buffered = (bufferedMs.toFloat() / total.toFloat()).coerceIn(0f, 1f)

    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = formatClock(shown),
            modifier = Modifier.width(42.dp),
            color = VynlColors.dim,
            fontSize = 12.sp,
            fontFamily = VynlFonts.display,
            fontWeight = FontWeight.W500,
        )
        Canvas(
            modifier = Modifier
                .weight(1f)
                .height(30.dp)
                .pointerInput(total) {
                    detectTapGestures { offset ->
                        val fraction = (offset.x / size.width).coerceIn(0f, 1f)
                        onSeek((fraction * total).toLong())
                    }
                }
                .pointerInput(total) {
                    detectHorizontalDragGestures(
                        onDragStart = { offset ->
                            dragging = true
                            preview = (offset.x / size.width).coerceIn(0f, 1f)
                        },
                        onDragEnd = {
                            if (dragging) {
                                val target = (preview.takeIf { it >= 0f } ?: -1f)
                                if (target >= 0f) {
                                    onSeek((target * total).toLong())
                                }
                            }
                            dragging = false
                            preview = -1f
                        },
                        onDragCancel = {
                            dragging = false
                            preview = -1f
                        },
                    ) { change, _ ->
                        preview = (change.position.x / size.width).coerceIn(0f, 1f)
                    }
                },
        ) {
            val trackHeight = 3.dp.toPx()
            val top = (size.height - trackHeight) / 2f
            val radius = androidx.compose.ui.geometry.CornerRadius(VynlRadius.control.toPx())

            drawRoundRect(
                color = VynlColors.text.copy(alpha = 0.20f),
                topLeft = Offset(0f, top),
                size = Size(size.width, trackHeight),
                cornerRadius = radius,
            )
            if (buffered > 0f) {
                drawRoundRect(
                    color = VynlColors.text.copy(alpha = 0.34f),
                    topLeft = Offset(0f, top),
                    size = Size(size.width * buffered, trackHeight),
                    cornerRadius = radius,
                )
            }
            if (progress > 0f) {
                drawRoundRect(
                    color = accent,
                    topLeft = Offset(0f, top),
                    size = Size(size.width * progress, trackHeight),
                    cornerRadius = radius,
                )
            }

            val side = if (dragging) 13.dp.toPx() else 9.dp.toPx()
            val center = Offset(size.width * progress, size.height / 2f)
            val glowSide = side + 10.dp.toPx()
            rotate(degrees = 45f, pivot = center) {
                drawRoundRect(
                    color = accent.copy(alpha = if (dragging) 0.34f else 0.2f),
                    topLeft = Offset(center.x - glowSide / 2f, center.y - glowSide / 2f),
                    size = Size(glowSide, glowSide),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(3.dp.toPx()),
                )
                drawRoundRect(
                    color = accent,
                    topLeft = Offset(center.x - side / 2f, center.y - side / 2f),
                    size = Size(side, side),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(2.dp.toPx()),
                )
            }
        }
        Text(
            text = formatClock(durationMs),
            modifier = Modifier
                .width(42.dp)
                .padding(start = 11.dp),
            color = VynlColors.dim,
            fontSize = 12.sp,
            fontFamily = VynlFonts.display,
            fontWeight = FontWeight.W500,
        )
    }
}

internal fun formatClock(milliseconds: Long): String {
    val total = (milliseconds / 1000).coerceAtLeast(0)
    val minutes = (total / 60) % 60
    val seconds = total % 60
    return "$minutes:${seconds.toString().padStart(2, '0')}"
}
