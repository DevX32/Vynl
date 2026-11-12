package com.vynl.vynl_mobile.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.InlineTextContent
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.withFrameNanos
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.Placeholder
import androidx.compose.ui.text.PlaceholderVerticalAlign
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.data.LyricLine
import com.vynl.vynl_mobile.data.Lyrics
import com.vynl.vynl_mobile.data.LyricsStatus
import com.vynl.vynl_mobile.data.isInstrumental
import com.vynl.vynl_mobile.data.stanzasOf
import com.vynl.vynl_mobile.data.wordFill
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import kotlinx.coroutines.delay

/**
 * The lyrics of the song that is playing, shown in place of the cover.
 * Synced lines light up one after another, and the words inside a line
 * sweep across as they are sung.
 */
@Composable
fun LyricsPanel(
    lyrics: Lyrics?,
    status: LyricsStatus,
    positionMs: Long,
    accent: Color,
    onSeek: (Long) -> Unit,
    onRetry: () -> Unit,
    modifier: Modifier = Modifier,
    hasArt: Boolean = true,
    playing: Boolean = true,
) {
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(VynlRadius.art))
            .background(VynlColors.bg.copy(alpha = if (hasArt) 0.30f else 0.62f)),
    ) {
        Box(Modifier.fillMaxSize().background(Color(0x660E0E10)))
        when {
            lyrics != null && lyrics.synced -> SyncedLines(lyrics, positionMs, playing, onSeek)
            lyrics != null -> PlainStanzas(lyrics)
            status == LyricsStatus.Loading -> Pending(isLoading = true, onRetry = onRetry)
            status == LyricsStatus.Missing -> Pending(label = "No lyrics found", onRetry = onRetry)
            else -> Pending(label = "Couldn't load lyrics", onRetry = onRetry)
        }
    }
}

@Composable
private fun Pending(isLoading: Boolean = false, label: String = "", onRetry: () -> Unit) {
    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        if (isLoading) {
            CircularProgressIndicator(
                strokeWidth = 2.dp,
                color = VynlColors.dim,
                modifier = Modifier.size(20.dp),
            )
            return
        }
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = label,
                color = VynlColors.dim,
                fontSize = 14.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                text = "Retry",
                color = VynlColors.accent,
                fontSize = 14.sp,
                fontWeight = FontWeight.W700,
                fontFamily = VynlFonts.display,
                modifier = Modifier
                    .clickable(onClick = onRetry)
                    .padding(horizontal = 12.dp, vertical = 8.dp),
            )
        }
    }
}

private val lineShadow = Shadow(color = Color(0x8A000000), offset = Offset.Zero, blurRadius = 6f)

@Composable
private fun SyncedLines(
    lyrics: Lyrics,
    positionMs: Long,
    playing: Boolean,
    onSeek: (Long) -> Unit,
) {
    val lines = lyrics.lines
    val listState = remember(lyrics) { LazyListState() }
    val active = lyrics.activeIndex(positionMs)
    val instrumentals = remember(lyrics) { lines.map { isInstrumental(it.text) } }
    var userScrolling by remember(lyrics) { mutableStateOf(false) }
    val scrolling = listState.isScrollInProgress

    LaunchedEffect(scrolling) {
        if (scrolling) {
            userScrolling = true
        } else {
            delay(3_000)
            userScrolling = false
        }
    }
    LaunchedEffect(active, userScrolling) {
        if (!userScrolling && active >= 0 && active < lines.size) {
            listState.animateScrollToItem(active)
        }
    }

    LazyColumn(
        state = listState,
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 14.dp, top = 84.dp, end = 14.dp, bottom = 84.dp),
    ) {
        itemsIndexed(lines, key = { index, line -> "$index-${line.timeMs}" }) { index, line ->
            val isActive = index == active
            SyncedLineRow(
                line = line,
                isInstrumental = instrumentals[index],
                isActive = isActive,
                positionMs = if (isActive) positionMs else null,
                playing = playing,
                onSeek = onSeek,
            )
        }
    }
}

