package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import kotlin.math.abs

/** One colour in the accent chooser, drawn as a small squared-off swatch. */
@Composable
fun AccentSwatch(
    color: Color,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 30.dp,
) {
    Box(
        modifier = modifier
            .size(size)
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(color)
            .then(
                if (selected) {
                    Modifier.border(2.dp, VynlColors.text, RoundedCornerShape(VynlRadius.control))
                } else {
                    Modifier
                },
            )
            .clickable { onClick() },
        contentAlignment = Alignment.Center,
    ) {
        if (selected) {
            Icon(
                Icons.Rounded.Check,
                contentDescription = null,
                tint = accentTextFor(color),
                modifier = Modifier.size(size * 0.6f),
            )
        }
    }
}

/**
 * The accent chooser: the presets first, then the whole colour space for
 * anyone who wants a shade that is theirs alone.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AccentSheet(
    accent: Color,
    onChanged: (Color) -> Unit,
    onDismiss: () -> Unit,
) {
    val start = hsvOf(accent)
    var value by remember { mutableStateOf(accent) }
    var hue by remember { mutableFloatStateOf(start.first) }
    var saturation by remember { mutableFloatStateOf(start.second) }
    var brightness by remember { mutableFloatStateOf(start.third) }
    var hex by remember { mutableStateOf(hexOf(accent)) }

    fun emit(next: Color) {
        value = next
        hex = hexOf(next)
        onChanged(next)
    }

    fun emitHsv() {
        emit(colorOfHsv(hue, saturation, brightness))
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor = VynlColors.surface,
        contentColor = VynlColors.text,
        tonalElevation = 0.dp,
        dragHandle = null,
        shape = RoundedCornerShape(topStart = VynlRadius.hero, topEnd = VynlRadius.hero),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 20.dp, end = 20.dp, top = 18.dp, bottom = 26.dp),
        ) {
            Text(
                text = "ACCENT COLOR",
                color = VynlColors.faint,
                fontSize = 11.sp,
                fontWeight = FontWeight.W800,
                letterSpacing = 1.3.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(14.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                VynlColors.presets.forEach { preset ->
                    AccentSwatch(
                        color = preset,
                        selected = value == preset,
                        onClick = {
                            val hsv = hsvOf(preset)
                            hue = hsv.first
                            saturation = hsv.second
                            brightness = hsv.third
                            emit(preset)
                        },
                        size = 30.dp,
                    )
                }
            }
            Spacer(Modifier.height(22.dp))
            SaturationValueField(
                hue = hue,
                saturation = saturation,
                brightness = brightness,
                onChange = { s, v ->
                    saturation = s
                    brightness = v
                    emitHsv()
                },
            )
            Spacer(Modifier.height(16.dp))
            HueField(
                hue = hue,
                onChange = { next ->
                    hue = next
                    emitHsv()
                },
            )
            Spacer(Modifier.height(18.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    modifier = Modifier
                        .size(44.dp)
                        .clip(RoundedCornerShape(VynlRadius.control))
                        .background(value)
                        .border(1.dp, VynlColors.line, RoundedCornerShape(VynlRadius.control)),
                )
                Spacer(Modifier.width(12.dp))
                OutlinedTextField(
                    value = hex,
                    onValueChange = { raw ->
                        val cleaned = raw.filter { it.isLetterOrDigit() }.take(6).uppercase()
                        hex = cleaned
                        if (cleaned.length == 6) {
                            parseHex(cleaned)?.let { parsed ->
                                val hsv = hsvOf(parsed)
                                value = parsed
                                hue = hsv.first
                                saturation = hsv.second
                                brightness = hsv.third
                                onChanged(parsed)
                            }
                        }
                    },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    prefix = {
                        Text("#", color = VynlColors.faint, fontSize = 13.sp)
                    },
                    textStyle = TextStyle(
                        color = VynlColors.text,
                        fontSize = 13.sp,
                        letterSpacing = 0.6.sp,
                        fontFamily = VynlFonts.mono,
                    ),
                    keyboardOptions = KeyboardOptions(
                        capitalization = KeyboardCapitalization.Characters,
                        imeAction = ImeAction.Done,
                    ),
                    keyboardActions = KeyboardActions(
                        onDone = { parseHex(hex)?.let { emit(it) } },
                    ),
                    shape = RoundedCornerShape(VynlRadius.control),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedContainerColor = VynlColors.surfaceRaised,
                        unfocusedContainerColor = VynlColors.surfaceRaised,
                        focusedBorderColor = VynlColors.line,
                        unfocusedBorderColor = VynlColors.line,
                        cursorColor = VynlColors.accent,
                        focusedTextColor = VynlColors.text,
                        unfocusedTextColor = VynlColors.text,
                    ),
                )
            }
            Spacer(Modifier.height(20.dp))
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(46.dp)
                    .clip(RoundedCornerShape(VynlRadius.control))
                    .background(VynlColors.accent)
                    .clickable { onDismiss() },
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "Done",
                    color = VynlColors.accentOn,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.W700,
                    fontFamily = VynlFonts.display,
                )
            }
        }
    }
}

@Composable
private fun SaturationValueField(
    hue: Float,
    saturation: Float,
    brightness: Float,
    onChange: (Float, Float) -> Unit,
) {
    val currentHue by rememberUpdatedStateValue(hue)
    val report by rememberUpdatedStateValue(onChange)
    Canvas(
        modifier = Modifier
            .fillMaxWidth()
            .height(132.dp)
            .clip(RoundedCornerShape(VynlRadius.control))
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val send: (Offset) -> Unit = { point ->
                        report(
                            (point.x / size.width).coerceIn(0f, 1f),
                            (1f - point.y / size.height).coerceIn(0f, 1f),
                        )
                    }
                    send(down.position)
                    do {
                        val event = awaitPointerEvent()
                        event.changes.forEach { change ->
                            if (change.pressed) {
                                send(change.position)
                                change.consume()
                            }
                        }
                    } while (event.changes.any { it.pressed })
                }
            },
    ) {
        drawRect(
            brush = Brush.horizontalGradient(
                listOf(Color.White, Color.hsv(currentHue, 1f, 1f)),
            ),
        )
        drawRect(
            brush = Brush.verticalGradient(
                listOf(Color.Transparent, Color.Black),
            ),
        )
        val center = Offset(
            x = saturation * size.width,
            y = (1f - brightness) * size.height,
        )
        val radius = 7.dp.toPx()
        drawCircle(
            color = Color.hsv(currentHue, saturation, brightness),
            radius = radius,
            center = center,
        )
        drawCircle(
            color = Color.White,
            radius = radius,
            center = center,
            style = Stroke(width = 2.dp.toPx()),
        )
    }
}

@Composable
private fun HueField(
    hue: Float,
    onChange: (Float) -> Unit,
) {
    val report by rememberUpdatedStateValue(onChange)
    Canvas(
        modifier = Modifier
            .fillMaxWidth()
            .height(20.dp)
            .clip(RoundedCornerShape(VynlRadius.control))
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val send: (Offset) -> Unit = { point ->
                        report((point.x / size.width).coerceIn(0f, 1f) * 360f)
                    }
                    send(down.position)
                    do {
                        val event = awaitPointerEvent()
                        event.changes.forEach { change ->
                            if (change.pressed) {
                                send(change.position)
                                change.consume()
                            }
                        }
                    } while (event.changes.any { it.pressed })
                }
            },
    ) {
        val stops = (0..12).map { step -> Color.hsv(step * 30f, 1f, 1f) }
        drawRect(brush = Brush.horizontalGradient(stops))
        val thumb = 16.dp.toPx()
        val left = (size.width - thumb) * (hue / 360f)
        drawRoundRect(
            color = Color.hsv(hue, 1f, 1f),
            topLeft = Offset(left, 2.dp.toPx()),
            size = Size(thumb, thumb),
            cornerRadius = CornerRadius(3.dp.toPx()),
        )
        drawRoundRect(
            color = Color.White,
            topLeft = Offset(left, 2.dp.toPx()),
            size = Size(thumb, thumb),
            cornerRadius = CornerRadius(3.dp.toPx()),
            style = Stroke(width = 2.dp.toPx()),
        )
    }
}

/** Compose helper: hold the newest value for pointer loops that outlive a frame. */
@Composable
private fun <T> rememberUpdatedStateValue(value: T): androidx.compose.runtime.State<T> =
    androidx.compose.runtime.rememberUpdatedState(value)

