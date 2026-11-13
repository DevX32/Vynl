package com.vynl.vynl_mobile.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Repeat
import androidx.compose.material.icons.rounded.RepeatOne
import androidx.compose.material.icons.rounded.Shuffle
import androidx.compose.material.icons.rounded.SkipNext
import androidx.compose.material.icons.rounded.SkipPrevious
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.text.font.FontWeight
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.ArtTile
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.components.VynlSeekBar
import com.vynl.vynl_mobile.ui.components.VynlVolumeBar
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import kotlinx.coroutines.delay

/**
 * The full-screen remote for the desktop, in the style of the player page:
 * the same cover, seek bar, transport and queue, but it drives the desktop's
 * own playback over the local network instead of this phone's.
 */
@Composable
fun RemoteScreen(
    viewModel: AppViewModel,
    onClose: () -> Unit,
) {
    val remote by viewModel.remote.collectAsStateWithLifecycle()

    LaunchedEffect(Unit) {
        while (true) {
            viewModel.refreshRemote()
            delay(1000L)
        }
    }

    BackHandler { onClose() }

    val track = remote?.track
    val shuffle = remote?.shuffle == true
    val loopMode = remote?.loopMode ?: "off"
    val durationMs = ((remote?.duration ?: 0.0) * 1000.0).toLong().coerceAtLeast(1_000L)
    val positionMs = ((remote?.position ?: 0.0) * 1000.0).toLong().coerceIn(0L, durationMs)
    val volume = remote?.volume ?: 75
    val coverPath = remember(track?.id) { track?.let(viewModel::remoteCoverPath) }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg),
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(horizontal = 22.dp),
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = onClose) {
                    Icon(
                        Icons.Rounded.KeyboardArrowDown,
                        contentDescription = "Back to the phone",
                        tint = VynlColors.dim,
                        modifier = Modifier.size(30.dp),
                    )
                }
                Text(
                    text = "DESKTOP",
                    modifier = Modifier.weight(1f),
                    textAlign = TextAlign.Center,
                    color = VynlColors.faint,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.W800,
                    letterSpacing = 1.8.sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.width(48.dp))
            }

            Spacer(Modifier.height(10.dp))

            if (track == null) {
                Spacer(Modifier.height(64.dp))
                ArtTile(
                    coverUrl = null,
                    size = 150.dp,
                    radius = VynlRadius.card,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                )
                Spacer(Modifier.height(22.dp))
                Text(
                    text = if (remote == null) "Looking for the desktop…" else "Nothing playing",
                    color = VynlColors.text,
                    fontSize = 17.sp,
                    fontWeight = FontWeight.W700,
                    letterSpacing = (-0.2).sp,
                    fontFamily = VynlFonts.display,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                )
                Spacer(Modifier.height(8.dp))
                Text(
                    text = "Start something on the desktop and it shows up here.",
                    color = VynlColors.dim,
                    fontSize = 13.sp,
                    lineHeight = 20.sp,
                    textAlign = TextAlign.Center,
                    fontFamily = VynlFonts.display,
                    modifier = Modifier
                        .align(Alignment.CenterHorizontally)
                        .padding(horizontal = 30.dp),
                )
                Spacer(Modifier.height(40.dp))
                return@Column
            }

            ArtTile(
                coverUrl = track.cover?.takeIf { it.startsWith("http") },
                coverPath = coverPath,
                size = 238.dp,
                radius = VynlRadius.card,
                modifier = Modifier.align(Alignment.CenterHorizontally),
            )

            Spacer(Modifier.height(20.dp))

            Text(
                text = track.title,
                color = VynlColors.text,
                fontSize = 20.sp,
                fontWeight = FontWeight.W800,
                letterSpacing = (-0.3).sp,
                lineHeight = 25.sp,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
                fontFamily = VynlFonts.display,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(6.dp))
            Text(
                text = track.artist,
                color = VynlColors.dim,
                fontSize = 13.sp,
                textAlign = TextAlign.Center,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                fontFamily = VynlFonts.display,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(Modifier.height(18.dp))

            VynlSeekBar(
                positionMs = positionMs,
                durationMs = durationMs,
                bufferedMs = durationMs,
                accent = VynlColors.accent,
                onSeek = { viewModel.remoteSeek(it / 1000.0) },
            )

            Spacer(Modifier.height(2.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = { viewModel.remoteShuffle() }) {
                    Icon(
                        Icons.Rounded.Shuffle,
                        contentDescription = if (shuffle) "Shuffle on" else "Shuffle off",
                        tint = if (shuffle) VynlColors.accent else VynlColors.dim,
                        modifier = Modifier.size(22.dp),
                    )
                }
                Spacer(Modifier.width(10.dp))
                VynlControl(
                    onTap = { viewModel.remoteAction("previous") },
                    icon = Icons.Rounded.SkipPrevious,
                    style = VynlControlStyle.plain,
                    size = 42.dp,
                    iconSize = 22.dp,
                )
                Spacer(Modifier.width(8.dp))
                VynlControl(
                    onTap = { viewModel.remoteAction("toggle") },
                    icon = if (remote?.playing == true) Icons.Rounded.Pause else Icons.Rounded.PlayArrow,
                    style = VynlControlStyle.accent,
                    size = 52.dp,
                    iconSize = 29.dp,
                )
                Spacer(Modifier.width(8.dp))
                VynlControl(
                    onTap = { viewModel.remoteAction("next") },
                    icon = Icons.Rounded.SkipNext,
                    style = VynlControlStyle.plain,
                    size = 42.dp,
                    iconSize = 22.dp,
                )
                Spacer(Modifier.width(10.dp))
                IconButton(onClick = { viewModel.remoteRepeat() }) {
                    Icon(
                        if (loopMode == "one") Icons.Rounded.RepeatOne else Icons.Rounded.Repeat,
                        contentDescription = when (loopMode) {
                            "one" -> "Repeat one"
                            "all" -> "Repeat all"
                            else -> "Repeat off"
                        },
                        tint = if (loopMode != "off") VynlColors.accent else VynlColors.dim,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }

            Spacer(Modifier.height(14.dp))

            VynlVolumeBar(
                volume = volume,
                accent = VynlColors.accent,
                onVolumeChange = { viewModel.remoteVolume(it) },
            )

            Spacer(Modifier.height(6.dp))

            Spacer(Modifier.height(24.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "UP NEXT",
                    color = VynlColors.faint,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.W800,
                    letterSpacing = 1.3.sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.width(8.dp))
                Text(
                    text = "${remote?.queue?.size ?: 0}",
                    color = VynlColors.faint,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.W600,
                    fontFamily = VynlFonts.display,
                )
            }
            Spacer(Modifier.height(6.dp))

            val queue = remote?.queue.orEmpty()
            if (queue.isEmpty()) {
                Text(
                    text = "The desktop has nothing queued up.",
                    color = VynlColors.faint,
                    fontSize = 12.5.sp,
                    fontFamily = VynlFonts.display,
                    modifier = Modifier.padding(vertical = 8.dp),
                )
            } else {
                for ((index, item) in queue.take(30).withIndex()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(VynlRadius.control))
                            .clickable { viewModel.remotePlayUpcoming(index) }
                            .padding(horizontal = 2.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "${index + 1}",
                            color = VynlColors.faint,
                            fontSize = 12.sp,
                            fontFamily = VynlFonts.display,
                            modifier = Modifier.width(26.dp),
                        )
                        Column(Modifier.weight(1f)) {
                            Text(
                                text = item.title,
                                color = VynlColors.text,
                                fontSize = 12.5.sp,
                                fontWeight = FontWeight.W600,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                fontFamily = VynlFonts.display,
                            )
                            Text(
                                text = item.artist,
                                color = VynlColors.faint,
                                fontSize = 11.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                fontFamily = VynlFonts.display,
                            )
                        }
                        Spacer(Modifier.width(8.dp))
                        Icon(
                            Icons.Rounded.PlayArrow,
                            contentDescription = "Play this",
                            tint = VynlColors.accent,
                            modifier = Modifier.size(19.dp),
                        )
                    }
                }
            }

            Spacer(Modifier.height(32.dp))
        }
    }
}
