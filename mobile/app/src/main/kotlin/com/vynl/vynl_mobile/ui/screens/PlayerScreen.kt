package com.vynl.vynl_mobile.ui.screens

import android.os.Build
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.QueueMusic
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Repeat
import androidx.compose.material.icons.rounded.RepeatOne
import androidx.compose.material.icons.rounded.SkipNext
import androidx.compose.material.icons.rounded.SkipPrevious
import androidx.compose.material.icons.rounded.Shuffle
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.vynl.vynl_mobile.playback.PlaybackState
import com.vynl.vynl_mobile.playback.RepeatMode
import com.vynl.vynl_mobile.ui.components.TrackTile
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.animation.core.tween
import androidx.compose.ui.zIndex
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.rounded.DragIndicator
import com.vynl.vynl_mobile.ui.components.LyricsPanel
import com.vynl.vynl_mobile.ui.components.VynlSeekBar
import com.vynl.vynl_mobile.data.Lyrics
import com.vynl.vynl_mobile.data.LyricsStatus

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
fun PlayerScreen(
    playback: PlaybackState,
    accent: Color,
    queueTracks: List<com.vynl.vynl_mobile.data.CatalogueTrack>,
    onClose: () -> Unit,
    onToggle: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
    onSeek: (Long) -> Unit,
    onRetryLyrics: () -> Unit,
    onReorder: (Int, Int) -> Unit,
    onPlayAt: (Int) -> Unit,
    onToggleShuffle: () -> Unit,
    onToggleRepeat: () -> Unit,
    lyrics: Lyrics?,
    lyricsStatus: LyricsStatus,
    lyricsAvailable: Boolean,
    lyricsOpen: Boolean,
    onToggleLyrics: () -> Unit,
) {
    val track = playback.track
    var showQueue by remember { mutableStateOf(false) }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg),
    ) {
        ArtBackdrop(
            coverUrl = track?.cover,
            lyricsOn = lyricsOpen,
            modifier = Modifier.fillMaxSize(),
        )

        BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
            val width = maxWidth
            val height = maxHeight
            val artSize = if (width < height) width - 72.dp else height * 0.42f

            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .statusBarsPadding()
                    .navigationBarsPadding()
                    .padding(horizontal = 26.dp)
                    .pointerInput(Unit) {
                        detectTapGestures(onDoubleTap = { onToggleLyrics() })
                    },
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    IconButton(onClick = onClose) {
                        Icon(
                            Icons.Rounded.KeyboardArrowDown,
                            contentDescription = "Close the player",
                            tint = VynlColors.dim,
                            modifier = Modifier.size(30.dp),
                        )
                    }
                    Text(
                        text = if (playback.queueSize > 0) "${playback.queueIndex + 1} / ${playback.queueSize}" else "â€” / â€”",
                        modifier = Modifier.weight(1f),
                        textAlign = TextAlign.Center,
                        color = VynlColors.dim,
                        fontSize = 12.sp,
                        fontWeight = FontWeight.W700,
                        letterSpacing = 1.4.sp,
                        fontFamily = VynlFonts.display,
                    )
                    IconButton(onClick = { showQueue = true }) {
                        Icon(
                            Icons.Rounded.QueueMusic,
                            contentDescription = "Queue",
                            tint = VynlColors.dim,
                            modifier = Modifier.size(26.dp),
                        )
                    }
                }

                Spacer(Modifier.weight(1f))

                Box(
                    modifier = Modifier.fillMaxWidth(),
                    contentAlignment = Alignment.Center,
                ) {
                    AnimatedContent(
                        targetState = lyricsOpen,
                        transitionSpec = {
                            (fadeIn(tween(320, easing = FastOutSlowInEasing)) +
                                scaleIn(initialScale = 0.96f, animationSpec = spring(dampingRatio = 0.85f, stiffness = Spring.StiffnessLow)))
                                .togetherWith(
                                    fadeOut(tween(230)) +
                                        scaleOut(targetScale = 0.96f, animationSpec = spring(stiffness = Spring.StiffnessMediumLow)),
                                )
                        },
                        contentAlignment = Alignment.Center,
                        label = "artworkLyrics",
                    ) { toggled ->
                        if (toggled) {
                            LyricsPanel(
                                lyrics = lyrics,
                                status = lyricsStatus,
                                positionMs = playback.positionMs,
                                accent = accent,
                                onSeek = onSeek,
                                onRetry = onRetryLyrics,
                                modifier = Modifier.size(artSize),
                                hasArt = track?.cover != null,
                                playing = playback.playing,
                            )
                        } else {
                            AsyncImage(
                                model = track?.cover,
                                contentDescription = null,
                                contentScale = ContentScale.Crop,
                                modifier = Modifier
                                    .size(artSize)
                                    .clip(RoundedCornerShape(VynlRadius.art))
                                    .graphicsLayer { shadowElevation = 24.dp.toPx() },
                            )
                        }
                    }
                }

                Spacer(Modifier.weight(1f))

                Text(
                    text = track?.title ?: "Nothing playing",
                    modifier = Modifier.fillMaxWidth(),
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    textAlign = TextAlign.Center,
                    color = VynlColors.text,
                    fontSize = 23.sp,
                    fontWeight = FontWeight.W800,
                    lineHeight = 27.sp,
                    letterSpacing = (-0.5).sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.height(7.dp))
                Text(
                    text = listOfNotNull(track?.artist, track?.album?.takeIf { it.isNotBlank() })
                        .joinToString("  Â·  "),
                    modifier = Modifier.fillMaxWidth(),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    textAlign = TextAlign.Center,
                    color = VynlColors.dim,
                    fontSize = 13.5.sp,
                    fontFamily = VynlFonts.display,
                )

                Spacer(Modifier.height(26.dp))

                VynlSeekBar(
                    positionMs = playback.positionMs,
                    durationMs = playback.durationMs,
                    bufferedMs = playback.bufferedMs,
                    accent = accent,
                    onSeek = onSeek,
                )

                Spacer(Modifier.height(14.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Spacer(Modifier.weight(1f))
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(18.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VynlControl(
                            onTap = onPrevious,
                            icon = Icons.Rounded.SkipPrevious,
                            style = VynlControlStyle.raised,
                            size = 42.dp,
                            iconSize = 25.dp,
                        )
                        VynlControl(
                            onTap = onToggle,
                            icon = if (playback.playing) Icons.Rounded.Pause else Icons.Rounded.PlayArrow,
                            style = VynlControlStyle.accent,
                            color = accent,
                            size = 52.dp,
                            iconSize = 30.dp,
                        )
                        VynlControl(
                            onTap = onNext,
                            icon = Icons.Rounded.SkipNext,
                            style = VynlControlStyle.raised,
                            size = 42.dp,
                            iconSize = 25.dp,
                        )
                    }
                    Spacer(Modifier.weight(1f))
                }

                Spacer(Modifier.height(18.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    IconButton(onClick = onToggleShuffle) {
                        Icon(
                            Icons.Rounded.Shuffle,
                            contentDescription = "Shuffle",
                            tint = if (playback.shuffle) accent else VynlColors.dim,
                            modifier = Modifier.size(24.dp),
                        )
                    }
                    IconButton(onClick = onToggleRepeat) {
                        Icon(
                            when (playback.repeatMode) {
                                RepeatMode.One -> Icons.Rounded.RepeatOne
                                else -> Icons.Rounded.Repeat
                            },
                            contentDescription = "Repeat",
                            tint = if (playback.repeatMode != RepeatMode.Off) accent else VynlColors.dim,
                            modifier = Modifier.size(24.dp),
                        )
                    }
                }

                Spacer(Modifier.weight(1f))

                if (lyricsAvailable && !lyricsOpen) {
                    Text(
                        text = "DOUBLE TAP FOR LYRICS",
                        color = VynlColors.faint.copy(alpha = 0.6f),
                        fontSize = 10.sp,
                        fontWeight = FontWeight.W700,
                        letterSpacing = 1.4.sp,
                        fontFamily = VynlFonts.display,
                        modifier = Modifier.padding(bottom = 8.dp),
                    )
                }
            }
        }
    }

    if (showQueue) {
        ModalBottomSheet(
            onDismissRequest = { showQueue = false },
            containerColor = VynlColors.surface,
            shape = RoundedCornerShape(topStart = VynlRadius.control, topEnd = VynlRadius.control),
        ) {
            val itemHeight = 68.dp
            val size = queueTracks.size
            val density = androidx.compose.ui.platform.LocalDensity.current
            val itemHeightPx = with(density) { itemHeight.toPx() }
            var dragFrom by remember { mutableStateOf(-1) }
            var dragOffset by remember { mutableStateOf(0f) }
            val dragTarget = if (dragFrom >= 0) {
                (dragFrom + (dragOffset / itemHeightPx)).toInt().coerceIn(0, size - 1)
            } else {
                -1
            }
            Column(modifier = Modifier.padding(bottom = 28.dp)) {
                Text(
                    text = "UP NEXT",
                    color = VynlColors.faint,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.W800,
                    letterSpacing = 1.3.sp,
                    fontFamily = VynlFonts.display,
                    textAlign = TextAlign.Center,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 16.dp, bottom = 6.dp),
                )
                LazyColumn(modifier = Modifier.height(360.dp)) {
                    if (queueTracks.isEmpty()) {
                        item {
                            Text(
                                text = "Nothing in queue",
                                color = VynlColors.faint,
                                fontSize = 13.5.sp,
                                fontFamily = VynlFonts.display,
                                modifier = Modifier.padding(start = 22.dp, top = 10.dp),
                            )
                        }
                        return@LazyColumn
                    }
                    itemsIndexed(queueTracks, key = { index, item -> "$index-${item.id}" }) { index, track ->
                        val isDragged = index == dragFrom
                        val slide by animateFloatAsState(
                            targetValue = when {
                                dragFrom < 0 || isDragged -> 0f
                                dragTarget > dragFrom && index in (dragFrom + 1)..dragTarget -> -itemHeightPx
                                dragTarget < dragFrom && index in dragTarget until dragFrom -> itemHeightPx
                                else -> 0f
                            },
                            animationSpec = tween(150, easing = FastOutSlowInEasing),
                            label = "queueSlide",
                        )
                        val clampMin = -dragFrom * itemHeightPx
                        val clampMax = (size - 1 - dragFrom) * itemHeightPx
                        Row(
                            modifier = Modifier
                                .height(itemHeight)
                                .fillMaxWidth()
                                .graphicsLayer {
                                    translationY = if (isDragged) dragOffset.coerceIn(clampMin, clampMax) else slide
                                }
                                .zIndex(if (isDragged) 1f else 0f)
                                .then(
                                    if (isDragged) {
                                        Modifier.background(VynlColors.surfaceRaised, RoundedCornerShape(VynlRadius.tile))
                                    } else {
                                        Modifier
                                    },
                                ),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Box(modifier = Modifier.weight(1f)) {
                                TrackTile(
                                    track = track,
                                    active = index == playback.queueIndex,
                                    coverPath = null,
                                    showStatus = false,
                                    coverSize = 44.dp,
                                    onTap = {
                                        if (dragFrom >= 0) return@TrackTile
                                        onPlayAt(index)
                                        showQueue = false
                                    },
                                )
                            }
                            Icon(
                                Icons.Rounded.DragIndicator,
                                contentDescription = "Drag to reorder",
                                tint = VynlColors.faint,
                                modifier = Modifier
                                    .padding(horizontal = 6.dp)
                                    .height(44.dp)
                                    .width(40.dp)
                                    .pointerInput(index, size) {
                                        detectVerticalDragGestures(
                                            onDragStart = {
                                                dragFrom = index
                                                dragOffset = 0f
                                            },
                                            onDragEnd = {
                                                val from = dragFrom
                                                if (from >= 0) {
                                                    val target = (from + dragOffset / itemHeightPx)
                                                        .toInt()
                                                        .coerceIn(0, size - 1)
                                                    if (target != from) onReorder(from, target)
                                                }
                                                dragFrom = -1
                                                dragOffset = 0f
                                            },
                                            onDragCancel = {
                                                dragFrom = -1
                                                dragOffset = 0f
                                            },
                                            onVerticalDrag = { change, amount ->
                                                change.consume()
                                                dragOffset += amount
                                            },
                                        )
                                    },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ArtBackdrop(coverUrl: String?, lyricsOn: Boolean, modifier: Modifier = Modifier) {
    val blurRadius = if (lyricsOn) 120.dp else 72.dp
    val baseDim = if (lyricsOn) 0.42f else 0.30f
    Box(modifier = modifier) {
        if (!coverUrl.isNullOrEmpty()) {
            AsyncImage(
                model = coverUrl,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer { scaleX = 1.35f; scaleY = 1.35f }
                    .then(
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                            Modifier.blur(blurRadius)
                        } else {
                            Modifier
                        },
                    )
                    .drawWithContent {
                        drawContent()
                        drawRect(Color(0xFF0E0E10).copy(alpha = baseDim))
                        drawRect(
                            brush = Brush.radialGradient(
                                colors = listOf(Color.Transparent, Color(0xFF0E0E10).copy(alpha = 0.88f)),
                                center = center,
                                radius = size.minDimension * 0.95f,
                            ),
                        )
                    },
            )
        }
    }
}
