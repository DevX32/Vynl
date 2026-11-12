package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.Lyrics
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.SkipNext
import androidx.compose.material.icons.rounded.SkipPrevious
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.playback.PlaybackState
import com.vynl.vynl_mobile.ui.components.ArtTile
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@Composable
fun PlayerScreen(
    playback: PlaybackState,
    onClose: () -> Unit,
    onToggle: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
    onSeek: (Long) -> Unit,
) {
    val track = playback.track
    var dragging by remember { mutableStateOf(false) }
    var dragValue by remember { mutableFloatStateOf(0f) }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg),
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 24.dp),
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
                        modifier = Modifier.size(26.dp),
                    )
                }
                Text(
                    text = "NOW PLAYING",
                    modifier = Modifier.weight(1f),
                    textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    color = VynlColors.faint,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.W800,
                    letterSpacing = 1.3.sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.width(48.dp))
            }

            Spacer(Modifier.weight(0.6f))

            ArtTile(
                coverUrl = track?.cover,
                size = 300.dp,
                radius = VynlRadius.hero,
                modifier = Modifier
                    .size(300.dp)
                    .clip(RoundedCornerShape(VynlRadius.hero)),
            )

            Spacer(Modifier.height(28.dp))

            Text(
                text = track?.title ?: "Nothing playing",
                modifier = Modifier.fillMaxWidth(),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.text,
                fontSize = 20.sp,
                fontWeight = FontWeight.W800,
                letterSpacing = (-0.4).sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = track?.artist.orEmpty(),
                modifier = Modifier.fillMaxWidth(),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.dim,
                fontSize = 13.5.sp,
                fontFamily = VynlFonts.display,
            )

            Spacer(Modifier.height(20.dp))

            val duration = playback.durationMs.coerceAtLeast(1L)
            val position = if (dragging) dragValue.toLong() else playback.positionMs
            Slider(
                value = position.toFloat().coerceIn(0f, duration.toFloat()),
                onValueChange = {
                    dragging = true
                    dragValue = it
                },
                onValueChangeFinished = {
                    dragging = false
                    onSeek(dragValue.toLong())
                },
                colors = SliderDefaults.colors(
                    thumbColor = VynlColors.text,
                    activeTrackColor = VynlColors.text,
                    inactiveTrackColor = VynlColors.line,
                ),
            )
            Row(modifier = Modifier.fillMaxWidth()) {
                Text(
                    text = formatClock(position),
                    color = VynlColors.faint,
                    fontSize = 11.5.sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.weight(1f))
                Text(
                    text = formatClock(playback.durationMs),
                    color = VynlColors.faint,
                    fontSize = 11.5.sp,
                    fontFamily = VynlFonts.display,
                )
            }

            Spacer(Modifier.height(18.dp))

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(22.dp),
            ) {
                VynlControl(
                    onTap = onPrevious,
                    icon = Icons.Rounded.SkipPrevious,
                    style = VynlControlStyle.plain,
                    size = 44.dp,
                    iconSize = 32.dp,
                )
                VynlControl(
                    onTap = onToggle,
                    icon = if (playback.playing) Icons.Rounded.Pause else Icons.Rounded.PlayArrow,
                    style = VynlControlStyle.accent,
                    color = VynlColors.accent,
                    size = 62.dp,
                    iconSize = 34.dp,
                )
                VynlControl(
                    onTap = onNext,
                    icon = Icons.Rounded.SkipNext,
                    style = VynlControlStyle.plain,
                    size = 44.dp,
                    iconSize = 32.dp,
                )
            }

            Spacer(Modifier.weight(0.6f))

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(22.dp),
            ) {
                IconButton(onClick = { }) {
                    Icon(
                        Icons.Rounded.Lyrics,
                        contentDescription = "Lyrics",
                        tint = VynlColors.dim,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }
        }
    }
}

private fun formatClock(milliseconds: Long): String {
    val total = (milliseconds / 1000).coerceAtLeast(0)
    val minutes = (total / 60) % 60
    val seconds = total % 60
    return "$minutes:${seconds.toString().padStart(2, '0')}"
}