@Composable
private fun SyncedLineRow(
    line: LyricLine,
    isInstrumental: Boolean,
    isActive: Boolean,
    positionMs: Long?,
    playing: Boolean,
    onSeek: (Long) -> Unit,
) {
    val instrumental = isInstrumental
    val scale by animateFloatAsState(
        targetValue = if (isActive) 1f else 0.98f,
        animationSpec = spring(dampingRatio = 0.82f, stiffness = Spring.StiffnessLow),
        label = "lineScale",
    )
    val color by animateColorAsState(
        targetValue = if (isActive) VynlColors.text else VynlColors.dim,
        animationSpec = spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessLow),
        label = "lineColor",
    )
    val baseStyle = TextStyle(
        fontFamily = VynlFonts.lyrics,
        fontSize = 16.sp,
        lineHeight = 24.sp,
        letterSpacing = if (isActive) (-0.4).sp else (-0.15).sp,
        fontWeight = FontWeight(700),
        color = color,
        shadow = lineShadow,
    )
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (line.timeMs >= 0) Modifier.clickable { onSeek(line.timeMs) } else Modifier)
            .padding(vertical = 10.dp)
            .graphicsLayer {
                scaleX = scale
                scaleY = scale
            },
        contentAlignment = Alignment.Center,
    ) {
        when {
            instrumental -> InstrumentalDots(active = isActive)
            isActive && line.words.isNotEmpty() -> KaraokeLine(
                line = line,
                positionMs = positionMs ?: 0L,
                playing = playing,
                style = baseStyle,
            )
            else -> Text(text = line.text, textAlign = TextAlign.Center, style = baseStyle)
        }
    }
}

private const val KEY_SWEEP = "sweep"

@Composable
private fun KaraokeLine(line: LyricLine, positionMs: Long, playing: Boolean, style: TextStyle) {
    val words = line.words
    var current = -1
    for (i in words.indices) {
        if (words[i].timeMs <= positionMs) current = i else break
    }
    val lit = VynlColors.text
    val unlit = VynlColors.dim
    val measurer = rememberTextMeasurer()
    val density = LocalDensity.current
    val sweepStyle = style.copy(color = VynlColors.dim, fontWeight = FontWeight(700))
    val featherPx = with(LocalDensity.current) { 12.sp.toPx() }
    val activeLayout = remember(words, current, style) {
        if (current < 0) {
            null
        } else {
            measurer.measure(AnnotatedString(words[current].text), sweepStyle)
        }
    }
    val fillState = remember(words, current) {
        mutableFloatStateOf(
            if (current >= 0) wordFill(words, current, positionMs) else 0f,
        )
    }
    LaunchedEffect(words, current, playing, positionMs) {
        if (!playing) {
            fillState.floatValue = if (current >= 0) wordFill(words, current, positionMs) else 0f
            return@LaunchedEffect
        }
        var startNanos = -1L
        val startPos = positionMs
        while (true) {
            val nanos = withFrameNanos { it }
            if (startNanos < 0L) startNanos = nanos
            fillState.floatValue = if (current >= 0) {
                wordFill(words, current, startPos + (nanos - startNanos) / 1_000_000)
            } else {
                0f
            }
        }
    }

    val text = remember(words, current) {
        buildAnnotatedString {
            for (i in words.indices) {
                if (i > 0) append(" ")
                val word = words[i]
                when {
                    i < current -> withStyle(SpanStyle(color = lit, fontWeight = FontWeight(700))) { append(word.text) }
                    i > current || current < 0 -> withStyle(SpanStyle(color = unlit, fontWeight = FontWeight(700))) { append(word.text) }
                    else -> {
                        pushStringAnnotation(KEY_SWEEP, "")
                        append(word.text)
                        pop()
                    }
                }
            }
        }
    }

    val inline = remember(words, current, density) {
        val layout = activeLayout ?: return@remember emptyMap()
        val widthPx = layout.size.width.toFloat()
        val heightPx = layout.size.height.toFloat()
        if (widthPx <= 0f) return@remember emptyMap()
        mapOf(
            KEY_SWEEP to InlineTextContent(
                placeholder = Placeholder(
                    width = with(density) { widthPx.toSp() },
                    height = with(density) { heightPx.toSp() },
                    placeholderVerticalAlign = PlaceholderVerticalAlign.TextCenter,
                ),
            ) {
                Canvas(
                    modifier = Modifier
                        .clipToBounds()
                        .size(with(density) { widthPx.toDp() }, with(density) { heightPx.toDp() }),
                ) {
                    drawText(layout, topLeft = Offset.Zero)
                    val fill = fillState.floatValue.coerceIn(0f, 1f)
                    if (fill > 0f) {
                        val feather = (featherPx / widthPx).coerceIn(0.02f, 0.5f)
                        val edge = fill
                        val brush = Brush.linearGradient(
                            colorStops = arrayOf(
                                0f to lit,
                                (edge - feather).coerceIn(0f, 1f) to lit,
                                edge to lit.copy(alpha = 0f),
                                1f to lit.copy(alpha = 0f),
                            ),
                            start = Offset.Zero,
                            end = Offset(widthPx, 0f),
                        )
                        drawText(
                            layout,
                            brush = brush,
                            topLeft = Offset.Zero,
                            shadow = Shadow(
                                color = lit.copy(alpha = 0.55f),
                                offset = Offset.Zero,
                                blurRadius = 12f,
                            ),
                        )
                    }
                }
            },
        )
    }

    Text(
        text = text,
        inlineContent = inline,
        textAlign = TextAlign.Center,
        style = style.copy(color = VynlColors.dim),
    )
}