/** Hue in degrees, saturation and value as 0..1. */
fun hsvOf(color: Color): Triple<Float, Float, Float> {
    val max = maxOf(color.red, color.green, color.blue)
    val min = minOf(color.red, color.green, color.blue)
    val delta = max - min
    var hue = 0f
    if (delta > 0.0001f) {
        hue = when (max) {
            color.red -> 60f * (((color.green - color.blue) / delta) % 6f)
            color.green -> 60f * (((color.blue - color.red) / delta) + 2f)
            else -> 60f * (((color.red - color.green) / delta) + 4f)
        }
    }
    if (hue < 0f) hue += 360f
    val saturation = if (max == 0f) 0f else delta / max
    return Triple(hue, saturation, max)
}

/** A colour from hue in degrees and saturation/value in 0..1. */
fun colorOfHsv(hue: Float, saturation: Float, value: Float): Color {
    val h = ((hue % 360f) + 360f) % 360f
    val s = saturation.coerceIn(0f, 1f)
    val v = value.coerceIn(0f, 1f)
    val c = v * s
    val x = c * (1f - abs(((h / 60f) % 2f) - 1f))
    val m = v - c
    val (r, g, b) = when {
        h < 60f -> Triple(c, x, 0f)
        h < 120f -> Triple(x, c, 0f)
        h < 180f -> Triple(0f, c, x)
        h < 240f -> Triple(0f, x, c)
        h < 300f -> Triple(x, 0f, c)
        else -> Triple(c, 0f, x)
    }
    return Color(r + m, g + m, b + m)
}

fun hexOf(color: Color): String {
    val r = (color.red * 255f).toInt().coerceIn(0, 255)
    val g = (color.green * 255f).toInt().coerceIn(0, 255)
    val b = (color.blue * 255f).toInt().coerceIn(0, 255)
    return "%02X%02X%02X".format(r, g, b)
}

fun accentTextFor(color: Color): Color {
    val luminance = 0.2126f * color.red + 0.7152f * color.green + 0.0722f * color.blue
    return if (luminance > 0.55f) VynlColors.accentOn else VynlColors.text
}

private fun parseHex(raw: String): Color? {
    val cleaned = raw.removePrefix("#").trim()
    if (cleaned.length != 6) return null
    val number = cleaned.toLongOrNull(16) ?: return null
    return Color(0xFF000000L or number)
}