@Composable
private fun InstrumentalDots(active: Boolean) {
    val transition = rememberInfiniteTransition(label = "instrumentalDots")
    val shifts: List<State<Float>> = if (active) {
        List(3) { index ->
            transition.animateFloat(
                initialValue = 0f,
                targetValue = 1f,
                animationSpec = infiniteRepeatable(
                    animation = tween(650, delayMillis = index * 140, easing = FastOutSlowInEasing),
                    repeatMode = RepeatMode.Reverse,
                ),
                label = "dot$index",
            )
        }
    } else {
        List(3) { mutableStateOf(0f) }
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        for (i in 0 until 3) {
            val shift = shifts[i].value
            Box(
                modifier = Modifier
                    .padding(horizontal = 5.dp)
                    .offset(y = (-5f * shift).dp)
                    .size(6.dp)
                    .clip(CircleShape)
                    .background(VynlColors.text.copy(alpha = if (active) 0.35f + 0.65f * shift else 0.3f)),
            )
        }
    }
}

@Composable
private fun PlainStanzas(lyrics: Lyrics) {
    val stanzas = remember(lyrics) { stanzasOf(lyrics.text) }
    if (stanzas.isEmpty()) {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text(
                text = "Instrumental",
                color = VynlColors.dim,
                fontSize = 15.sp,
                fontFamily = VynlFonts.display,
            )
        }
        return
    }
    LazyColumn(
        state = remember(lyrics) { LazyListState() },
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 16.dp, top = 24.dp, end = 16.dp, bottom = 28.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        itemsIndexed(stanzas) { index, stanza ->
            Column {
                stanza.forEachIndexed { lineIndex, line ->
                    Text(
                        text = line,
                        style = TextStyle(
                            fontFamily = VynlFonts.lyrics,
                            fontSize = 14.5.sp,
                            lineHeight = 21.sp,
                            letterSpacing = (-0.2).sp,
                            fontWeight = FontWeight(460),
                            color = VynlColors.text,
                            shadow = Shadow(color = Color(0x73000000), offset = Offset.Zero, blurRadius = 5f),
                        ),
                        textAlign = TextAlign.Center,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(bottom = if (lineIndex == stanza.lastIndex) 0.dp else 5.dp),
                    )
                }
            }
        }
    }
}
